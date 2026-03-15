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
