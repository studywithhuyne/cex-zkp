// core/src/api/state.rs
// Shared application state injected into every Axum handler via `.with_state()`.
// All fields are Clone + Send + Sync; cheap to clone because each is Arc-backed.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use parking_lot::{Mutex, RwLock};
use metrics_exporter_prometheus::PrometheusHandle;
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc};

use crate::db::worker::PersistenceEvent;
use crate::engine::Engine;

use super::ws::{WsEvent, BROADCAST_CAPACITY};

// ─────────────────────────────────────────────────────────────────────────────
// AppState
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    /// The multi-symbol matching engine — sync, guarded by a parking_lot RwLock.
    /// Write lock for match/cancel; read lock for depth snapshots.
    pub engine: Arc<RwLock<Engine>>,

    /// Monotonically increasing counter for generating unique u64 order IDs.
    /// Shared across handler clones via Arc; fetch_add is lock-free.
    pub next_order_id: Arc<AtomicU64>,

    /// sqlx connection pool for async DB reads (balances, order history).
    pub db: PgPool,

    /// Channel sender to the async persistence worker.
    /// Handlers send OrderPlaced / TradeFilled / OrderCancelled without blocking.
    pub events: mpsc::Sender<PersistenceEvent>,

    /// Maps order_id → (user_id, symbol) for ownership checks and routing on cancel.
    /// Populated when an order is placed; entries are retained until server restart.
    pub order_users: Arc<Mutex<HashMap<u64, (u64, String)>>>,

    /// Broadcast sender for the WebSocket event bus.
    /// Each WebSocket connection clones a Receiver via `subscribe()`.
    /// `send` is synchronous and non-blocking; ignored if no active receivers.
    pub broadcast: broadcast::Sender<WsEvent>,

    /// Prometheus exporter handle used by GET /metrics.
    pub metrics: PrometheusHandle,
}

impl AppState {
    pub fn new(db: PgPool, events: mpsc::Sender<PersistenceEvent>, metrics: PrometheusHandle) -> Self {
        let (broadcast_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            engine:        Arc::new(RwLock::new(Engine::new())),
            next_order_id: Arc::new(AtomicU64::new(1)),
            db,
            events,
            order_users:   Arc::new(Mutex::new(HashMap::new())),
            broadcast:     broadcast_tx,
            metrics,
        }
    }

    /// Atomically allocate the next order ID (monotonically increasing).
    #[inline]
    pub fn alloc_order_id(&self) -> u64 {
        self.next_order_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Register an order → (user, symbol) mapping when a new order is submitted.
    #[inline]
    pub fn register_order_user(&self, order_id: u64, user_id: u64, symbol: String) {
        self.order_users.lock().insert(order_id, (user_id, symbol));
    }

    /// Look up the owner and symbol of an order; returns `None` if the order is unknown.
    #[inline]
    pub fn get_order_user(&self, order_id: u64) -> Option<(u64, String)> {
        self.order_users.lock().get(&order_id).cloned()
    }
}
