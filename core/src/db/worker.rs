// core/src/db/worker.rs
// Async background worker: drains a tokio mpsc channel and persists
// trading events to PostgreSQL without ever blocking the matching engine.
//
// ┌───────────────────────────────────────────────────────────────────────┐
// │  Caller ordering contract (MUST be respected at the call site)        │
// │                                                                       │
// │  1. Send OrderPlaced for the TAKER order first.                       │
// │  2. Then send TradeFilled for every fill that references it.          │
// │                                                                       │
// │  Maker orders were already sent as OrderPlaced when they were         │
// │  submitted, so their rows exist in orders_log before any fill lands.  │
// │  This preserves the FK:  trades_log.{maker,taker}_order_id            │
// │      → orders_log.order_id                                            │
// └───────────────────────────────────────────────────────────────────────┘

use chrono::{DateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::engine::{Order, Side, Trade};

// ─────────────────────────────────────────────────────────────────────────────
// Event type
// ─────────────────────────────────────────────────────────────────────────────

/// Events produced by the API layer and consumed by the persistence worker.
/// Every variant is self-contained so the worker holds no engine lock.
#[derive(Debug)]
pub enum PersistenceEvent {
    /// A new limit order was submitted — log it as 'open' before any fills.
    OrderPlaced {
        order:       Order,
        base_asset:  String,
        quote_asset: String,
    },

    /// A fill event from the matching engine.
    /// MUST be sent AFTER the OrderPlaced event for the taker order.
    TradeFilled {
        trade:         Trade,
        maker_user_id: u64,
        taker_user_id: u64,
        /// Side of the TAKER order; determines buyer/seller for balance updates.
        taker_side:    Side,
        base_asset:    String,
        quote_asset:   String,
    },

    /// User cancelled their resting order.
    OrderCancelled { order_id: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Spawn the persistence worker on the tokio runtime.
///
/// Returns the `Sender` (to be injected into Axum `AppState` in API-01)
/// and the task `JoinHandle` (hold it to keep the worker alive).
///
/// # Arguments
/// * `pool`   — cloned PgPool (Arc-backed, clone is O(1)).
/// * `buffer` — mpsc channel capacity; callers block when full (backpressure).
pub fn spawn_persistence_worker(
    pool:   PgPool,
    buffer: usize,
) -> (mpsc::Sender<PersistenceEvent>, tokio::task::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(buffer);
    let handle = tokio::spawn(run_worker(pool, rx));
    (tx, handle)
}

/// Recommended channel buffer for typical CEX load.
pub const WORKER_BUFFER: usize = 1_024;

// ─────────────────────────────────────────────────────────────────────────────
// Worker loop
// ─────────────────────────────────────────────────────────────────────────────

async fn run_worker(pool: PgPool, mut rx: mpsc::Receiver<PersistenceEvent>) {
    info!("Persistence worker started");

    while let Some(event) = rx.recv().await {
        match event {
            PersistenceEvent::OrderPlaced { order, base_asset, quote_asset } => {
                if let Err(e) = insert_order(&pool, &order, &base_asset, &quote_asset).await {
                    error!(error = ?e, order_id = order.id, "Failed to log order placement");
                }
            }

            PersistenceEvent::TradeFilled {
                trade,
                maker_user_id,
                taker_user_id,
                taker_side,
                base_asset,
                quote_asset,
            } => {
                // Insert the trade record first …
                if let Err(e) = insert_trade(
                    &pool,
                    &trade,
                    maker_user_id,
                    taker_user_id,
                    &base_asset,
                    &quote_asset,
                )
                .await
                {
                    error!(error = ?e, maker = trade.maker_order_id, taker = trade.taker_order_id, "Failed to log trade");
                }

                // … then update the filled counters on both sides.
                if let Err(e) = apply_fill(&pool, trade.maker_order_id, trade.amount).await {
                    error!(error = ?e, order_id = trade.maker_order_id, "Failed to update maker fill");
                }
                if let Err(e) = apply_fill(&pool, trade.taker_order_id, trade.amount).await {
                    error!(error = ?e, order_id = trade.taker_order_id, "Failed to update taker fill");
                }

                // … finally update user balances (skip self-trades — net effect is zero).
                if maker_user_id != taker_user_id {
                    let (buyer_id, seller_id) = match taker_side {
                        Side::Buy  => (taker_user_id, maker_user_id),
                        Side::Sell => (maker_user_id, taker_user_id),
                    };
                    if let Err(e) = update_balances(
                        &pool,
                        buyer_id,
                        seller_id,
                        &base_asset,
                        &quote_asset,
                        trade.amount,
                        trade.price,
                    )
                    .await
                    {
                        error!(error = ?e, buyer = buyer_id, seller = seller_id, "Failed to update balances after trade");
                    }
                }

                // … finally aggregate into OHLCV candles for all standard intervals.
                let symbol   = format!("{}_{}", base_asset, quote_asset);
                let trade_ts = Utc::now();
                for &(label, secs) in CANDLE_INTERVALS {
                    let open_time = floor_to_interval_secs(trade_ts, secs);
                    if let Err(e) = upsert_candle(
                        &pool, &symbol, label, open_time, trade.price, trade.amount,
                    )
                    .await
                    {
                        error!(
                            error = ?e, symbol = %symbol, interval = label,
                            "Failed to upsert OHLCV candle"
                        );
                    }
                }
            }

            PersistenceEvent::OrderCancelled { order_id } => {
                if let Err(e) = mark_cancelled(&pool, order_id).await {
                    error!(error = ?e, order_id, "Failed to log order cancellation");
                }
            }
        }
    }

    info!("Persistence worker shut down (channel closed)");
}

// ─────────────────────────────────────────────────────────────────────────────
// DB helpers
//
// Using sqlx::query (dynamic) rather than sqlx::query! (compile-time) because
// the latter requires DATABASE_URL at build time or a pre-generated .sqlx cache
// (`cargo sqlx prepare`).  Queries can be migrated to sqlx::query! once offline
// mode is configured.
//
// PostgreSQL ENUM columns (order_side, order_status) receive TEXT values from
// Rust and are cast in-SQL with the `::type_name` syntax.
// ─────────────────────────────────────────────────────────────────────────────

#[inline]
fn side_str(side: Side) -> &'static str {
    match side {
        Side::Buy  => "buy",
        Side::Sell => "sell",
    }
}

/// INSERT INTO orders_log — idempotent via ON CONFLICT DO NOTHING.
async fn insert_order(
    pool:        &PgPool,
    order:       &Order,
    base_asset:  &str,
    quote_asset: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO orders_log
            (order_id, user_id, side, price, amount, filled, status, base_asset, quote_asset)
        VALUES
            ($1, $2, $3::order_side, $4, $5, 0, 'open'::order_status, $6, $7)
        ON CONFLICT (order_id) DO NOTHING
        "#,
    )
    .bind(order.id      as i64)
    .bind(order.user_id as i64)
    .bind(side_str(order.side))
    .bind(order.price)
    .bind(order.amount)
    .bind(base_asset)
    .bind(quote_asset)
    .execute(pool)
    .await?;

    Ok(())
}

/// INSERT INTO trades_log.
async fn insert_trade(
    pool:          &PgPool,
    trade:         &Trade,
    maker_user_id: u64,
    taker_user_id: u64,
    base_asset:    &str,
    quote_asset:   &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO trades_log
            (maker_order_id, taker_order_id, maker_user_id, taker_user_id,
             price, amount, base_asset, quote_asset)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(trade.maker_order_id as i64)
    .bind(trade.taker_order_id as i64)
    .bind(maker_user_id as i64)
    .bind(taker_user_id as i64)
    .bind(trade.price)
    .bind(trade.amount)
    .bind(base_asset)
    .bind(quote_asset)
    .execute(pool)
    .await?;

    Ok(())
}

/// UPDATE orders_log: add `filled_qty` to `filled`; advance status to
/// 'partial' or 'filled'.  Uses the pre-update `filled` value in the CASE
/// expression (PostgreSQL evaluates SET clauses against the original row).
async fn apply_fill(
    pool:       &PgPool,
    order_id:   u64,
    filled_qty: Decimal,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE orders_log
        SET
            filled     = filled + $1,
            status     = CASE
                             WHEN filled + $1 >= amount THEN 'filled'::order_status
                             ELSE                            'partial'::order_status
                         END,
            updated_at = now()
        WHERE order_id = $2
        "#,
    )
    .bind(filled_qty)
    .bind(order_id as i64)
    .execute(pool)
    .await?;

    Ok(())
}

