use std::collections::HashMap;

use blake3::Hasher;
use rust_decimal::Decimal;
use thiserror::Error;

use crate::engine::{Order, Side, Trade};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalanceState {
    pub free: Decimal,
    pub locked: Decimal,
}

#[derive(Debug, Clone)]
pub struct UserBalanceSnapshot {
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
}

#[derive(Debug, Clone)]
struct Reservation {
    side: Side,
    user_id: u64,
    base_asset: String,
    quote_asset: String,
    limit_price: Decimal,
    remaining: Decimal,
    internal_id: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LedgerError {
    #[error("insufficient free balance: user_id={user_id}, asset={asset}, required={required}, available={available}")]
    InsufficientFreeBalance {
        user_id: u64,
        asset: String,
        required: Decimal,
        available: Decimal,
    },

    #[error("order reservation not found: order_id={0}")]
    ReservationNotFound(u64),

    #[error("reservation underflow: order_id={order_id}, remaining={remaining}, fill={fill}")]
    ReservationUnderflow {
        order_id: u64,
        remaining: Decimal,
        fill: Decimal,
    },

    #[error("invalid user id in balances snapshot")]
    InvalidUserId,
}

#[derive(Debug, Default)]
pub struct InMemoryLedger {
    balances: HashMap<(u64, String), BalanceState>,
    reservations: HashMap<u64, Reservation>,
    nonce: u64,
}

impl InMemoryLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_rows(rows: &[(i64, String, Decimal, Decimal)]) -> Result<Self, LedgerError> {
        let mut ledger = Self::new();

        for (user_id_i64, asset, free, locked) in rows {
            if *user_id_i64 <= 0 {
                return Err(LedgerError::InvalidUserId);
            }
            let user_id = *user_id_i64 as u64;
            ledger.balances.insert(
                (user_id, asset.clone()),
                BalanceState {
                    free: *free,
                    locked: *locked,
                },
            );
        }

        Ok(ledger)
    }

    pub fn reserve_for_new_order(
        &mut self,
        order: &Order,
        base_asset: &str,
        quote_asset: &str,
    ) -> Result<(), LedgerError> {
        let (asset_to_lock, lock_amount) = match order.side {
            Side::Buy => (quote_asset.to_string(), order.price * order.amount),
            Side::Sell => (base_asset.to_string(), order.amount),
        };

        self.move_free_to_locked(order.user_id, &asset_to_lock, lock_amount)?;

        let reservation = Reservation {
            side: order.side,
            user_id: order.user_id,
            base_asset: base_asset.to_string(),
            quote_asset: quote_asset.to_string(),
            limit_price: order.price,
            remaining: order.amount,
            internal_id: self.next_internal_id(order.id),
        };

        self.reservations.insert(order.id, reservation);
        Ok(())
    }

    pub fn cancel_reservation(&mut self, order_id: u64) -> Result<(), LedgerError> {
        let reservation = self
            .reservations
            .remove(&order_id)
            .ok_or(LedgerError::ReservationNotFound(order_id))?;

        match reservation.side {
            Side::Buy => {
                let release = reservation.remaining * reservation.limit_price;
                self.move_locked_to_free(reservation.user_id, &reservation.quote_asset, release)?;
            }
            Side::Sell => {
                self.move_locked_to_free(reservation.user_id, &reservation.base_asset, reservation.remaining)?;
            }
        }

        Ok(())
    }

    pub fn apply_trade_fill(&mut self, trade: &Trade) -> Result<(), LedgerError> {
        self.apply_fill_for_order(trade.maker_order_id, trade.amount, trade.price)?;
        self.apply_fill_for_order(trade.taker_order_id, trade.amount, trade.price)?;
        Ok(())
    }

    pub fn balances_for_user(&self, user_id: u64) -> Vec<UserBalanceSnapshot> {
        let mut out: Vec<UserBalanceSnapshot> = self
            .balances
            .iter()
            .filter_map(|((uid, asset), state)| {
                if *uid != user_id {
                    return None;
                }
                Some(UserBalanceSnapshot {
                    asset: asset.clone(),
                    free: state.free,
                    locked: state.locked,
                })
            })
            .collect();

        out.sort_by(|a, b| a.asset.cmp(&b.asset));
        out
    }

    pub fn deposit(&mut self, user_id: u64, asset: &str, amount: Decimal) -> Result<Decimal, LedgerError> {
        let key = (user_id, asset.to_string());
        let entry = self
            .balances
            .entry(key)
            .or_insert(BalanceState {
                free: Decimal::ZERO,
                locked: Decimal::ZERO,
            });
        entry.free += amount;
        Ok(entry.free)
    }

    fn apply_fill_for_order(&mut self, order_id: u64, fill_qty: Decimal, exec_price: Decimal) -> Result<(), LedgerError> {
        let reservation = self
            .reservations
            .get(&order_id)
            .cloned()
            .ok_or(LedgerError::ReservationNotFound(order_id))?;

        if fill_qty > reservation.remaining {
            return Err(LedgerError::ReservationUnderflow {
                order_id,
                remaining: reservation.remaining,
                fill: fill_qty,
            });
        }

        match reservation.side {
            Side::Buy => {
                let max_quote_for_fill = reservation.limit_price * fill_qty;
                let spent_quote = exec_price * fill_qty;
                let refund = max_quote_for_fill - spent_quote;

                self.decrease_locked(reservation.user_id, &reservation.quote_asset, max_quote_for_fill)?;
                if refund > Decimal::ZERO {
                    self.increase_free(reservation.user_id, &reservation.quote_asset, refund);
                }
                self.increase_free(reservation.user_id, &reservation.base_asset, fill_qty);
            }
            Side::Sell => {
                let proceeds = exec_price * fill_qty;
                self.decrease_locked(reservation.user_id, &reservation.base_asset, fill_qty)?;
                self.increase_free(reservation.user_id, &reservation.quote_asset, proceeds);
            }
        }

        let should_remove = {
            let entry = self
                .reservations
                .get_mut(&order_id)
                .ok_or(LedgerError::ReservationNotFound(order_id))?;
            entry.remaining -= fill_qty;
            let _ = &entry.internal_id;
            entry.remaining.is_zero()
        };

        if should_remove {
            self.reservations.remove(&order_id);
        }

        Ok(())
    }

