use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::api::{auth::UserId, state::AppState};

use zkp::tree::{build_poseidon_merkle_sum_tree, BalanceSnapshot};

#[derive(Debug, Deserialize)]
pub struct ZkpProofQuery {
    pub asset: Option<String>,
    pub cold_wallet_assets: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ZkpProofStepDto {
    pub sibling_hash: String,
    pub sibling_balance: String,
    pub sibling_is_left: bool,
}

#[derive(Debug, Serialize)]
pub struct ZkpProofResponse {
    pub user_id: u64,
    pub asset: String,
    pub snapshot_size: usize,
    pub leaf_index: usize,
    pub leaf_balance: String,
    pub root_hash: String,
    pub root_balance: String,
    pub merkle_path: Vec<ZkpProofStepDto>,
    pub public_inputs: ZkpPublicInputsDto,
    pub solvency: ZkpSolvencyDto,
}

#[derive(Debug, Serialize)]
pub struct ZkpPublicInputsDto {
    pub expected_root_hash: String,
    pub expected_root_balance: String,
    pub expected_user_id: u64,
    pub expected_cold_wallet_assets: String,
}

#[derive(Debug, Serialize)]
pub struct ZkpSolvencyDto {
    pub total_liabilities: String,
    pub cold_wallet_assets: String,
    pub liabilities_leq_assets: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct SolvencyRow {
    user_id: i64,
    balance: Decimal,
}

pub async fn proof_handler(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Query(query): Query<ZkpProofQuery>,
) -> Result<Json<ZkpProofResponse>, (StatusCode, Json<serde_json::Value>)> {
    let asset = query
        .asset
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("USDT")
        .to_ascii_uppercase();

    let rows: Vec<SolvencyRow> = sqlx::query_as(
        "SELECT user_id, (available + locked) AS balance
         FROM balances
         WHERE asset_symbol = $1
         ORDER BY user_id ASC",
    )
    .bind(&asset)
    .fetch_all(&state.db)
    .await
    .map_err(internal_error)?;

    if rows.is_empty() {
        return Err(not_found("no balances found for selected asset"));
    }

    let mut snapshots = Vec::with_capacity(rows.len());
    let mut user_leaf_index: Option<usize> = None;

    for (index, row) in rows.iter().enumerate() {
        if row.user_id <= 0 {
            return Err(internal_error_msg("invalid user_id in balances snapshot"));
        }

        let snapshot_user_id = row.user_id as u64;
        if snapshot_user_id == user_id {
            user_leaf_index = Some(index);
        }

        snapshots.push(BalanceSnapshot {
            user_id: snapshot_user_id,
            balance: row.balance,
        });
    }

    let user_leaf_index = user_leaf_index.ok_or_else(|| not_found("user balance not found for selected asset"))?;

    let tree = build_poseidon_merkle_sum_tree(&snapshots)
        .map_err(|e| internal_error_msg(&format!("failed to build solvency tree: {e}")))?;

    let proof = tree
        .generate_proof(user_leaf_index)
        .map_err(|e| internal_error_msg(&format!("failed to generate merkle path: {e}")))?;

    let merkle_path = proof
        .path
        .into_iter()
        .map(|step| ZkpProofStepDto {
            sibling_hash: hash_to_hex(&step.sibling_hash),
            sibling_balance: step.sibling_balance.to_string(),
            sibling_is_left: step.sibling_is_left,
        })
        .collect();

    let cold_wallet_assets = resolve_cold_wallet_assets(&asset, query.cold_wallet_assets.as_deref())?;
    let total_liabilities = proof.root.balance;
    let liabilities_leq_assets = total_liabilities <= cold_wallet_assets;

    Ok(Json(ZkpProofResponse {
        user_id,
        asset,
        snapshot_size: tree.original_leaf_count(),
        leaf_index: proof.leaf_index,
        leaf_balance: proof.leaf.balance.to_string(),
        root_hash: hash_to_hex(&proof.root.hash),
        root_balance: proof.root.balance.to_string(),
        merkle_path,
        public_inputs: ZkpPublicInputsDto {
            expected_root_hash: hash_to_hex(&proof.root.hash),
            expected_root_balance: proof.root.balance.to_string(),
            expected_user_id: user_id,
            expected_cold_wallet_assets: cold_wallet_assets.to_string(),
        },
        solvency: ZkpSolvencyDto {
            total_liabilities: total_liabilities.to_string(),
            cold_wallet_assets: cold_wallet_assets.to_string(),
            liabilities_leq_assets,
        },
    }))
}

fn resolve_cold_wallet_assets(
    asset: &str,
    query_value: Option<&str>,
) -> Result<Decimal, (StatusCode, Json<serde_json::Value>)> {
    if let Some(raw) = query_value.map(str::trim).filter(|s| !s.is_empty()) {
        return parse_decimal(raw, "invalid query param cold_wallet_assets");
    }

    let env_key = format!("COLD_WALLET_ASSETS_{}", asset);
    if let Ok(raw) = std::env::var(&env_key) {
        return parse_decimal(raw.trim(), &format!("invalid env {env_key}"));
    }

    Err((
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": format!(
                "missing cold wallet assets value; pass ?cold_wallet_assets=... or set env {}",
                env_key
            )
        })),
    ))
}

fn parse_decimal(value: &str, err_prefix: &str) -> Result<Decimal, (StatusCode, Json<serde_json::Value>)> {
    let decimal = value.parse::<Decimal>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{err_prefix}: {value}") })),
        )
    })?;

    if decimal.is_sign_negative() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{err_prefix}: value must be non-negative") })),
        ));
    }

    Ok(decimal)
}

fn hash_to_hex(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[inline]
fn internal_error(err: sqlx::Error) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": format!("database error: {err}") })),
    )
}

#[inline]
fn internal_error_msg(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": msg })),
    )
}

#[inline]
fn not_found(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": msg })))
}
