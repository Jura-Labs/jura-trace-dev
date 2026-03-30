//! Axum route handlers for the local REST API (port 8300).
//!
//! All handlers are async and run on the tokio executor shared with Tauri.
//! Blocking work (rusqlite queries, CPU-bound verification) is dispatched via
//! `tokio::task::spawn_blocking` so the async executor is never stalled.

use axum::{
    body::Bytes,
    extract::{Multipart, Path as AxumPath, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::{fingerprint, watermark, AppState};

use super::{
    auth::hash_key,
    error::ApiError,
    types::{
        ApiResponse, ClaimCheckRequest, CreateKeyRequest, CreateKeyResponse, FingerprintEntry,
        FingerprintResponse, HealthResponse, StatsResponse, VerifyUrlRequest,
    },
};

// Re-export to allow utoipa path resolution from mod.rs
#[allow(unused_imports)]
use super::types;

/// Application start time — used for uptime calculation.
static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Initialise the start-time clock. Called once from `start_server`.
pub fn init_start_time() {
    START_TIME.get_or_init(Instant::now);
}

type SharedState = Arc<Mutex<AppState>>;

// ── Health ───────────────────────────────────────────────────────────────────

/// `GET /api/v1/health` — server liveness and capability probe.
///
/// No authentication required.
#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "System",
    operation_id = "health",
    summary = "Health check",
    description = "Returns server status and sidecar availability. No auth required.",
    security(()),
    responses(
        (status = 200, description = "Server is running", body = HealthResponse)
    )
)]
pub async fn health(State(state): State<SharedState>) -> Json<HealthResponse> {
    let sidecar_available = tokio::task::spawn_blocking(move || {
        state
            .lock()
            .map(|g| g.sidecar.is_available())
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false);

    let uptime = START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0);

    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        sidecar_available,
        uptime_seconds: uptime,
    })
}

// ── Verify ───────────────────────────────────────────────────────────────────

