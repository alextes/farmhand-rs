mod coingecko;
mod config;
mod price_changes;

use axum::{extract::Path, response::IntoResponse, routing::post, Extension, Json, Router};
use coingecko::GetIdFromSymbolError;
use lru::LruCache;
use price_changes::HistoricPriceCache;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
struct State {
    coingecko_client: reqwest::Client,
    historic_price_cache: HistoricPriceCache,
}

type Base = String;

#[derive(Debug, Deserialize)]
struct PriceBody {
    base: Base,
}

async fn handle_get_coin_price(
    Path(coin): Path<String>,
    Json(payload): Json<PriceBody>,
    Extension(state): Extension<State>,
) -> Result<impl IntoResponse, StatusCode> {
    log::debug!(
        "get coin price for symbol {}, in base {}",
        coin,
        payload.base
    );

    let id = coingecko::get_id_from_symbol(&state.coingecko_client, &coin)
        .await
        .map_err(|e| match e {
            GetIdFromSymbolError::SymbolNotFound => {
                log::warn!("no coingecko id for symbol {}", coin);
                StatusCode::NOT_FOUND
            }
            GetIdFromSymbolError::ReqwestError(error) => {
                log::error!("failed to get id, {}", error);
                error
                    .status()
                    .unwrap_or_else(|| StatusCode::INTERNAL_SERVER_ERROR)
            }
        })?;

    log::debug!("found id {}, for symbol {}", &id, &coin);

    let price = coingecko::get_price(&state.coingecko_client, &id, &payload.base).await?;

    log::debug!("found price {}, for symbol {}", &price, &coin);

    Ok(Json(json!({ "price": price })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PriceChangeBody {
    base: Base,
    days_ago: u32,
}

async fn handle_get_coin_price_change(
    Path(coin): Path<String>,
    Json(payload): Json<PriceChangeBody>,
    Extension(state): Extension<State>,
) -> Result<impl IntoResponse, StatusCode> {
    log::debug!(
        "get coin price change for {}, in {}, since {} days ago",
        &coin,
        &payload.base,
        &payload.days_ago
    );

    let id = coingecko::get_id_from_symbol(&state.coingecko_client, &coin)
        .await
        .map_err(|error| match error {
            GetIdFromSymbolError::SymbolNotFound => {
                log::warn!("no coingecko id for symbol {}", coin);
                StatusCode::NOT_FOUND
            }
            GetIdFromSymbolError::ReqwestError(error) => {
                log::error!("failed to get price, {}", error);
                error
                    .status()
                    .map_or_else(|| StatusCode::INTERNAL_SERVER_ERROR, |status| status)
            }
        })?;

    let price_change = price_changes::get_historic_price_with_cache(
        &state.coingecko_client,
        state.historic_price_cache,
        &id,
        &payload.base,
        &payload.days_ago,
    )
    .await?;

    Ok(Json(json!({ "priceChange": price_change })))
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let config = config::Config::new();

    let coingecko_client = reqwest::Client::new();

    let shared_state = State {
        coingecko_client,
        historic_price_cache: Arc::new(Mutex::new(LruCache::new(10_000))),
    };

    let app = Router::new()
        .route("/coin/:coin/price", post(handle_get_coin_price))
        .route(
            "/coin/:coin/price-change",
            post(handle_get_coin_price_change),
        )
        .layer(Extension(shared_state));

    let address = format!("0.0.0.0:{}", config.port)
        .parse()
        .expect("address with port is not a valid address");

    log::info!("listening on port {}", config.port);

    let server = axum::Server::bind(&address).serve(app.into_make_service());

    if let Err(err) = server.await {
        eprintln!("server error: {}", err)
    }
}
