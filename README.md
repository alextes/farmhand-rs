# Farmhand

Provides data needed to track yield farming returns in sheets.

```
           ______
      _.-"`      `"-._
    .'__ .. ._ .  .   '.
   / |__ /\ |_)|\/|     \
  /  |  /``\| \|  |      \
 ;                    _   ;
 |        |_| /\ |\ || \  |
 |     _. | |/``\| \||_/  |
 ;    /__`A   ,_          ;
  \   |= |;._.}{__       /
_.-""-|.' # '. `  `.-"{}<._
      / 1938  \     \  x   `"
 ----/         \_.-'|--X----
 -=_ |         |    |- X.  =_
- __ |_________|_.-'|_X-X##
jgs `'-._|_|;:;_.-'` '::.  `"-
 .:;.      .:.   ::.     '::.
```

## Ideas

- Simple APY is an extrapolation of earnings since start. In other words, take the value at t=0, take the value at t=now, and draw a line through both points to t=1y. We probably get a more accurate prediction by including data of each days value, i.e. a trend line.

## CoinGecko API Use

We use `/coins/list` to figure out CoinGecko coin IDs.
We use `/simple/price` to get a token price.
We use `/coins/{id}/market_chart` to get percent changes in price over arbitrary periods.

## Caching

We'd like to not hit the CoinGecko API for every requested price. We therefore implement the following caching strategies.

- The full list of tickers is cached for 24 hours.
- A simple price is cached for 60 minutes.
- A percent change is cached for 24 hours.
- The most recently requested 100,000 historic prices are stored for percent change calculation.