/// `POST /api/v1/verify` — multipart file upload → full verification pipeline.
///
/// The multipart form must contain a field named `file` with the binary
/// content of the media file.  An optional `mode` text field controls the
/// investigation depth: `quick`, `standard` (default), `deep`, `archival`.
#[utoipa::path(
    post,
    path = "/api/v1/verify",
    tag = "Verify",
    operation_id = "verifyFile",
    summary = "Verify a file",
    description = "Upload a file (image, video, audio, PDF) for full verification.",
    request_body(
        content_type = "multipart/form-data",
        description = "Multipart form with 'file' (binary) and optional 'mode' (string) fields"
    ),
    responses(
        (status = 200, description = "Verification result"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn verify_file(
    State(state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<crate::VerificationResult>>, ApiError> {
    let mut file_bytes: Option<Bytes> = None;
    let mut mode: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Invalid multipart data: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                file_bytes = Some(field.bytes().await.map_err(|e| {
                    ApiError::bad_request(format!("Failed to read file field: {e}"))
                })?);
            }
            "mode" => {
                let text = field.text().await.map_err(|e| {
                    ApiError::bad_request(format!("Failed to read mode field: {e}"))
                })?;
                mode = Some(text);
            }
            _ => {} // ignore unknown fields
        }
    }

    let bytes = file_bytes.ok_or_else(|| ApiError::bad_request("Missing 'file' field"))?;

    if bytes.is_empty() {
        return Err(ApiError::bad_request("Uploaded file is empty"));
    }

    // Write to a named temp file so the verify pipeline can access it by path.
    let tmp_path = tokio::task::spawn_blocking({
        let bytes = bytes.clone();
        move || -> Result<std::path::PathBuf, ApiError> {
            let mut tmp = tempfile::NamedTempFile::new()
                .map_err(|e| ApiError::internal(format!("Failed to create temp file: {e}")))?;
            tmp.write_all(&bytes)
                .map_err(|e| ApiError::internal(format!("Failed to write temp file: {e}")))?;
            let path = tmp.into_temp_path();
            // Keep the file alive by converting into a PathBuf (unlinking is deferred).
            path.keep()
                .map_err(|e| ApiError::internal(format!("Failed to persist temp file: {e}")))
        }
    })
    .await
    .map_err(|_| ApiError::internal("Temp file task panicked"))??;

    let mode_clone = mode.clone();
    let result = tokio::task::spawn_blocking(move || {
        let result = crate::verify_content_inner(
            &tmp_path.to_string_lossy(),
            "upload",
            mode_clone.as_deref(),
            &state,
        );
        // Clean up temp file.
        let _ = std::fs::remove_file(&tmp_path);
        result
    })
    .await
    .map_err(|_| ApiError::internal("Verification task panicked"))?
    .map_err(ApiError::from)?;

    let degraded =
        result.ela_result.is_none() && result.deepfake_result.is_none() && result.mode != "quick";

    Ok(Json(if degraded {
        ApiResponse::degraded(result)
    } else {
        ApiResponse::ok(result)
    }))
}

/// `POST /api/v1/verify/url` — download a URL and verify its content.
#[utoipa::path(
    post,
    path = "/api/v1/verify/url",
    tag = "Verify",
    operation_id = "verifyUrl",
    summary = "Verify a URL",
    description = "Download and verify the content at a public URL.",
    request_body(content = VerifyUrlRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Verification result"),
        (status = 400, description = "Invalid or unsafe URL"),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn verify_url(
    State(state): State<SharedState>,
    Json(body): Json<VerifyUrlRequest>,
) -> Result<Json<ApiResponse<crate::VerificationResult>>, ApiError> {
    let mode = body.mode.clone();
    let url = body.url.clone();

    let result =
        tokio::task::spawn_blocking(move || crate::verify_url_inner(&url, mode.as_deref(), &state))
            .await
            .map_err(|_| ApiError::internal("Verification task panicked"))?
            .map_err(ApiError::from)?;

    let degraded =
        result.ela_result.is_none() && result.deepfake_result.is_none() && result.mode != "quick";

    Ok(Json(if degraded {
        ApiResponse::degraded(result)
    } else {
        ApiResponse::ok(result)
    }))
}

// ── Protect: Sign ────────────────────────────────────────────────────────────

/// `POST /api/v1/protect/sign` — sign an uploaded file with C2PA credentials.
///
/// Multipart fields:
/// - `file` — binary content of the media file
/// - `creator_name` — creator / rights holder name
/// - `license` (optional) — SPDX licence identifier
///
/// Returns the signed file as an `application/octet-stream` binary response.
#[utoipa::path(
    post,
    path = "/api/v1/protect/sign",
    tag = "Protect",
    operation_id = "protectSign",
    summary = "Sign a file with C2PA credentials",
    description = "Embed a C2PA content credential into the uploaded file. \
                   Returns the signed file as application/octet-stream.",
    request_body(
        content_type = "multipart/form-data",
        description = "Multipart form with 'file' (binary), 'creator_name' (string), \
                       and optional 'license' (SPDX identifier string) fields"
    ),
    responses(
        (status = 200, description = "Signed file (application/octet-stream)"),
        (status = 400, description = "Missing required fields"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Signing failed — unsupported format or certificate error"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn protect_sign(
    State(state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Response, ApiError> {
    let mut file_bytes: Option<Bytes> = None;
    let mut creator_name = String::new();
    let mut license: Option<String> = None;
    let mut original_filename: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Invalid multipart data: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                // Capture filename hint before consuming the field.
                original_filename = field.file_name().map(str::to_string);
                file_bytes = Some(field.bytes().await.map_err(|e| {
                    ApiError::bad_request(format!("Failed to read file field: {e}"))
                })?);
            }
            "creator_name" => {
                creator_name = field.text().await.map_err(|e| {
                    ApiError::bad_request(format!("Failed to read creator_name: {e}"))
                })?;
            }
            "license" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::bad_request(format!("Failed to read license: {e}")))?;
                license = Some(text);
            }
            _ => {}
        }
    }

    let bytes = file_bytes.ok_or_else(|| ApiError::bad_request("Missing 'file' field"))?;
    if bytes.is_empty() {
        return Err(ApiError::bad_request("Uploaded file is empty"));
    }
    if creator_name.is_empty() {
        return Err(ApiError::bad_request("Missing 'creator_name' field"));
    }

    // Derive a safe extension from the first few magic bytes.
    let ext = infer_extension(&bytes);
    let filename_hint = original_filename.as_deref().unwrap_or("upload").to_string();

    let signed_bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, ApiError> {
        // Write source file to a temp file.
        let mut src_tmp = tempfile::NamedTempFile::new()
            .map_err(|e| ApiError::internal(format!("Failed to create temp file: {e}")))?;
        src_tmp
            .write_all(&bytes)
            .map_err(|e| ApiError::internal(format!("Failed to write temp file: {e}")))?;
        let src_path = src_tmp.path().to_path_buf();

        // Output path for the signed file.
        let out_tmp = tempfile::NamedTempFile::new()
            .map_err(|e| ApiError::internal(format!("Failed to create output temp: {e}")))?;
        let out_path = out_tmp.path().to_path_buf();

        // Acquire state to get the data directory for certificate storage.
        let guard = state
            .lock()
            .map_err(|_| ApiError::internal("State lock poisoned"))?;

        // Resolve certificate directory from the database path sibling.
        let db_path = std::path::PathBuf::from(&guard.db_path);
        let data_dir = db_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        drop(guard); // release lock before CPU-heavy signing

        let (cert_bytes, key_bytes) = crate::c2pa::ensure_certificate(&data_dir)
            .map_err(|e| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "C2pa", e))?;
        crate::c2pa::sign_file(
            &src_path,
            &out_path,
            &creator_name,
            license.as_deref(),
            &cert_bytes,
            &key_bytes,
        )
        .map_err(|e| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "C2pa", e))?;

        std::fs::read(&out_path)
            .map_err(|e| ApiError::internal(format!("Failed to read signed output: {e}")))
    })
    .await
    .map_err(|_| ApiError::internal("Signing task panicked"))??;

    // Determine a download filename.
    let download_name = format!(
        "{}_signed.{}",
        std::path::Path::new(&filename_hint)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file"),
        ext
    );

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{download_name}\""),
            ),
        ],
        signed_bytes,
    )
        .into_response())
}

