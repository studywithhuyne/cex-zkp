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

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{
    api::{auth::UserId, state::AppState},
    db::worker::PersistenceEvent,
    engine::{Order, Side},
};

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
///   6. Return 201 Created with { order_id, trades_count }.
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
    let order    = Order::new(order_id, user_id, side, price, amount);

    // Register owner before matching so TradeFilled lookup always succeeds.
    state.register_order_user(order_id, user_id);

    // --- Match (sync, engine write-locked, no async I/O inside) ---
    let trades = {
        let mut engine = state.engine.write();
        engine.match_order(order.clone())
            .map_err(|e| bad_request(&e.to_string()))?
    };
    // Engine lock released here — async persistence happens below.

    // --- Persist: OrderPlaced MUST arrive before TradeFilled ---
    let _ = state.events.send(PersistenceEvent::OrderPlaced {
        order:       order.clone(),
        base_asset:  req.base_asset.clone(),
        quote_asset: req.quote_asset.clone(),
    }).await;

    let trades_count = trades.len();
    for trade in trades {
        // Look up maker's user_id from the in-memory registry.
        // Falls back to 0 if the maker's order was somehow unregistered (safeguard).
        let maker_user_id = state.get_order_user(trade.maker_order_id).unwrap_or(0);
        let _ = state.events.send(PersistenceEvent::TradeFilled {
            trade,
            maker_user_id,
            taker_user_id: user_id,
            base_asset:    req.base_asset.clone(),
            quote_asset:   req.quote_asset.clone(),
        }).await;
    }

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
///   4. Return 204 No Content.
pub async fn cancel_order(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(order_id): Path<u64>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // --- Ownership check (no lock needed — order_users is a separate Mutex) ---
    let owner_id = state
        .get_order_user(order_id)
        .ok_or_else(|| not_found("order not found"))?;

    if owner_id != user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "forbidden: order belongs to a different user" })),
        ));
    }

    // --- Cancel in engine (sync, write-locked) ---
    {
        let mut engine = state.engine.write();
        engine
            .cancel_order(order_id)
            .map_err(|_| not_found("order not found or already filled"))?;
    }

    // --- Persist async ---
    let _ = state.events.send(PersistenceEvent::OrderCancelled { order_id }).await;

    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────────
// Response helpers
// ─────────────────────────────────────────────────────────────────────────────

#[inline]
fn bad_request(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": msg })))
}

#[inline]
fn not_found(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": msg })))
}
