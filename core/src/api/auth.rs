// core/src/api/auth.rs
// Dummy authentication via the `x-user-id` HTTP header.
//
// Design: Axum custom extractor (`FromRequestParts`) rather than a tower
// middleware layer.  Any handler that declares `UserId` as a parameter will
// have the header parsed and validated automatically; requests with a missing
// or invalid header are rejected before the handler runs.
//
// Per copilot-instructions.md:
//   "DO NOT implement JWT, OAuth, or Web3 Wallet Signatures.
//    Use ONLY the HTTP Header `x-user-id` (parsing a `u64`)"

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    Json,
};
use serde_json::{json, Value};

// ─────────────────────────────────────────────────────────────────────────────
// Public type
// ─────────────────────────────────────────────────────────────────────────────

/// The authenticated caller's user ID, extracted from the `x-user-id` header.
///
/// Usage — add as a handler parameter:
/// ```rust,ignore
/// async fn my_handler(UserId(uid): UserId, ...) { /* ... */ }
/// ```
/// Axum will reject the request with 401/400 if the header is absent or invalid.
#[derive(Debug, Clone, Copy)]
pub struct UserId(pub u64);

// ─────────────────────────────────────────────────────────────────────────────
// Extractor implementation
// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl<S> FromRequestParts<S> for UserId
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<Value>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Header must be present.
        let header_val = parts
            .headers
            .get("x-user-id")
            .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing x-user-id header"))?;

        // 2. Header value must be valid ASCII/UTF-8.
        let str_val = header_val
            .to_str()
            .map_err(|_| err(StatusCode::BAD_REQUEST, "x-user-id must be valid ASCII"))?;

        // 3. Value must parse as a non-zero u64.
        let user_id = str_val
            .parse::<u64>()
            .map_err(|_| err(StatusCode::BAD_REQUEST, "x-user-id must be a positive integer (u64)"))?;

        if user_id == 0 {
            return Err(err(StatusCode::BAD_REQUEST, "x-user-id must be greater than 0"));
        }

        Ok(UserId(user_id))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper
// ─────────────────────────────────────────────────────────────────────────────

#[inline]
fn err(status: StatusCode, message: &str) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "error": message })))
}
