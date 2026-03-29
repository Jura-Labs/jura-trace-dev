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
pub struct ApiError {
    /// Machine-readable error code (stable across releases).
    pub code: &'static str,
    /// Human-readable error message.
    pub message: String,
    /// Optional request identifier for log correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip)]
    pub status: StatusCode,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            request_id: None,
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
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;
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
