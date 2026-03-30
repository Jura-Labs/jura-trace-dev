//! Token-bucket rate limiter for the local REST API.
//!
//! Each API key has a bucket initialised from the `rate_limit` column in the
//! `api_keys` table (requests per minute).  The middleware runs **after** auth,
//! so the authenticated `key_id` is already available in request extensions.
//!
//! Endpoints that bypass auth also bypass rate limiting:
//! - `GET /api/v1/health`
//! - `GET /openapi.json`
//! - anything under `/swagger-ui`

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::AppState;

use super::error::ApiError;

// ── Token bucket ─────────────────────────────────────────────────────────────

/// A single token bucket for one API key.
#[derive(Debug)]
pub struct TokenBucket {
    /// Maximum tokens (= `rate_limit` requests per minute).
    pub capacity: u64,
    /// Tokens currently available.
    pub tokens: u64,
    /// When this bucket was last refilled.
    pub last_refill: Instant,
    /// Refill period (always 60 s in the current implementation).
    pub refill_period: Duration,
}

impl TokenBucket {
    /// Create a new full bucket with the given capacity.
    pub fn new(capacity: u64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            last_refill: Instant::now(),
            refill_period: Duration::from_secs(60),
        }
    }

    /// Attempt to consume one token.
    ///
    /// Returns `Ok(tokens_remaining)` on success or
    /// `Err(seconds_until_refill)` when the bucket is empty.
    pub fn try_consume(&mut self) -> Result<u64, u64> {
        // Eager refill: top up if a full period has elapsed.
        let elapsed = self.last_refill.elapsed();
        if elapsed >= self.refill_period {
            self.tokens = self.capacity;
            self.last_refill = Instant::now();
        }

        if self.tokens > 0 {
            self.tokens -= 1;
            Ok(self.tokens)
        } else {
            let reset_in = self.refill_period.saturating_sub(elapsed).as_secs().max(1);
            Err(reset_in)
        }
    }

    /// Seconds until the next refill (for headers even on success).
    pub fn seconds_until_reset(&self) -> u64 {
        self.refill_period
            .saturating_sub(self.last_refill.elapsed())
            .as_secs()
            .max(1)
    }
}

// ── Shared limiter state ─────────────────────────────────────────────────────

/// Thread-safe map of `key_id → TokenBucket`.
///
/// Wrapped in `Arc` so it can be cloned cheaply into middleware and the
/// background refill task.
#[derive(Clone, Default)]
pub struct RateLimiter {
    buckets: Arc<DashMap<String, TokenBucket>>,
}

impl RateLimiter {
    /// Return a new, empty rate limiter.
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
        }
    }

    /// Get-or-insert a bucket for `key_id`, using `capacity` when a new bucket
    /// is created.  If the bucket already exists its capacity is **not** changed.
    pub fn get_or_create(
        &self,
        key_id: &str,
        capacity: u64,
    ) -> dashmap::mapref::one::RefMut<'_, String, TokenBucket> {
        if !self.buckets.contains_key(key_id) {
            self.buckets
                .insert(key_id.to_string(), TokenBucket::new(capacity));
        }
        self.buckets.get_mut(key_id).expect("just inserted")
    }
}

// ── Middleware ───────────────────────────────────────────────────────────────

/// Axum middleware that enforces per-key rate limits.
///
/// Must be layered **after** `require_api_key` so that `AuthenticatedKey` is
/// already present in request extensions.
pub async fn rate_limit_middleware(
    State((app_state, limiter)): State<(Arc<Mutex<AppState>>, RateLimiter)>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let path = request.uri().path().to_string();

    // Same bypass list as auth middleware.
    // In a nested router the `/api` prefix is stripped, so check both forms.
    if path == "/v1/health"
        || path == "/api/v1/health"
        || path == "/openapi.json"
        || path.starts_with("/swagger-ui")
    {
        return Ok(next.run(request).await);
    }

    // Retrieve the authenticated key_id injected by `require_api_key`.
    let auth_key = request.extensions().get::<AuthenticatedKey>().cloned();

    let auth_key = match auth_key {
        Some(k) => k,
        // No extension means auth was skipped or middleware order is wrong.
        // In either case, let the request through — auth already decided it.
        None => return Ok(next.run(request).await),
    };

    // Look up the rate limit for this key.
    let capacity = {
        let guard = app_state
            .lock()
            .map_err(|_| ApiError::internal("State lock poisoned during rate check"))?;
        guard
            .db
            .verify_api_key(&auth_key.key_hash)
            .map_err(|e| ApiError::internal(format!("DB error during rate check: {e}")))?
            .map(|r| r.rate_limit as u64)
            .unwrap_or(100)
    };

    let mut bucket = limiter.get_or_create(&auth_key.key_id, capacity);

    let limit = bucket.capacity;
    let reset_in = bucket.seconds_until_reset();

    match bucket.try_consume() {
        Ok(remaining) => {
            drop(bucket);
            let mut response = next.run(request).await;
            let headers = response.headers_mut();
            headers.insert("X-RateLimit-Limit", limit.to_string().parse().unwrap());
            headers.insert(
                "X-RateLimit-Remaining",
                remaining.to_string().parse().unwrap(),
            );
            headers.insert("X-RateLimit-Reset", reset_in.to_string().parse().unwrap());
            Ok(response)
        }
        Err(retry_after) => {
            drop(bucket);
            Err(ApiError::rate_limited(limit, 0, retry_after))
        }
    }
}

// ── Extension type injected by auth middleware ────────────────────────────────

/// Identifies the validated API key for downstream middleware (e.g. rate limiter).
#[derive(Clone, Debug)]
pub struct AuthenticatedKey {
    /// The `key_id` UUID from the `api_keys` table.
    pub key_id: String,
    /// SHA-256 hash of the raw key (used to re-query the DB for rate limit).
    pub key_hash: String,
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_consumes_tokens() {
        let mut b = TokenBucket::new(5);
        assert_eq!(b.try_consume(), Ok(4));
        assert_eq!(b.try_consume(), Ok(3));
    }

    #[test]
    fn bucket_exhaustion_returns_retry_after() {
        let mut b = TokenBucket::new(2);
        b.try_consume().unwrap();
        b.try_consume().unwrap();
        assert!(b.try_consume().is_err());
    }

    #[test]
    fn bucket_refills_after_period() {
        let mut b = TokenBucket::new(3);
        // Drain all tokens.
        b.try_consume().unwrap();
        b.try_consume().unwrap();
        b.try_consume().unwrap();
        assert!(b.try_consume().is_err());

        // Simulate the passage of time by forcing last_refill back.
        b.last_refill = Instant::now() - Duration::from_secs(61);

        // Next consume should succeed after eager refill.
        assert!(b.try_consume().is_ok());
        assert_eq!(b.tokens, b.capacity - 1);
    }

    #[test]
    fn new_bucket_is_full() {
        let b = TokenBucket::new(100);
        assert_eq!(b.tokens, 100);
        assert_eq!(b.capacity, 100);
    }

    #[test]
    fn limiter_get_or_create_idempotent() {
        let rl = RateLimiter::new();
        {
            let mut b = rl.get_or_create("key1", 50);
            b.tokens = 10; // drain partially
        }
        // Second call with a different capacity should NOT reset the bucket.
        let b = rl.get_or_create("key1", 200);
        assert_eq!(b.tokens, 10, "existing bucket should not be replaced");
        assert_eq!(
            b.capacity, 50,
            "capacity should not change for existing bucket"
        );
    }
}
