//! API error type that maps internal [`AppError`] variants to HTTP status codes.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::error::AppError;

/// JSON body returned on every error response.
///
/// ```json
/// { "code": "NotFound", "message": "No resource found at this path", "request_id": null }
/// ```
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    /// Machine-readable error code (stable across releases).
    pub code: &'static str,
    /// Human-readable error message.
    pub message: String,
    /// Optional request identifier for log correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Rate-limit metadata included in 429 responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_info: Option<RateLimitInfo>,
    #[serde(skip)]
    pub status: StatusCode,
    /// Rate-limit headers to attach to the response (skipped from JSON).
    #[serde(skip)]
    pub rate_limit_headers: Option<RateLimitHeaders>,
}

/// Rate limit metadata embedded in 429 JSON bodies.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitInfo {
    pub limit: u64,
    pub remaining: u64,
    pub reset_seconds: u64,
}

/// HTTP headers to attach to 429 responses.
#[derive(Debug, Clone)]
pub struct RateLimitHeaders {
    pub limit: u64,
    pub remaining: u64,
    pub reset_seconds: u64,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            request_id: None,
            rate_limit_info: None,
            rate_limit_headers: None,
            status,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NotFound", message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "BadRequest", message)
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "Unauthorized",
            "Valid API key required. Supply an Authorization: Bearer jt_<key> header.",
        )
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal", message)
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "ServiceUnavailable",
            message,
        )
    }

    /// Build a 429 Too Many Requests error with rate-limit metadata.
    pub fn rate_limited(limit: u64, remaining: u64, reset_seconds: u64) -> Self {
        let info = RateLimitInfo {
            limit,
            remaining,
            reset_seconds,
        };
        let headers = RateLimitHeaders {
            limit,
            remaining,
            reset_seconds,
        };
        Self {
            code: "RateLimitExceeded",
            message: format!(
                "Rate limit of {limit} requests per minute exceeded. \
                 Retry after {reset_seconds} seconds."
            ),
            request_id: None,
            rate_limit_info: Some(info),
            rate_limit_headers: Some(headers),
            status: StatusCode::TOO_MANY_REQUESTS,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;
        // Attach rate-limit headers when present.
        if let Some(ref rl) = self.rate_limit_headers {
            let limit_str = rl.limit.to_string();
            let remaining_str = rl.remaining.to_string();
            let reset_str = rl.reset_seconds.to_string();
            let retry_str = rl.reset_seconds.to_string();
            return (
                status,
                [
                    ("X-RateLimit-Limit", limit_str.as_str()),
                    ("X-RateLimit-Remaining", remaining_str.as_str()),
                    ("X-RateLimit-Reset", reset_str.as_str()),
                    ("Retry-After", retry_str.as_str()),
                ],
                Json(self),
            )
                .into_response();
        }
        (status, Json(self)).into_response()
    }
}

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        match &err {
            AppError::Validation(msg) => {
                // Validation messages are constructed by our own code and safe to surface.
                Self::bad_request(msg.clone())
            }
            AppError::FileSystem(_) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "FileSystem",
                "File operation failed",
            ),
            AppError::Database(_) => Self::internal("Database operation failed"),
            AppError::Sidecar(_) => Self::service_unavailable("Analysis service unavailable"),
            AppError::C2pa(_) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "C2pa",
                "Content credential operation failed",
            ),
            AppError::Internal(_) => Self::internal("An internal error occurred"),
        }
    }
}
