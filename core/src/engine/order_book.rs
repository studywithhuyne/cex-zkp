use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, VecDeque};

use rust_decimal::Decimal;

use super::error::EngineError;
use super::types::{Order, Side, Trade};

pub type DepthLevel = (Decimal, Decimal);
pub type DepthSnapshot = (Vec<DepthLevel>, Vec<DepthLevel>);

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

    /// Return the top-`limit` price levels on each side as (price, total_remaining) pairs.
    ///
    /// Bids are returned highest-price-first; asks lowest-price-first.
    /// `total_remaining` is the sum of `remaining` across all orders at that level.
    /// Used by the API layer to serve the /api/orderbook depth snapshot.
    pub fn depth_snapshot(&self, limit: usize) -> DepthSnapshot {
        let bids = self
            .bids
            .iter()
            .take(limit)
            .map(|(Reverse(price), queue)| {
                let total: Decimal = queue.iter().map(|o| o.remaining).sum();
                (*price, total)
            })
            .collect();

        let asks = self
            .asks
            .iter()
            .take(limit)
            .map(|(price, queue)| {
                let total: Decimal = queue.iter().map(|o| o.remaining).sum();
                (*price, total)
            })
            .collect();

        (bids, asks)
    }

    /// Insert a limit order into the book **without** attempting to match it.
    ///
    /// Use this to place a resting (passive) order directly. For incoming
    /// aggressive orders that should match first, use `match_order`.
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

    /// Match an incoming taker order against resting orders in the book.
    ///
    /// Algorithm
    /// ─────────
    /// 1. Validate price > 0 and amount > 0.
    /// 2. Reject duplicate order IDs (taker must not already be in the book).
    /// 3. Loop — find the best opposite-side price level that crosses the taker:
    ///    - Buy  taker: best ask must be ≤ taker.price
    ///    - Sell taker: best bid must be ≥ taker.price
    /// 4. Take the front order at that level (FIFO priority within the level).
    /// 5. fill_qty = min(taker.remaining, maker.remaining)
    /// 6. Emit a Trade at the maker's limit price (price-time priority).
    /// 7. Decrement remaining on both sides.
    /// 8. If maker is fully filled → pop from queue, remove price level if
    ///    empty, remove from order_map.
    /// 9. Repeat until taker is filled or no crossing price level remains.
    /// 10. If taker still has remaining quantity → place it as a resting order.
    ///
    /// Returns the list of Trades generated (empty if no price crossing exists).
    pub fn match_order(&mut self, taker: Order) -> Result<Vec<Trade>, EngineError> {
        // --- Validation ---
        if taker.price <= Decimal::ZERO {
            return Err(EngineError::InvalidPrice(taker.price));
        }
        if taker.amount <= Decimal::ZERO {
            return Err(EngineError::InvalidAmount(taker.amount));
        }
        if self.order_map.contains_key(&taker.id) {
            return Err(EngineError::DuplicateOrderId(taker.id));
        }

        let mut taker = taker;
        let mut trades = Vec::new();

        match taker.side {
            Side::Buy => self.fill_buy(&mut taker, &mut trades),
            Side::Sell => self.fill_sell(&mut taker, &mut trades),
        }

        // Taker still has remaining quantity → becomes a resting limit order.
        if !taker.is_filled() {
            let price = taker.price;
            let id = taker.id;
            let side = taker.side;
            match side {
                Side::Buy => self.bids.entry(Reverse(price)).or_default().push_back(taker),
                Side::Sell => self.asks.entry(price).or_default().push_back(taker),
            }
            self.order_map.insert(id, (side, price));
        }

        Ok(trades)
    }

    // ── Private matching helpers ────────────────────────────────────────────

    /// Inner loop for a Buy taker: consume resting asks from lowest to highest.
    fn fill_buy(&mut self, taker: &mut Order, trades: &mut Vec<Trade>) {
        loop {
            // Best ask must be ≤ taker price for a price crossing.
            let best_ask = match self.asks.keys().next().copied() {
                Some(p) if p <= taker.price => p,
                _ => break,
            };

            // Inner scope so the mutable borrow of `self.asks` is released
            // before we potentially remove the price level below.
            let (fill_qty, maker_id, maker_filled) = {
                let queue = self.asks.get_mut(&best_ask).unwrap();
                let maker = queue.front_mut().unwrap();
                let fill_qty = taker.remaining.min(maker.remaining);
                maker.remaining -= fill_qty;
                taker.remaining -= fill_qty;
                (fill_qty, maker.id, maker.is_filled())
            };

            trades.push(Trade {
                maker_order_id: maker_id,
                taker_order_id: taker.id,
                symbol:         taker.symbol.clone(),
                price:          best_ask, // execution at the maker's (ask) price
                amount:         fill_qty,
            });

            if maker_filled {
                let queue = self.asks.get_mut(&best_ask).unwrap();
                queue.pop_front();
                if queue.is_empty() {
                    self.asks.remove(&best_ask);
                }
                self.order_map.remove(&maker_id);
            }

            if taker.is_filled() {
                break;
            }
        }
    }

    /// Inner loop for a Sell taker: consume resting bids from highest to lowest.
    fn fill_sell(&mut self, taker: &mut Order, trades: &mut Vec<Trade>) {
        loop {
            // Best bid must be ≥ taker price for a price crossing.
            let best_bid = match self.bids.keys().next().map(|Reverse(p)| *p) {
                Some(p) if p >= taker.price => p,
                _ => break,
            };

            // Inner scope so the mutable borrow of `self.bids` is released
            // before we potentially remove the price level below.
            let (fill_qty, maker_id, maker_filled) = {
                let queue = self.bids.get_mut(&Reverse(best_bid)).unwrap();
                let maker = queue.front_mut().unwrap();
                let fill_qty = taker.remaining.min(maker.remaining);
                maker.remaining -= fill_qty;
                taker.remaining -= fill_qty;
                (fill_qty, maker.id, maker.is_filled())
            };

            trades.push(Trade {
                maker_order_id: maker_id,
                taker_order_id: taker.id,
                symbol:         taker.symbol.clone(),
                price:          best_bid, // execution at the maker's (bid) price
                amount:         fill_qty,
            });

            if maker_filled {
                let queue = self.bids.get_mut(&Reverse(best_bid)).unwrap();
                queue.pop_front();
                if queue.is_empty() {
                    self.bids.remove(&Reverse(best_bid));
                }
                self.order_map.remove(&maker_id);
            }

            if taker.is_filled() {
                break;
            }
        }
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
        Order::new(id, 1, "BTC_USDT", Side::Buy, price, amount)
    }

    fn sell(id: u64, price: Decimal, amount: Decimal) -> Order {
        Order::new(id, 2, "BTC_USDT", Side::Sell, price, amount)
    }

    // ─── add_order ────────────────────────────────────────────────────────

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

    // ─── cancel_order ─────────────────────────────────────────────────────

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
        assert!(book.bids.is_empty());
        assert!(book.best_bid().is_none());
    }

    #[test]
    fn cancel_middle_order_in_queue_preserves_others() {
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(100), dec!(1))).unwrap();
        book.add_order(buy(2, dec!(100), dec!(2))).unwrap();
        book.add_order(buy(3, dec!(100), dec!(3))).unwrap();
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
        book.cancel_order(1).unwrap();
        assert_eq!(book.best_bid(), Some(dec!(100)));
    }

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

    // ─── match_order: no crossing ─────────────────────────────────────────

    #[test]
    fn no_crossing_buy_rests_on_book() {
        let mut book = OrderBook::new();
        // Sell at 101, buy at 100 → no crossing
        book.add_order(sell(1, dec!(101), dec!(10))).unwrap();
        let trades = book.match_order(buy(2, dec!(100), dec!(5))).unwrap();

        assert!(trades.is_empty());
        // Taker rests as a bid at 100
        assert_eq!(book.len(), 2);
        assert_eq!(book.best_bid(), Some(dec!(100)));
    }

    #[test]
    fn no_crossing_sell_rests_on_book() {
        let mut book = OrderBook::new();
        // Buy at 99, sell at 100 → no crossing
        book.add_order(buy(1, dec!(99), dec!(10))).unwrap();
        let trades = book.match_order(sell(2, dec!(100), dec!(5))).unwrap();

        assert!(trades.is_empty());
        assert_eq!(book.len(), 2);
        assert_eq!(book.best_ask(), Some(dec!(100)));
    }

    // ─── match_order: full fill ───────────────────────────────────────────

    #[test]
    fn full_fill_buy_taker_consumes_ask() {
        // Resting sell 10 @ 100. Taker buys 10 @ 100 → 1 trade, book empty.
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(100), dec!(10))).unwrap();

        let trades = book.match_order(buy(2, dec!(100), dec!(10))).unwrap();

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, dec!(100));
        assert_eq!(trades[0].amount, dec!(10));
        assert_eq!(trades[0].maker_order_id, 1);
        assert_eq!(trades[0].taker_order_id, 2);
        assert!(book.is_empty());
    }

    #[test]
    fn full_fill_sell_taker_consumes_bid() {
        // Resting buy 5 @ 101. Taker sells 5 @ 101 → 1 trade, book empty.
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(101), dec!(5))).unwrap();

        let trades = book.match_order(sell(2, dec!(101), dec!(5))).unwrap();

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, dec!(101));
        assert_eq!(trades[0].amount, dec!(5));
        assert!(book.is_empty());
    }

    // ─── match_order: partial fill ────────────────────────────────────────

    #[test]
    fn partial_fill_taker_larger_than_maker() {
        // Maker sells 3 @ 100. Taker buys 10 → 1 trade of 3, taker rests with 7.
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(100), dec!(3))).unwrap();

        let trades = book.match_order(buy(2, dec!(100), dec!(10))).unwrap();

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].amount, dec!(3));
        // Taker rests on bids with 7 remaining
        assert_eq!(book.len(), 1);
        assert_eq!(book.best_bid(), Some(dec!(100)));
        assert!(book.best_ask().is_none());
    }

    #[test]
    fn partial_fill_maker_larger_than_taker() {
        // Maker sells 10 @ 100. Taker buys 3 → 1 trade of 3, maker still resting with 7.
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(100), dec!(10))).unwrap();

        let trades = book.match_order(buy(2, dec!(100), dec!(3))).unwrap();

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].amount, dec!(3));
        // Maker still alive with 7 remaining
        assert_eq!(book.len(), 1);
        assert_eq!(book.best_ask(), Some(dec!(100)));
        let queue = book.asks.get(&dec!(100)).unwrap();
        assert_eq!(queue.front().unwrap().remaining, dec!(7));
    }

    // ─── match_order: walking the book ────────────────────────────────────

    #[test]
    fn buy_taker_walks_multiple_ask_levels() {
        // Asks: 5 @ 100, 5 @ 101, 5 @ 102
        // Taker buys 12 @ 102 → consumes level 100 (5), level 101 (5), 2 from 102.
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(100), dec!(5))).unwrap();
        book.add_order(sell(2, dec!(101), dec!(5))).unwrap();
        book.add_order(sell(3, dec!(102), dec!(5))).unwrap();

        let trades = book.match_order(buy(10, dec!(102), dec!(12))).unwrap();

        assert_eq!(trades.len(), 3);
        assert_eq!(trades[0].price, dec!(100));
        assert_eq!(trades[0].amount, dec!(5));
        assert_eq!(trades[1].price, dec!(101));
        assert_eq!(trades[1].amount, dec!(5));
        assert_eq!(trades[2].price, dec!(102));
        assert_eq!(trades[2].amount, dec!(2));
        // Level 100 and 101 fully consumed; level 102 still has 3 remaining.
        assert_eq!(book.best_ask(), Some(dec!(102)));
        assert_eq!(book.len(), 1);
    }

    #[test]
    fn sell_taker_walks_multiple_bid_levels() {
        // Bids: 5 @ 102, 5 @ 101, 5 @ 100
        // Taker sells 12 @ 100 → consumes level 102 (5), level 101 (5), 2 from 100.
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(102), dec!(5))).unwrap();
        book.add_order(buy(2, dec!(101), dec!(5))).unwrap();
        book.add_order(buy(3, dec!(100), dec!(5))).unwrap();

        let trades = book.match_order(sell(10, dec!(100), dec!(12))).unwrap();

        assert_eq!(trades.len(), 3);
        assert_eq!(trades[0].price, dec!(102));
        assert_eq!(trades[0].amount, dec!(5));
        assert_eq!(trades[1].price, dec!(101));
        assert_eq!(trades[1].amount, dec!(5));
        assert_eq!(trades[2].price, dec!(100));
        assert_eq!(trades[2].amount, dec!(2));
        // Level 102 and 101 consumed; level 100 still has 3 remaining.
        assert_eq!(book.best_bid(), Some(dec!(100)));
        assert_eq!(book.len(), 1);
    }

    // ─── match_order: execution price is always maker's price ─────────────

    #[test]
    fn execution_price_is_maker_price_not_taker() {
        // Maker posted ask at 100. Taker bids at 105 (aggressive).
        // Trade must execute at 100 (maker price), not 105.
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(100), dec!(10))).unwrap();

        let trades = book.match_order(buy(2, dec!(105), dec!(10))).unwrap();

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, dec!(100)); // maker's price
    }

    // ─── ENG-07: FIFO during matching ─────────────────────────────────────

    #[test]
    fn buy_taker_matches_makers_at_same_level_in_fifo_order() {
        // Three sellers all at price 100 placed in order A→B→C.
        // Taker buys 7 → A(qty=3) filled first, B(qty=3) filled second,
        // C partially filled (1 of 3), C must remain at front of queue.
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(100), dec!(3))).unwrap(); // maker A
        book.add_order(sell(2, dec!(100), dec!(3))).unwrap(); // maker B
        book.add_order(sell(3, dec!(100), dec!(3))).unwrap(); // maker C

        let trades = book.match_order(buy(10, dec!(100), dec!(7))).unwrap();

        assert_eq!(trades.len(), 3);
        assert_eq!(trades[0].maker_order_id, 1); // A first
        assert_eq!(trades[0].amount, dec!(3));
        assert_eq!(trades[1].maker_order_id, 2); // B second
        assert_eq!(trades[1].amount, dec!(3));
        assert_eq!(trades[2].maker_order_id, 3); // C third — partial
        assert_eq!(trades[2].amount, dec!(1));

        // C still resting at the front of the queue with 2 remaining
        let queue = book.asks.get(&dec!(100)).unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.front().unwrap().id, 3);
        assert_eq!(queue.front().unwrap().remaining, dec!(2));
    }

    #[test]
    fn sell_taker_matches_makers_at_same_level_in_fifo_order() {
        // Three buyers all at price 100 placed in order A→B→C.
        // Taker sells 7 → A(qty=3) filled first, B(qty=3) second,
        // C partially filled (1 of 3), C remains at front.
        let mut book = OrderBook::new();
        book.add_order(buy(1, dec!(100), dec!(3))).unwrap(); // maker A
        book.add_order(buy(2, dec!(100), dec!(3))).unwrap(); // maker B
        book.add_order(buy(3, dec!(100), dec!(3))).unwrap(); // maker C

        let trades = book.match_order(sell(10, dec!(100), dec!(7))).unwrap();

        assert_eq!(trades.len(), 3);
        assert_eq!(trades[0].maker_order_id, 1);
        assert_eq!(trades[0].amount, dec!(3));
        assert_eq!(trades[1].maker_order_id, 2);
        assert_eq!(trades[1].amount, dec!(3));
        assert_eq!(trades[2].maker_order_id, 3);
        assert_eq!(trades[2].amount, dec!(1));

        let queue = book.bids.get(&Reverse(dec!(100))).unwrap();
        assert_eq!(queue.front().unwrap().remaining, dec!(2));
    }

    // ─── ENG-07: full book exhaustion ─────────────────────────────────────

    #[test]
    fn buy_taker_exhausts_entire_ask_book_and_rests_with_leftover() {
        // Asks: 5@100, 5@101.  Taker buys 20@102 → consumes both levels (10
        // total), 10 remaining can't match and rests as a new bid @ 102.
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(100), dec!(5))).unwrap();
        book.add_order(sell(2, dec!(101), dec!(5))).unwrap();

        let trades = book.match_order(buy(10, dec!(102), dec!(20))).unwrap();

        assert_eq!(trades.len(), 2);
        let filled: Decimal = trades.iter().map(|t| t.amount).sum();
        assert_eq!(filled, dec!(10));

        // Ask side empty; taker's leftover rests as a bid
        assert!(book.best_ask().is_none());
        assert_eq!(book.best_bid(), Some(dec!(102)));
        assert_eq!(book.len(), 1);
    }

    // ─── ENG-07: partially-filled maker can be cancelled ──────────────────

    #[test]
    fn partially_filled_maker_can_be_cancelled() {
        // Maker sells 10@100. Taker buys 3 → maker has 7 remaining.
        // Maker is then cancelled and the returned order shows remaining=7.
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(100), dec!(10))).unwrap();
        book.match_order(buy(2, dec!(100), dec!(3))).unwrap();

        let cancelled = book.cancel_order(1).unwrap();
        assert_eq!(cancelled.remaining, dec!(7)); // not original amount=10
        assert!(book.is_empty());
    }

    // ─── ENG-07: conservation of quantity ─────────────────────────────────

    #[test]
    fn sum_of_trade_amounts_equals_taker_filled_quantity() {
        // Taker buys 12 against three separate ask levels (5+5+5).
        // The sum of all trade amounts must equal 12 (taker fully filled).
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(100), dec!(5))).unwrap();
        book.add_order(sell(2, dec!(101), dec!(5))).unwrap();
        book.add_order(sell(3, dec!(102), dec!(5))).unwrap();

        let trades = book.match_order(buy(10, dec!(102), dec!(12))).unwrap();

        let total_traded: Decimal = trades.iter().map(|t| t.amount).sum();
        assert_eq!(total_traded, dec!(12));
    }

    // ─── ENG-07: order_map consistency ────────────────────────────────────

    #[test]
    fn order_map_len_stays_consistent_through_add_match_cancel() {
        let mut book = OrderBook::new();

        // Add 4 resting orders
        book.add_order(sell(1, dec!(100), dec!(5))).unwrap();
        book.add_order(sell(2, dec!(101), dec!(5))).unwrap();
        book.add_order(buy(3, dec!(99), dec!(5))).unwrap();
        book.add_order(buy(4, dec!(98), dec!(5))).unwrap();
        assert_eq!(book.len(), 4);

        // match_order: taker buys 5@100 → fully fills maker 1 (removed),
        //              taker is filled, not added to book.
        book.match_order(buy(5, dec!(100), dec!(5))).unwrap();
        assert_eq!(book.len(), 3); // 1, 2, 3, 4 minus 1 = 3 remaining

        // Cancel one resting bid
        book.cancel_order(3).unwrap();
        assert_eq!(book.len(), 2); // 2 and 4 remain
    }

    // ─── ENG-07: decimal precision ────────────────────────────────────────

    #[test]
    fn fractional_decimal_amounts_fill_correctly() {
        // Maker sells 0.005 @ 100.50. Taker buys 0.005 @ 100.50.
        // Exact fill with fractional decimals — no rounding loss.
        let mut book = OrderBook::new();
        book.add_order(sell(1, dec!(100.50), dec!(0.005))).unwrap();

        let trades = book.match_order(buy(2, dec!(100.50), dec!(0.005))).unwrap();

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].amount, dec!(0.005));
        assert_eq!(trades[0].price, dec!(100.50));
        assert!(book.is_empty());
    }
}

