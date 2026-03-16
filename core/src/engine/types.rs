use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Order side: Buy (bid) or Sell (ask).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

/// A limit order placed by a user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Order {
    pub id:        u64,
    pub user_id:   u64,
    /// Trading pair symbol, e.g. "BTC_USDT". Used for multi-symbol routing.
    pub symbol:    String,
    pub side:      Side,
    /// Limit price (rust_decimal — never f32/f64).
    pub price:     Decimal,
    /// Original order quantity.
    pub amount:    Decimal,
    /// Unfilled quantity remaining; decremented on each partial fill.
    pub remaining: Decimal,
}

impl Order {
    pub fn new(
        id:      u64,
        user_id: u64,
        symbol:  impl Into<String>,
        side:    Side,
        price:   Decimal,
        amount:  Decimal,
    ) -> Self {
        Self {
            id,
            user_id,
            symbol: symbol.into(),
            side,
            price,
            amount,
            remaining: amount,
        }
    }

    /// Returns true when the order has been completely filled.
    pub fn is_filled(&self) -> bool {
        self.remaining.is_zero()
    }
}

/// A matched trade produced by the matching engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trade {
    /// The passive (resting) order that provided liquidity.
    pub maker_order_id: u64,
    /// The aggressive (incoming) order that consumed liquidity.
    pub taker_order_id: u64,
    /// Trading pair symbol, e.g. "BTC_USDT".
    pub symbol:         String,
    /// Execution price — always the maker's limit price.
    pub price:          Decimal,
    /// Quantity exchanged in this fill.
    pub amount:         Decimal,
}
