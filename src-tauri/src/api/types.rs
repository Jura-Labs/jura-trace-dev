// SPDX-License-Identifier: AGPL-3.0-or-later

//! API-specific request and response types.
//!
//! All types that appear in the OpenAPI spec derive [`utoipa::ToSchema`].

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request body for `POST /api/v1/verify/url`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyUrlRequest {
    /// The URL to download and verify.
    #[schema(example = "https://example.com/image.jpg")]
    pub url: String,
    /// Investigation mode: `"quick"`, `"standard"` (default), `"deep"`, `"archival"`.
    #[schema(example = "standard")]
    pub mode: Option<String>,
}

/// Request body for `POST /api/v1/claims/check`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ClaimCheckRequest {
    /// The claim text to verify against the knowledge base.
    #[schema(example = "The photograph was taken in 2023.")]
    pub claim: String,
    /// Optional surrounding context to include with the claim.
    pub context: Option<String>,
}

/// Wrapper applied to all successful API responses.
///
/// `degraded = true` signals that some detectors were unavailable (sidecar
/// offline or optional models not installed) and the result may be incomplete.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T: Serialize + 'static> {
    pub data: T,
    /// API version string, e.g. `"1.0"`.
    #[schema(example = "1.0")]
    pub api_version: String,
    /// `true` when one or more analysis services were unavailable.
    #[schema(example = false)]
    pub degraded: bool,
}

impl<T: Serialize + 'static> ApiResponse<T> {
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
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    /// Always `"ok"` when the server is running.
    #[schema(example = "ok")]
    pub status: &'static str,
    /// Application version string.
    #[schema(example = "0.9.0")]
    pub version: &'static str,
    /// Whether the Python ML sidecar is reachable on port 8200.
    #[schema(example = true)]
    pub sidecar_available: bool,
    /// Seconds since the API server started.
    #[schema(example = 3600)]
    pub uptime_seconds: u64,
}

/// Request body for `POST /api/v1/auth/keys`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateKeyRequest {
    /// Human-readable label for this API key (e.g. `"CI pipeline"`, `"n8n workflow"`).
    #[schema(example = "CI pipeline")]
    pub name: String,
    /// Requests per minute limit. Defaults to 100.
    #[schema(example = 100)]
    pub rate_limit: Option<i64>,
}

/// Response for a newly created API key.
///
/// The `key` field contains the raw key prefixed with `jt_`. It is returned
/// exactly once — it cannot be retrieved again after this call.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateKeyResponse {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub key_id: String,
    /// Raw key including `jt_` prefix. Store securely — shown once only.
    #[schema(example = "jt_abc123def456...")]
    pub key: String,
    #[schema(example = "CI pipeline")]
    pub name: String,
    #[schema(example = 100)]
    pub rate_limit: i64,
    #[schema(example = "2026-03-29T12:00:00Z")]
    pub created_at: String,
}

/// Database statistics summary.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    #[schema(example = 42)]
    pub total_assets: u64,
    #[schema(example = 18)]
    pub total_verifications: u64,
    #[schema(example = 38)]
    pub total_fingerprints: u64,
    #[schema(example = 12)]
    pub c2pa_signed_count: u64,
}

/// Single result within a batch verification response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchVerifyItem {
    /// Original filename from the multipart upload.
    #[schema(example = "photo.jpg")]
    pub filename: String,
    /// `true` if this file was analysed successfully.
    pub success: bool,
    /// The verification result (present when `success = true`).
    /// Typed as Value in the OpenAPI spec; actual shape is VerificationResult.
    #[schema(value_type = Object)]
    pub result: Option<serde_json::Value>,
    /// Error message (present when `success = false`).
    pub error: Option<String>,
    /// Whether the result is degraded (sidecar offline).
    #[schema(example = false)]
    pub degraded: bool,
}

/// Response for `POST /api/v1/verify/batch`.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchVerifyResponse {
    /// Per-file results in the same order as the uploaded files.
    pub items: Vec<BatchVerifyItem>,
    /// Total files processed.
    #[schema(example = 3)]
    pub total: usize,
    /// Number that succeeded.
    #[schema(example = 2)]
    pub succeeded: usize,
    /// Number that failed.
    #[schema(example = 1)]
    pub failed: usize,
}

/// Fingerprint result for a single hash algorithm.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintEntry {
    /// Algorithm name: `"aHash"`, `"dHash"`, or `"pHash"`.
    #[schema(example = "pHash")]
    pub algorithm: String,
    /// 64-bit hash as a 16-character hex string.
    #[schema(example = "f0e0c0a080604020")]
    pub hash_hex: String,
}

/// Response for `POST /api/v1/protect/fingerprint`.
#[derive(Debug, Serialize, ToSchema)]
pub struct FingerprintResponse {
    pub hashes: Vec<FingerprintEntry>,
}
