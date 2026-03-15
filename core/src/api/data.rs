// core/src/api/data.rs
// Read-only REST handlers: public market data and authenticated user balances.
//
// Routes (registered in router.rs):
//   GET /api/orderbook  — top-50 depth snapshot (public, no auth required)
//   GET /api/balances   — balance per asset for the authenticated user (requires x-user-id)
//
// Performance notes:
//   - /api/orderbook acquires a read lock on the engine and returns immediately;
//     it never touches the database.
//   - /api/balances is a simple indexed query on the balances table (PK lookup).
//
// Serialization:
//   price, amount, available, locked are all returned as Decimal strings (not
//   JSON numbers) to preserve full 8-decimal-place precision and avoid f64
//   round-trip loss on the client side.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::{
    api::{auth::UserId, state::AppState},
    db::schema::Balance,
};

/// Number of price levels returned per side in the orderbook snapshot.
const ORDERBOOK_DEPTH: usize = 50;

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

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/orderbook — public depth snapshot from the in-memory engine.
///
/// Acquires a read lock, collects up to 50 levels per side, then releases.
/// No database query — latency is dominated by the JSON serialization.
pub async fn orderbook_handler(
    State(state): State<AppState>,
) -> Json<OrderBookResponse> {
    let (raw_bids, raw_asks) = {
        let engine = state.engine.read();
        engine.depth_snapshot(ORDERBOOK_DEPTH)
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
