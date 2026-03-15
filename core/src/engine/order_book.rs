use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, VecDeque};

use rust_decimal::Decimal;

use super::error::EngineError;
use super::types::{Order, Side};

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
/// order_map – `order_id → (side, limit_price)` index for O(1) cancel lookup.
///             Storing `Side` alongside price lets cancel_order route directly
///             to the correct BTreeMap without scanning both sides.
pub struct OrderBook {
    pub(super) bids: BTreeMap<Reverse<Decimal>, VecDeque<Order>>,
    pub(super) asks: BTreeMap<Decimal, VecDeque<Order>>,
    /// Maps order_id to (side, price) for fast cancel lookup.
    pub(super) order_map: HashMap<u64, (Side, Decimal)>,
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

    /// Insert a limit order into the book.
    ///
    /// Algorithm
    /// ─────────
    /// 1. Validate: price > 0 and amount > 0.
    /// 2. Reject duplicate order IDs (order_map lookup, O(1)).
    /// 3. Route to the correct side (bids / asks).
    /// 4. Push the order to the back of the VecDeque at its price level,
    ///    preserving FIFO priority within each level.
    /// 5. Register order_id → (side, price) in order_map for O(1) cancel lookup.
    ///
    /// Complexity: O(log P) where P = number of distinct price levels.
    pub fn add_order(&mut self, order: Order) -> Result<(), EngineError> {
        // --- Validation ---
        if order.price <= Decimal::ZERO {
            return Err(EngineError::InvalidPrice(order.price));
        }
        if order.amount <= Decimal::ZERO {
            return Err(EngineError::InvalidAmount(order.amount));
        }
        if self.order_map.contains_key(&order.id) {
            return Err(EngineError::DuplicateOrderId(order.id));
        }

        // --- Insert into the correct side ---
        let price = order.price;
        let id = order.id;
        let side = order.side;

        match side {
            Side::Buy => {
                self.bids
                    .entry(Reverse(price))
                    .or_default()
                    .push_back(order);
            }
            Side::Sell => {
                self.asks
                    .entry(price)
                    .or_default()
                    .push_back(order);
            }
        }

        // --- Register for fast cancel lookup ---
        self.order_map.insert(id, (side, price));

        Ok(())
    }

    /// Remove a resting order from the book by its ID.
    ///
    /// Algorithm
    /// ─────────
    /// 1. Lookup (side, price) from order_map — O(1).
    ///    If not found, return Err(OrderNotFound).
    /// 2. Remove from order_map.
    /// 3. Navigate to the correct BTreeMap + price level — O(log P).
    /// 4. Scan the VecDeque to find and remove the order by ID — O(Q).
    /// 5. If the VecDeque is now empty, remove the price level from the
    ///    BTreeMap to reclaim memory and keep best_bid/best_ask accurate.
    ///
    /// Complexity: O(log P + Q) where P = price levels, Q = queue depth.
    pub fn cancel_order(&mut self, order_id: u64) -> Result<Order, EngineError> {
        // --- Lookup side and price — O(1) ---
        let (side, price) = self
            .order_map
            .remove(&order_id)
            .ok_or(EngineError::OrderNotFound(order_id))?;

        // --- Locate the price level and remove the order — O(log P + Q) ---
        let cancelled = match side {
            Side::Buy => Self::remove_from_level(&mut self.bids, Reverse(price), order_id),
            Side::Sell => Self::remove_from_level(&mut self.asks, price, order_id),
        };

        // Invariant: order_map and BTreeMap must stay in sync.
        // If the order was somehow missing from the BTreeMap despite being in
        // order_map, that is a bug — panic in debug, silently ignore in release.
        debug_assert!(
            cancelled.is_some(),
            "order {order_id} was in order_map but missing from the price level queue"
        );

        cancelled.ok_or(EngineError::OrderNotFound(order_id))
    }

    /// Generic helper: remove `order_id` from the VecDeque at `key` in `map`.
    /// Drops the price level entry if the queue becomes empty.
    fn remove_from_level<K>(
        map: &mut BTreeMap<K, VecDeque<Order>>,
        key: K,
        order_id: u64,
    ) -> Option<Order>
    where
        K: Ord,
    {
        let queue = map.get_mut(&key)?;

        // Find the order in the queue (O(Q)); position 0 is the common case
        // for FIFO cancellations but we must handle any position.
        let pos = queue.iter().position(|o| o.id == order_id)?;
        let order = queue.remove(pos)?;

        // Drop the price level if no orders remain to keep best_bid/best_ask clean.
        if queue.is_empty() {
            map.remove(&key);
        }

        Some(order)
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn buy(id: u64, price: Decimal, amount: Decimal) -> Order {
        Order::new(id, 1, Side::Buy, price, amount)
    }

    fn sell(id: u64, price: Decimal, amount: Decimal) -> Order {
        Order::new(id, 2, Side::Sell, price, amount)
    }

    // ─── add_order: happy paths ───────────────────────────────────────────

    #[test]
    fn buy_order_appears_in_bids() {
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(100), dec!(10))).unwrap();

