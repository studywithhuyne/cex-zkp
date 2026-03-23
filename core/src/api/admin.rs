use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::state::AppState;

// --- Dashboard & Metrics ---

#[derive(Serialize)]
pub struct AdminMetrics {
    pub volume_24h_usdt: String,
    pub total_users: i64,
    pub active_orders: i64,
}

pub async fn admin_metrics_handler(
    State(state): State<AppState>,
) -> Result<Json<AdminMetrics>, (StatusCode, String)> {
    // Queries without macro to avoid compile-time DB dependency
    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
        
    let active_orders: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders_log WHERE status IN ('open', 'partial_filled')")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let volume_usdt: Option<Decimal> = sqlx::query_scalar(
        "SELECT SUM(price * amount) FROM trades_log WHERE executed_at > NOW() - INTERVAL '1 day'"
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(None);

    Ok(Json(AdminMetrics {
        volume_24h_usdt: volume_usdt.unwrap_or(Decimal::ZERO).to_string(),
        total_users,
        active_orders,
    }))
}

#[derive(Serialize)]
pub struct TreasuryMetrics {
    pub total_exchange_funds: String,
    pub total_user_liabilities: String,
    pub solvency_ratio: String,
}

pub async fn admin_treasury_handler(
    State(state): State<AppState>,
) -> Result<Json<TreasuryMetrics>, (StatusCode, String)> {
    let total_assets = state.exchange_funds.lock().total_exchange_usdt;

    let liab: Option<Decimal> = sqlx::query_scalar(
        "SELECT SUM(free_balance + locked_balance) FROM balances WHERE asset = 'USDT'"
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(None);

    let total_liabilities = liab.unwrap_or(Decimal::ZERO);

    let solvency_ratio = if total_liabilities > Decimal::ZERO {
        (total_assets / total_liabilities).to_string()
    } else {
        "infinity".to_string()
    };

    Ok(Json(TreasuryMetrics {
        total_exchange_funds: total_assets.to_string(),
        total_user_liabilities: total_liabilities.to_string(),
        solvency_ratio,
    }))
}

// --- Asset & Market Management ---

#[derive(Serialize)]
pub struct AdminAssetDto {
    pub symbol: String,
    pub name: String,
    pub decimals: i32,
    pub is_active: bool,
}

pub async fn get_assets_handler() -> Json<Vec<AdminAssetDto>> {
    // Mock for now, typical exchange has this driven by a DB table
    Json(vec![
        AdminAssetDto { symbol: "BTC".into(), name: "Bitcoin".into(), decimals: 8, is_active: true },
        AdminAssetDto { symbol: "USDT".into(), name: "Tether".into(), decimals: 4, is_active: true },
        AdminAssetDto { symbol: "ETH".into(), name: "Ethereum".into(), decimals: 8, is_active: false },
        AdminAssetDto { symbol: "SOL".into(), name: "Solana".into(), decimals: 8, is_active: false },
    ])
}

#[derive(Deserialize)]
pub struct MarketHaltReq {
    pub symbol: String,
}

pub async fn halt_market_handler(
    Json(req): Json<MarketHaltReq>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Stub implementation to mimic halting
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": format!("Market {} has been halted.", req.symbol)
    })))
}

// --- User Management ---

#[derive(Serialize)]
pub struct UserListDto {
    pub user_id: i64,
    pub username: String,
    pub is_suspended: bool,
}

pub async fn admin_users_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<UserListDto>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, (i32, String)>("SELECT id, username FROM users ORDER BY id ASC")
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let users = rows.into_iter().map(|(id, username)| UserListDto {
        user_id: id as i64,
        username,
        is_suspended: false, // Default stub
    }).collect();

    Ok(Json(users))
}

pub async fn suspend_user_handler(
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Stub
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": format!("User {} has been suspended strictly.", user_id)
    })))
}

// --- ZKP Audit Operations ---

pub async fn trigger_zkp_snapshot_handler(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Mock triggering a global snapshot
    // In reality this would lock the ledger or grab a consistent read
    
    // We already have a background snapshot mechanism, but an admin trigger forces one immediately.
    let snapshot_id = format!("snap_{}", chrono::Utc::now().timestamp());

    Ok(Json(serde_json::json!({
        "status": "success",
        "snapshot_id": snapshot_id,
        "message": "Global balance snapshot and Merkle tree generation initialized."
    })))
}

pub async fn zkp_history_handler() -> Json<Vec<serde_json::Value>> {
    Json(vec![
        serde_json::json!({
            "snapshot_id": "snap_1700000000",
            "timestamp": "2026-03-22T00:00:00Z",
            "root_hash": "0x123abc...",
            "users_included": 1500
        })
    ])
}