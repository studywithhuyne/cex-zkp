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
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    api::{auth::UserId, state::AppState},
    db::schema::{Balance, Candle, OrderLog, TradeLog},
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

/// GET /api/balances — all asset balances for the authenticated user.
///
/// Reads from the `balances` table via an indexed primary-key lookup.
/// Note: balances are updated asynchronously by the persistence worker,
/// so there may be a brief lag after a trade before the balance reflects it.
pub async fn balances_handler(
    State(state): State<AppState>,
    UserId(user_id): UserId,
) -> Result<Json<Vec<BalanceDto>>, (StatusCode, Json<serde_json::Value>)> {
    let rows: Vec<Balance> = sqlx::query_as(
        "SELECT user_id, asset_symbol, available, locked, updated_at
         FROM balances
         WHERE user_id = $1",
    )
    .bind(user_id as i64)
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
        .map(|b| BalanceDto {
            asset:     b.asset_symbol,
            available: b.available.to_string(),
            locked:    b.locked.to_string(),
        })
        .collect();

    Ok(Json(dtos))
}

/// GET /api/orders/open — open and partially filled orders for the authenticated user.
pub async fn open_orders_handler(
    State(state): State<AppState>,
    UserId(user_id): UserId,
) -> Result<Json<Vec<OpenOrderDto>>, (StatusCode, Json<serde_json::Value>)> {
    let rows: Vec<OrderLog> = sqlx::query_as(
        "SELECT id, order_id, user_id, side::text, price, amount, filled,
                status::text, base_asset, quote_asset, created_at, updated_at
         FROM orders_log
         WHERE user_id = $1 AND status::text IN ('open', 'partial')
         ORDER BY created_at DESC",
    )
    .bind(user_id as i64)
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
        .map(|o| OpenOrderDto {
            order_id:    o.order_id,
            side:        o.side,
            price:       o.price.to_string(),
            amount:      o.amount.to_string(),
            filled:      o.filled.to_string(),
            status:      o.status,
            base_asset:  o.base_asset,
            quote_asset: o.quote_asset,
            created_at:  o.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(dtos))
}

/// GET /api/trades/recent — last 50 trades globally (public, no auth).
pub async fn recent_trades_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<RecentTradeDto>>, (StatusCode, Json<serde_json::Value>)> {
    let rows: Vec<TradeLog> = sqlx::query_as(
        "SELECT id, maker_order_id, taker_order_id, maker_user_id, taker_user_id,
                price, amount, base_asset, quote_asset, executed_at
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
        .map(|t| RecentTradeDto {
            price:       t.price.to_string(),
            amount:      t.amount.to_string(),
            base_asset:  t.base_asset,
            quote_asset: t.quote_asset,
            executed_at: t.executed_at.to_rfc3339(),
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
        "SELECT symbol, interval, open_time, open, high, low, close, volume
         FROM candles
         WHERE symbol = $1 AND interval = $2
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