    fn move_free_to_locked(&mut self, user_id: u64, asset: &str, amount: Decimal) -> Result<(), LedgerError> {
        let key = (user_id, asset.to_string());
        let entry = self
            .balances
            .entry(key.clone())
            .or_insert(BalanceState {
                free: Decimal::ZERO,
                locked: Decimal::ZERO,
            });

        if entry.free < amount {
            return Err(LedgerError::InsufficientFreeBalance {
                user_id,
                asset: asset.to_string(),
                required: amount,
                available: entry.free,
            });
        }

        entry.free -= amount;
        entry.locked += amount;
        Ok(())
    }

    fn move_locked_to_free(&mut self, user_id: u64, asset: &str, amount: Decimal) -> Result<(), LedgerError> {
        self.decrease_locked(user_id, asset, amount)?;
        self.increase_free(user_id, asset, amount);
        Ok(())
    }

    fn decrease_locked(&mut self, user_id: u64, asset: &str, amount: Decimal) -> Result<(), LedgerError> {
        let key = (user_id, asset.to_string());
        let entry = self
            .balances
            .entry(key.clone())
            .or_insert(BalanceState {
                free: Decimal::ZERO,
                locked: Decimal::ZERO,
            });

        if entry.locked < amount {
            return Err(LedgerError::InsufficientFreeBalance {
                user_id,
                asset: asset.to_string(),
                required: amount,
                available: entry.locked,
            });
        }

        entry.locked -= amount;
        Ok(())
    }

    fn increase_free(&mut self, user_id: u64, asset: &str, amount: Decimal) {
        let key = (user_id, asset.to_string());
        let entry = self
            .balances
            .entry(key)
            .or_insert(BalanceState {
                free: Decimal::ZERO,
                locked: Decimal::ZERO,
            });
        entry.free += amount;
    }

    fn next_internal_id(&mut self, seed_order_id: u64) -> String {
        self.nonce = self.nonce.wrapping_add(1);
        let mut hasher = Hasher::new();
        hasher.update(seed_order_id.to_string().as_bytes());
        hasher.update(self.nonce.to_string().as_bytes());
        hasher.finalize().to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    #[test]
    fn reserve_and_cancel_buy_moves_between_free_and_locked() {
        let rows = vec![(1_i64, "USDT".to_string(), dec!(1000), dec!(0))];
        let mut ledger = InMemoryLedger::from_rows(&rows).unwrap();
        let order = Order::new(10, 1, "BTC_USDT", Side::Buy, dec!(100), dec!(2));

        ledger.reserve_for_new_order(&order, "BTC", "USDT").unwrap();
        let usdt_after_reserve = ledger
            .balances_for_user(1)
            .into_iter()
            .find(|b| b.asset == "USDT")
            .unwrap();
        assert_eq!(usdt_after_reserve.free, dec!(800));
        assert_eq!(usdt_after_reserve.locked, dec!(200));

        ledger.cancel_reservation(10).unwrap();
        let usdt_after_cancel = ledger
            .balances_for_user(1)
            .into_iter()
            .find(|b| b.asset == "USDT")
            .unwrap();
        assert_eq!(usdt_after_cancel.free, dec!(1000));
        assert_eq!(usdt_after_cancel.locked, dec!(0));
    }

    #[test]
    fn fill_buy_trade_refunds_price_improvement() {
        let rows = vec![
            (1_i64, "USDT".to_string(), dec!(1000), dec!(0)),
            (1_i64, "BTC".to_string(), dec!(0), dec!(0)),
            (2_i64, "BTC".to_string(), dec!(2), dec!(0)),
            (2_i64, "USDT".to_string(), dec!(0), dec!(0)),
        ];
        let mut ledger = InMemoryLedger::from_rows(&rows).unwrap();

        let taker = Order::new(100, 1, "BTC_USDT", Side::Buy, dec!(100), dec!(1));
        let maker = Order::new(99, 2, "BTC_USDT", Side::Sell, dec!(95), dec!(1));

        ledger.reserve_for_new_order(&maker, "BTC", "USDT").unwrap();
        ledger.reserve_for_new_order(&taker, "BTC", "USDT").unwrap();

        let trade = Trade {
            maker_order_id: 99,
            taker_order_id: 100,
            symbol: "BTC_USDT".to_string(),
            price: dec!(95),
            amount: dec!(1),
        };

        ledger.apply_trade_fill(&trade).unwrap();

        let buyer_usdt = ledger
            .balances_for_user(1)
            .into_iter()
            .find(|b| b.asset == "USDT")
            .unwrap();
        let buyer_btc = ledger
            .balances_for_user(1)
            .into_iter()
            .find(|b| b.asset == "BTC")
            .unwrap();

        assert_eq!(buyer_usdt.locked, dec!(0));
        assert_eq!(buyer_usdt.free, dec!(905));
        assert_eq!(buyer_btc.free, dec!(1));
    }
}
