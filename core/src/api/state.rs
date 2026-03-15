// core/src/api/state.rs
// Shared application state injected into every Axum handler via `.with_state()`.
// All fields are Clone + Send + Sync; cheap to clone because each is Arc-backed.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use parking_lot::{Mutex, RwLock};
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::db::worker::PersistenceEvent;
use crate::engine::OrderBook;

// ─────────────────────────────────────────────────────────────────────────────
// AppState
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    /// The in-memory order book — sync engine guarded by a parking_lot RwLock.
    /// Read lock for queries (orderbook depth); write lock for add/cancel/match.
    pub engine: Arc<RwLock<OrderBook>>,

    /// Monotonically increasing counter for generating unique u64 order IDs.
    /// Shared across handler clones via Arc; fetch_add is lock-free.
    pub next_order_id: Arc<AtomicU64>,

    /// sqlx connection pool for async DB reads (balances, order history).
    pub db: PgPool,

    /// Channel sender to the async persistence worker.
    /// Handlers send OrderPlaced / TradeFilled / OrderCancelled without blocking.
    pub events: mpsc::Sender<PersistenceEvent>,

    /// Maps order_id → user_id for ownership checks and TradeFilled events.
    /// Populated when an order is placed; entries are retained until server restart.
    /// Lock contention is minimal as it is only touched outside the engine lock.
    pub order_users: Arc<Mutex<HashMap<u64, u64>>>,
}

impl AppState {
    pub fn new(db: PgPool, events: mpsc::Sender<PersistenceEvent>) -> Self {
        Self {
            engine:        Arc::new(RwLock::new(OrderBook::new())),
            next_order_id: Arc::new(AtomicU64::new(1)),
            db,
            events,
            order_users:   Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Atomically allocate the next order ID (monotonically increasing).
    #[inline]
    pub fn alloc_order_id(&self) -> u64 {
        self.next_order_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Register an order → user mapping when a new order is submitted.
    #[inline]
    pub fn register_order_user(&self, order_id: u64, user_id: u64) {
        self.order_users.lock().insert(order_id, user_id);
    }

    /// Look up the owner of an order; returns `None` if the order is unknown.
    #[inline]
    pub fn get_order_user(&self, order_id: u64) -> Option<u64> {
        self.order_users.lock().get(&order_id).copied()
    }
}