// ── Protect: Fingerprint ─────────────────────────────────────────────────────

/// `POST /api/v1/protect/fingerprint` — compute perceptual hashes for an image.
///
/// Returns aHash, dHash, and pHash values. Only images are supported;
/// non-image uploads return a 422.
#[utoipa::path(
    post,
    path = "/api/v1/protect/fingerprint",
    tag = "Protect",
    operation_id = "protectFingerprint",
    summary = "Compute perceptual hashes",
    description = "Compute aHash, dHash, and pHash perceptual fingerprints for an image. \
                   Non-image files return 422.",
    request_body(
        content_type = "multipart/form-data",
        description = "Multipart form with 'file' (binary image) field"
    ),
    responses(
        (status = 200, description = "Hash values", body = FingerprintResponse),
        (status = 400, description = "Missing file field"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Unsupported format or hashing failed"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn protect_fingerprint(
    State(_state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<FingerprintResponse>>, ApiError> {
    let mut file_bytes: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Invalid multipart data: {e}")))?
    {
        if field.name().unwrap_or("") == "file" {
            file_bytes =
                Some(field.bytes().await.map_err(|e| {
                    ApiError::bad_request(format!("Failed to read file field: {e}"))
                })?);
        }
    }

    let bytes = file_bytes.ok_or_else(|| ApiError::bad_request("Missing 'file' field"))?;
    if bytes.is_empty() {
        return Err(ApiError::bad_request("Uploaded file is empty"));
    }

    let hashes = tokio::task::spawn_blocking(move || -> Result<Vec<FingerprintEntry>, ApiError> {
        // supports_fingerprinting expects content-type like "image", not MIME.
        // Map the MIME type to a content-type category for the check.
        let mime = infer_extension_str(&bytes);
        let content_type_str = if mime.starts_with("image/") {
            "image"
        } else if mime.starts_with("video/") {
            "video"
        } else if mime.starts_with("audio/") {
            "audio"
        } else {
            "unknown"
        };
        if !fingerprint::supports_fingerprinting(content_type_str) {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "UnsupportedFormat",
                "Perceptual fingerprinting requires an image file (JPEG, PNG, WebP, etc.)",
            ));
        }

        // Write to a temp file with the correct extension so the image crate
        // can determine the format by extension (magic-byte fallback is
        // not universally available across image crate versions).
        let ext = infer_extension(&bytes);
        let mut tmp = tempfile::Builder::new()
            .suffix(&format!(".{ext}"))
            .tempfile()
            .map_err(|e| ApiError::internal(format!("Failed to create temp file: {e}")))?;
        tmp.write_all(&bytes)
            .map_err(|e| ApiError::internal(format!("Failed to write temp file: {e}")))?;
        let path = tmp.path().to_path_buf();

        let results = fingerprint::compute_hashes(&path);
        if results.is_empty() {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "FingerprintFailed",
                "Could not compute hashes — the file may not be a valid image",
            ));
        }

        Ok(results
            .into_iter()
            .map(|h| FingerprintEntry {
                algorithm: h.algorithm.as_str().to_string(),
                hash_hex: h.hash_hex,
            })
            .collect())
    })
    .await
    .map_err(|_| ApiError::internal("Fingerprint task panicked"))??;

    Ok(Json(ApiResponse::ok(FingerprintResponse { hashes })))
}

