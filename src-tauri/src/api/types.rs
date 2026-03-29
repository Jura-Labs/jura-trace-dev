//! API-specific request and response types.

use serde::{Deserialize, Serialize};

/// Request body for `POST /api/v1/verify/url`.
#[derive(Debug, Deserialize)]
pub struct VerifyUrlRequest {
    /// The URL to download and verify.
    pub url: String,
    /// Investigation mode: `"quick"`, `"standard"` (default), `"deep"`, `"archival"`.
    pub mode: Option<String>,
}

/// Request body for `POST /api/v1/claims/check`.
#[derive(Debug, Deserialize)]
pub struct ClaimCheckRequest {
    /// The claim text to verify against the knowledge base.
    pub claim: String,
    /// Optional surrounding context to include with the claim.
    pub context: Option<String>,
}

/// Wrapper applied to all successful API responses.
///
/// `degraded = true` signals that some detectors were unavailable (sidecar
/// offline or optional models not installed) and the result may be incomplete.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    /// API version string, e.g. `"1.0"`.
    pub api_version: String,
    /// `true` when one or more analysis services were unavailable.
    pub degraded: bool,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            data,
            api_version: "1.0".to_string(),
            degraded: false,
        }
    }

    pub fn degraded(data: T) -> Self {
        Self {
            data,
            api_version: "1.0".to_string(),
            degraded: true,
        }
    }
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub sidecar_available: bool,
    pub uptime_seconds: u64,
}

/// Request body for `POST /api/v1/auth/keys`.
#[derive(Debug, Deserialize)]
pub struct CreateKeyRequest {
    /// Human-readable label for this API key (e.g. `"CI pipeline"`, `"n8n workflow"`).
    pub name: String,
    /// Requests per minute limit. Defaults to 100.
    pub rate_limit: Option<i64>,
}

/// Response for a newly created API key.
///
/// The `key` field contains the raw key prefixed with `jt_`. It is returned
/// exactly once — it cannot be retrieved again after this call.
#[derive(Debug, Serialize)]
pub struct CreateKeyResponse {
    pub key_id: String,
    /// Raw key including `jt_` prefix. Store securely — shown once only.
    pub key: String,
    pub name: String,
    pub rate_limit: i64,
    pub created_at: String,
}

/// Database statistics summary.
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_assets: u64,
    pub total_verifications: u64,
    pub total_fingerprints: u64,
    pub c2pa_signed_count: u64,
}

/// Fingerprint result for a single hash algorithm.
#[derive(Debug, Serialize)]
pub struct FingerprintEntry {
    pub algorithm: String,
    pub hash_hex: String,
}

/// Response for `POST /api/v1/protect/fingerprint`.
#[derive(Debug, Serialize)]
pub struct FingerprintResponse {
    pub hashes: Vec<FingerprintEntry>,
}