/// UPDATE orders_log SET status = 'cancelled'.
async fn mark_cancelled(pool: &PgPool, order_id: u64) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE orders_log
        SET status = 'cancelled'::order_status, updated_at = now()
        WHERE order_id = $1
        "#,
    )
    .bind(order_id as i64)
    .execute(pool)
    .await?;

    Ok(())
}

/// UPDATE balances for both sides of a completed trade.
///
/// Buyer  receives `amount` of base_asset  (BTC) and spends `amount * price` of quote_asset (USDT).
/// Seller receives `amount * price` USDT   and spends `amount` BTC.
async fn update_balances(
    pool:           &PgPool,
    buyer_user_id:  u64,
    seller_user_id: u64,
    base_asset:     &str,    // e.g. "BTC"
    quote_asset:    &str,    // e.g. "USDT"
    amount:         Decimal, // BTC quantity traded
    price:          Decimal, // execution price
) -> Result<(), sqlx::Error> {
    let quote_amount = amount * price;

    // Buyer gains BTC
    sqlx::query(
        "UPDATE balances SET available = available + $1, updated_at = now()
         WHERE user_id = $2 AND asset_symbol = $3",
    )
    .bind(amount)
    .bind(buyer_user_id as i64)
    .bind(base_asset)
    .execute(pool)
    .await?;

    // Buyer spends USDT
    sqlx::query(
        "UPDATE balances SET available = available - $1, updated_at = now()
         WHERE user_id = $2 AND asset_symbol = $3",
    )
    .bind(quote_amount)
    .bind(buyer_user_id as i64)
    .bind(quote_asset)
    .execute(pool)
    .await?;

    // Seller spends BTC
    sqlx::query(
        "UPDATE balances SET available = available - $1, updated_at = now()
         WHERE user_id = $2 AND asset_symbol = $3",
    )
    .bind(amount)
    .bind(seller_user_id as i64)
    .bind(base_asset)
    .execute(pool)
    .await?;

    // Seller gains USDT
    sqlx::query(
        "UPDATE balances SET available = available + $1, updated_at = now()
         WHERE user_id = $2 AND asset_symbol = $3",
    )
    .bind(quote_amount)
    .bind(seller_user_id as i64)
    .bind(quote_asset)
    .execute(pool)
    .await?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// OHLCV helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Standard candlestick intervals: (label, duration_in_seconds).
/// Each trade fill is aggregated into all four intervals simultaneously.
const CANDLE_INTERVALS: &[(&str, i64)] = &[
    ("1m",  60),
    ("5m",  300),
    ("1h",  3_600),
    ("1d",  86_400),
];

/// Floor `time` down to the nearest multiple of `interval_secs`.
/// E.g. 12:47:33 with interval=60 → 12:47:00.
fn floor_to_interval_secs(time: DateTime<Utc>, interval_secs: i64) -> DateTime<Utc> {
    let ts      = time.timestamp();
    let floored = (ts / interval_secs) * interval_secs;
    Utc.timestamp_opt(floored, 0)
        .single()
        .unwrap_or(time)
}

/// UPSERT one trade into the appropriate OHLCV candle row.
///
/// On INSERT: sets open = high = low = close = `price`, volume = `amount`.
/// On CONFLICT (same symbol+interval+open_time):
///   - high  = GREATEST(existing, new)
///   - low   = LEAST(existing, new)
///   - close = new price (last trade wins — sequential worker guarantees order)
///   - volume accumulates
async fn upsert_candle(
    pool:      &PgPool,
    symbol:    &str,
    interval:  &str,
    open_time: DateTime<Utc>,
    price:     Decimal,
    amount:    Decimal,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO candles (symbol, interval, open_time, open, high, low, close, volume)
        VALUES ($1, $2, $3, $4, $4, $4, $4, $5)
        ON CONFLICT (symbol, interval, open_time) DO UPDATE SET
            high   = GREATEST(candles.high,  EXCLUDED.high),
            low    = LEAST(candles.low,   EXCLUDED.low),
            close  = EXCLUDED.close,
            volume = candles.volume + EXCLUDED.volume
        "#,
    )
    .bind(symbol)
    .bind(interval)
    .bind(open_time)
    .bind(price)
    .bind(amount)
    .execute(pool)
    .await?;

    Ok(())
}