// ── Protect: Watermark embed ─────────────────────────────────────────────────

/// `POST /api/v1/protect/watermark/embed` — embed an invisible watermark.
///
/// Multipart fields:
/// - `file` — source image
/// - `payload_hex` — 32-char hex string (16 bytes / UUID) to embed
/// - `strength` (optional) — `1` (low), `2` (medium, default), `3` (high)
///
/// Returns the watermarked image as `image/png`.
#[utoipa::path(
    post,
    path = "/api/v1/protect/watermark/embed",
    tag = "Protect",
    operation_id = "protectWatermarkEmbed",
    summary = "Embed an invisible watermark",
    description = "Embed a DWT-DCT-SVD invisible watermark into an image. \
                   Returns the watermarked image as image/png.",
    request_body(
        content_type = "multipart/form-data",
        description = "Multipart form: 'file' (binary image), 'payload_hex' \
                       (32-char hex), optional 'strength' (1=low, 2=medium, 3=high)"
    ),
    responses(
        (status = 200, description = "Watermarked PNG image (image/png)"),
        (status = 400, description = "Missing required fields"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Watermarking failed"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn protect_watermark_embed(
    State(_state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Response, ApiError> {
    let mut file_bytes: Option<Bytes> = None;
    let mut payload_hex = String::new();
    let mut strength: Option<u32> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Invalid multipart data: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                file_bytes = Some(field.bytes().await.map_err(|e| {
                    ApiError::bad_request(format!("Failed to read file field: {e}"))
                })?);
            }
            "payload_hex" => {
                payload_hex = field.text().await.map_err(|e| {
                    ApiError::bad_request(format!("Failed to read payload_hex: {e}"))
                })?;
            }
            "strength" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::bad_request(format!("Failed to read strength: {e}")))?;
                strength = text.parse::<u32>().ok();
            }
            _ => {}
        }
    }

    let bytes = file_bytes.ok_or_else(|| ApiError::bad_request("Missing 'file' field"))?;
    if bytes.is_empty() {
        return Err(ApiError::bad_request("Uploaded file is empty"));
    }
    if payload_hex.is_empty() {
        return Err(ApiError::bad_request("Missing 'payload_hex' field"));
    }

    let result = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, ApiError> {
        let mut src_tmp = tempfile::NamedTempFile::new()
            .map_err(|e| ApiError::internal(format!("Failed to create temp file: {e}")))?;
        src_tmp
            .write_all(&bytes)
            .map_err(|e| ApiError::internal(format!("Failed to write temp file: {e}")))?;
        let src_path = src_tmp.path().to_path_buf();

        let out_tmp = tempfile::NamedTempFile::new()
            .map_err(|e| ApiError::internal(format!("Failed to create output temp: {e}")))?;
        let out_path = out_tmp.path().to_path_buf();

        let options = watermark::WatermarkOptions {
            payload_hex: payload_hex.clone(),
            strength,
        };

        watermark::embed_watermark(&src_path, &out_path, &options)
            .map_err(|e| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "Watermark", e))?;

        std::fs::read(&out_path)
            .map_err(|e| ApiError::internal(format!("Failed to read watermarked output: {e}")))
    })
    .await
    .map_err(|_| ApiError::internal("Watermark embed task panicked"))??;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/png"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"watermarked.png\"",
            ),
        ],
        result,
    )
        .into_response())
}

