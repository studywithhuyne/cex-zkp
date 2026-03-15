// Engine, api, and db modules are declared in lib.rs (matching_engine crate).

use matching_engine::api::{router, state::AppState};
use matching_engine::db::{pool, worker};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Matching Engine starting...");

    // ── DB layer ──────────────────────────────────────────────────────────────
    let db_pool = pool::create_pool()
        .await
        .expect("Failed to initialise database pool");
    tracing::info!("Database pool ready (max_connections=5, migrations applied)");

    let (events_tx, _worker_handle) =
        worker::spawn_persistence_worker(db_pool.clone(), worker::WORKER_BUFFER);
    tracing::info!("Persistence worker spawned (buffer={})", worker::WORKER_BUFFER);

    // ── Axum server ───────────────────────────────────────────────────────────
    let app_state = AppState::new(db_pool, events_tx);
    let app = router::build_router(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind port 3000");
    tracing::info!("Listening on http://0.0.0.0:3000");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
