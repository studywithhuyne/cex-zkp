// core/src/api/data.rs
// Read-only REST handlers: public market data and authenticated user data.
//
// Routes (registered in router.rs):
//   GET /api/orderbook          — top-50 depth snapshot (public, ?symbol=BTC_USDT)
//   GET /api/balances           — balance per asset for the authenticated user (requires x-user-id)
//   GET /api/orders/open        — open/partial orders for the authenticated user (requires x-user-id)
//   GET /api/trades/recent      — last 50 trades globally (public, no auth required)
//   GET /api/candles            — OHLCV candle data (?symbol=BTC_USDT&interval=1m&limit=100)
//
// Serialization:
//   price, amount, available, locked are all returned as Decimal strings (not
//   JSON numbers) to preserve full 8-decimal-place precision and avoid f64
//   round-trip loss on the client side.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use reqwest::header::HeaderValue;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    api::{auth::UserId, state::AppState},
    db::schema::{Candle, TradeLog},
    engine::Side,
};

/// Number of price levels returned per side in the orderbook snapshot.
const ORDERBOOK_DEPTH: usize = 50;

// ─────────────────────────────────────────────────────────────────────────────
// Query parameter types
// ─────────────────────────────────────────────────────────────────────────────

/// Query params for GET /api/orderbook.
#[derive(Deserialize)]
pub struct OrderbookQuery {
    /// Trading pair symbol, e.g. "BTC_USDT". Defaults to "BTC_USDT" if omitted.
    pub symbol: Option<String>,
}

/// Query params for GET /api/candles.
#[derive(Deserialize)]
pub struct CandlesQuery {
    /// Trading pair symbol, e.g. "BTC_USDT".
    pub symbol:   String,
    /// Candlestick interval: "1m", "5m", "1h", or "1d". Defaults to "1m".
    pub interval: Option<String>,
    /// Number of candles to return (max 500). Defaults to 100.
    pub limit:    Option<i64>,
}