// ── Protect: Watermark extract ───────────────────────────────────────────────

/// `POST /api/v1/protect/watermark/extract` — extract a watermark from an image.
///
/// Multipart fields:
/// - `file` — image to inspect
/// - `payload_len_bytes` (optional) — expected payload length in bytes (default: 16)
/// - `reference_hex` (optional) — expected payload hex for comparison
#[utoipa::path(
    post,
    path = "/api/v1/protect/watermark/extract",
    tag = "Protect",
    operation_id = "protectWatermarkExtract",
    summary = "Extract a watermark",
    description = "Extract a DWT-DCT-SVD watermark payload from an image.",
    request_body(
        content_type = "multipart/form-data",
        description = "Multipart form: 'file' (binary image), optional \
                       'payload_len_bytes' (integer), optional 'reference_hex' (string)"
    ),
    responses(
        (status = 200, description = "Extracted watermark payload"),
        (status = 400, description = "Missing file field"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Extraction failed"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn protect_watermark_extract(
    State(_state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<watermark::ExtractResult>>, ApiError> {
    let mut file_bytes: Option<Bytes> = None;
    let mut payload_len: Option<usize> = None;
    let mut reference_hex: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Invalid multipart data: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                file_bytes = Some(field.bytes().await.map_err(|e| {
                    ApiError::bad_request(format!("Failed to read file field: {e}"))
                })?);
            }
            "payload_len_bytes" => {
                let text = field.text().await.ok().unwrap_or_default();
                payload_len = text.parse().ok();
            }
            "reference_hex" => {
                let text = field.text().await.map_err(|e| {
                    ApiError::bad_request(format!("Failed to read reference_hex: {e}"))
                })?;
                reference_hex = Some(text);
            }
            _ => {}
        }
    }

    let bytes = file_bytes.ok_or_else(|| ApiError::bad_request("Missing 'file' field"))?;
    if bytes.is_empty() {
        return Err(ApiError::bad_request("Uploaded file is empty"));
    }

    let result =
        tokio::task::spawn_blocking(move || -> Result<watermark::ExtractResult, ApiError> {
            let mut tmp = tempfile::NamedTempFile::new()
                .map_err(|e| ApiError::internal(format!("Failed to create temp file: {e}")))?;
            tmp.write_all(&bytes)
                .map_err(|e| ApiError::internal(format!("Failed to write temp file: {e}")))?;
            let path = tmp.path().to_path_buf();

            let len = payload_len.unwrap_or(16);
            watermark::extract_watermark(&path, len, reference_hex.as_deref())
                .map_err(|e| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "Watermark", e))
        })
        .await
        .map_err(|_| ApiError::internal("Watermark extract task panicked"))??;

    Ok(Json(ApiResponse::ok(result)))
}

