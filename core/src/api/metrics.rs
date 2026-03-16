use axum::{
    extract::State,
    http::header,
    response::IntoResponse,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::api::state::AppState;

pub async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    observe_runtime_gauges(&state).await;
    let body = state.metrics.render();

    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

async fn observe_runtime_gauges(state: &AppState) {
    let active_symbols = {
        let engine = state.engine.read();
        engine.symbols().len() as f64
    };
    metrics::gauge!("cex_active_symbols").set(active_symbols);

    let total_locked = sqlx::query_scalar::<_, Decimal>(
        "SELECT COALESCE(SUM(locked), 0) AS total_locked FROM balances",
    )
    .fetch_one(&state.db)
    .await;

    if let Ok(value) = total_locked {
        metrics::gauge!("cex_total_locked_value").set(value.to_f64().unwrap_or(0.0));
    }
}
