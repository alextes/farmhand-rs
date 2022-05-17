use crate::coingecko;
use cached::proc_macro::cached;

pub enum GetPriceError {
    PriceNotFound,
    ReqwestError(reqwest::Error),
}

impl From<reqwest::Error> for GetPriceError {
    fn from(error: reqwest::Error) -> Self {
        GetPriceError::ReqwestError(error)
    }
}

#[cached(time = 3600, result = true, sync_writes = true)]
pub async fn get_price(id: String, base: String) -> Result<f64, GetPriceError> {
    let price_envelope = coingecko::get_price(&id, &base).await?;

    let price = price_envelope
        .get(&id)
        .and_then(|price_map| price_map.get(&base));

    match price {
        None => Err(GetPriceError::PriceNotFound),
        Some(price) => Ok(*price),
    }
}
