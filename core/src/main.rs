// Engine, api, and db modules are declared in lib.rs (matching_engine crate).
// Import them here as needed when implementing Axum routes in API-01+.

use matching_engine::db::pool;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Matching Engine starting...");

    // Connect to PostgreSQL and run any pending sqlx migrations.
    // The pool will be moved into Axum AppState in API-01.
    let _pool = pool::create_pool()
        .await
        .expect("Failed to initialise database pool");

    tracing::info!("Database pool ready (max_connections=5, migrations applied)");
}