/// Property-based tests (ENG-08): generate thousands of random order sequences
/// and assert the conservation invariant — no quantity is ever created or lost.
#[cfg(test)]
mod proptest_suite {
    use super::*;
    use proptest::prelude::*;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    /// Sum the `remaining` field of every live order across both sides of the book.
    fn total_remaining_in_book(book: &OrderBook) -> Decimal {
        let bid_rem: Decimal = book
            .bids
            .values()
            .flat_map(|q| q.iter())
            .map(|o| o.remaining)
            .sum();
        let ask_rem: Decimal = book
            .asks
            .values()
            .flat_map(|q| q.iter())
            .map(|o| o.remaining)
            .sum();
        bid_rem + ask_rem
    }

    proptest! {
        /// Conservation of quantity invariant:
        ///
        /// For any sequence of match_order calls, the following must always hold:
        ///
        ///   sum(order.amount) == 2 × sum(trade.amount) + sum(remaining in book)
        ///
        /// Rationale: each trade decrements `remaining` on BOTH maker and taker
        /// by exactly fill_qty.  Summing all initial amounts and subtracting all
        /// matched amounts (counted once per side) leaves only the resting quantity.
        #[test]
        fn no_quantity_created_or_destroyed(
            input in proptest::collection::vec(
                // (is_buy, price ∈ [90, 110], amount ∈ [1, 10])
                // Prices intentionally overlap to guarantee frequent matches.
                (any::<bool>(), 90u32..=110u32, 1u32..=10u32),
                1..=100
            )
        ) {
            let mut book = OrderBook::new();
            let mut total_initial = Decimal::ZERO;
            let mut total_traded  = Decimal::ZERO;

            for (i, (is_buy, price_raw, amount_raw)) in input.iter().enumerate() {
                let id     = (i + 1) as u64;
                let price  = Decimal::from(*price_raw);
                let amount = Decimal::from(*amount_raw);
                let side   = if *is_buy { Side::Buy } else { Side::Sell };

                total_initial += amount;

                // Sequential IDs guarantee no DuplicateOrderId.
                // price >= 90 > 0 and amount >= 1 > 0, so no validation errors.
                let order  = Order::new(id, 1, "BTC_USDT", side, price, amount);
                let trades = book.match_order(order)
                    .expect("randomly generated order must not produce EngineError");

                for trade in &trades {
                    // Every fill must be a positive quantity.
                    prop_assert!(
                        trade.amount > Decimal::ZERO,
                        "trade amount must be positive, got {}",
                        trade.amount
                    );
                    total_traded += trade.amount;
                }
            }

            let total_remaining = total_remaining_in_book(&book);

            // Core conservation invariant.
            prop_assert_eq!(
                total_initial,
                total_traded * dec!(2) + total_remaining,
                "conservation violated — initial={} 2×traded={} remaining={}",
                total_initial,
                total_traded * dec!(2),
                total_remaining
            );

            // Internal consistency: order_map must stay in sync with BTreeMap queues.
            let queue_total: usize = book.bids.values().map(|q| q.len()).sum::<usize>()
                + book.asks.values().map(|q| q.len()).sum::<usize>();
            prop_assert_eq!(
                book.len(),
                queue_total,
                "order_map out of sync with price-level queues"
            );
        }

        /// The engine must never panic for any sequence of valid orders.
        /// This test is a lighter-weight sanity check complementing the
        /// conservation test above.
        #[test]
        fn engine_never_panics_on_valid_orders(
            input in proptest::collection::vec(
                (any::<bool>(), 1u32..=200u32, 1u32..=50u32),
                0..=200
            )
        ) {
            let mut book = OrderBook::new();
            for (i, (is_buy, price_raw, amount_raw)) in input.iter().enumerate() {
                let id     = (i + 1) as u64;
                let price  = Decimal::from(*price_raw);
                let amount = Decimal::from(*amount_raw);
                let side   = if *is_buy { Side::Buy } else { Side::Sell };
                let order  = Order::new(id, 1, "BTC_USDT", side, price, amount);
                let _      = book.match_order(order);
            }
            // If we reach here without a panic, the test passes.
        }
    }
}