/// Query params for GET /api/market/tickers/live.
#[derive(Deserialize)]
pub struct LiveTickersQuery {
    /// Comma-separated symbols, e.g. "BTCUSDT,ETHUSDT".
    pub symbols: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Response types
// ─────────────────────────────────────────────────────────────────────────────

/// A single aggregated price level: total resting quantity at one price.
#[derive(Serialize)]
pub struct PriceLevelDto {
    /// Limit price as a full-precision decimal string, e.g. "100.50000000"
    pub price:  String,
    /// Sum of `remaining` across all resting orders at this level.
    pub amount: String,
}

#[derive(Serialize)]
pub struct OrderBookResponse {
    /// Buy side — sorted highest price first (best bid first).
    pub bids: Vec<PriceLevelDto>,
    /// Sell side — sorted lowest price first (best ask first).
    pub asks: Vec<PriceLevelDto>,
}

#[derive(Serialize)]
pub struct BalanceDto {
    pub asset:     String,
    /// Funds available to place new orders, as a decimal string.
    pub available: String,
    /// Funds currently locked by open orders, as a decimal string.
    pub locked:    String,
}

#[derive(Serialize)]
pub struct AssetDto {
    pub symbol: String,
    pub name: String,
    pub decimals: i16,
}

#[derive(Serialize)]
pub struct OpenOrderDto {
    pub order_id:    i64,
    pub side:        String,
    pub price:       String,
    pub amount:      String,
    pub filled:      String,
    pub status:      String,
    pub base_asset:  String,
    pub quote_asset: String,
    pub created_at:  String,
}

#[derive(Serialize)]
pub struct RecentTradeDto {
    pub market_symbol: String,
    pub price:       String,
    pub amount:      String,
    pub base_asset:  String,
    pub quote_asset: String,
    pub executed_at: String,
}

/// One OHLCV candle returned by GET /api/candles.
#[derive(Serialize)]
pub struct CandleDto {
    /// Open timestamp in milliseconds (Unix epoch), e.g. 1700000000000.
    pub time:   i64,
    pub open:   String,
    pub high:   String,
    pub low:    String,
    pub close:  String,
    pub volume: String,
}

#[derive(Serialize)]
pub struct AveragePriceResponse {
    pub symbol: String,
    pub best_bid: Option<String>,
    pub best_ask: Option<String>,
    pub mid_price: Option<String>,
    pub micro_price: Option<String>,
}

#[derive(Serialize)]
pub struct LiveTickerDto {
    pub symbol: String,
    pub last_price: String,
    pub price_change_percent_24h: String,
    pub quote_volume_24h: String,
}

#[derive(Deserialize)]
struct BinanceTickerRaw {
    symbol: String,
    #[serde(rename = "lastPrice")]
    last_price: String,
    #[serde(rename = "priceChangePercent")]
    price_change_percent_24h: String,
    #[serde(rename = "quoteVolume")]
    quote_volume_24h: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/orderbook?symbol=BTC_USDT — public depth snapshot from the in-memory engine.
///
/// Acquires a read lock, collects up to 50 levels per side, then releases.
/// No database query — latency is dominated by the JSON serialization.
/// Symbol defaults to "BTC_USDT" if the query param is omitted.
pub async fn orderbook_handler(
    State(state): State<AppState>,
    Query(params): Query<OrderbookQuery>,
) -> Json<OrderBookResponse> {
    let symbol = params.symbol.as_deref().unwrap_or("BTC_USDT");
    let (raw_bids, raw_asks) = {
        let engine = state.engine.read();
        engine.depth_snapshot(symbol, ORDERBOOK_DEPTH)
    };

    let to_dto = |(price, amount): (Decimal, Decimal)| PriceLevelDto {
        price:  price.to_string(),
        amount: amount.to_string(),
    };

    Json(OrderBookResponse {
        bids: raw_bids.into_iter().map(to_dto).collect(),
        asks: raw_asks.into_iter().map(to_dto).collect(),
    })
}

/// GET /api/balances/:asset — single asset balance for the authenticated user.
///
/// More efficient than fetching all balances when only one asset is needed.
/// Returns 404 if the user has no balance record for the given asset.
pub async fn balance_asset_handler(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    axum::extract::Path(asset): axum::extract::Path<String>,
) -> Result<Json<BalanceDto>, (StatusCode, Json<serde_json::Value>)> {
    let asset_upper = asset.trim().to_ascii_uppercase();

    let balance = state
        .ledger
        .lock()
        .balances_for_user(user_id)
        .into_iter()
        .find(|b| b.asset == asset_upper);

    match balance {
        Some(b) => Ok(Json(BalanceDto {
            asset:     b.asset,
            available: b.free.to_string(),
            locked:    b.locked.to_string(),
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("no balance found for asset {asset_upper}") })),
        )),
    }
}


///
/// Reads from the `balances` table via an indexed primary-key lookup.
/// Note: balances are updated asynchronously by the persistence worker,
/// so there may be a brief lag after a trade before the balance reflects it.
pub async fn balances_handler(
    State(state): State<AppState>,
    UserId(user_id): UserId,
) -> Result<Json<Vec<BalanceDto>>, (StatusCode, Json<serde_json::Value>)> {
    let dtos = state
        .ledger
        .lock()
        .balances_for_user(user_id)
        .into_iter()
        .map(|b| BalanceDto {
            asset:     b.asset,
            available: b.free.to_string(),
            locked:    b.locked.to_string(),
        })
        .collect();

    Ok(Json(dtos))
}

/// GET /api/assets — all supported assets configured in exchange.
pub async fn assets_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<AssetDto>>, (StatusCode, Json<serde_json::Value>)> {
    let rows: Vec<(String, String, i16)> = sqlx::query_as(
        "SELECT symbol, name, decimals
         FROM assets
         ORDER BY symbol ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("database error: {e}") })),
        )
    })?;

    Ok(Json(
        rows.into_iter()
            .map(|(symbol, name, decimals)| AssetDto {
                symbol,
                name,
                decimals,
            })
            .collect(),
    ))
}

