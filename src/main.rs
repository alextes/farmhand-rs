mod coingecko;
// mod price_change;
// mod request;

use axum::{
    extract::Path,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use coingecko::{get_symbol_id_map, GetPriceError};
use log::error;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;

// #[derive(Debug)]
// struct State {
//     name: String,
// }

type Base = String;

#[derive(Debug, Deserialize)]
struct PriceBody {
    base: Base,
}

// #[derive(Deserialize)]
// struct PriceChangeBody {
//     base: Base,
// }

async fn handle_get_coin_price(
    Path(coin): Path<String>,
    Json(payload): Json<PriceBody>,
) -> Result<impl IntoResponse, StatusCode> {
    let symbol_id_map = get_symbol_id_map().await.map_err(|e| {
        error!("failed to get symbol id map, {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let id = symbol_id_map.get(&coin).map_or_else(
        || Err(StatusCode::NOT_FOUND),
        |ids| Ok(ids.first().unwrap()),
    )?;

    match coingecko::get_price(id.to_string(), payload.base).await {
        Err(GetPriceError::PriceNotFound) => Err(StatusCode::NOT_FOUND),
        Err(GetPriceError::ReqwestError(e)) => {
            error!("failed to get price, {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        Ok(price) => Ok((StatusCode::OK, Json(json!({ "price": price })))),
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    // let shared_state = Arc::new(State {
    //     name: "alex".to_string(),
    // });

    let app = Router::new().route("/coin/:coin/price", post(handle_get_coin_price));
    // .layer(Extension(shared_state));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
