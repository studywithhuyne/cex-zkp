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

    /// Insert a limit order into the book.
    ///
    /// Algorithm
    /// ─────────
    /// 1. Validate: price > 0 and amount > 0.
    /// 2. Reject duplicate order IDs (order_map lookup, O(1)).
    /// 3. Route to the correct side (bids / asks).
    /// 4. Push the order to the back of the VecDeque at its price level,
    ///    preserving FIFO priority within each level.
    /// 5. Register order_id → price in order_map for O(1) cancel lookup.
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

        match order.side {
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
        self.order_map.insert(id, price);

        Ok(())
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

        // best_bid must be 101 (highest)
        assert_eq!(book.best_bid(), Some(dec!(101)));
    }

    #[test]
    fn asks_sorted_lowest_first() {
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(102), dec!(1))).unwrap();
        book.add_order(sell(2, dec!(100), dec!(1))).unwrap();
        book.add_order(sell(3, dec!(101), dec!(1))).unwrap();

        // best_ask must be 100 (lowest)
        assert_eq!(book.best_ask(), Some(dec!(100)));
    }

    #[test]
    fn multiple_orders_at_same_price_level_fifo() {
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(100), dec!(5))).unwrap();
        book.add_order(buy(2, dec!(100), dec!(3))).unwrap();

        // Both orders at the same price level → queue length is 2
        let queue = book.bids.get(&Reverse(dec!(100))).unwrap();
        assert_eq!(queue.len(), 2);
        // FIFO: first inserted is at the front
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
}