// ── Claims ───────────────────────────────────────────────────────────────────

/// `POST /api/v1/claims/check` — check a factual claim against the knowledge base.
///
/// Requires Ollama to be running with the Qwen2.5 model pulled. Returns a
/// degraded response if Ollama is unavailable.
#[utoipa::path(
    post,
    path = "/api/v1/claims/check",
    tag = "Claims",
    operation_id = "claimsCheck",
    summary = "Verify a factual claim",
    description = "Check a claim against the RAG knowledge base using Ollama/Qwen2.5. \
                   Returns 503 when Ollama is unavailable.",
    request_body(content = ClaimCheckRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Claim verification result"),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 503, description = "Ollama/Qwen2.5 unavailable"),
    )
)]
pub async fn claims_check(
    State(state): State<SharedState>,
    Json(body): Json<ClaimCheckRequest>,
) -> Result<Json<ApiResponse<crate::sidecar::ClaimCheckResult>>, ApiError> {
    let claim_text = if let Some(ctx) = body.context {
        format!("{}\n\nClaim: {}", ctx, body.claim)
    } else {
        body.claim
    };

    let result = tokio::task::spawn_blocking(move || {
        let guard = state
            .lock()
            .map_err(|_| ApiError::internal("State lock poisoned"))?;
        guard
            .sidecar
            .check_claim(&claim_text)
            .map_err(|e| ApiError::service_unavailable(e.to_string()))
    })
    .await
    .map_err(|_| ApiError::internal("Claim check task panicked"))??;

    Ok(Json(ApiResponse::ok(result)))
}

// ── Stats ────────────────────────────────────────────────────────────────────

