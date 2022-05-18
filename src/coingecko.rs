use cached::proc_macro::{cached, once};
use phf::phf_map;
use reqwest::StatusCode;
use serde::Deserialize;
use std::{collections::HashMap, fmt::Display};

const ID_URL: &str = "https://api.coingecko.com/api/v3/coins/list";

#[derive(Clone, Debug, Deserialize)]
pub struct CoinId {
    pub id: String,
    pub symbol: String,
    pub name: String,
}

pub async fn get_coin_list(client: &reqwest::Client) -> reqwest::Result<Vec<CoinId>> {
    client.get(ID_URL).send().await?.json::<Vec<CoinId>>().await
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

#[once(time = 14400, result = true, sync_writes = true)]
pub async fn get_symbol_id_map(client: &reqwest::Client) -> reqwest::Result<IdMap> {
    log::debug!("getting fresh symbol id map from coingecko");

    let coin_ids = get_coin_list(client).await?;

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
pub async fn get_id_from_symbol(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<String, GetIdFromSymbolError> {
    let symbol_id_map = get_symbol_id_map(client).await?;

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

pub enum GetPriceError {
    NotFound(String),
    ReqwestError(reqwest::Error),
}

impl Display for GetPriceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReqwestError(error) => error.fmt(f),
            Self::NotFound(id) => write!(f, "price not found for symbol {}", id),
        }
    }
}

impl From<GetPriceError> for StatusCode {
    fn from(error: GetPriceError) -> Self {
        match error {
            GetPriceError::NotFound(_) => StatusCode::NOT_FOUND,
            GetPriceError::ReqwestError(error) => error
                .status()
                .unwrap_or_else(|| StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

impl From<reqwest::Error> for GetPriceError {
    fn from(error: reqwest::Error) -> Self {
        GetPriceError::ReqwestError(error)
    }
}

type PriceResponse = HashMap<String, HashMap<String, f64>>;

#[cached(
    time = 3600,
    result = true,
    sync_writes = true,
    key = "String",
    convert = r#"{ format!("{}{}", id, base) }"#
)]
pub async fn get_price(
    client: &reqwest::Client,
    id: &str,
    base: &str,
) -> Result<f64, GetPriceError> {
    log::debug!("getting fresh price for id {}, in base {}", id, base);

    let res = client
        .get(make_price_url(&id, &base))
        .send()
        .await?
        .json::<PriceResponse>()
        .await?;

    // CoinGecko returns a 200 response with an empty body for ids that are not found. Expect an
    // empty HashMap here.
    res.get(id)
        .and_then(|base_map| base_map.get(base))
        .map_or_else(
            || {
                log::debug!("coingecko price for id {}, not found", id);
                Err(GetPriceError::NotFound(id.to_string()))
            },
            |price| Ok(price.clone()),
        )
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
