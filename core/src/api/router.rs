// core/src/api/router.rs
// Assembles the Axum Router and registers all routes.
//
// Routes are added incrementally across API tasks:
//   API-02  middleware: x-user-id auth
//   API-03  POST   /api/orders
//           DELETE /api/orders/:id
//   API-04  GET    /api/orderbook
//           GET    /api/balances
//   API-05  GET    /ws  (WebSocket upgrade)

use axum::{
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::{json, Value};

use super::{data, metrics, orders, wallet, ws, zkp, state::AppState};

// ─────────────────────────────────────────────────────────────────────────────
// Public factory
// ─────────────────────────────────────────────────────────────────────────────

/// Build the application Router with AppState injected.
/// Call once at startup and pass the result to `axum::serve`.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics::metrics_handler))
        // API-03: order management
        .route("/api/orders",      post(orders::place_order))
        .route("/api/orders/open", get(data::open_orders_handler))
        .route("/api/orders/:id",  delete(orders::cancel_order))
        // API-04: market data and user balances
        .route("/api/orderbook", get(data::orderbook_handler))
        .route("/api/balances",  get(data::balances_handler))
        // Wallet: deposit and personal trade history
        .route("/api/deposit",       post(wallet::deposit_handler))
        .route("/api/trades/recent", get(data::recent_trades_handler))
        .route("/api/trades/user",   get(wallet::user_trades_handler))
        // OHLCV candlestick data for the chart
        .route("/api/candles",       get(data::candles_handler))
        // ZKP-05: solvency proof package for authenticated user
        .route("/api/zkp/proof", get(zkp::proof_handler))
        // API-05: real-time WebSocket feed
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// GET /health — liveness probe used by Docker healthcheck and load balancers.
async fn health_handler() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}
