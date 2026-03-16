// core/src/db/schema.rs
// Rust mirror of every PostgreSQL table.
// Each struct derives sqlx::FromRow so it can be populated directly by sqlx::query_as!.
//
// Type mapping:
//   BIGINT        -> i64   (PostgreSQL BIGINT is signed; u64 user IDs are cast on the boundary)
//   UUID          -> uuid::Uuid
//   TEXT          -> String
//   NUMERIC(30,8) -> rust_decimal::Decimal  (via sqlx "rust_decimal" feature)
//   TIMESTAMPTZ   -> chrono::DateTime<Utc>  (via sqlx "chrono" feature)
//   SMALLINT      -> i16
//   order_side / order_status ENUMs -> String  (DB enforces validity; avoids custom sqlx type registration)

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// users
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    /// Matches the `x-user-id` header value (u64 cast to i64 at the API boundary).
    pub id:         i64,
    pub username:   String,
    pub created_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────
// assets
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct Asset {
    pub symbol:     String,   // e.g. "BTC"
    pub name:       String,   // e.g. "Bitcoin"
    pub decimals:   i16,
    pub created_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────
// balances
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct Balance {
    pub user_id:      i64,
    pub asset_symbol: String,
    /// Funds available for new orders.
    pub available:    Decimal,
    /// Funds currently reserved by open orders.
    pub locked:       Decimal,
    pub updated_at:   DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────
// orders_log
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct OrderLog {
    pub id:          Uuid,
    /// Matches `Order.id` in the in-memory engine (u64 cast to i64).
    pub order_id:    i64,
    pub user_id:     i64,
    /// "buy" | "sell"  — DB enforces via `order_side` ENUM.
    pub side:        String,
    pub price:       Decimal,
    pub amount:      Decimal,
    pub filled:      Decimal,
    /// "open" | "partial" | "filled" | "cancelled" — DB enforces via `order_status` ENUM.
    pub status:      String,
    pub base_asset:  String,
    pub quote_asset: String,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────
// candles
// ─────────────────────────────────────────────────────────────────────────────

/// One OHLCV candle row from the `candles` table.
/// Written by the persistence worker; never touched during live matching.
#[derive(Debug, sqlx::FromRow)]
pub struct Candle {
    pub symbol:    String,
    /// Interval label: "1m", "5m", "1h", or "1d".
    pub interval:  String,
    /// Start timestamp of this candle (UTC, floored to interval boundary).
    pub open_time: DateTime<Utc>,
    pub open:      Decimal,
    pub high:      Decimal,
    pub low:       Decimal,
    pub close:     Decimal,
    pub volume:    Decimal,
}

// ─────────────────────────────────────────────────────────────────────────────
// trades_log
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct TradeLog {
    pub id:             Uuid,
    /// FK -> orders_log.order_id (the resting maker).
    pub maker_order_id: i64,
    /// FK -> orders_log.order_id (the aggressing taker).
    pub taker_order_id: i64,
    pub maker_user_id:  i64,
    pub taker_user_id:  i64,
    pub price:          Decimal,
    pub amount:         Decimal,
    pub base_asset:     String,
    pub quote_asset:    String,
    pub executed_at:    DateTime<Utc>,
}
