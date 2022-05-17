mod coingecko;
mod price_changes;
mod prices;

use axum::{extract::Path, response::IntoResponse, routing::post, Extension, Json, Router};
use axum_macros::debug_handler;
use coingecko::GetIdFromSymbolError;
use log::{error, warn};
use lru::LruCache;
use price_changes::HistoricPriceCache;
use prices::GetPriceError;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
struct State {
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
) -> Result<impl IntoResponse, StatusCode> {
    let id = coingecko::get_id_from_symbol(&coin)
        .await
        .map_err(|e| match e {
            GetIdFromSymbolError::SymbolNotFound => {
                warn!("no coingecko id for symbol {}", coin);
                StatusCode::NOT_FOUND
            }
            GetIdFromSymbolError::ReqwestError(error) => {
                error!("failed to get id, {}", error);
                error
                    .status()
                    .map_or_else(|| StatusCode::INTERNAL_SERVER_ERROR, |status| status)
            }
        })?;

    let price = prices::get_price(id, payload.base)
        .await
        .map_err(|e| match e {
            GetPriceError::PriceNotFound => StatusCode::NOT_FOUND,
            GetPriceError::ReqwestError(error) => {
                error!("failed to get price, {}", error);
                error
                    .status()
                    .map_or_else(|| StatusCode::INTERNAL_SERVER_ERROR, |status| status)
            }
        })?;

    Ok(Json(json!({ "price": price })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PriceChangeBody {
    base: Base,
    days_ago: u32,
}

#[debug_handler]
async fn handle_get_coin_price_change(
    Path(coin): Path<String>,
    Json(payload): Json<PriceChangeBody>,
    Extension(state): Extension<State>,
) -> Result<impl IntoResponse, StatusCode> {
    let id = coingecko::get_id_from_symbol(&coin)
        .await
        .map_err(|error| match error {
            GetIdFromSymbolError::SymbolNotFound => {
                warn!("no coingecko id for symbol {}", coin);
                StatusCode::NOT_FOUND
            }
            GetIdFromSymbolError::ReqwestError(error) => {
                error!("failed to get price, {}", error);
                error
                    .status()
                    .map_or_else(|| StatusCode::INTERNAL_SERVER_ERROR, |status| status)
            }
        })?;

    price_changes::get_historic_price_with_cache(
        state.historic_price_cache,
        &id,
        &payload.base,
        &payload.days_ago,
    )
    .await
    .map_err(|error| match error {
        GetPriceError::PriceNotFound => StatusCode::NOT_FOUND,
        GetPriceError::ReqwestError(error) => {
            error!("failed to get price, {}", error);
            error
                .status()
                .map_or_else(|| StatusCode::INTERNAL_SERVER_ERROR, |status| status)
        }
    })
    .map(|price_change| Json(json!({ "priceChange": price_change })))
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let shared_state = State {
        historic_price_cache: Arc::new(Mutex::new(LruCache::new(10_000))),
    };

    let app = Router::new()
        .route("/coin/:coin/price", post(handle_get_coin_price))
        .route(
            "/coin/:coin/price-change",
            post(handle_get_coin_price_change),
        )
        .layer(Extension(shared_state));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