/// GET /api/price/average?symbol=BTC_USDT
///
/// Computes current market averages from top-of-book levels:
/// - mid_price   = (best_bid + best_ask) / 2
/// - micro_price = liquidity-weighted top-book midpoint
pub async fn average_price_handler(
    State(state): State<AppState>,
    Query(params): Query<OrderbookQuery>,
) -> Json<AveragePriceResponse> {
    let symbol = params
        .symbol
        .as_deref()
        .unwrap_or("BTC_USDT")
        .to_string();

    let (bids, asks) = {
        let engine = state.engine.read();
        engine.depth_snapshot(&symbol, 1)
    };

    let best_bid = bids.first().map(|(p, _)| *p);
    let best_ask = asks.first().map(|(p, _)| *p);

    let mid_price = match (best_bid, best_ask) {
        (Some(bid), Some(ask)) => Some(((bid + ask) / Decimal::from(2_u64)).round_dp(4)),
        _ => None,
    };

    let micro_price = match (bids.first(), asks.first()) {
        (Some((bid_price, bid_qty)), Some((ask_price, ask_qty))) => {
            let denom = *bid_qty + *ask_qty;
            if denom.is_zero() {
                None
            } else {
                let weighted_sum = (*bid_price * *ask_qty) + (*ask_price * *bid_qty);
                Some((weighted_sum / denom).round_dp(4))
            }
        }
        _ => None,
    };

    Json(AveragePriceResponse {
        symbol,
        best_bid: best_bid.map(|v| v.to_string()),
        best_ask: best_ask.map(|v| v.to_string()),
        mid_price: mid_price.map(|v| v.to_string()),
        micro_price: micro_price.map(|v| v.to_string()),
    })
}

/// GET /api/orders/open — open and partially filled orders for the authenticated user.
pub async fn open_orders_handler(
    State(state): State<AppState>,
    UserId(user_id): UserId,
) -> Result<Json<Vec<OpenOrderDto>>, (StatusCode, Json<serde_json::Value>)> {
    let open_orders = {
        let engine = state.engine.read();
        engine.open_orders_by_user(user_id)
    };

    let dtos = open_orders
        .into_iter()
        .map(|order| {
            let (base_asset, quote_asset) = split_symbol_assets(&order.symbol);
            let filled = order.amount - order.remaining;
            let status = if filled.is_zero() { "open" } else { "partial" };

            OpenOrderDto {
                order_id:    order.id as i64,
                side:        match order.side {
                    Side::Buy => "buy".to_string(),
                    Side::Sell => "sell".to_string(),
                },
                price:       order.price.to_string(),
                amount:      order.amount.to_string(),
                filled:      filled.to_string(),
                status:      status.to_string(),
                base_asset,
                quote_asset,
                // In-memory orders don't carry DB timestamp; order_id is monotonic
                // and already sorted newest-first in engine.open_orders_by_user.
                created_at:  String::new(),
            }
        })
        .collect();

    Ok(Json(dtos))
}

fn split_symbol_assets(symbol: &str) -> (String, String) {
    match symbol.split_once('_') {
        Some((base, quote)) => (base.to_string(), quote.to_string()),
        None => (symbol.to_string(), String::new()),
    }
}

/// GET /api/trades/recent — last 50 trades globally (public, no auth).
pub async fn recent_trades_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<RecentTradeDto>>, (StatusCode, Json<serde_json::Value>)> {
    let rows: Vec<TradeLog> = sqlx::query_as(
        "SELECT id, maker_order_id, taker_order_id, maker_user_id, taker_user_id,
                market_symbol, price, amount, executed_at
         FROM trades_log
         ORDER BY executed_at DESC
         LIMIT 50",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("database error: {e}") })),
        )
    })?;

    let dtos = rows
        .into_iter()
        .map(|t| {
            let (base_asset, quote_asset) = split_symbol_assets(&t.market_symbol);
            RecentTradeDto {
            market_symbol: t.market_symbol,
            price:       t.price.to_string(),
            amount:      t.amount.to_string(),
            base_asset,
            quote_asset,
            executed_at: t.executed_at.to_rfc3339(),
            }
        })
        .collect();

    Ok(Json(dtos))
}

