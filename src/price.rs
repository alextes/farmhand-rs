use crate::ServerState;
use crate::{id, request};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::time::Duration;
use tide::prelude::*;
use tide::{Request, Response, StatusCode};

#[derive(Clone, Deserialize, Serialize, Debug)]
struct Coin {
    symbol: String,
    current_price: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MultiPrice {
    usd: f64,
    btc: f64,
    eth: f64,
}

#[derive(Debug)]
struct LookupError {
    details: String,
}

impl LookupError {
    fn new(msg: String) -> LookupError {
        LookupError { details: msg }
    }
}

impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for LookupError {
    fn description(&self) -> &str {
        &self.details
    }
}

async fn fetch_multi_price(ids: Vec<&String>) -> reqwest::Result<MultiPrice> {
    let prices: HashMap<String, MultiPrice> =
        request::get_json(Duration::from_secs(5), &uri).await?;

    prices
        .get(id)
        .ok_or(tide::Error::new(
            StatusCode::NotFound,
            LookupError::new(format!("no price for id: {}", id)),
        ))
        .map(|r| r.to_owned())
}

// pub async fn handle_get_price(req: Request<ServerState>) -> tide::Result {
//     let symbol = req.param("symbol")?;

//     let id_map = id::get_coingecko_id_map().await?;

//     // TODO: pick the token with the highest market cap
//     let m_id = id_map.get(symbol).and_then(|ids| ids.first());
//     let id = match m_id {
//         Some(id) => id,
//         None => {
//             return Ok(Response::builder(StatusCode::NotFound)
//                 .body(format!("no coingecko symbol found for {}", symbol))
//                 .build())
//         }
//     };

//     fetch_multi_price(id).await.map_or_else(
//         |err| {
//             if err.status() == StatusCode::TooManyRequests {
//                 Ok(Response::new(StatusCode::TooManyRequests))
//             } else {
//                 Err(err)
//             }
//         },
//         |prices| {
//             Ok(Response::builder(StatusCode::Ok)
//                 .body(json!(prices))
//                 .build())
//         },
//     )
// }
