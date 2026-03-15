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

use super::{data, orders, state::AppState};

// ─────────────────────────────────────────────────────────────────────────────
// Public factory
// ─────────────────────────────────────────────────────────────────────────────

/// Build the application Router with AppState injected.
/// Call once at startup and pass the result to `axum::serve`.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        // API-03: order management
        .route("/api/orders",     post(orders::place_order))
        .route("/api/orders/:id", delete(orders::cancel_order))
        // API-04: market data and user balances
        .route("/api/orderbook", get(data::orderbook_handler))
        .route("/api/balances",  get(data::balances_handler))
        // API-05:
        //   .route("/ws", get(ws::handler))
        .with_state(state)
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// GET /health — liveness probe used by Docker healthcheck and load balancers.
async fn health_handler() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}