        assert_eq!(book.len(), 1);
        assert_eq!(book.best_bid(), Some(dec!(100)));
        assert!(book.best_ask().is_none());
    }

    #[test]
    fn sell_order_appears_in_asks() {
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(101), dec!(10))).unwrap();

        assert_eq!(book.len(), 1);
        assert_eq!(book.best_ask(), Some(dec!(101)));
        assert!(book.best_bid().is_none());
    }

    #[test]
    fn bids_sorted_highest_first() {
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(99), dec!(1))).unwrap();
        book.add_order(buy(2, dec!(101), dec!(1))).unwrap();
        book.add_order(buy(3, dec!(100), dec!(1))).unwrap();

        assert_eq!(book.best_bid(), Some(dec!(101)));
    }

    #[test]
    fn asks_sorted_lowest_first() {
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(102), dec!(1))).unwrap();
        book.add_order(sell(2, dec!(100), dec!(1))).unwrap();
        book.add_order(sell(3, dec!(101), dec!(1))).unwrap();

        assert_eq!(book.best_ask(), Some(dec!(100)));
    }

    #[test]
    fn multiple_orders_at_same_price_level_fifo() {
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(100), dec!(5))).unwrap();
        book.add_order(buy(2, dec!(100), dec!(3))).unwrap();

        let queue = book.bids.get(&Reverse(dec!(100))).unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.front().unwrap().id, 1);
        assert_eq!(queue.back().unwrap().id, 2);
    }

    // ─── add_order: error paths ───────────────────────────────────────────

    #[test]
    fn duplicate_order_id_is_rejected() {
        let mut book = OrderBook::new();
        book.add_order(buy(42, dec!(100), dec!(1))).unwrap();
        let err = book.add_order(buy(42, dec!(100), dec!(1))).unwrap_err();
        assert_eq!(err, EngineError::DuplicateOrderId(42));
    }

    #[test]
    fn zero_price_is_rejected() {
        let mut book = OrderBook::new();
        let err = book.add_order(buy(1, dec!(0), dec!(1))).unwrap_err();
        assert_eq!(err, EngineError::InvalidPrice(dec!(0)));
    }

    #[test]
    fn negative_price_is_rejected() {
        let mut book = OrderBook::new();
        let err = book.add_order(buy(1, dec!(-1), dec!(1))).unwrap_err();
        assert_eq!(err, EngineError::InvalidPrice(dec!(-1)));
    }

    #[test]
    fn zero_amount_is_rejected() {
        let mut book = OrderBook::new();
        let err = book.add_order(sell(1, dec!(100), dec!(0))).unwrap_err();
        assert_eq!(err, EngineError::InvalidAmount(dec!(0)));
    }

    // ─── cancel_order: happy paths ────────────────────────────────────────

    #[test]
    fn cancel_buy_order_removes_from_bids() {
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(100), dec!(10))).unwrap();

        let cancelled = book.cancel_order(1).unwrap();
        assert_eq!(cancelled.id, 1);
        assert_eq!(book.len(), 0);
        assert!(book.best_bid().is_none());
    }

    #[test]
    fn cancel_sell_order_removes_from_asks() {
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(101), dec!(5))).unwrap();

        let cancelled = book.cancel_order(1).unwrap();
        assert_eq!(cancelled.id, 1);
        assert_eq!(book.len(), 0);
        assert!(book.best_ask().is_none());
    }

    #[test]
    fn cancel_removes_empty_price_level() {
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(100), dec!(1))).unwrap();
        book.cancel_order(1).unwrap();

        // Price level must be cleaned up so best_bid reflects reality
        assert!(book.bids.is_empty());
        assert!(book.best_bid().is_none());
    }

    #[test]
    fn cancel_middle_order_in_queue_preserves_others() {
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(100), dec!(1))).unwrap();
        book.add_order(buy(2, dec!(100), dec!(2))).unwrap();
        book.add_order(buy(3, dec!(100), dec!(3))).unwrap();

        // Cancel the middle order
        book.cancel_order(2).unwrap();

        let queue = book.bids.get(&Reverse(dec!(100))).unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.front().unwrap().id, 1);
        assert_eq!(queue.back().unwrap().id, 3);
        assert_eq!(book.len(), 2);
    }

    #[test]
    fn cancel_updates_best_bid_when_top_level_removed() {
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(101), dec!(1))).unwrap();
        book.add_order(buy(2, dec!(100), dec!(1))).unwrap();

        book.cancel_order(1).unwrap(); // remove the best bid
        assert_eq!(book.best_bid(), Some(dec!(100)));
    }

    // ─── cancel_order: error paths ────────────────────────────────────────

    #[test]
    fn cancel_nonexistent_order_returns_error() {
        let mut book = OrderBook::new();
        let err = book.cancel_order(99).unwrap_err();
        assert_eq!(err, EngineError::OrderNotFound(99));
    }

    #[test]
    fn cancel_same_order_twice_returns_error() {
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(100), dec!(1))).unwrap();
        book.cancel_order(1).unwrap();

        let err = book.cancel_order(1).unwrap_err();
        assert_eq!(err, EngineError::OrderNotFound(1));
    }
}
