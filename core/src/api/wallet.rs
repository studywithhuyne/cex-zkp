// core/src/api/wallet.rs
// Wallet-related handlers: mock deposit and personal trade history.
//
// Routes (registered in router.rs):
//   POST /api/deposit      — add test funds to a user's balance (requires x-user-id)
//   GET  /api/trades/user  — personal trade history for the authenticated user

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::api::{auth::UserId, state::AppState};

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DepositRequest {
    pub asset:  String,
    pub amount: String,
}

#[derive(Serialize)]
pub struct DepositResponse {
    pub asset:         String,
    pub deposited:     String,
    pub new_available: String,
}

#[derive(Serialize)]
pub struct UserTradeDto {
    pub id:             String,
    pub maker_order_id: i64,
    pub taker_order_id: i64,
    pub side:           String,
    pub price:          String,
    pub amount:         String,
    pub base_asset:     String,
    pub quote_asset:    String,
    pub executed_at:    String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

type ApiError = (StatusCode, Json<serde_json::Value>);

type TradeRow = (
    uuid::Uuid,
    i64,
    i64,
    i64,
    i64,
    Decimal,
    Decimal,
    String,
    String,
    chrono::DateTime<chrono::Utc>,
);

fn db_err(e: sqlx::Error) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": format!("database error: {e}") })),
    )
}

fn bad_request(msg: &str) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": msg })),
    )
}

/// POST /api/deposit — inject test funds into the authenticated user's balance.
pub async fn deposit_handler(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(body): Json<DepositRequest>,
) -> Result<Json<DepositResponse>, ApiError> {
    let asset = body.asset.to_uppercase();
    if asset != "BTC" && asset != "USDT" {
        return Err(bad_request("asset must be BTC or USDT"));
    }

    let amount: Decimal = body
        .amount
        .parse()
        .map_err(|_| bad_request("amount must be a valid decimal string"))?;

    if amount <= Decimal::ZERO {
        return Err(bad_request("amount must be greater than 0"));
    }

    let row: Option<(Decimal,)> = sqlx::query_as(
        "UPDATE balances
         SET available = available + $1, updated_at = now()
         WHERE user_id = $2 AND asset_symbol = $3
         RETURNING available",
    )
    .bind(amount)
    .bind(user_id as i64)
    .bind(&asset)
    .fetch_optional(&state.db)
    .await
    .map_err(db_err)?;

    match row {
        Some((new_available,)) => Ok(Json(DepositResponse {
            asset,
            deposited:     amount.to_string(),
            new_available: new_available.to_string(),
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "user/asset combination not found" })),
        )),
    }
}

/// GET /api/trades/user — personal trade history (as maker or taker).
pub async fn user_trades_handler(
    State(state): State<AppState>,
    UserId(user_id): UserId,
) -> Result<Json<Vec<UserTradeDto>>, ApiError> {
    let uid = user_id as i64;

    let rows: Vec<TradeRow> =
        sqlx::query_as(
            "SELECT id, maker_order_id, taker_order_id, maker_user_id, taker_user_id,
                    price, amount, base_asset, quote_asset, executed_at
             FROM trades_log
             WHERE maker_user_id = $1 OR taker_user_id = $1
             ORDER BY executed_at DESC
             LIMIT 100",
        )
        .bind(uid)
        .fetch_all(&state.db)
        .await
        .map_err(db_err)?;

    let dtos = rows
        .into_iter()
        .map(|(id, maker_oid, taker_oid, maker_uid, _taker_uid, price, amount, base, quote, exec_at)| {
            let side = if maker_uid == uid { "maker" } else { "taker" };
            UserTradeDto {
                id:             id.to_string(),
                maker_order_id: maker_oid,
                taker_order_id: taker_oid,
                side:           side.to_string(),
                price:          price.to_string(),
                amount:         amount.to_string(),
                base_asset:     base,
                quote_asset:    quote,
                executed_at:    exec_at.to_rfc3339(),
            }
        })
        .collect();

    Ok(Json(dtos))
}
