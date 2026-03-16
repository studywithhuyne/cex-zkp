// core/src/api/orders.rs
// REST handlers for order placement and cancellation.
//
// Routes (registered in router.rs):
//   POST   /api/orders       — place a new limit order
//   DELETE /api/orders/:id   — cancel a resting order
//
// Auth:
//   Both endpoints require `x-user-id` header (UserId extractor, API-02).
//
// Persistence contract (from db/worker.rs):
//   1. Send OrderPlaced for the taker BEFORE any TradeFilled that references it.
//   2. Maker orders already exist in orders_log from their own OrderPlaced events.
//
// WebSocket broadcast contract:
//   After every book mutation (place or cancel), fire:
//     - WsEvent::TradeExecuted  for each generated fill (place_order only).
//     - WsEvent::OrderbookUpdate with a fresh depth snapshot.
//   Broadcast is synchronous (no .await) and fire-and-forget.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Instant;

use crate::{
    api::{
        auth::UserId,
        state::AppState,
        ws::{WsEvent, WsPriceLevel},
    },
    db::worker::PersistenceEvent,
    engine::{Order, Side},
};

#[derive(Debug, sqlx::FromRow)]
struct OpenOrderLookupRow {
    user_id: i64,
    base_asset: String,
    quote_asset: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types
// ─────────────────────────────────────────────────────────────────────────────

/// JSON body for POST /api/orders.
/// price and amount are accepted as decimal strings to preserve full precision
/// (avoids f64 round-trip loss — critical for financial data).
#[derive(Deserialize)]
pub struct PlaceOrderRequest {
    /// "buy" or "sell" (case-insensitive)
    pub side:        String,
    /// Limit price, e.g. "100.50"
    pub price:       String,
    /// Order quantity, e.g. "0.5"
    pub amount:      String,
    pub base_asset:  String,
    pub quote_asset: String,
}

#[derive(Serialize)]
pub struct PlaceOrderResponse {
    pub order_id:     u64,
    /// Number of trades generated immediately (0 = order rested on the book).
    pub trades_count: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// POST /api/orders — submit a new limit order.
///
/// Flow:
///   1. Parse and validate (side, price, amount, assets).
///   2. Allocate order ID; register order → user mapping for trade attribution.
///   3. Acquire engine write lock → match_order → Vec<Trade>, then release lock.
///   4. Fire OrderPlaced event (must arrive before TradeFilled in the channel).
///   5. Fire TradeFilled event per generated trade.
///   6. Broadcast TradeExecuted + OrderbookUpdate via WebSocket channel.
///   7. Return 201 Created with { order_id, trades_count }.
pub async fn place_order(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(req): Json<PlaceOrderRequest>,
) -> Result<(StatusCode, Json<PlaceOrderResponse>), (StatusCode, Json<serde_json::Value>)> {
    // --- Parse side ---
    let side = match req.side.to_lowercase().as_str() {
        "buy"  => Side::Buy,
        "sell" => Side::Sell,
        _      => return Err(bad_request("side must be 'buy' or 'sell'")),
    };

    // --- Parse price and amount as Decimal (no f64 intermediate) ---
    let price = Decimal::from_str(&req.price)
        .map_err(|_| bad_request("price must be a valid decimal number"))?;
    let amount = Decimal::from_str(&req.amount)
        .map_err(|_| bad_request("amount must be a valid decimal number"))?;

    // --- Validate ---
    if req.base_asset.trim().is_empty() {
        return Err(bad_request("base_asset must not be empty"));
    }
    if req.quote_asset.trim().is_empty() {
        return Err(bad_request("quote_asset must not be empty"));
    }

    // --- Allocate ID and build Order ---
    let order_id = state.alloc_order_id();
    // Derive canonical symbol from base/quote pair (e.g. "BTC_USDT").
    let symbol   = format!("{}_{}", req.base_asset, req.quote_asset);
    let side_label = match side {
        Side::Buy => "buy",
        Side::Sell => "sell",
    };
    let order    = Order::new(order_id, user_id, &symbol, side, price, amount);

    // Register owner + symbol before matching so cancel and TradeFilled lookup always succeed.
    state.register_order_user(order_id, user_id, symbol.clone());

    // --- Match (sync, engine write-locked, no async I/O inside) ---
    let start = Instant::now();
    let trades = {
        let mut engine = state.engine.write();
        engine.match_order(order.clone())
            .map_err(|e| bad_request(&e.to_string()))?
    };
    let match_latency_us = start.elapsed().as_secs_f64() * 1_000_000.0;
    metrics::histogram!(
        "cex_order_match_latency_us",
        "symbol" => symbol.clone(),
        "side" => side_label
    )
    .record(match_latency_us);
    metrics::counter!(
        "cex_orders_total",
        "symbol" => symbol.clone(),
        "side" => side_label
    )
    .increment(1);
    // Engine lock released here — async persistence and broadcasts happen below.

    // --- Persist: OrderPlaced MUST arrive before TradeFilled ---
    let _ = state.events.send(PersistenceEvent::OrderPlaced {
        order:       order.clone(),
        base_asset:  req.base_asset.clone(),
        quote_asset: req.quote_asset.clone(),
    }).await;

    let trades_count = trades.len();
    if trades_count > 0 {
        metrics::counter!(
            "cex_trades_total",
            "symbol" => symbol.clone()
        )
        .increment(trades_count as u64);
    }
    for trade in &trades {
        // Persist (clone Trade because PersistenceEvent takes ownership).
        let maker_user_id = state.get_order_user(trade.maker_order_id)
            .map(|(uid, _)| uid)
            .unwrap_or(0);
        let _ = state.events.send(PersistenceEvent::TradeFilled {
            trade:         trade.clone(),
            maker_user_id,
            taker_user_id: user_id,
            taker_side:    side,
            base_asset:    req.base_asset.clone(),
            quote_asset:   req.quote_asset.clone(),
        }).await;

        // Broadcast individual fill to WebSocket clients (synchronous, non-blocking).
        let _ = state.broadcast.send(WsEvent::TradeExecuted {
            symbol: symbol.clone(),
            price:  trade.price.to_string(),
            amount: trade.amount.to_string(),
        });
    }

    // Broadcast a fresh depth snapshot so clients see the updated book.
    broadcast_orderbook_snapshot(&state, &symbol);
    let active_symbols = {
        let engine = state.engine.read();
        engine.symbols().len() as f64
    };
    metrics::gauge!("cex_active_symbols").set(active_symbols);

    Ok((
        StatusCode::CREATED,
        Json(PlaceOrderResponse { order_id, trades_count }),
    ))
}

/// DELETE /api/orders/:id — cancel a resting order.
///
/// Flow:
///   1. Check ownership via order_users map (O(1), no engine lock needed).
///   2. Acquire engine write lock → cancel_order, then release lock.
///   3. Fire OrderCancelled event.
///   4. Broadcast updated orderbook snapshot.
///   5. Return 204 No Content.
pub async fn cancel_order(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(order_id): Path<u64>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // --- Ownership + symbol resolution ---
    // Fast path: in-memory map (hot path while process is alive)
    // Fallback: DB lookup (covers stale rows after restart).
    let (owner_id, symbol) = if let Some((owner_id, symbol)) = state.get_order_user(order_id) {
        (owner_id, symbol)
    } else {
        let row: OpenOrderLookupRow = sqlx::query_as(
            "SELECT user_id, base_asset, quote_asset
             FROM orders_log
             WHERE order_id = $1 AND status::text IN ('open', 'partial')",
        )
        .bind(order_id as i64)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("database error: {e}") })),
            )
        })?
        .ok_or_else(|| not_found("order not found"))?;

        if row.user_id <= 0 {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "invalid order owner in database" })),
            ));
        }

        (row.user_id as u64, format!("{}_{}", row.base_asset, row.quote_asset))
    };

    if owner_id != user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "forbidden: order belongs to a different user" })),
        ));
    }

    // --- Cancel in engine if currently loaded (sync, write-locked) ---
    // If not found in memory (e.g. after restart), continue and persist cancellation to DB.
    {
        let mut engine = state.engine.write();
        let _ = engine.cancel_order(&symbol, order_id);
    }

    // --- Persist async ---
    let _ = state.events.send(PersistenceEvent::OrderCancelled { order_id }).await;
    metrics::counter!(
        "cex_order_cancellations_total",
        "symbol" => symbol.clone()
    )
    .increment(1);

    // --- Broadcast updated orderbook snapshot ---
    broadcast_orderbook_snapshot(&state, &symbol);
    let active_symbols = {
        let engine = state.engine.read();
        engine.symbols().len() as f64
    };
    metrics::gauge!("cex_active_symbols").set(active_symbols);

    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Snapshot the engine depth for `symbol` and broadcast an OrderbookUpdate event.
/// Acquires a read lock (not write), so this never contends with matching.
/// Returns immediately if no WebSocket clients are connected.
fn broadcast_orderbook_snapshot(state: &AppState, symbol: &str) {
    let (raw_bids, raw_asks) = {
        let engine = state.engine.read();
        engine.depth_snapshot(symbol, 50)
    };

    let to_level = |(price, amount): (Decimal, Decimal)| WsPriceLevel {
        price:  price.to_string(),
        amount: amount.to_string(),
    };

    // Err(SendError) is returned only when there are no active receivers;
    // this is normal during startup or when no clients are connected.
    let _ = state.broadcast.send(WsEvent::OrderbookUpdate {
        symbol: symbol.to_owned(),
        bids:   raw_bids.into_iter().map(to_level).collect(),
        asks:   raw_asks.into_iter().map(to_level).collect(),
    });
}

#[inline]
fn bad_request(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": msg })))
}

#[inline]
fn not_found(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": msg })))
}