/// `GET /api/v1/stats` — database statistics.
#[utoipa::path(
    get,
    path = "/api/v1/stats",
    tag = "System",
    operation_id = "getStats",
    summary = "Database statistics",
    description = "Returns counts of assets, verifications, fingerprints, and C2PA-signed files.",
    responses(
        (status = 200, description = "Stats summary", body = StatsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn get_stats(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<StatsResponse>>, ApiError> {
    let stats = tokio::task::spawn_blocking(move || {
        let guard = state
            .lock()
            .map_err(|_| ApiError::internal("State lock poisoned"))?;
        guard
            .db
            .get_stats()
            .map_err(|e| ApiError::internal(format!("Database error: {e}")))
    })
    .await
    .map_err(|_| ApiError::internal("Stats task panicked"))??;

    Ok(Json(ApiResponse::ok(StatsResponse {
        total_assets: stats.total_assets,
        total_verifications: stats.total_verifications,
        total_fingerprints: stats.total_fingerprints,
        c2pa_signed_count: stats.c2pa_signed_count,
    })))
}

// ── API key management ───────────────────────────────────────────────────────

/// `POST /api/v1/auth/keys` — create a new API key.
///
/// The raw key is returned once in the `key` field and cannot be retrieved
/// again. Store it securely.
#[utoipa::path(
    post,
    path = "/api/v1/auth/keys",
    tag = "Auth",
    operation_id = "createApiKey",
    summary = "Create API key",
    description = "Create a new API key. The raw key is returned once only — store it securely.",
    request_body(content = CreateKeyRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "New key (shown once only)", body = CreateKeyResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn create_api_key(
    State(state): State<SharedState>,
    Json(body): Json<CreateKeyRequest>,
) -> Result<Json<ApiResponse<CreateKeyResponse>>, ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::bad_request("Key name must not be empty"));
    }

    let name = body.name.trim().to_string();
    let rate_limit = body.rate_limit.unwrap_or(100).max(1);

    let response = tokio::task::spawn_blocking(move || -> Result<CreateKeyResponse, ApiError> {
        // Generate a cryptographically random raw key.
        let raw_key = uuid::Uuid::new_v4().to_string().replace('-', "");
        let key_id = uuid::Uuid::new_v4().to_string();
        let key_hash = hash_key(&raw_key);
        let now = chrono::Utc::now().to_rfc3339();

        let guard = state
            .lock()
            .map_err(|_| ApiError::internal("State lock poisoned"))?;
        guard
            .db
            .create_api_key(&key_id, &name, &key_hash, rate_limit)
            .map_err(|e| ApiError::internal(format!("Database error: {e}")))?;
        drop(guard);

        Ok(CreateKeyResponse {
            key_id,
            key: format!("jt_{raw_key}"),
            name,
            rate_limit,
            created_at: now,
        })
    })
    .await
    .map_err(|_| ApiError::internal("Key creation task panicked"))??;

    Ok(Json(ApiResponse::ok(response)))
}

/// `GET /api/v1/auth/keys` — list all API keys (raw key values are never returned).
///
/// The `key_hash` field is stripped from each record before returning —
/// the hash is only used for internal lookup and must never be exposed.
#[utoipa::path(
    get,
    path = "/api/v1/auth/keys",
    tag = "Auth",
    operation_id = "listApiKeys",
    summary = "List API keys",
    description = "List all API keys. Raw key values and hashes are never returned.",
    responses(
        (status = 200, description = "Key list (no raw values)"),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn list_api_keys_handler(
    state: State<SharedState>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    let keys = tokio::task::spawn_blocking({
        let state = state.0.clone();
        move || {
            let guard = state
                .lock()
                .map_err(|_| ApiError::internal("State lock poisoned"))?;
            guard
                .db
                .list_api_keys()
                .map_err(|e| ApiError::internal(format!("Database error: {e}")))
        }
    })
    .await
    .map_err(|_| ApiError::internal("List keys task panicked"))??;

    let sanitized: Vec<serde_json::Value> = keys
        .into_iter()
        .map(|k| {
            json!({
                "keyId": k.key_id,
                "name": k.name,
                "rateLimit": k.rate_limit,
                "revoked": k.revoked,
                "createdAt": k.created_at,
            })
        })
        .collect();

    Ok(Json(ApiResponse::ok(sanitized)))
}

/// `DELETE /api/v1/auth/keys/{key_id}` — revoke an API key.
#[utoipa::path(
    delete,
    path = "/api/v1/auth/keys/{key_id}",
    tag = "Auth",
    operation_id = "revokeApiKey",
    summary = "Revoke API key",
    description = "Permanently revoke an API key. Authentication using the revoked key \
                   will immediately return 401.",
    params(
        ("key_id" = String, Path, description = "UUID of the key to revoke")
    ),
    responses(
        (status = 200, description = "Key revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn revoke_api_key(
    State(state): State<SharedState>,
    AxumPath(key_id): AxumPath<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    tokio::task::spawn_blocking(move || {
        let guard = state
            .lock()
            .map_err(|_| ApiError::internal("State lock poisoned"))?;
        guard
            .db
            .revoke_api_key(&key_id)
            .map_err(|e| ApiError::internal(format!("Database error: {e}")))
    })
    .await
    .map_err(|_| ApiError::internal("Revoke key task panicked"))??;

    Ok(Json(ApiResponse::ok(json!({ "revoked": true }))))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Infer a file extension from the first bytes using the `infer` crate.
fn infer_extension(bytes: &[u8]) -> &'static str {
    if let Some(kind) = infer::get(bytes) {
        return match kind.extension() {
            "jpg" | "jpeg" => "jpg",
            "png" => "png",
            "webp" => "webp",
            "gif" => "gif",
            "tiff" | "tif" => "tiff",
            "mp4" => "mp4",
            "mov" => "mov",
            "mp3" => "mp3",
            "wav" => "wav",
            "pdf" => "pdf",
            _ => "bin",
        };
    }
    "bin"
}

/// Infer a MIME-like content type string for fingerprint support check.
fn infer_extension_str(bytes: &[u8]) -> String {
    if let Some(kind) = infer::get(bytes) {
        kind.mime_type().to_string()
    } else {
        "application/octet-stream".to_string()
    }
}
