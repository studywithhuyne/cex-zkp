// core/src/api/ws.rs
// WebSocket broadcaster: pushes real-time market events to connected clients.
//
// Route (registered in router.rs):
//   GET /ws  — HTTP → WebSocket upgrade; no auth required (public feed)
//
// Design:
//   A single tokio::sync::broadcast channel carries WsEvent messages.
//   The Sender (stored in AppState) is cloned by every REST handler that
//   produces events (order placements, fills, cancellations).
//   Each WebSocket connection subscribes by calling Sender::subscribe() to
//   obtain its own Receiver.
//
// Event types:
//   orderbook_update — full top-50 depth snapshot after any book change.
//   trade_executed   — price + amount of a single fill event.
//
// Backpressure:
//   The channel is bounded (BROADCAST_CAPACITY).  If a receiver falls behind,
//   it is flagged as "lagged"; it logs a warning and resumes from the next
//   available message rather than disconnecting.  This keeps slow clients from
//   blocking fast ones.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use serde::Serialize;
use tokio::sync::broadcast;

use crate::api::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Number of events buffered per broadcast channel.
/// Receivers that fall this many messages behind receive a `Lagged` error.
pub const BROADCAST_CAPACITY: usize = 256;

// ─────────────────────────────────────────────────────────────────────────────
// Event types (sent to all WebSocket subscribers)
// ─────────────────────────────────────────────────────────────────────────────

/// One price level in the orderbook snapshot — price and total resting qty.
#[derive(Clone, Serialize)]
pub struct WsPriceLevel {
    pub price:  String,
    pub amount: String,
}

/// All events emitted over the WebSocket feed.
/// Serialized as adjacently tagged JSON: `{"type": "...", "data": {...}}`.
#[derive(Clone, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum WsEvent {
    /// Full top-N depth snapshot; sent after every book mutation.
    OrderbookUpdate {
        /// Trading pair symbol, e.g. "BTC_USDT".
        symbol: String,
        bids:   Vec<WsPriceLevel>,
        asks:   Vec<WsPriceLevel>,
    },
    /// A single trade fill — broadcast once per matched trade.
    TradeExecuted {
        /// Trading pair symbol, e.g. "BTC_USDT".
        symbol: String,
        price:  String,
        amount: String,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Handler
// ─────────────────────────────────────────────────────────────────────────────

/// GET /ws — upgrade HTTP connection to a WebSocket, then stream WsEvents.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-connection loop
// ─────────────────────────────────────────────────────────────────────────────

/// Drive one WebSocket connection: forward broadcast events until the client
/// disconnects or the broadcast channel closes.
async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // Each call to `subscribe()` creates an independent Receiver with its own
    // read cursor; messages published before this point are not replayed.
    let mut rx: broadcast::Receiver<WsEvent> = state.broadcast.subscribe();

    loop {
        tokio::select! {
            // ── Incoming broadcast event → forward to client ──────────────
            result = rx.recv() => {
                let event = match result {
                    Ok(e) => e,

                    // The receiver missed some messages due to slow consumption.
                    // Log a warning and continue from the next available event.
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WebSocket client lagged, dropped {n} message(s)");
                        continue;
                    }

                    // Sender dropped — server shutting down.
                    Err(broadcast::error::RecvError::Closed) => break,
                };

                let json = match serde_json::to_string(&event) {
                    Ok(s)  => s,
                    Err(e) => {
                        tracing::error!("Failed to serialise WsEvent: {e}");
                        continue;
                    }
                };

                if socket.send(Message::Text(json)).await.is_err() {
                    break; // client disconnected
                }
            }

            // ── Incoming frame from client ─────────────────────────────────
            msg = socket.recv() => {
                match msg {
                    // Clean close or connection dropped.
                    Some(Ok(Message::Close(_))) | None => break,

                    // Respond to ping frames to keep the connection alive.
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }

                    // All other frames (text, binary) are silently ignored —
                    // this is a server-push-only feed.
                    _ => {}
                }
            }
        }
    }
}
