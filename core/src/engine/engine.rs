// core/src/engine/engine.rs
// Multi-symbol routing layer over individual OrderBook instances.
//
// Design
// ──────
// `Engine` owns a `HashMap<String, OrderBook>` keyed by symbol (e.g. "BTC_USDT").
// Lookup is O(1) average-case — satisfying the spec requirement of O(1) routing.
// A new OrderBook is created on-demand the first time an order for a symbol arrives.
//
// Concurrency note:
//   The whole Engine is wrapped in an `Arc<RwLock<Engine>>` at the AppState level.
//   The sync methods here hold no locks themselves — all locking responsibility
//   lives in the Axum handler layer.

use std::collections::HashMap;

use super::{EngineError, Order, OrderBook, Trade};
use super::order_book::DepthSnapshot;

/// Top-level engine: routes each order to the correct per-symbol OrderBook
/// via a `HashMap` for O(1) average lookup.
pub struct Engine {
    books: HashMap<String, OrderBook>,
}

impl Engine {
    pub fn new() -> Self {
        Self { books: HashMap::new() }
    }

    /// Match `order` against the book for `order.symbol`.
    /// The book is created lazily on the first order for a new symbol.
    pub fn match_order(&mut self, order: Order) -> Result<Vec<Trade>, EngineError> {
        let book = self.books.entry(order.symbol.clone()).or_default();
        book.match_order(order)
    }

    /// Cancel a resting order in the `symbol` book by its `order_id`.
    /// Returns `OrderNotFound` if the symbol has no matching book or the
    /// order is not in the book.
    pub fn cancel_order(&mut self, symbol: &str, order_id: u64) -> Result<Order, EngineError> {
        self.books
            .get_mut(symbol)
            .ok_or(EngineError::OrderNotFound(order_id))?
            .cancel_order(order_id)
    }

    /// Return the top-`limit` depth levels for `symbol`.
    /// Returns empty vecs if no book exists for the symbol yet.
    pub fn depth_snapshot(&self, symbol: &str, limit: usize) -> DepthSnapshot {
        self.books
            .get(symbol)
            .map(|b| b.depth_snapshot(limit))
            .unwrap_or_else(|| (vec![], vec![]))
    }

    /// All symbols for which at least one order has been placed.
    pub fn symbols(&self) -> Vec<&str> {
        self.books.keys().map(String::as_str).collect()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
