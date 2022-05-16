/**
 * Fetches the current price for a given token.
 * example:
 * =FHPRICE("BTC", "USD")
 *
 * @param {string} ticker - the ticker symbol you want the price for.
 * @param {string} [base] - the currency to denominate the price in.
 * @customfunction
 * @return a price
 **/
function FHPRICE(ticker, base = "usd") {
  if (ticker === undefined) {
    throw new Error("need a ticker to quote")
  }
  if (typeof ticker !== "string") {
    throw new Error("ticker should be text")
  }

  var lowercaseTicker = ticker.toLowerCase();
  var lowercaseBase = base.toLowerCase();

  var cache = CacheService.getScriptCache();
  var cached = cache.get(`price-${lowercaseTicker}-${lowercaseBase}`);
  if (cached != null) {
    console.log(ticker, base, "cache hit");
    return Number(cached);
  }
  console.log(ticker, base, "cache miss");

  var response = UrlFetchApp.fetch(`https://farmhand.dev/coin/${lowercaseTicker}/price`);
  if (response.getResponseCode() === 404) {
    return "N/A"
  }
  var price = JSON.parse(response.getContentText());

  cache.put(`price-${lowercaseTicker}-usd`, price.usd, 3600);
  cache.put(`price-${lowercaseTicker}-btc`, price.btc, 3600);
  cache.put(`price-${lowercaseTicker}-eth`, price.eth, 3600);

  return Number(price[lowercaseBase]);
}

/**
 * Calculates the percent change in a given token's price.
 * example:
 * =FHCHANGE("BTC", 7, "USD")
 *
 * @param {string} ticker - the ticker symbol of the token you want the price for.
 * @param {string} [daysAgo] - number of days back in time to compare the price to.
 * @param {string} [base] - the currency to denominate the price in.
 * @customfunction
 * @return a percent change in price
 **/
function FHCHANGE(ticker, daysAgo = 1, base = "usd") {
  if (ticker === undefined) {
    throw new Error("need a ticker to quote")
  }
  if (typeof ticker !== "string") {
    throw new Error("ticker should be text")
  }

  var lowercaseTicker = ticker.toLowerCase();
  var lowercaseBase = base.toLowerCase();

  var cache = CacheService.getScriptCache();
  var cached = cache.get(`priceChange-${lowercaseTicker}-${daysAgo}-${lowercaseBase}`);
  if (cached != null) {
    return Number(cached);
  }

  var options = {
    'method' : 'post',
    'contentType': 'application/json',
    'payload' : JSON.stringify({
       base: lowercaseBase,
       daysAgo,
    })
  };
  var response = UrlFetchApp.fetch(`https://farmhand.dev/coin/${lowercaseTicker}/price-change/`, options);
  if (response.getResponseCode() === 404) {
    return "N/A"
  }
  var priceChange = JSON.parse(response.getContentText());

  cache.put(`priceChange-${lowercaseTicker}-${daysAgo}-${lowercaseBase}`, priceChange, 3600);

  return Number(priceChange);
}
