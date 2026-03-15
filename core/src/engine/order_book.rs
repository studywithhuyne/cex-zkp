use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, VecDeque};

use rust_decimal::Decimal;

use super::types::Order;

/// Central limit order book.
///
/// Layout
/// ──────
/// bids  – buy  orders keyed by `Reverse<Decimal>` so the highest price
///         comes first when iterating (BTreeMap is ascending by default).
/// asks  – sell orders keyed by `Decimal`, lowest price first.
///
/// Each price level holds a `VecDeque<Order>` for FIFO matching:
/// the front of the queue is always the oldest (highest-priority) order.
///
/// order_map – `order_id → limit_price` index used to locate and remove
///             an order in O(1) + O(log P + Q) where P = number of price
///             levels and Q = queue depth at that level.
pub struct OrderBook {
    pub(super) bids: BTreeMap<Reverse<Decimal>, VecDeque<Order>>,
    pub(super) asks: BTreeMap<Decimal, VecDeque<Order>>,
    /// Maps order_id to its limit price for fast cancel lookup.
    pub(super) order_map: HashMap<u64, Decimal>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            order_map: HashMap::new(),
        }
    }

    /// Best (highest) bid price, or `None` if the book has no buy orders.
    pub fn best_bid(&self) -> Option<Decimal> {
        self.bids.keys().next().map(|Reverse(p)| *p)
    }

    /// Best (lowest) ask price, or `None` if the book has no sell orders.
    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.keys().next().copied()
    }

    /// Total number of live orders tracked by the order map.
    pub fn len(&self) -> usize {
        self.order_map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order_map.is_empty()
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}
