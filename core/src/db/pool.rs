// core/src/db/pool.rs
// Builds a sqlx PgPool from DATABASE_URL and runs pending migrations.
// Called once at process startup; the resulting pool is injected into
// Axum AppState in API-01 so handlers can borrow it without blocking.

use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, PgPool};
use thiserror::Error;

/// Maximum simultaneous DB connections from this single process.
/// The matching engine never blocks on the DB, so a small pool is plenty.
const MAX_CONNECTIONS: u32 = 5;

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum DbError {
    #[error("DATABASE_URL environment variable is not set")]
    MissingUrl,

    #[error("Database connection error: {0}")]
    Connection(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

// ─────────────────────────────────────────────────────────────────────────────
// Pool factory
// ─────────────────────────────────────────────────────────────────────────────

/// Create a `PgPool` and run all pending sqlx migrations.
///
/// Steps:
///   1. Load `.env` file if present (dev convenience; no-op in Docker/prod).
///   2. Read `DATABASE_URL` from the environment.
///   3. Open a connection pool (max 5 connections, 5 s acquire timeout).
///   4. Run migrations from `core/migrations/` (embedded at compile time).
pub async fn create_pool() -> Result<PgPool, DbError> {
    // Load .env if present; failures are silently ignored (e.g. in CI/Docker)
    let _ = dotenvy::dotenv();

    let database_url = std::env::var("DATABASE_URL").map_err(|_| DbError::MissingUrl)?;

    let pool = PgPoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        // How long to wait for a connection from the pool before erroring
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await?;

    // Embed migrations from `core/migrations/` at compile time and apply them.
    // This is idempotent: already-applied migrations are skipped.
    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}
