use std::collections::HashMap;

use cached::proc_macro::cached;
use phf::phf_map;
use serde::Deserialize;

const ID_URL: &str = "https://api.coingecko.com/api/v3/coins/list";

#[derive(Clone, Debug, Deserialize)]
pub struct CoinId {
    pub id: String,
    pub symbol: String,
    pub name: String,
}

pub async fn get_coin_list() -> reqwest::Result<Vec<CoinId>> {
    let coin_list = reqwest::get(ID_URL).await?.json::<Vec<CoinId>>().await?;
    Ok(coin_list)
}

// TODO: getIdMapSortedByMarketCap

const ID_OVERRIDES_MAP: phf::Map<&'static str, &'static str> = phf_map! {
  "boo" => "spookyswap",
  "comp" => "compound-governance-token",
  "ftt" => "ftx-token",
  "time" => "wonderland",
  "uni" => "uniswap",
};

type IdMap = HashMap<String, Vec<String>>;

#[cached(time = 14400, result = true, sync_writes = true)]
pub async fn get_symbol_id_map() -> reqwest::Result<IdMap> {
    let coin_ids = get_coin_list().await?;
    let mut symbol_id_map = HashMap::new();

    for coin in coin_ids {
        symbol_id_map
            .entry(coin.symbol)
            .or_insert(Vec::new())
            .push(coin.id)
    }

    // Some symbols have multiple IDs, we don't support retrieving the map sorted by highest market
    // cap yet, or by contract, so we hardcode some overwrites that are probably returning the
    // token the caller is looking for instead of CoinGecko's default.
    for (&key, &value) in ID_OVERRIDES_MAP.into_iter() {
        symbol_id_map.insert(key.to_string(), vec![value.to_string()]);
    }

    Ok(symbol_id_map)
}

pub enum GetIdFromSymbolError {
    SymbolNotFound,
    ReqwestError(reqwest::Error),
}

impl From<reqwest::Error> for GetIdFromSymbolError {
    fn from(error: reqwest::Error) -> Self {
        GetIdFromSymbolError::ReqwestError(error)
    }
}

/// Get a CoinGecko id associated with a symbol if any exists. Returns the first when multiple exist.
pub async fn get_id_from_symbol(symbol: &str) -> Result<String, GetIdFromSymbolError> {
    let symbol_id_map = get_symbol_id_map().await?;

    symbol_id_map
        .get(symbol)
        .ok_or(GetIdFromSymbolError::SymbolNotFound)
        .map(|ids| {
            ids.first()
                .expect("id lists associated with symbols should be non-empty")
                .to_string()
        })
}

fn make_price_url(id: &str, base: &str) -> String {
    format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies={}",
        id, base
    )
}

type PriceEnvelope = HashMap<String, HashMap<String, f64>>;

pub async fn get_price(id: &str, base: &str) -> reqwest::Result<PriceEnvelope> {
    reqwest::get(make_price_url(&id, &base))
        .await?
        .json::<PriceEnvelope>()
        .await
}

fn make_market_chart_url(id: &str, base: &str, coingecko_days_ago: &u32) -> String {
    format!(
        "https://api.coingecko.com/api/v3/coins/{id}/market_chart?vs_currency={base}&days={coingecko_days_ago}&interval=daily",
        id=id,
        base=base,
        coingecko_days_ago=coingecko_days_ago
    )
}

/// Unix timestamp in miliseconds.
type MsTimestamp = i64;
/// A price for a cryptocurrency.
type Price = f64;
type PriceInTime = (MsTimestamp, Price);

/// CoinGecko market data by day for a single coin.
#[derive(Clone, Debug, Deserialize)]
struct MarketChart {
    prices: Vec<PriceInTime>,
}

pub async fn get_market_chart(
    id: &str,
    base: &str,
    days_ago: &u32,
) -> reqwest::Result<Vec<PriceInTime>> {
    // CoinGecko uses 'days' as today up to but excluding n 'days' ago, we want
    // including so we add 1 here.
    let coingecko_days_ago = days_ago + 1;

    let url = make_market_chart_url(id, base, &coingecko_days_ago);
    let res = reqwest::get(&url).await?;

    match res.error_for_status() {
        Ok(res) => res
            .json::<MarketChart>()
            .await
            .map(|market_chart| market_chart.prices),
        Err(error) => Err(error),
    }
}
