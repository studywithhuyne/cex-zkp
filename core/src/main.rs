// Engine, api, and db modules are declared in lib.rs (matching_engine crate).
// Import them here as needed when implementing Axum routes in API-01+.

use matching_engine::db::{pool, worker};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Matching Engine starting...");

    // Connect to PostgreSQL and run any pending sqlx migrations.
    let db_pool = pool::create_pool()
        .await
        .expect("Failed to initialise database pool");
    tracing::info!("Database pool ready (max_connections=5, migrations applied)");

    // Spawn the async persistence worker.
    // The Sender (_tx) will be moved into Axum AppState in API-01 so that
    // request handlers can forward trade/order events without blocking.
    let (_tx, _worker) = worker::spawn_persistence_worker(db_pool, worker::WORKER_BUFFER);
    tracing::info!("Persistence worker spawned (buffer={})", worker::WORKER_BUFFER);
}