/// GET /api/candles?symbol=BTC_USDT&interval=1m&limit=100
///
/// Returns OHLCV candlestick data from the `candles` table.
/// Data is populated asynchronously by the persistence worker after each trade fill.
/// Candles are returned in descending open_time order (newest first).
pub async fn candles_handler(
    State(state): State<AppState>,
    Query(params): Query<CandlesQuery>,
) -> Result<Json<Vec<CandleDto>>, (StatusCode, Json<serde_json::Value>)> {
    let interval  = params.interval.as_deref().unwrap_or("1m");
    let limit     = params.limit.unwrap_or(100).clamp(1, 500);

    let rows: Vec<Candle> = sqlx::query_as(
           "SELECT market_symbol, interval, open_time, open, high, low, close, volume
         FROM candles
            WHERE market_symbol = $1 AND interval = $2
         ORDER BY open_time DESC
         LIMIT $3",
    )
    .bind(&params.symbol)
    .bind(interval)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("database error: {e}") })),
        )
    })?;

    let dtos = rows
        .into_iter()
        .map(|c| CandleDto {
            time:   c.open_time.timestamp_millis(),
            open:   c.open.to_string(),
            high:   c.high.to_string(),
            low:    c.low.to_string(),
            close:  c.close.to_string(),
            volume: c.volume.to_string(),
        })
        .collect();

    Ok(Json(dtos))
}

/// GET /api/market/tickers/live?symbols=BTCUSDT,ETHUSDT
///
/// Proxies Binance 24h ticker API via backend so API keys are never exposed to frontend bundles.
/// Uses env vars:
/// - BINANCE_API_BASE_URL (default: https://api.binance.com/api/v3)
/// - BINANCE_API_KEY (optional for public endpoints, included if provided)
pub async fn live_tickers_handler(
    Query(params): Query<LiveTickersQuery>,
) -> Result<Json<Vec<LiveTickerDto>>, (StatusCode, Json<serde_json::Value>)> {
    let base_url = std::env::var("BINANCE_API_BASE_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "https://api.binance.com/api/v3".to_string());

    let symbols: Vec<String> = params
        .symbols
        .as_deref()
        .unwrap_or("BTCUSDT,ETHUSDT,SOLUSDT,BNBUSDT")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_uppercase())
        .collect();

    if symbols.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "symbols must not be empty" })),
        ));
    }

    let symbols_json = format!(
        "[{}]",
        symbols
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<_>>()
            .join(",")
    );

    let mut request = reqwest::Client::new()
        .get(format!("{}/ticker/24hr", base_url.trim_end_matches('/')))
        .query(&[("symbols", symbols_json)]);

    if let Ok(key) = std::env::var("BINANCE_API_KEY") {
        let k = key.trim();
        if !k.is_empty() {
            if let Ok(header_value) = HeaderValue::from_str(k) {
                request = request.header("X-MBX-APIKEY", header_value);
            }
        }
    }

    let response = request.send().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": format!("failed to reach Binance: {e}") })),
        )
    })?;

    if !response.status().is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": format!("Binance returned status {}", response.status())
            })),
        ));
    }

    let raw: Vec<BinanceTickerRaw> = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": format!("invalid Binance response: {e}") })),
        )
    })?;

    let out = raw
        .into_iter()
        .map(|t| LiveTickerDto {
            symbol: t.symbol,
            last_price: t.last_price,
            price_change_percent_24h: t.price_change_percent_24h,
            quote_volume_24h: t.quote_volume_24h,
        })
        .collect();

    Ok(Json(out))
}
