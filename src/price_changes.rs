use crate::coingecko::{self, GetPriceError};
use chrono::{Duration, DurationRound, TimeZone, Utc};
use lru::LruCache;
use std::{convert::TryInto, hash::Hash, sync::Arc};
use tokio::sync::Mutex;

#[derive(Eq, Hash, PartialEq)]
struct HistoricPriceKey {
    base: String,
    id: String,
    timestamp: i64,
}

pub type HistoricPriceCache = Arc<Mutex<LruCache<String, f64>>>;

fn key_from_historic_price_target(id: &str, base: &str, timestamp: &i64) -> String {
    format!("{}-{}-{}", id, base, timestamp)
}

fn timestamp_from_days_ago(days_ago: &u32) -> i64 {
    let start_of_today = Utc::now().duration_trunc(Duration::days(1)).unwrap();
    let days_ago_i64 = (*days_ago).try_into().unwrap();
    (start_of_today - Duration::days(days_ago_i64)).timestamp()
}

pub async fn get_historic_price_with_cache(
    client: &reqwest::Client,
    historic_price_cache: HistoricPriceCache,
    id: &str,
    base: &str,
    days_ago: &u32,
) -> Result<f64, GetPriceError> {
    let timestamp = timestamp_from_days_ago(&days_ago);

    let key = key_from_historic_price_target(id, base, &timestamp);

    let mut historic_price_cache = historic_price_cache.lock().await;

    let historic_price = match historic_price_cache.get(&key) {
        None => {
            let prices = coingecko::get_market_chart(id, base, days_ago).await?;
            for (ms_timestamp, price) in &prices {
                let timestamp = Utc.timestamp_millis(*ms_timestamp);
                let key = key_from_historic_price_target(id, base, &timestamp.timestamp());
                historic_price_cache.put(key, *price);
            }

            prices.first().unwrap().1
        }
        Some(price) => *price,
    };

    let current_price = coingecko::get_price(client, id, base).await?;

    Ok(current_price / historic_price - 1f64)
}
