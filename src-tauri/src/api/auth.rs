//! API key authentication middleware for the local REST API.
//!
//! Extracts the `Authorization: Bearer jt_<key>` header, SHA-256 hashes the
//! raw key, and validates it against the `api_keys` table. Returns 401 when the
//! key is missing, malformed, or revoked.
//!
//! Routes that bypass auth: `GET /api/v1/health` and `GET /openapi.json`.

use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

use crate::AppState;

use super::error::ApiError;
use super::rate_limit::AuthenticatedKey;

/// Tauri v2 state extractor alias used inside middleware.
type SharedState = Arc<Mutex<AppState>>;

/// Middleware: validate `Authorization: Bearer jt_<key>` on every request
/// except health and OpenAPI endpoints.
///
/// On success the validated [`AuthenticatedKey`] is inserted into request
/// extensions so the downstream rate-limit middleware can read it without
/// re-querying the database.
pub async fn require_api_key(
    State(state): State<SharedState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let path = request.uri().path().to_string();

    // Routes that do not require authentication.
    // The middleware is on the inner (nested) router, so the path is stripped
    // of the `/api` prefix by axum — use `/v1/health` not `/api/v1/health`.
    if path == "/v1/health"
        || path == "/api/v1/health"
        || path == "/openapi.json"
        || path.starts_with("/swagger-ui")
    {
        return Ok(next.run(request).await);
    }

    // Extract the raw key from the Authorization header.
    let raw_key = extract_bearer_key(request.headers()).ok_or_else(ApiError::unauthorized)?;

    // Hash the raw key for lookup.
    let key_hash = hash_key(&raw_key);

    // Validate against the database and retrieve the key record.
    let key_record = tokio::task::spawn_blocking({
        let key_hash = key_hash.clone();
        move || {
            let guard = state
                .lock()
                .map_err(|_| ApiError::internal("State lock poisoned"))?;
            guard
                .db
                .verify_api_key(&key_hash)
                .map_err(|_| ApiError::internal("Database error during key verification"))
        }
    })
    .await
    .map_err(|_| ApiError::internal("Auth task join error"))??;

    if let Some(record) = key_record {
        // Inject the key identity into extensions for the rate limiter.
        request.extensions_mut().insert(AuthenticatedKey {
            key_id: record.key_id,
            key_hash,
        });
        Ok(next.run(request).await)
    } else {
        Err(ApiError::unauthorized())
    }
}

/// Extract the raw key value from `Authorization: Bearer jt_<value>`.
///
/// Returns `None` if the header is absent, malformed, or does not start with
/// the expected `jt_` prefix.
fn extract_bearer_key(headers: &axum::http::HeaderMap) -> Option<String> {
    let value = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok())?;

    let token = value.strip_prefix("Bearer ")?;
    let key = token.strip_prefix("jt_")?;

    if key.is_empty() {
        return None;
    }

    Some(key.to_string())
}

/// SHA-256 hash a raw key string, returning a lowercase hex string.
pub fn hash_key(raw_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_key_is_deterministic() {
        let h1 = hash_key("test-key-123");
        let h2 = hash_key("test-key-123");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn extract_bearer_key_valid() {
        use axum::http::HeaderMap;
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer jt_abc123".parse().unwrap());
        assert_eq!(extract_bearer_key(&headers), Some("abc123".to_string()));
    }

    #[test]
    fn extract_bearer_key_missing() {
        use axum::http::HeaderMap;
        let headers = HeaderMap::new();
        assert!(extract_bearer_key(&headers).is_none());
    }

    #[test]
    fn extract_bearer_key_wrong_prefix() {
        use axum::http::HeaderMap;
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer sk_something".parse().unwrap());
        assert!(extract_bearer_key(&headers).is_none());
    }

    #[test]
    fn extract_bearer_key_empty_value() {
        use axum::http::HeaderMap;
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer jt_".parse().unwrap());
        assert!(extract_bearer_key(&headers).is_none());
    }
}
