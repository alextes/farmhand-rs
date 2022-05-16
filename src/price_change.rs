use crate::{base::Base, request};
use crate::{id, ServerState};
use async_std::sync::{Arc, Mutex, MutexGuardArc};
use chrono::{Duration, DurationRound, Utc};
use lru::LruCache;
use serde::Deserialize;
use std::convert::TryInto;
use tide::prelude::*;
use tide::{Request, Response, StatusCode};

pub type HistoricPriceCache = Arc<Mutex<LruCache<String, f64>>>;
pub type HistoricPriceCacheL = MutexGuardArc<LruCache<String, f64>>;

/// Timestamp in miliseconds
type MsTimestamp = i64;
/// A price for a cryptocurrency
type Price = f64;
type PriceInTime = (MsTimestamp, Price);
type NumberInTime = (MsTimestamp, Price);

#[derive(Clone, Debug, Deserialize)]
/// CoinGecko price history for a cryptocurrency
struct History {
    prices: Vec<PriceInTime>,
    market_caps: Vec<NumberInTime>,
    total_volumes: Vec<NumberInTime>,
}

async fn get_historic_price(
    historic_price_cache: HistoricPriceCache,
    id: &String,
    base: &Base,
    days_ago: &i32,
) -> surf::Result<f64> {
    let start_of_today = Utc::now().duration_trunc(Duration::days(1)).unwrap();
    let days_ago_i64 = (*days_ago).try_into().unwrap();
    let target_timestamp = (start_of_today - Duration::days(days_ago_i64)).timestamp();
    let key = format!("{}-{}-{}", target_timestamp, id, base);
    let mut historic_price_cache: HistoricPriceCacheL = historic_price_cache.lock_arc().await;
    let m_historic_price = historic_price_cache.get(&key);
    if m_historic_price.is_some() {
        return Ok(m_historic_price.unwrap().clone());
    }

    // CoinGecko uses 'days' as today up to but excluding n 'days' ago, we want
    // including so we add 1 here.
    let coingecko_days_ago = days_ago + 1;
    let uri = format!("https://api.coingecko.com/api/v3/coins/{id}/market_chart?vs_currency={base}&days={coingecko_days_ago}&interval=daily", id=id, base=base, coingecko_days_ago=coingecko_days_ago);
    let history: History = request::get_json(std::time::Duration::from_secs(5), &uri).await?;

    for (ms_timestamp, price) in &history.prices {
        // to unix time
        let timestamp = ms_timestamp / 1000;
        let historic_price_key = format!("{}-{}-{}", timestamp, id, base);
        historic_price_cache.put(historic_price_key, price.to_owned());
    }

    let (_, price) = history.prices.first().unwrap().to_owned();

    Ok(price)
}

async fn get_price_change(
    historic_price_cache: HistoricPriceCache,
    id: &String,
    base: &Base,
    days_ago: &i32,
) -> surf::Result<f64> {
    let historic_price =
        get_historic_price(historic_price_cache.clone(), id, base, days_ago).await?;

    let m_today_timestamp = Utc::now()
        .duration_trunc(Duration::days(1))
        .map(|dt| dt.timestamp());
    let today_timestamp = match m_today_timestamp {
        Ok(dt) => dt,
        Err(err) => panic!("{}", err),
    };
    let mut _guard: MutexGuardArc<LruCache<String, f64>> = historic_price_cache.lock_arc().await;
    let key = format!("{}-{}-{}", today_timestamp, id, base);
    let m_today_price = (*_guard).get(&key);

    return match m_today_price {
        Some(price) => Ok(price.to_owned() / historic_price - 1.0),
        None => get_historic_price(historic_price_cache, id, base, days_ago).await,
    };
}

pub async fn handle_get_price_change(mut req: Request<ServerState>) -> tide::Result {
    let Body { base, days_ago } = req.body_json().await?;
    let symbol = req.param("symbol").unwrap();

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Body {
        base: Base,
        days_ago: i32,
    }

    let id_map = id::get_coingecko_id_map().await?;

    // TODO: pick the token with the highest market cap
    let m_id = id_map.get(symbol.clone()).and_then(|ids| ids.first());
    let id = match m_id {
        Some(id) => id,
        None => {
            return Ok(Response::builder(StatusCode::NotFound)
                .body(format!("no coingecko symbol found for {}", symbol))
                .build())
        }
    };

    let cache = req.state().historic_price_cache.clone();
    let m_historic_prices = get_price_change(cache, id, &base, &days_ago).await;
    m_historic_prices.map_or_else(
        |err| {
            if err.status() == StatusCode::TooManyRequests {
                Ok(Response::new(StatusCode::TooManyRequests))
            } else {
                Err(err)
            }
        },
        |historic_prices| {
            Ok(Response::builder(StatusCode::Ok)
                .body(json!(historic_prices))
                .build())
        },
    )
}
