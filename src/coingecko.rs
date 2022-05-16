use cached::proc_macro::cached;
use phf::phf_map;
use serde::Deserialize;
use std::collections::HashMap;

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

#[cached(time = 14400, result = true)]
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
fn make_price_url(id: &str, base: &str) -> String {
    format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies={}",
        id, base
    )
}

pub enum GetPriceError {
    PriceNotFound,
    ReqwestError(reqwest::Error),
}

impl From<reqwest::Error> for GetPriceError {
    fn from(error: reqwest::Error) -> Self {
        GetPriceError::ReqwestError(error)
    }
}

#[cached(time = 3600, result = true)]
pub async fn get_price(id: String, base: String) -> Result<f64, GetPriceError> {
    let price_envelope = reqwest::get(make_price_url(&id, &base))
        .await?
        .json::<HashMap<String, HashMap<String, f64>>>()
        .await?;

    match price_envelope
        .get(&id)
        .and_then(|price_map| price_map.get(&base))
    {
        None => Err(GetPriceError::PriceNotFound),
        Some(price) => Ok(*price),
    }
}
