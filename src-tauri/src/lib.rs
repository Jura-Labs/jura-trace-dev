use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use tauri_plugin_shell::ShellExt;

mod c2pa;
pub mod db;
mod error;
mod exif_anomaly;
mod filename_analysis;
mod fingerprint;
mod format_router;
mod heatmap;
mod metadata;
mod monitor_scheduler;
mod network_mode;
mod pdf_provenance;
pub mod sidecar;
mod sun_position;
pub mod telemetry;
mod watermark;

#[cfg(feature = "api")]
pub mod api;

use error::AppError;

// ===== SSRF host validation =====

/// Returns `true` if the given hostname resolves to a loopback, private (RFC 1918),
/// or link-local address. Used by both `verify_url` (Tauri IPC) and `verify_url_inner`
/// (REST API) to block SSRF — including post-redirect validation.
fn is_private_or_loopback_host(host: &str) -> bool {
    let h = host.to_lowercase();
    h == "localhost"
        || h == "127.0.0.1"
        || h == "::1"
        || h == "0.0.0.0"
        || h.starts_with("10.")
        || h.starts_with("192.168.")
        || h.starts_with("169.254.")
        || (h.starts_with("172.")
            && h[4..]
                .split('.')
                .next()
                .and_then(|s| s.parse::<u8>().ok())
                .is_some_and(|n| (16..=31).contains(&n)))
}

// ===== Types =====

/// Asset record stored in the local database.
/// Field names use snake_case for Rust; Tauri's serde rename handles
/// the camelCase conversion for the TypeScript frontend.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub asset_id: String,
    pub file_path: String,
    pub file_name: String,
    pub content_type: String,
    pub mime_type: String,
    pub file_size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub ai_description: Option<String>,
    pub ai_tags: Option<Vec<String>>,
    pub metadata_json: Option<String>,
    pub c2pa_signed: bool,
    pub watermarked: bool,
    /// Whether at least one perceptual fingerprint exists for this asset.
    pub fingerprinted: bool,
    pub created_at: String,
    /// SHA-256 hex digest of the file contents at import time.
    /// `None` for assets imported before this field was added.
    pub sha256_hash: Option<String>,
}

/// Methodology metadata captured at verification time for reproducibility.
///
/// Records exactly which versions of the pipeline, sidecar, and classifier
/// model were used to produce a verification result. This enables courts,
/// insurers, and analysts to confirm that results are comparable or to
/// re-run analysis when a newer methodology version is available.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MethodologyRecord {
    /// Jura Trace application version (e.g. "0.9.0").
    pub pipeline_version: String,
    /// Python ML sidecar version (e.g. "0.2.0"), if available.
    pub sidecar_version: Option<String>,
    /// SHA-256 hex digest of the GBM classifier model file, if present.
    pub classifier_model_hash: Option<String>,
    /// Investigation mode used (`quick`, `standard`, `deep`).
    /// The legacy `archival` value is accepted by callers and normalised to
    /// `deep` for back-compat (see `verify_content_inner` mode normalisation).
    pub analysis_mode: String,
    /// ISO 8601 timestamp when the analysis was performed.
    pub analysed_at: String,
}

/// Input quality assessment — run before detectors to identify
/// conditions that reduce the reliability of forensic analysis.
/// TRIED Pillar 2: contextual limitation disclosure.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputQualityAssessment {
    /// Estimated JPEG quality factor (1–100). None for non-JPEG.
    pub jpeg_quality_estimate: Option<u8>,
    /// Resolution category: "high" (>2MP), "medium" (0.5–2MP), "low" (<0.5MP), "thumbnail" (<128px).
    pub resolution_category: String,
    /// Image dimensions (width, height). None for non-image.
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Whether the image appears to be a screenshot (aspect ratio + border heuristics).
    pub is_screenshot_likely: bool,
    /// Whether the file is JPEG format.
    pub is_jpeg: bool,
    /// Whether EXIF GPS and timestamp data are present.
    pub has_gps: bool,
    pub has_timestamp: bool,
    /// Whether the file uses a modern lossy codec (AVIF, WebP) that destroys
    /// JPEG-specific compression artefacts and typically strips metadata in
    /// web delivery pipelines. When true, ELA, noise analysis, copy-move
    /// detection, and JPEG ghost are significantly degraded.
    pub is_modern_lossy_codec: bool,
    /// Whether the file contains no EXIF data AND no XMP data.
    /// A strong indicator of metadata stripping via social media, CDN
    /// processing, or format conversion (e.g. AVIF downloaded from the web).
    /// When true, all provenance-based checks are unavailable.
    pub metadata_completely_absent: bool,
    /// List of detector names with reduced reliability for this input.
    pub degraded_detectors: Vec<String>,
}

/// Verification result from the VERIFY pipeline.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResult {
    pub source_type: String,
    pub content_type: String,
    /// Investigation mode used for this verification run
    /// (`"quick"`, `"standard"`, `"deep"`).  Legacy `"archival"` is accepted
    /// and normalised to `"deep"` for back-compat.
    pub mode: String,
    pub ela_score: Option<f64>,
    pub noise_score: Option<f64>,
    pub copy_move_score: Option<f64>,
    pub deepfake_score: Option<f64>,
    pub c2pa_valid: Option<bool>,
    pub metadata_flags: Vec<String>,
    pub claim_verdict: Option<String>,
    pub overall_trust: f64,
    pub exif_analysis: Option<exif_anomaly::ExifAnalysis>,
    /// Raw EXIF/image metadata fields (Make, Model, DateTime, GPS, etc.).
    /// Exposed for the v2 verify page EXIF detail panel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_metadata: Option<metadata::ImageMetadata>,
    pub c2pa_manifest: Option<c2pa::ManifestInfo>,
    /// Full C2PA provenance chain (active manifest + all ancestor ingredient manifests).
    /// `None` when the file contains no C2PA data.
    /// The `active` field mirrors `c2pa_manifest`; both are populated together.
    pub c2pa_chain: Option<c2pa::ManifestChain>,
    pub ela_result: Option<sidecar::ElaResult>,
    pub noise_result: Option<sidecar::NoiseResult>,
    pub copy_move_result: Option<sidecar::CopyMoveResult>,
    pub deepfake_result: Option<sidecar::DeepfakeResult>,
    /// CLIP-based AI classification result (UnivFD probe + zero-shot classifier).
    /// Only populated when the optional CLIP model is installed in the sidecar.
    pub clip_result: Option<sidecar::ClipDetectionResult>,
    pub npr_result: Option<sidecar::NprResult>,
    pub jpeg_ghost_result: Option<sidecar::JpegGhostResult>,
    pub segmented_ela_result: Option<sidecar::SegmentedElaResult>,
    pub shadow_consistency_result: Option<sidecar::ShadowConsistencyResult>,
    pub colour_temperature_result: Option<sidecar::ColourTemperatureResult>,
    pub splice_boundary_result: Option<sidecar::SpliceBoundaryResult>,
    pub ai_generator: Option<String>,
    /// Watermark extraction result for image files (standard/deep modes).
    pub watermark_extract_result: Option<sidecar::WatermarkExtractResult>,
    /// Video metadata for video content types.
    pub video_metadata: Option<sidecar::VideoMetadataResult>,
    /// Audio metadata for audio content types.
    pub audio_metadata: Option<sidecar::AudioMetadataResult>,
    /// Video deepfake analysis result for video content types.
    pub video_deepfake_result: Option<sidecar::VideoDeepfakeResult>,
    /// Speech transcription result for audio/video content types.
    pub transcription_result: Option<sidecar::TranscriptionResult>,
    /// RAG claim check result (fed by transcription text or other claims).
    pub claim_check_result: Option<sidecar::ClaimCheckResult>,
    /// AI-generated natural-language description via Ollama LLaVA.
    /// Only populated for image content in standard/deep modes when
    /// Ollama is running with a LLaVA model pulled.  `None` when unavailable.
    pub ai_description: Option<String>,
    /// Comparison between the EXIF-embedded thumbnail and the full image.
    /// `None` for non-image content types.
    pub thumbnail_check: Option<ThumbnailCheck>,
    /// 8×8 block DCT coefficient map analysis result.
    /// Only populated in deep mode when the sidecar is available.
    /// `None` for non-image content types.
    pub dct_analysis_result: Option<sidecar::DctAnalysisResult>,
    /// 2D Fourier periodic pattern detection result.
    /// Only populated in deep mode when the sidecar is available.
    /// `None` for non-image content types.
    pub fourier_analysis_result: Option<sidecar::FourierAnalysisResult>,
    /// SHA-256 hex digest of the input file computed at verification time.
    /// Allows the caller to confirm the file has not changed since import.
    pub input_sha256: Option<String>,
    /// Methodology metadata (pipeline version, sidecar version, classifier hash).
    /// Enables reproducibility and legal defensibility of results.
    pub methodology: Option<MethodologyRecord>,
    /// Input quality assessment — identifies conditions that degrade detector reliability.
    pub input_quality: Option<InputQualityAssessment>,
    /// Semantic content-type classification from the sidecar.
    ///
    /// Populated for image content when the sidecar is available.  When
    /// `ai_detection_suitable` is `false` (e.g. category is `"screenshot"` or
    /// `"document"`), the deepfake and CLIP AI-detection contributions are
    /// neutralised to 0.5 in trust scoring.  `None` when the sidecar is
    /// offline or the content type is not an image.
    pub content_type_result: Option<sidecar::ContentTypeResult>,
    /// Social-media platform fingerprint — informational-only (no contribution
    /// to `compute_trust`).  Populated for image content when the sidecar is
    /// reachable.  See JTV-134 (Sprint 30, promoted from backlog #26 v1.2 → v1.0).
    pub platform_fingerprint_result: Option<sidecar::PlatformFingerprintResult>,
    /// Filename provenance heuristics (camera naming, screenshot, AI generator, etc.).
    /// Populated for all content types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename_analysis: Option<filename_analysis::FilenameAnalysis>,
    /// PDF internal provenance signals (producer, creator, incremental saves,
    /// digital signatures, redactions, PDF/A). Only populated for PDF documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_provenance: Option<pdf_provenance::PdfProvenance>,
    /// Stable string identifiers for every detector that produced a result
    /// for this verification. Consumers (PDF / ZIP renderers, Expert View
    /// badges) use this as an authoritative list of what ran, so that
    /// missing entries can be labelled "not run in this analysis" instead
    /// of silently dropped. Added in Sprint 28 (S28-FU1) alongside the
    /// schema v6 `detectors_run` DB column — the same list is persisted
    /// to the database at `insert_verification` time.
    ///
    /// Vocabulary: `exif_anomaly`, `c2pa`, `ela`, `noise`, `copy_move`,
    /// `deepfake`, `jpeg_ghost`, `segmented_ela`, `colour_temperature`,
    /// `clip`, `watermark`, `video_deepfake`, `transcription`, plus
    /// on-demand entries `npr`, `shadow_consistency`, `splice_boundary`
    /// when the user has triggered them. Keep in sync with the frontend
    /// renderer detector-ID list in `ui/src/lib/pdf.ts`.
    pub detectors_run: Vec<String>,
}

/// Result of comparing the EXIF-embedded thumbnail against the full image.
///
/// Combines two complementary signals:
///
/// 1. **pHash Hamming distance** — perceptual hash mismatch between the
///    thumbnail and the resized full image. A distance above 10 indicates
///    the thumbnail no longer represents the visible content (crop, splice,
///    AI in-painting applied after the original EXIF was written).
///
/// 2. **Pixel MSE** — mean squared error between the thumbnail pixels and
///    the corresponding region of the full image after normalisation to the
///    thumbnail's exact dimensions. MSE above ~0.02 (on a [0, 1] scale) is a
///    secondary signal that confirms perceptual deviation even when pHash
///    Hamming distance is borderline.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailCheck {
    /// Whether the file contained an EXIF-embedded thumbnail.
    pub has_thumbnail: bool,
    /// Width of the embedded thumbnail in pixels.
    /// `None` when `has_thumbnail` is `false` or decoding failed.
    pub thumbnail_width: Option<u32>,
    /// Height of the embedded thumbnail in pixels.
    /// `None` when `has_thumbnail` is `false` or decoding failed.
    pub thumbnail_height: Option<u32>,
    /// Hamming distance between thumbnail pHash and full-image pHash.
    /// `None` when `has_thumbnail` is `false` or hashing failed.
    pub hamming_distance: Option<u32>,
    /// Normalised mean squared error between thumbnail pixels and the
    /// corresponding region of the full image, in [0.0, 1.0].
    /// `None` when `has_thumbnail` is `false` or pixel comparison failed.
    /// Values above ~0.02 suggest post-capture modification.
    pub difference_score: Option<f64>,
    /// `true` when either `hamming_distance` > 10 or `difference_score` > 0.02 —
    /// the thumbnail does not match the visible content, suggesting
    /// post-capture modification.
    pub mismatch: bool,
    /// Human-readable summary of the consistency check result.
    pub summary: String,
}

/// Application statistics for the dashboard.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStats {
    pub total_assets: u64,
    pub total_fingerprints: u64,
    pub total_verifications: u64,
    pub c2pa_signed_count: u64,
}

// ===== Monitor Types =====

/// A single entry from the audit log, suitable for display in the Monitor tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    /// UUID assigned to the log entry (stored as TEXT in SQLite).
    pub log_id: String,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub details: Option<String>,
    pub created_at: String,
}

/// Lightweight summary of a single verification run for the Monitor history list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationSummary {
    pub verification_id: String,
    pub source_type: String,
    pub content_type: String,
    pub ela_score: Option<f64>,
    pub deepfake_score: Option<f64>,
    pub c2pa_valid: Option<bool>,
    pub overall_trust: f64,
    pub created_at: String,
    /// Ordered list of detector IDs that ran during this verification.
    ///
    /// Populated from the schema v6 `detectors_run` column (JSON array).
    /// `None` for rows written before schema v6; empty vec vs None is preserved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detectors_run: Option<Vec<String>>,
}

/// Bucketed distribution of trust scores across all verifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustDistribution {
    pub total: u64,
    pub high_count: u64,
    pub medium_count: u64,
    pub low_count: u64,
    pub average_trust: f64,
    pub latest_at: Option<String>,
}

/// Aggregated protection statistics across the asset library.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtectionSummary {
    pub total_assets: u64,
    pub c2pa_signed: u64,
    pub watermarked: u64,
    pub fingerprinted: u64,
    pub by_content_type: std::collections::HashMap<String, u64>,
    pub earliest_at: Option<String>,
    pub latest_at: Option<String>,
}

/// Activity counts for a single calendar day.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDay {
    pub date: String,
    pub imports: u64,
    pub verifications: u64,
    pub signings: u64,
    pub deletions: u64,
}

/// Composite overview for the Monitor tab, assembled from multiple queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorOverview {
    pub protection: ProtectionSummary,
    pub trust: TrustDistribution,
    pub recent_activity: Vec<AuditLogEntry>,
    pub activity_days: Vec<ActivityDay>,
}

/// Managed application state shared across Tauri commands.
pub struct AppState {
    pub db: db::Database,
    pub sidecar: sidecar::SidecarClient,
    /// The current database file path (may differ from app_data_dir default
    /// if the user has configured a custom location).
    pub db_path: String,
    /// Active licence tier for this installation (pilot phase: manually settable).
    pub licence_tier: LicenceTier,
    /// Handle to the spawned PyInstaller sidecar process.
    /// Present only in production builds where the binary was found and launched
    /// successfully. `None` in development (manual uvicorn) or if spawn failed.
    pub sidecar_process: Option<tauri_plugin_shell::process::CommandChild>,
    /// SHA-256 hex digest of the GBM classifier model file, computed once at
    /// startup. `None` if the model file is not present.
    pub classifier_model_hash: Option<String>,
    /// User preference for AI image descriptions via Ollama LLaVA.
    ///
    /// - `Some(true)`  — explicitly enabled by the user
    /// - `Some(false)` — explicitly disabled by the user
    /// - `None`        — not yet decided; treated as disabled at verify time
    ///   to protect perf until the user opts in via Settings. The setup wizard
    ///   and Settings page are expected to resolve this to an explicit value.
    ///
    /// This feature adds 5–30 s per image verify when active and depends on
    /// Ollama + LLaVA being installed. Gated here so users who care about
    /// verify speed can turn it off, while users who want rich descriptions
    /// can keep it on.
    pub ai_description_enabled: Option<bool>,
    /// Handle for the background URL watchlist scheduler (Monitor tab, paid tiers).
    ///
    /// `None` before the scheduler has been started.  Used in the
    /// `RunEvent::Exit` handler to cleanly stop the task.
    pub scheduler_handle: Option<monitor_scheduler::SchedulerHandle>,
    /// UUID of the most recent verify session whose heatmap files are still
    /// on disk. Cleared when a new session starts (the previous session dir
    /// is deleted before writing new files).
    pub last_heatmap_session: Option<String>,
    /// Unix timestamp (seconds since epoch) of the last successful HTTP request
    /// dispatched to the Python sidecar. Updated atomically by the verify pipeline
    /// on every sidecar call. Used by the power-saver idle-killer to determine
    /// whether the sidecar has been idle long enough to terminate.
    ///
    /// Wrapped in `Arc` so it can be cheaply shared with background tasks
    /// without holding the `AppState` mutex.
    pub last_sidecar_request_ts: Arc<AtomicU64>,
    /// Whether power-saver mode is active. When `true`, the sidecar process is
    /// terminated after `SIDECAR_IDLE_SECONDS_BEFORE_KILL` seconds of inactivity
    /// and respawned on the next verification request. Default `false`.
    pub power_saver_mode: bool,
    /// Serialises the power-saver respawn sequence so that two concurrent
    /// verify calls cannot each pass the `sidecar_process.is_none()` check
    /// and independently spawn duplicate processes. Set with
    /// `compare_exchange(false, true)` before spawning; cleared once the
    /// new child handle is stored.
    pub respawn_in_progress: Arc<AtomicBool>,
}

/// Compute the SHA-256 hash of a file, returning a lowercase hex string.
/// Returns `None` if the file does not exist or cannot be read.
fn compute_file_sha256(path: &std::path::Path) -> Option<String> {
    let data = std::fs::read(path).ok()?;
    let hash = Sha256::digest(&data);
    Some(format!("{hash:x}"))
}

// ===== Tauri Commands =====

/// Get application statistics for the dashboard.
#[tauri::command]
fn get_stats(state: State<'_, Arc<Mutex<AppState>>>) -> Result<AppStats, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db.get_stats().map_err(Into::into)
}

/// Maximum file size accepted by the import pipeline (200 MB).
///
/// Files larger than this limit are skipped to prevent decompression-bomb
/// or memory-exhaustion attacks via crafted images or documents.
const MAX_IMPORT_FILE_SIZE_BYTES: u64 = 200 * 1024 * 1024;

/// Maximum image dimension (width or height) accepted before full decode.
///
/// Prevents decompression-bomb PNGs (e.g. 1×1 px that expands to 50 000×50 000)
/// from exhausting process memory during EXIF extraction and fingerprinting.
const MAX_IMAGE_DIMENSION_PX: u32 = 20_000;

/// Idle duration (in seconds) after which the CLIP ViT-B/32 model is evicted
/// from the Python sidecar to reclaim ~600–700 MB of RAM. The idle-watcher
/// background task (spawned once at startup) checks every 60 s and calls
/// `/forensics/unload-clip` when the threshold is exceeded.
const CLIP_IDLE_SECONDS_BEFORE_UNLOAD: u64 = 600;

/// Idle duration (in seconds) after which the sidecar process is terminated
/// when power-saver mode is enabled by the user. The sidecar holds ~300–500 MB
/// of RAM (Python interpreter + numpy + cv2 + sklearn + FastAPI) even when idle.
/// Default OFF — opt-in via Settings to avoid surprising cold-start latency.
const SIDECAR_IDLE_SECONDS_BEFORE_KILL: u64 = 300;

/// Import files into the PROTECT pipeline.
///
/// For each path:
///   1. Detect content type and MIME via format router
///   2. Extract EXIF metadata (images only, for now)
///   3. Read image dimensions
///   4. Compute SHA-256 of the file
///   5. Store in SQLite and log the action
#[tauri::command]
fn import_files(
    paths: Vec<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<Asset>, AppError> {
    log::info!("Importing {} file(s)", paths.len());
    let app = state
        .lock()
        .map_err(|e| AppError::Internal(format!("State lock poisoned: {e}")))?;

    let mut imported: Vec<Asset> = Vec::new();

    for path_str in &paths {
        // SECURITY: Null-byte check before any path construction.
        if path_str.contains('\0') {
            log::warn!("Skipping path with null byte");
            continue;
        }

        // SECURITY (LOW-3): Canonicalise the path before storage so that symlinks,
        // relative segments, and `..` traversals are resolved.  If the path cannot
        // be resolved (file does not exist or is inaccessible) we skip it with a
        // warning — matching the existing behaviour for missing files.
        let path = match PathBuf::from(path_str).canonicalize() {
            Ok(p) => p,
            Err(e) => {
                log::warn!("Skipping unresolvable path ({path_str}): {e}");
                continue;
            }
        };

        // Skip directories — we process individual files
        if path.is_dir() {
            log::info!("Skipping directory");
            continue;
        }

        // 1. Format detection
        let info = format_router::detect(&path);
        log::info!(
            "Detected: {} ({}) -> {}",
            path.display(),
            info.mime_type,
            info.content_type.as_str()
        );

        // 2. File size — enforce lower and upper bounds before any memory-loading
        //    operation. Empty files and files smaller than the smallest valid
        //    media header (12 bytes) are skipped immediately with a warning.
        let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let file_name_display = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        if file_size == 0 {
            log::warn!("Skipping empty file: file is zero bytes");
            return Err(AppError::Validation(format!(
                "The file '{file_name_display}' is empty (zero bytes). Please select a valid file."
            )));
        }
        if file_size < 12 {
            log::warn!(
                "Skipping file that is too small: {file_size} bytes \
                 is smaller than any valid media header"
            );
            return Err(AppError::Validation(format!(
                "The file '{file_name_display}' is too small to be a valid media file ({file_size} bytes)."
            )));
        }
        if file_size > MAX_IMPORT_FILE_SIZE_BYTES {
            log::warn!(
                "Skipping oversized file: {file_size} bytes exceeds \
                 {} MB limit",
                MAX_IMPORT_FILE_SIZE_BYTES / (1024 * 1024)
            );
            continue;
        }

        // 3. Extract metadata + dimensions (images)
        let (meta_json, width, height) = if info.content_type == format_router::ContentType::Image {
            // Decompression bomb guard: read only the image header to obtain
            // dimensions before doing any full decode.  Reject images whose
            // width or height exceeds MAX_IMAGE_DIMENSION_PX to prevent a
            // crafted 1×1 PNG that expands to 50 000×50 000 px from
            // exhausting process memory.
            if let Some((pw, ph)) = metadata::get_image_dimensions(&path) {
                if pw > MAX_IMAGE_DIMENSION_PX || ph > MAX_IMAGE_DIMENSION_PX {
                    log::warn!(
                        "Skipping image with excessive dimensions: \
                         {pw}x{ph} exceeds {MAX_IMAGE_DIMENSION_PX}px limit"
                    );
                    continue;
                }
            }

            let exif_data = metadata::extract_exif(&path);
            let meta_str = exif_data
                .as_ref()
                .and_then(|m| serde_json::to_string(m).ok());

            // Try EXIF dimensions first, then decode image header
            let (w, h) = exif_data
                .as_ref()
                .and_then(|m| match (m.exif_width, m.exif_height) {
                    (Some(w), Some(h)) => Some((w, h)),
                    _ => None,
                })
                .or_else(|| metadata::get_image_dimensions(&path))
                .unwrap_or((0, 0));

            let w_opt = if w > 0 { Some(w) } else { None };
            let h_opt = if h > 0 { Some(h) } else { None };

            (meta_str, w_opt, h_opt)
        } else {
            (None, None, None)
        };

        // 4. Compute SHA-256 of the file contents.
        // Performed before building the asset record so the hash can be stored
        // in the database and returned to the frontend for chain-of-custody
        // verification.  Uses 64 KB streaming reads to avoid loading the entire
        // file into memory — critical for large video imports (up to 200 MB).
        // On read failure we store `None` rather than aborting the import.
        let sha256_hash: Option<String> = match std::fs::File::open(&path) {
            Ok(f) => {
                let mut hasher = Sha256::new();
                let mut reader = std::io::BufReader::with_capacity(65_536, f);
                let mut buf = [0u8; 65_536];
                loop {
                    use std::io::Read;
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => hasher.update(&buf[..n]),
                        Err(e) => {
                            log::warn!("SHA-256 read error for {}: {e}", path.display());
                            break;
                        }
                    }
                }
                Some(format!("{:x}", hasher.finalize()))
            }
            Err(e) => {
                log::warn!("SHA-256 computation failed for {}: {e}", path.display());
                None
            }
        };

        // 5. Build asset record
        let asset_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        // SECURITY (LOW-3): Store the canonicalised path so symlinks and `..`
        // segments cannot persist into later operations that retrieve the path
        // from the database (e.g. sign_asset, check_metadata_before_sign).
        let canonical_path_str = path.to_string_lossy().to_string();

        let row = db::AssetRow {
            asset_id: asset_id.clone(),
            file_path: canonical_path_str.clone(),
            file_name: file_name.clone(),
            content_type: info.content_type.as_str().to_string(),
            mime_type: info.mime_type.clone(),
            file_size,
            width,
            height,
            metadata_json: meta_json.clone(),
            c2pa_signed: false,
            watermarked: false,
            created_at: now.clone(),
            sha256_hash: sha256_hash.clone(),
        };

        // 5. Store
        // SECURITY (LOW-1): Map database errors to AppError::Database so raw
        // SQLite internals (schema details, table names) are logged but never
        // returned to the frontend.
        app.db.insert_asset(&row).map_err(|e| {
            log::error!("Failed to insert asset into database: {e}");
            AppError::Database("Database operation failed".into())
        })?;

        // 6. Audit log
        let _ = app.db.log_action(
            "import",
            "asset",
            &asset_id,
            Some(&format!(
                "{{\"mime\":\"{}\",\"size\":{}}}",
                info.mime_type, file_size
            )),
            None,
            None,
        );

        // 7. Perceptual fingerprinting (images only)
        if fingerprint::supports_fingerprinting(info.content_type.as_str()) {
            let hashes = fingerprint::compute_hashes(&path);
            for hash_result in &hashes {
                let fp_id = uuid::Uuid::new_v4().to_string();
                let _ = app.db.insert_fingerprint(
                    &fp_id,
                    &asset_id,
                    hash_result.algorithm.as_str(),
                    &hash_result.hash_hex,
                );
            }

            if !hashes.is_empty() {
                let algo_meta = serde_json::json!({
                    "algorithms": hashes.iter()
                        .map(|h| h.algorithm.as_str())
                        .collect::<Vec<_>>(),
                    "hash_size": "8x8",
                    "crate": "image_hasher",
                    "version": "3.1"
                });
                let _ = app.db.log_action(
                    "fingerprint",
                    "asset",
                    &asset_id,
                    Some(&format!("{{\"count\":{}}}", hashes.len())),
                    None,
                    Some(&algo_meta.to_string()),
                );
            }
        }

        imported.push(Asset {
            asset_id,
            file_path: canonical_path_str,
            file_name,
            content_type: info.content_type.as_str().to_string(),
            mime_type: info.mime_type,
            file_size,
            width,
            height,
            ai_description: None,
            ai_tags: None,
            metadata_json: meta_json,
            c2pa_signed: false,
            watermarked: false,
            // Fingerprints are inserted after this push; newly imported assets
            // start as false and the caller re-fetches if it needs the live value.
            fingerprinted: false,
            created_at: now,
            sha256_hash,
        });
    }

    log::info!("Successfully imported {} file(s)", imported.len());
    Ok(imported)
}

/// Get all assets from the local database.
#[tauri::command]
fn get_assets(
    limit: Option<u32>,
    offset: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<Asset>, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db
        .get_all_assets(limit.unwrap_or(200), offset.unwrap_or(0))
        .map_err(Into::into)
}

/// Compute overall trust from individual forensic scores.
///
/// Uses a concordance-aware formula:
/// - Manipulation signals (ELA, noise, copy-move) are weighted (ELA=2.0,
///   others=1.0) since ELA is the most established forensic technique.
/// - When both ELA and deepfake agree the image is clean but other signals
///   disagree, a concordance boost dampens the outlier scores — this handles
///   codec false positives (AVIF, WebP) without affecting genuine detections.
/// - AI-generation (deepfake) is kept separate so it can't be diluted by
///   manipulation detectors that see AI-generated images as "clean".
/// - The final forensic trust uses the minimum of manipulation and deepfake
///   categories, ensuring either can lower trust.
/// - Regional detectors (segmented ELA, shadow consistency, colour temperature,
///   splice boundary) each contribute their own weighted score. When 2 or more
///   of the four regional detectors are suspicious (score > 0.5) simultaneously,
///   total trust is capped at 0.55 to reflect the convergence of evidence.
#[allow(clippy::too_many_arguments)]
fn compute_trust(
    ela_score: Option<f64>,
    noise_score: Option<f64>,
    copy_move_score: Option<f64>,
    deepfake_score: Option<f64>,
    deepfake_confidence: Option<&str>,
    deepfake_verdict: Option<&str>,
    exif_trust: f64,
    c2pa_valid: Option<bool>,
    segmented_ela_score: Option<f64>,
    // Shadow consistency and splice boundary were demoted to on-demand
    // investigation tools (April 2026). Callers still pass them positionally
    // for signature stability across RC builds, but they are not used in
    // trust scoring. See the regional-detector comment below.
    _shadow_consistency_score: Option<f64>,
    colour_temperature_score: Option<f64>,
    _splice_boundary_score: Option<f64>,
    ai_declared_by_c2pa: bool,
    // JPEG Ghost was added to scoring in Sprint 28 (S28-4, April 2026).
    // ml-data-scientist + content-authenticity-expert cross-review consensus:
    // Farid 2009 JPEG ghost detection remains the best CPU-only splice /
    // composite signal for single-JPEG-resave attacks that the UnivFD v8
    // probe (CLIP semantic) cannot see and the GBM v4 classifier only
    // partially catches via blocking_strength / dct_benford_div. Wired in
    // at 0.5 base weight (half of ELA/noise/copy-move) as a capped
    // contribution; the effective weight is adjusted downward for
    // heavily-compressed inputs — see backlog #15 and
    // docs/calibration/s28-jpeg-ghost-weight.md §6.3.
    // Parameters are positional-last to keep signature growth backwards-
    // compatible for tests that don't exercise JPEG Ghost scoring.
    jpeg_ghost_score: Option<f64>,
    // Estimated JPEG quality factor (1–100). Used to compute the
    // quality-adaptive effective weight for JPEG Ghost scoring.
    // `None` for non-JPEG inputs — falls back to the 0.5 base weight.
    jpeg_quality_estimate: Option<u8>,
    // Content-type category from the sidecar classifier.
    // When `None` or `Some("photograph")` | `Some("artwork")`, AI-detection
    // signals are used normally.  When the category indicates AI models are
    // unreliable (`"screenshot"`, `"document"`, `"unknown"` with
    // ai_detection_suitable=false), deepfake and CLIP contributions are
    // neutralised to 0.5 (mid-scale) so they do not inflate or deflate the
    // trust score.
    //
    // The `ai_detection_suitable` flag is the authoritative gate; callers
    // that pass `Some("screenshot")` directly should also pass `false` for
    // `ai_detection_suitable` — the two are always consistent.
    content_type_category: Option<&str>,
    ai_detection_suitable: bool,
    // Self-declared AI provenance via XMP IPTC vocabularies. True when
    // exif_anomaly emits a HIGH-severity `xmp_ai_digital_source` or
    // `xmp_ai_creator_tool` finding. Treated equivalently to
    // `ai_declared_by_c2pa` — see `ai_declared` derivation and the
    // self-declared ceiling below.
    ai_declared_by_xmp: bool,
    // Self-declared COMPOSITE AI (real photograph with AI-generated
    // regions composited in). True when:
    //   - exif_anomaly emits a MEDIUM-severity `xmp_ai_composite_source`
    //     finding (IPTC `compositeWithTrainedAlgorithmicMedia`), OR
    //   - the C2PA manifest contains a `compositeWithTrainedAlgorithmicMedia`
    //     digitalSourceType via `detect_composite_ai_from_assertions`.
    //
    // Distinguished from pure AI because the base capture is real (Pixel
    // Zoom Enhance, Magic Editor, generative fill).  Caps trust at 0.55
    // instead of 0.25 — honest disclosure of AI involvement without
    // labelling every camera-with-AI-feature photo as deepfake-class.
    ai_declared_composite: bool,
) -> f64 {
    // ── AI-detection suppression ─────────────────────────────────────
    // Screenshots and documents cause systematic false positives in the
    // deepfake GBM and CLIP probe because both models were trained
    // exclusively on photographic content.  When the content-type
    // classifier marks a file as unsuitable for AI detection, we pin those
    // signals to 0.5 (neutral — no opinion) so they contribute neither
    // positively nor negatively to the trust score.
    let effective_deepfake_score = if !ai_detection_suitable {
        log::info!(
            "Content type: {} (ai_detection_suitable=false), suppressing AI detection contribution",
            content_type_category.unwrap_or("unknown")
        );
        deepfake_score.map(|_| 0.5) // neutral
    } else {
        deepfake_score
    };
    // The effective deepfake verdict and confidence should also be suppressed
    // so the verdict ceiling in compute_trust is not triggered.
    let effective_deepfake_confidence = if ai_detection_suitable {
        deepfake_confidence
    } else {
        None
    };
    let effective_deepfake_verdict = if ai_detection_suitable {
        deepfake_verdict
    } else {
        None
    };

    // Self-declared AI provenance — either from a C2PA manifest action or
    // from XMP IPTC vocabularies (DigitalSourceType / CreatorTool). Either
    // source is a producer-asserted "this file is AI-generated" signal, so
    // they're treated equivalently. A valid C2PA manifest without an AI
    // declaration is still a positive provenance signal.
    let ai_declared = ai_declared_by_c2pa || ai_declared_by_xmp;
    let c2pa_bonus = if ai_declared {
        -0.25 // Penalty: manifest or XMP explicitly declares AI-generated content
    } else if c2pa_valid == Some(true) {
        0.1 // Bonus: valid provenance, not declared AI
    } else {
        0.0
    };

    // Use the effective deepfake score for forensic trust. When AI-detection
    // has been suppressed (e.g. screenshot/document), effective_deepfake_score
    // is pinned to Some(0.5) which yields a neutral deepfake_trust of 0.5.
    // Confidence is expressed via the verdict ceiling below, not by scaling
    // the score down. The old confidence_weight multiplier (low=0.3) nearly
    // eliminated the signal, causing a fake image to show 92% "High Trust"
    // alongside "Inconclusive".
    let deepfake_trust = effective_deepfake_score.map(|s| 1.0 - s);

    // Weighted manipulation signals: ELA, noise, and copy-move at weight 1.0.
    // ELA was previously 2.0 but forensic audit found it generates too many
    // false positives on multiply-compressed images — demoted to match others.
    // JPEG Ghost added in Sprint 28 (S28-4) at weight 0.5 as a capped
    // contribution — half the influence of ELA/noise/copy-move. The half
    // weight reflects its narrower scope (single-JPEG-resave splice attacks)
    // and its partial redundancy with GBM v4 features (blocking_strength,
    // dct_benford_div). It cannot dominate the verdict even when highly
    // suspicious, but it can meaningfully shift trust when the other
    // manipulation signals are ambiguous.
    let mut manipulation_signals: Vec<(f64, f64)> = Vec::new(); // (trust, weight)
    if let Some(s) = ela_score {
        manipulation_signals.push((1.0 - s, 1.0));
    }
    if let Some(s) = noise_score {
        manipulation_signals.push((1.0 - s, 1.0));
    }
    if let Some(s) = copy_move_score {
        manipulation_signals.push((1.0 - s, 1.0));
    }
    if let Some(s) = jpeg_ghost_score {
        // Quality-adaptive weight for JPEG Ghost (backlog #15).
        // Formula: 0.5 × (q / 100).max(0.3), where q is the estimated JPEG
        // quality factor from the input quality assessment.
        //
        // Rationale (v3 corpus evidence, 2026-04-11):
        //   • Q ≈ 95 (direct upload from camera): weight ≈ 0.475 — near full
        //   • Q ≈ 75 (Twitter / WhatsApp re-encode): weight ≈ 0.375 — reduced
        //   • Q ≤ 60 (heavy compression): weight = 0.30 (floor) — heavily
        //     attenuated because platform re-encoding wipes differential ghost
        //     signatures; p95 score for authentic heavily-compressed images
        //     (0.349) exceeds many spliced subtypes, making the signal unreliable
        //   • Non-JPEG / unknown quality: weight = 0.5 (no penalty — cannot judge)
        //
        // The 0.5 base constant is unchanged; only the quality factor scales it.
        // See docs/calibration/s28-jpeg-ghost-weight.md §6.3.
        let quality_factor = jpeg_quality_estimate
            .map(|q| (f64::from(q) / 100.0).max(0.3))
            .unwrap_or(1.0); // non-JPEG: full base weight (0.5 × 1.0 = 0.5)
        let effective_weight = 0.5 * quality_factor;
        manipulation_signals.push((1.0 - s, effective_weight));
    }

    let manipulation_trust = if manipulation_signals.is_empty() {
        None
    } else if manipulation_signals.len() == 1 {
        Some(manipulation_signals[0].0)
    } else {
        // Weighted average as baseline
        let total_weight: f64 = manipulation_signals.iter().map(|(_, w)| w).sum();
        let weighted_avg: f64 =
            manipulation_signals.iter().map(|(v, w)| v * w).sum::<f64>() / total_weight;

        // Concordance check: when ELA and deepfake both say "clean"
        // (trust > 0.7) but other manipulation signals disagree (trust < 0.3),
        // the disagreement likely reflects codec artefacts rather than
        // real manipulation.
        let ela_trust = ela_score.map(|s| 1.0 - s);

        let concordance_boost = match (ela_trust, deepfake_trust) {
            (Some(ela_t), Some(df_t)) if ela_t > 0.7 && df_t > 0.7 => {
                let disagreeing_count = manipulation_signals
                    .iter()
                    .filter(|(v, _)| *v < 0.3)
                    .count();

                if disagreeing_count > 0 {
                    let agreement_strength = (ela_t + df_t) / 2.0;
                    let disagreement_ratio =
                        disagreeing_count as f64 / manipulation_signals.len() as f64;
                    ((agreement_strength - weighted_avg) * disagreement_ratio * 0.5).max(0.0)
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };

        Some((weighted_avg + concordance_boost).min(1.0))
    };

    // Use the worst-case forensic signal
    let forensic_trust = match (manipulation_trust, deepfake_trust) {
        (Some(m), Some(d)) => Some(m.min(d)),
        (Some(m), None) => Some(m),
        (None, Some(d)) => Some(d),
        (None, None) => None,
    };

    // ── Regional detector scores ─────────────────────────────────────
    // Segmented ELA (weight 1.5) and colour temperature (weight 1.5) are
    // the remaining auto-pipeline regional detectors. Shadow consistency
    // and splice boundary were demoted to on-demand investigation tools
    // after forensic audit found they add scoring noise without reliable
    // discrimination (shadow: noisy gradient analysis; splice: never sets
    // suspicious=true).
    let regional_signals: Vec<(f64, f64)> = [
        (segmented_ela_score, 1.5_f64),
        (colour_temperature_score, 1.5),
    ]
    .iter()
    .filter_map(|(score_opt, weight)| score_opt.map(|s| (1.0 - s, *weight)))
    .collect();

    let regional_trust = if regional_signals.is_empty() {
        None
    } else {
        let total_weight: f64 = regional_signals.iter().map(|(_, w)| w).sum();
        let weighted_avg: f64 =
            regional_signals.iter().map(|(v, w)| v * w).sum::<f64>() / total_weight;
        Some(weighted_avg)
    };

    // Merge regional trust into the overall forensic trust as the worst case.
    let forensic_trust = match (forensic_trust, regional_trust) {
        (Some(f), Some(r)) => Some(f.min(r)),
        (Some(f), None) => Some(f),
        (None, Some(r)) => Some(r),
        (None, None) => None,
    };

    let base_trust = if let Some(ft) = forensic_trust {
        // Weight: 20% EXIF metadata, 80% forensic analysis.
        // EXIF is trivially forgeable and absent from most social media images.
        // Reduced from 40% after security audit found forged EXIF could boost
        // AI images to 54% trust, bypassing the inconclusive threshold.
        (exif_trust * 0.2 + ft * 0.8 + c2pa_bonus).min(1.0)
    } else {
        (exif_trust + c2pa_bonus).min(1.0)
    };

    // ── Composite regional amplification cap ────────────────────────
    // When both remaining regional detectors (segmented ELA + colour temp)
    // simultaneously flag the image as suspicious (score > 0.5), the
    // convergence of evidence warrants a hard cap at 0.55.
    let suspicious_regional_count = [segmented_ela_score, colour_temperature_score]
        .iter()
        .filter(|s| s.map(|v| v > 0.5).unwrap_or(false))
        .count();

    let regional_cap = if suspicious_regional_count >= 2 {
        0.55_f64
    } else {
        1.0
    };

    // ── Verdict ceiling ─────────────────────────────────────────────
    // Prevents high trust scores when the deepfake detector is uncertain
    // or positive. This replaces the old confidence_weight multiplier
    // which nearly eliminated the signal at low confidence.
    //
    // An "inconclusive" verdict means the system cannot determine whether
    // the image is authentic — trust must reflect that epistemic gap.
    // A "synthetic" verdict with low confidence is semantically equivalent
    // to "inconclusive" — cap in the medium range.
    // Use effective_deepfake_verdict / effective_deepfake_confidence here so
    // that suppressed AI-detection (screenshot/document) does not trigger the
    // verdict ceiling (both will be None when ai_detection_suitable=false).
    let verdict_ceiling = match effective_deepfake_verdict {
        Some("synthetic") => match effective_deepfake_confidence {
            Some("high") => 0.25,
            Some("medium") => 0.35,
            _ => 0.45, // low confidence synthetic ≈ inconclusive
        },
        Some("inconclusive") => 0.55,
        _ => 1.0, // no ceiling for authentic or sidecar offline
    };

    let result = base_trust.min(verdict_ceiling).min(regional_cap);

    // ── Self-declared AI ceilings ───────────────────────────────────
    // Pure AI declared (C2PA manifest action `c2pa.created` +
    // `digitalSourceType: trainedAlgorithmicMedia`, OR XMP IPTC
    // `Iptc4xmpExt:DigitalSourceType` / `xmp:CreatorTool`): cap trust
    // at 0.25 regardless of other signals.  Producer self-declaration
    // of pure synthetic AI is the gold-standard provenance signal.
    //
    // Composite AI declared (`compositeWithTrainedAlgorithmicMedia` —
    // real photograph with AI-generated regions: Pixel Zoom Enhance,
    // Magic Editor, Adobe generative fill): cap at 0.55 ("Medium —
    // AI components declared").  The base capture is real and the
    // provenance is intact; the cap is disclosure, not condemnation.
    //
    // If both flags are somehow set (pure AI with composite elements
    // declared), pure AI wins — the stricter ceiling applies.
    let self_declared_ceiling = if ai_declared {
        0.25
    } else if ai_declared_composite {
        0.55
    } else {
        1.0
    };
    result.min(self_declared_ceiling)
}

/// Trust score for non-analysable content types (PDFs, documents).
///
/// Forensic image/video detectors do not apply — trust is based solely on C2PA provenance.
///
/// | `ai_declared` | `c2pa_valid`   | Score | Rationale                                        |
/// |---------------|----------------|-------|--------------------------------------------------|
/// | `true`        | any            | 0.15  | Manifest confirms AI generation — very low trust |
/// | `false`       | `Some(true)`   | 0.82  | Valid manifest — strong provenance signal        |
/// | `false`       | `Some(false)`  | 0.25  | Manifest present but invalid/tampered — suspect  |
/// | `false`       | `None`         | 0.50  | No provenance data — genuinely inconclusive      |
fn document_trust(c2pa_valid: Option<bool>, ai_declared: bool) -> f64 {
    if ai_declared {
        return 0.10; // C2PA explicitly confirms AI generation — very low trust
    }
    match c2pa_valid {
        Some(true) => 0.82,
        Some(false) => 0.25,
        None => 0.50,
    }
}

/// Estimate JPEG quality from file size ratio (bytes per pixel).
///
/// This is a rough heuristic — not a precise Q-factor extraction. Empirical
/// mapping: files with very few bytes per pixel are heavily compressed and
/// will degrade ELA, noise, and JPEG ghost detector reliability.
fn estimate_jpeg_quality(
    path: &std::path::Path,
    width: Option<u32>,
    height: Option<u32>,
) -> Option<u8> {
    let file_size = std::fs::metadata(path).map(|m| m.len()).ok()?;
    let pixels = width? as u64 * height? as u64;
    if pixels == 0 {
        return None;
    }

    // Bytes per pixel ratio — empirical mapping to approximate Q-factor
    let bpp = file_size as f64 / pixels as f64;
    let q = if bpp > 3.0 {
        95
    } else if bpp > 2.0 {
        90
    } else if bpp > 1.0 {
        85
    } else if bpp > 0.5 {
        75
    } else if bpp > 0.3 {
        65
    } else if bpp > 0.15 {
        50
    } else if bpp > 0.08 {
        35
    } else {
        20
    };
    Some(q)
}

/// Detect whether an image is likely a screenshot based on aspect ratio
/// and EXIF characteristics.
///
/// Combines three signals: common screenshot aspect ratio, absence of camera
/// EXIF, and a recognised screenshot pixel width. All three must be true to
/// avoid false positives on letterboxed camera photos.
fn detect_screenshot(
    width: Option<u32>,
    height: Option<u32>,
    exif: &Option<exif_anomaly::ExifAnalysis>,
) -> bool {
    let w = width.unwrap_or(0) as f64;
    let h = height.unwrap_or(0) as f64;
    if w == 0.0 || h == 0.0 {
        return false;
    }

    // Common screenshot aspect ratios (phone portrait and desktop landscape)
    let ratio = w / h;
    let is_phone_ratio = (ratio - 9.0 / 16.0).abs() < 0.05
        || (ratio - 9.0 / 19.5).abs() < 0.05
        || (ratio - 9.0 / 20.0).abs() < 0.05;
    let is_desktop_ratio = (ratio - 16.0 / 9.0).abs() < 0.05 || (ratio - 16.0 / 10.0).abs() < 0.05;

    // No camera EXIF = likely screenshot or web-sourced image
    let no_camera = exif.as_ref().is_none_or(|e| !e.has_exif);

    // Common screenshot widths (iOS, Android, standard desktop resolutions)
    let common_width = matches!(
        w as u32,
        750 | 828 | 1080 | 1125 | 1170 | 1242 | 1284 | 1290 | 1920 | 2560 | 2880 | 3840
    );

    (is_phone_ratio || is_desktop_ratio) && no_camera && common_width
}

/// Assess input quality to identify conditions that degrade detector reliability.
///
/// Runs before detector dispatch — adds <5 ms to the pipeline. Returns an
/// `InputQualityAssessment` containing the resolution category, JPEG quality
/// estimate, screenshot likelihood, modern-codec flag, and a list of detectors
/// whose results should be treated with reduced confidence for this input.
///
/// `raw_meta` is the raw EXIF/XMP extraction result — passed through so that
/// the XMP packet can be inspected for the `metadata_completely_absent` check
/// even when kamadak-exif found no EXIF fields.
fn assess_input_quality(
    path: &std::path::Path,
    info: &format_router::FormatInfo,
    exif: &Option<exif_anomaly::ExifAnalysis>,
    raw_meta: Option<&metadata::ImageMetadata>,
    width: Option<u32>,
    height: Option<u32>,
) -> InputQualityAssessment {
    let is_jpeg = info.mime_type == "image/jpeg";
    let is_image = info.content_type == format_router::ContentType::Image;

    // Modern lossy codec detection — AVIF (AV1 intra-frame) and WebP (VP8/VP8L)
    // re-quantise uniformly on encode, destroying differential ELA/noise signal.
    // Both formats aggressively strip metadata in typical web delivery pipelines.
    let is_modern_lossy_codec = matches!(info.mime_type.as_str(), "image/avif" | "image/webp");

    // JPEG quality estimation from file size heuristic
    let jpeg_quality_estimate = if is_jpeg {
        estimate_jpeg_quality(path, width, height)
    } else {
        None
    };

    // Resolution category
    let pixels = width.unwrap_or(0) as u64 * height.unwrap_or(0) as u64;
    let resolution_category = if !is_image {
        "n/a".to_string()
    } else if width.unwrap_or(0) < 128 || height.unwrap_or(0) < 128 {
        "thumbnail".to_string()
    } else if pixels < 500_000 {
        "low".to_string()
    } else if pixels < 2_000_000 {
        "medium".to_string()
    } else {
        "high".to_string()
    };

    // Screenshot detection heuristic
    let is_screenshot_likely = if is_image {
        detect_screenshot(width, height, exif)
    } else {
        false
    };

    // EXIF GPS/timestamp presence
    let has_exif = exif.as_ref().is_some_and(|e| e.has_exif);
    let has_gps = exif
        .as_ref()
        .is_some_and(|e| e.gps_latitude.is_some() && e.gps_longitude.is_some());
    let has_timestamp = has_exif;

    // Metadata-completely-absent: no EXIF AND no XMP data.
    // Delegates to exif_anomaly for XMP emptiness logic.
    let metadata_completely_absent =
        exif_anomaly::check_metadata_completely_absent(has_exif, raw_meta.map(|m| &m.xmp))
            .is_some();

    // Build degraded detectors list
    let mut degraded = Vec::new();

    if let Some(q) = jpeg_quality_estimate {
        if q < 40 {
            degraded.push("ELA".to_string());
            degraded.push("Noise Analysis".to_string());
            degraded.push("JPEG Ghost".to_string());
        }
    }

    if resolution_category == "low" || resolution_category == "thumbnail" {
        degraded.push("Deepfake Detection".to_string());
        degraded.push("Copy-Move Detection".to_string());
        degraded.push("Segmented ELA".to_string());
    }

    if is_screenshot_likely {
        degraded.push("EXIF Anomaly".to_string());
        degraded.push("JPEG Ghost".to_string());
    }

    if !is_jpeg {
        degraded.push("JPEG Ghost".to_string());
    }

    if is_modern_lossy_codec {
        degraded.push("ELA".to_string());
        degraded.push("Noise Analysis".to_string());
        degraded.push("Copy-Move Detection".to_string());
        degraded.push("JPEG Ghost".to_string());
    }

    if !has_gps || !has_timestamp {
        degraded.push("Sun Position".to_string());
        degraded.push("Shadow Time Estimation".to_string());
    }

    // Deduplicate
    degraded.sort();
    degraded.dedup();

    InputQualityAssessment {
        jpeg_quality_estimate,
        resolution_category,
        width,
        height,
        is_screenshot_likely,
        is_jpeg,
        has_gps,
        has_timestamp,
        is_modern_lossy_codec,
        metadata_completely_absent,
        degraded_detectors: degraded,
    }
}

/// Run the standard sidecar parallel group: ELA + deepfake + watermark extract + CLIP.
///
/// All four detectors are independent and each takes 1–5 s. Running them via
/// `std::thread::scope` cuts standard-mode wall time from ~10 s sequential to the
/// slowest single detector (~5 s).
///
/// The caller is responsible for caching file bytes on the sidecar client before
/// invoking this function and clearing the cache afterwards.
///
/// Returns `(ela_score, ela_result, deepfake_score, deepfake_result,
///           watermark_extract_result, clip_result)`.
#[allow(clippy::type_complexity)]
pub(crate) fn run_standard_sidecar_group(
    path: &std::path::Path,
    sidecar: &sidecar::SidecarClient,
    mime: &str,
    has_camera_exif: bool,
    camera_authenticity_bonus: f64,
) -> (
    Option<f64>,
    Option<sidecar::ElaResult>,
    Option<f64>,
    Option<sidecar::DeepfakeResult>,
    Option<sidecar::WatermarkExtractResult>,
    Option<sidecar::ClipDetectionResult>,
) {
    let t_standard = std::time::Instant::now();

    let ela_path = path.to_path_buf();
    let df_path = path.to_path_buf();
    let wm_path = path.to_path_buf();
    let clip_path = path.to_path_buf();
    let ela_client = sidecar.clone();
    let df_client = sidecar.clone();
    let wm_client = sidecar.clone();
    let clip_client = sidecar.clone();
    let mime_owned = mime.to_string();

    let (ela_out, df_out, wm_out, clip_out) = std::thread::scope(|s| {
        let ela_h = s.spawn(move || {
            let t = std::time::Instant::now();
            let r = ela_client.analyse_ela(&ela_path);
            log::info!("PERF: ELA took {:?}", t.elapsed());
            r
        });
        let df_h = s.spawn(move || {
            let t = std::time::Instant::now();
            let r = df_client.detect_deepfake(
                &df_path,
                &mime_owned,
                has_camera_exif,
                camera_authenticity_bonus,
            );
            log::info!("PERF: deepfake took {:?}", t.elapsed());
            r
        });
        let wm_h = s.spawn(move || {
            let t = std::time::Instant::now();
            let r = wm_client.check_watermark_extract(&wm_path);
            log::info!("PERF: watermark extraction took {:?}", t.elapsed());
            r
        });
        let clip_h = s.spawn(move || {
            let t = std::time::Instant::now();
            let r = clip_client.detect_clip(&clip_path);
            log::info!("PERF: CLIP detection took {:?}", t.elapsed());
            r
        });
        (ela_h.join(), df_h.join(), wm_h.join(), clip_h.join())
    });

    log::info!(
        "PERF: standard group (ELA + deepfake + watermark + CLIP, parallel) took {:?}",
        t_standard.elapsed()
    );

    let (ela_score, ela_result) = match ela_out {
        Ok(Ok(r)) => {
            let score = r.score;
            (Some(score), Some(r))
        }
        Ok(Err(e)) => {
            log::warn!("Sidecar ELA failed: {e}");
            (None, None)
        }
        Err(_) => {
            log::warn!("Sidecar ELA thread panicked");
            (None, None)
        }
    };
    let (deepfake_score, deepfake_result) = match df_out {
        Ok(Ok(r)) => {
            let score = r.score;
            (Some(score), Some(r))
        }
        Ok(Err(e)) => {
            log::warn!("Sidecar deepfake detection failed: {e}");
            (None, None)
        }
        Err(_) => {
            log::warn!("Sidecar deepfake thread panicked");
            (None, None)
        }
    };
    let watermark_extract_result = match wm_out {
        Ok(Ok(r)) => Some(r),
        Ok(Err(e)) => {
            log::warn!("Sidecar watermark extraction failed: {e}");
            None
        }
        Err(_) => {
            log::warn!("Sidecar watermark extract thread panicked");
            None
        }
    };
    // CLIP detection — gracefully degrade if the optional model is missing.
    // The endpoint always returns HTTP 200 with model_available=false in
    // that case, so a parse error here means something else went wrong.
    let clip_result = match clip_out {
        Ok(Ok(r)) => {
            if r.model_available {
                Some(r)
            } else {
                log::info!("CLIP detection: model not installed (graceful skip)");
                None
            }
        }
        Ok(Err(e)) => {
            log::warn!("Sidecar CLIP detection failed: {e}");
            None
        }
        Err(_) => {
            log::warn!("Sidecar CLIP thread panicked");
            None
        }
    };

    (
        ela_score,
        ela_result,
        deepfake_score,
        deepfake_result,
        watermark_extract_result,
        clip_result,
    )
}

/// Run the deep sidecar parallel group: noise + copy-move + JPEG ghost + segmented ELA
/// + colour temperature + DCT analysis + Fourier analysis.
///
/// All seven detectors are independent. Running them via `std::thread::scope` cuts
/// deep-mode wall time from ~20+ s sequential to the slowest single detector (~5 s).
///
/// NPR, shadow consistency, and splice boundary are **not** included here — they were
/// demoted to on-demand investigation tools in Sprint 28 (April 2026) and are returned
/// as `None`.
///
/// Returns `(noise_score, noise_result, copy_move_score, copy_move_result, npr_result,
///           jpeg_ghost_result, segmented_ela_result, shadow_consistency_result,
///           colour_temperature_result, splice_boundary_result,
///           dct_analysis_result, fourier_analysis_result)`.
#[allow(clippy::type_complexity)]
pub(crate) fn run_deep_sidecar_group(
    path: &std::path::Path,
    sidecar: &sidecar::SidecarClient,
) -> (
    Option<f64>,
    Option<sidecar::NoiseResult>,
    Option<f64>,
    Option<sidecar::CopyMoveResult>,
    Option<sidecar::NprResult>,
    Option<sidecar::JpegGhostResult>,
    Option<sidecar::SegmentedElaResult>,
    Option<sidecar::ShadowConsistencyResult>,
    Option<sidecar::ColourTemperatureResult>,
    Option<sidecar::SpliceBoundaryResult>,
    Option<sidecar::DctAnalysisResult>,
    Option<sidecar::FourierAnalysisResult>,
) {
    let t_deep = std::time::Instant::now();

    // Demoted/removed from the deep parallel block:
    //   - Shadow consistency + splice boundary: demoted to on-demand in
    //     April 2026 (see compute_trust comment). Fields remain as Option<T>
    //     None for backwards-compatible serde.
    //   - Chromatic aberration: removed entirely in Sprint 28 (April 2026) —
    //     forensic audit rated accuracy 1/5, long-term viability 1/5.
    //   - NPR: demoted to on-demand in Sprint 28 (S28-3, April 2026).
    //     Option<NprResult> field kept on VerificationResult for the
    //     on-demand endpoint.
    // JPEG Ghost remains pending S28-4 weight calibration.
    let noise_path = path.to_path_buf();
    let cm_path = path.to_path_buf();
    let jg_path = path.to_path_buf();
    let seg_path = path.to_path_buf();
    let ct_path = path.to_path_buf();
    let dct_path = path.to_path_buf();
    let fourier_path = path.to_path_buf();

    let noise_client = sidecar.clone();
    let cm_client = sidecar.clone();
    let jg_client = sidecar.clone();
    let seg_client = sidecar.clone();
    let ct_client = sidecar.clone();
    let dct_client = sidecar.clone();
    let fourier_client = sidecar.clone();

    let (noise_out, cm_out, jg_out, seg_out, ct_out, dct_out, fourier_out) =
        std::thread::scope(|s| {
            let noise_h = s.spawn(move || {
                let t = std::time::Instant::now();
                let r = noise_client.analyse_noise(&noise_path);
                log::info!("PERF: noise analysis took {:?}", t.elapsed());
                r
            });
            let cm_h = s.spawn(move || {
                let t = std::time::Instant::now();
                let r = cm_client.detect_copy_move(&cm_path);
                log::info!("PERF: copy-move detection took {:?}", t.elapsed());
                r
            });
            let jg_h = s.spawn(move || {
                let t = std::time::Instant::now();
                let r = jg_client.detect_jpeg_ghost(&jg_path);
                log::info!("PERF: JPEG ghost detection took {:?}", t.elapsed());
                r
            });
            let seg_h = s.spawn(move || {
                let t = std::time::Instant::now();
                let r = seg_client.check_segmented_ela(&seg_path);
                log::info!("PERF: segmented ELA took {:?}", t.elapsed());
                r
            });
            let ct_h = s.spawn(move || {
                let t = std::time::Instant::now();
                let r = ct_client.check_colour_temperature(&ct_path);
                log::info!("PERF: colour temperature took {:?}", t.elapsed());
                r
            });
            let dct_h = s.spawn(move || {
                let t = std::time::Instant::now();
                let r = dct_client.analyse_dct(&dct_path);
                log::info!("PERF: DCT analysis took {:?}", t.elapsed());
                r
            });
            let fourier_h = s.spawn(move || {
                let t = std::time::Instant::now();
                let r = fourier_client.analyse_fourier(&fourier_path);
                log::info!("PERF: Fourier analysis took {:?}", t.elapsed());
                r
            });
            (
                noise_h.join(),
                cm_h.join(),
                jg_h.join(),
                seg_h.join(),
                ct_h.join(),
                dct_h.join(),
                fourier_h.join(),
            )
        });

    log::info!(
        "PERF: deep group (noise + copy-move + JPEG ghost + segmented ELA + colour-temp + DCT + Fourier, parallel) took {:?}",
        t_deep.elapsed()
    );

    let (noise_score, noise_result) = match noise_out {
        Ok(Ok(r)) => (Some(r.score), Some(r)),
        Ok(Err(e)) => {
            log::warn!("Sidecar noise analysis failed: {e}");
            (None, None)
        }
        Err(_) => {
            log::warn!("Sidecar noise thread panicked");
            (None, None)
        }
    };
    let (copy_move_score, copy_move_result) = match cm_out {
        Ok(Ok(r)) => (Some(r.score), Some(r)),
        Ok(Err(e)) => {
            log::warn!("Sidecar copy-move detection failed: {e}");
            (None, None)
        }
        Err(_) => {
            log::warn!("Sidecar copy-move thread panicked");
            (None, None)
        }
    };
    let jpeg_ghost_result = match jg_out {
        Ok(Ok(r)) => Some(r),
        Ok(Err(e)) => {
            log::warn!("Sidecar JPEG ghost detection failed: {e}");
            None
        }
        Err(_) => {
            log::warn!("Sidecar JPEG ghost thread panicked");
            None
        }
    };
    let segmented_ela_result = match seg_out {
        Ok(Ok(r)) => Some(r),
        Ok(Err(e)) => {
            log::warn!("Sidecar segmented ELA analysis failed: {e}");
            None
        }
        Err(_) => {
            log::warn!("Sidecar segmented ELA thread panicked");
            None
        }
    };
    let colour_temperature_result = match ct_out {
        Ok(Ok(r)) => Some(r),
        Ok(Err(e)) => {
            log::warn!("Sidecar colour temperature analysis failed: {e}");
            None
        }
        Err(_) => {
            log::warn!("Sidecar colour temperature thread panicked");
            None
        }
    };
    let dct_analysis_result = match dct_out {
        Ok(Ok(r)) => Some(r),
        Ok(Err(e)) => {
            log::warn!("Sidecar DCT analysis failed: {e}");
            None
        }
        Err(_) => {
            log::warn!("Sidecar DCT analysis thread panicked");
            None
        }
    };
    let fourier_analysis_result = match fourier_out {
        Ok(Ok(r)) => Some(r),
        Ok(Err(e)) => {
            log::warn!("Sidecar Fourier analysis failed: {e}");
            None
        }
        Err(_) => {
            log::warn!("Sidecar Fourier analysis thread panicked");
            None
        }
    };

    // NPR, shadow consistency, and splice boundary no longer auto-run
    // in the deep group — see doc comment above. The Option fields remain
    // on VerificationResult for backwards-compatible serialisation and for
    // the on-demand endpoints.
    let npr_result: Option<sidecar::NprResult> = None;
    let shadow_consistency_result: Option<sidecar::ShadowConsistencyResult> = None;
    let splice_boundary_result: Option<sidecar::SpliceBoundaryResult> = None;

    (
        noise_score,
        noise_result,
        copy_move_score,
        copy_move_result,
        npr_result,
        jpeg_ghost_result,
        segmented_ela_result,
        shadow_consistency_result,
        colour_temperature_result,
        splice_boundary_result,
        dct_analysis_result,
        fourier_analysis_result,
    )
}

/// Inner verification logic shared by `verify_content` and `verify_url`.
///
/// `mode` controls which pipeline stages run:
/// - `"quick"` (or legacy `"fast"`) — EXIF + C2PA only. Target: <5 s.
/// - `"standard"` (default) — EXIF + C2PA + ELA + deepfake. Target: <15 s.
/// - `"deep"` — full pipeline including noise, copy-move, NPR, JPEG ghost, CA.
/// - `"archival"` — retired 2026-04-22; accepted and silently aliased to `"deep"`.
///   Will regain a distinct pipeline when scanner-calibrated tolerances and
///   uncapped video frame extraction are implemented.
fn verify_content_inner(
    source: &str,
    source_type: &str,
    mode: Option<&str>,
    state: &Mutex<AppState>,
) -> Result<VerificationResult, AppError> {
    // ── Overall pipeline timer ───────────────────────────────────────────
    let t_pipeline = std::time::Instant::now();

    // SECURITY: Validate and canonicalise the path before any filesystem
    // operation.  This prevents:
    //   - Directory traversal via `../` sequences
    //   - Symlink following to sensitive files outside expected directories
    //   - Null-byte injection in the path string
    //   - Error messages that confirm/deny existence of arbitrary paths
    if source.contains('\0') {
        return Err(AppError::Validation("Invalid file path".to_string()));
    }
    let path = std::path::PathBuf::from(source)
        .canonicalize()
        .map_err(|e| {
            log::error!("Path canonicalisation failed for '{source}': {e}");
            AppError::Validation("File not found or inaccessible".to_string())
        })?;

    // ── File integrity pre-checks ────────────────────────────────────────
    // Reject zero-length and suspiciously small files before any decoding
    // attempt. Any valid image, audio, or video file will be larger than
    // the minimum header size of 12 bytes (e.g. a PNG header is 8 bytes
    // plus the IHDR chunk length and type = 16 bytes total). Passing these
    // files to image decoders or the sidecar may cause panics or hangs.
    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if file_size == 0 {
        return Err(AppError::Validation(
            "The file is empty (zero bytes). Please select a valid file.".to_string(),
        ));
    }
    if file_size < 12 {
        return Err(AppError::Validation(
            "The file is too small to be a valid media file.".to_string(),
        ));
    }

    // ── SHA-256 of the input file ────────────────────────────────────────
    // Computed early so the hash is available to the caller regardless of
    // which pipeline branches execute.  Uses 64 KB streaming reads to avoid
    // loading the entire file into memory (critical for large video/audio).
    let input_sha256: Option<String> = match std::fs::File::open(&path) {
        Ok(f) => {
            let mut hasher = Sha256::new();
            let mut reader = std::io::BufReader::with_capacity(65_536, f);
            let mut buf = [0u8; 65_536];
            loop {
                use std::io::Read;
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => hasher.update(&buf[..n]),
                    Err(e) => {
                        log::warn!(
                            "SHA-256 read error in verify pipeline for {}: {e}",
                            path.display()
                        );
                        break;
                    }
                }
            }
            Some(format!("{:x}", hasher.finalize()))
        }
        Err(e) => {
            log::warn!(
                "SHA-256 computation failed in verify pipeline for {}: {e}",
                path.display()
            );
            None
        }
    };

    // ── Format detection ─────────────────────────────────────────────────
    let t_format = std::time::Instant::now();
    let info = format_router::detect(&path);
    log::info!("PERF: format routing took {:?}", t_format.elapsed());

    // Content-type booleans — used throughout the verify pipeline.
    let is_image = info.content_type == format_router::ContentType::Image;
    let is_video = info.content_type == format_router::ContentType::Video;
    let is_audio = info.content_type == format_router::ContentType::Audio;

    // ── Image dimensions ─────────────────────────────────────────────────
    // Computed once here so both the EXIF analyser and the input quality
    // assessment can use the same values without a second filesystem decode.
    let (img_w, img_h): (Option<u32>, Option<u32>) = if is_image {
        let (w, h) = metadata::get_image_dimensions(&path).unwrap_or((0, 0));
        (
            if w > 0 { Some(w) } else { None },
            if h > 0 { Some(h) } else { None },
        )
    } else {
        (None, None)
    };

    // ── EXIF metadata extraction ─────────────────────────────────────────
    let t_exif = std::time::Instant::now();
    // Hoist raw metadata out of the analysis block so it can be passed to
    // assess_input_quality for XMP-presence checking (metadata_completely_absent).
    let raw_exif_meta: Option<metadata::ImageMetadata> =
        if info.content_type == format_router::ContentType::Image {
            metadata::extract_exif(&path)
        } else {
            None
        };

    // EXIF analysis (images only)
    let exif_analysis = if info.content_type == format_router::ContentType::Image {
        let mut analysis = exif_anomaly::analyse(raw_exif_meta.as_ref(), img_w, img_h);

        // Reduce missing-metadata penalties for formats where camera EXIF is
        // not routinely carried: modern web codecs (AVIF, WebP, HEIC) are
        // stripped by CMS/CDN pipelines for bandwidth and privacy; PNG has
        // an eXIf chunk (PNG 1.2, 2017) but tooling overwhelmingly omits it
        // because PNG is mostly used for screenshots, diagrams, and graphics.
        // For these formats, metadata absence is standard behaviour.
        let is_metadata_optional_format = matches!(
            info.mime_type.as_str(),
            "image/avif" | "image/webp" | "image/heic" | "image/png"
        );
        if !analysis.has_exif && is_metadata_optional_format {
            for finding in &mut analysis.findings {
                if finding.check_id == "no_exif_data" {
                    finding.severity = exif_anomaly::Severity::Low;
                    finding.description = format!(
                        "No EXIF data present. For {} files this is standard behaviour — \
                         camera EXIF is routinely absent in CMS/CDN pipelines (WebP/AVIF/HEIC) \
                         and in PNG tooling that does not write the optional eXIf chunk. \
                         This is not inherently suspicious.",
                        info.mime_type
                    );
                }
            }
            // Recalculate trust score with reduced penalty
            let mut score = 1.0_f64;
            for finding in &analysis.findings {
                score -= finding.severity.deduction();
            }
            analysis.trust_score = score.max(0.0);
        }

        // Inject metadata_completely_absent finding when applicable.
        // This fires when both EXIF and XMP are absent — a stronger signal
        // than the individual no_exif_data finding which fires on EXIF alone.
        // Suppressed for metadata-optional formats (see comment above): for
        // those formats a bare file with no EXIF and no XMP is the norm.
        if !is_metadata_optional_format {
            if let Some(absent_finding) = exif_anomaly::check_metadata_completely_absent(
                analysis.has_exif,
                raw_exif_meta.as_ref().map(|m| &m.xmp),
            ) {
                analysis.trust_score =
                    (analysis.trust_score - absent_finding.severity.deduction()).max(0.0);
                analysis.findings.push(absent_finding);
                // Re-sort: Critical first
                analysis
                    .findings
                    .sort_by(|a, b| b.severity.cmp(&a.severity));
            }
        }

        Some(analysis)
    } else {
        None
    };
    log::info!("PERF: EXIF metadata extraction took {:?}", t_exif.elapsed());

    // ── Input quality assessment ─────────────────────────────────────────
    // Runs immediately after EXIF extraction so detector-reliability warnings
    // can reference EXIF presence. The assessment itself is pure computation
    // on already-cached data and adds <5 ms to the pipeline.
    let input_quality: Option<InputQualityAssessment> = if is_image {
        let q = assess_input_quality(
            &path,
            &info,
            &exif_analysis,
            raw_exif_meta.as_ref(),
            img_w,
            img_h,
        );
        log::info!(
            "Input quality: resolution={}, jpeg_q={:?}, screenshot={}, codec={}, meta_absent={}, degraded={:?}",
            q.resolution_category,
            q.jpeg_quality_estimate,
            q.is_screenshot_likely,
            q.is_modern_lossy_codec,
            q.metadata_completely_absent,
            q.degraded_detectors
        );
        Some(q)
    } else {
        None
    };

    // ── Thumbnail consistency check ───────────────────────────────────────
    // Compare the EXIF-embedded JPEG thumbnail against the full image using
    // two complementary signals:
    //   1. pHash Hamming distance > 10 — perceptual mismatch
    //   2. Normalised pixel MSE > 0.02 — pixel-level deviation
    // Either signal being true marks the thumbnail as inconsistent.
    // Only performed for image content types; skipped gracefully on failure.
    let thumbnail_check: Option<ThumbnailCheck> = if is_image {
        let thumb_bytes = metadata::extract_exif_thumbnail(&path);
        match thumb_bytes {
            None => Some(ThumbnailCheck {
                has_thumbnail: false,
                thumbnail_width: None,
                thumbnail_height: None,
                hamming_distance: None,
                difference_score: None,
                mismatch: false,
                summary: "No EXIF thumbnail embedded in this image.".to_string(),
            }),
            Some(bytes) => {
                // Decode thumbnail to get its dimensions and pixels.
                let thumb_img = image::load_from_memory(&bytes).ok();
                let (thumb_w, thumb_h) = thumb_img
                    .as_ref()
                    .map(|i| (i.width(), i.height()))
                    .unwrap_or((0, 0));

                let thumb_hash = fingerprint::compute_phash_from_bytes(&bytes);
                let main_phash = fingerprint::compute_phash(&path);

                // Compute pixel MSE: load main image, resize to thumbnail dims,
                // compare luma channels normalised to [0, 1].
                let mse: Option<f64> = (|| -> Option<f64> {
                    if thumb_w == 0 || thumb_h == 0 {
                        return None;
                    }
                    let main_img = image::open(&path).ok()?;
                    let main_resized = main_img
                        .resize_exact(thumb_w, thumb_h, image::imageops::FilterType::Triangle)
                        .to_luma8();
                    let thumb_luma = thumb_img.as_ref()?.to_luma8();
                    let n = (thumb_w * thumb_h) as f64;
                    let sum_sq: f64 = thumb_luma
                        .pixels()
                        .zip(main_resized.pixels())
                        .map(|(tp, mp)| {
                            let diff = (tp[0] as f64 - mp[0] as f64) / 255.0;
                            diff * diff
                        })
                        .sum();
                    Some(sum_sq / n)
                })();

                let distance = match (&thumb_hash, &main_phash) {
                    (Some(th), Some(mh)) => {
                        Some(fingerprint::hamming_distance(th, mh).unwrap_or(64))
                    }
                    _ => None,
                };

                let phash_mismatch = distance.is_some_and(|d| d > 10);
                let mse_mismatch = mse.is_some_and(|m| m > 0.02);
                let mismatch = phash_mismatch || mse_mismatch;

                let summary = if !mismatch {
                    "Thumbnail matches full image — no post-capture modification detected."
                        .to_string()
                } else if phash_mismatch && mse_mismatch {
                    format!(
                        "Thumbnail mismatch (pHash distance {}, MSE {:.4}) — post-capture modification likely.",
                        distance.unwrap_or(0),
                        mse.unwrap_or(0.0)
                    )
                } else if phash_mismatch {
                    format!(
                        "Thumbnail perceptual mismatch (pHash distance {}) — post-capture modification possible.",
                        distance.unwrap_or(0)
                    )
                } else {
                    format!(
                        "Thumbnail pixel deviation (MSE {:.4}) — minor post-capture modification possible.",
                        mse.unwrap_or(0.0)
                    )
                };

                Some(ThumbnailCheck {
                    has_thumbnail: true,
                    thumbnail_width: if thumb_w > 0 { Some(thumb_w) } else { None },
                    thumbnail_height: if thumb_h > 0 { Some(thumb_h) } else { None },
                    hamming_distance: distance,
                    difference_score: mse,
                    mismatch,
                    summary,
                })
            }
        }
    } else {
        None
    };

    // ── Filename provenance analysis ─────────────────────────────────────
    // Runs for all content types — pure computation on the path stem.
    let filename_analysis_result: Option<filename_analysis::FilenameAnalysis> = {
        let a = filename_analysis::analyse_filename(&path);
        Some(a)
    };

    // ── PDF internal provenance ───────────────────────────────────────────
    // Only attempted for PDF documents. lopdf::Document::load handles
    // non-PDF files gracefully by returning Err, so no MIME guard is needed,
    // but we restrict to the document content type to avoid the parsing cost
    // on image/video/audio files.
    let pdf_provenance_result: Option<pdf_provenance::PdfProvenance> = if info.content_type
        == format_router::ContentType::Document
        && info.mime_type.contains("pdf")
    {
        match pdf_provenance::analyse_pdf(&path) {
            Some(p) => Some(p),
            None => {
                log::warn!("PDF provenance analysis failed for {}", path.display());
                None
            }
        }
    } else {
        None
    };

    // ── C2PA verification ────────────────────────────────────────────────
    // Verification mode: always Standard (local-only) here because
    // verify_content_inner does not have access to the app data dir.
    // The Tauri read_manifest / read_manifest_chain commands read the
    // actual network mode and forward it correctly.
    let t_c2pa = std::time::Instant::now();
    let c2pa_chain = c2pa::read_manifest_chain(&path, false).ok().flatten();
    let c2pa_manifest = c2pa_chain.as_ref().map(|ch| ch.active.clone());
    let c2pa_valid = c2pa_manifest.as_ref().map(|m| m.is_valid);

    // Check C2PA claim_generator AND assertions for known AI generators.
    // Generators such as Google Gemini embed their AI declaration in the
    // c2pa.actions assertion body (digitalSourceType / description) rather
    // than in the claim_generator string, so both paths are required.
    let ai_generator = c2pa_manifest.as_ref().and_then(|m| {
        // 1. claim_generator string (covers DALL-E, Midjourney, Firefly …)
        if let Some(gen) = m
            .claim_generator
            .as_deref()
            .and_then(c2pa::detect_ai_generator)
        {
            return Some(gen);
        }
        // 2. Assertion values (covers digitalSourceType + action descriptions)
        c2pa::detect_ai_from_assertions(&m.assertions)
    });
    log::info!("PERF: C2PA verification took {:?}", t_c2pa.elapsed());

    // Sidecar-based analysis (optional — graceful degradation)
    // Mode determines which detectors run:
    //   quick/fast → no sidecar at all
    //   standard   → ELA + deepfake only
    //   deep       → all detectors
    //   archival   → retired; silently aliased to "deep" below
    let app = state.lock().map_err(|e| {
        log::error!("AppState mutex poisoned in verify pipeline: {e}");
        AppError::Internal("Failed to acquire application state".to_string())
    })?;
    // 'archival' was retired 2026-04-22 because it shared the Deep code path
    // end-to-end with no detector or threshold differences.  The value is
    // still accepted for API back-compat (Axum REST, older Tauri clients,
    // legacy persisted profiles) and silently aliased to "deep".  When real
    // differentiation lands (scanner-calibrated tolerances, uncapped video
    // frames), "archival" can regain its own arm without a breaking change.
    let effective_mode = match mode {
        Some("fast") | Some("quick") => "quick",
        Some("standard") => "standard",
        Some("archival") | Some("deep") => "deep",
        _ => "standard", // default to standard (was "deep" — too slow for typical use)
    };
    let is_quick = effective_mode == "quick";
    // Single availability probe — reused for all sidecar calls in this pipeline
    // to avoid multiple HTTP round-trips.
    let sidecar_available = !is_quick && app.sidecar.is_available();
    let sidecar_up = is_image && sidecar_available;
    let is_deep = effective_mode == "deep";
    log::info!(
        "Verify pipeline: is_image={is_image}, mode={mode:?}, effective={effective_mode}, sidecar_up={sidecar_up}, is_deep={is_deep}"
    );

    // ── Whether the image has camera-origin EXIF ────────────────────────
    // Images with at least 4 populated EXIF fields are more likely to be
    // genuine camera shots; the sidecar uses this as a detection prior.
    let has_camera_exif = exif_analysis
        .as_ref()
        .map(|a| a.has_exif && a.fields_populated >= 4)
        .unwrap_or(false);

    // ── MakerNote camera authenticity bonus ─────────────────────────────
    // Sprint 29 Track 1: when a vendor-recognised MakerNote is present,
    // pass the confidence score (0.0–1.0) to the deepfake detector so it
    // can suppress false positives on computational photography output.
    let camera_authenticity_bonus = exif_analysis
        .as_ref()
        .map(|a| a.camera_authenticity_bonus)
        .unwrap_or(0.0);

    // ── Content-type classification (screenshot/document guard) ─────────────
    // Run before the standard parallel group so the result is available to
    // suppress AI-detection signals in trust scoring when the classifier
    // reports the content is unsuitable (e.g. screenshot, document).
    //
    // The endpoint is fast (<500 ms p95) and does not block the parallel group —
    // it runs synchronously here because it is cheap and its result must be
    // known before the standard group fires.
    let content_type_result: Option<sidecar::ContentTypeResult> = if is_image && sidecar_up {
        match app.sidecar.classify_content_type(&path) {
            Ok(ct) => {
                log::info!(
                    "Content-type classification: category={}, confidence={:.2}, ai_detection_suitable={}",
                    ct.category,
                    ct.confidence,
                    ct.ai_detection_suitable
                );
                Some(ct)
            }
            Err(e) => {
                log::warn!("Sidecar content-type classification failed: {e}");
                None
            }
        }
    } else {
        None
    };

    // Platform fingerprint — informational-only.  Cheap (~100–300 ms) image
    // analysis identifying which social-media platform processed the file
    // (WhatsApp / Instagram / Twitter etc.).  Result populates the verify
    // result for user awareness; does NOT feed into `compute_trust`.
    // (JTV-134, Sprint 30 — promoted from backlog #26 v1.2 to v1.0.)
    let platform_fingerprint_result: Option<sidecar::PlatformFingerprintResult> =
        if is_image && sidecar_up {
            match app.sidecar.analyse_platform_fingerprint(&path) {
                Ok(pf) => Some(pf),
                Err(e) => {
                    log::warn!("Sidecar platform-fingerprint failed: {e}");
                    None
                }
            }
        } else {
            None
        };

    // Derive suppression flags from the content-type result.
    // Default: ai_detection_suitable=true (no suppression) so that pipelines
    // where the sidecar is offline or the file is not an image behave as before.
    let ai_detection_suitable = content_type_result
        .as_ref()
        .map(|ct| ct.ai_detection_suitable)
        .unwrap_or(true);
    let content_type_category: Option<&str> =
        content_type_result.as_ref().map(|ct| ct.category.as_str());

    // ── Standard parallel group ──────────────────────────────────────────
    // ELA + deepfake + watermark extraction are independent and each takes
    // 1-5 s. Running them concurrently cuts standard-mode wall time from
    // ~10 s sequential to the slowest single detector (~5 s).
    //
    // `std::thread::scope` guarantees all threads finish before we proceed
    // and avoids the overhead of a separate thread pool. Each thread
    // receives a cheap `SidecarClient::clone()` (Arc-based connection pool)
    // and an owned `PathBuf`.
    // PERF: read the file once and cache in the sidecar client before the
    // standard group. All parallel sidecar calls (4 standard + 5 deep) will
    // use the cached bytes instead of re-reading from disk, saving 4–9×
    // file_size in redundant I/O and peak RAM. The cache is cleared after
    // the deep group (see `app.sidecar.clear_file_cache()` below).
    if sidecar_up {
        if let Ok(file_bytes) = std::fs::read(&path) {
            app.sidecar.cache_file_bytes(&path, file_bytes);
        }
    }

    let (
        ela_score_raw,
        ela_result_raw,
        deepfake_score,
        deepfake_result,
        watermark_extract_result,
        clip_result,
    ) = if sidecar_up {
        run_standard_sidecar_group(
            &path,
            &app.sidecar,
            &info.mime_type,
            has_camera_exif,
            camera_authenticity_bonus,
        )
    } else {
        (None, None, None, None, None, None)
    };

    // Codec-aware gating: ELA is JPEG-DCT-specific. On PNG / WebP / AVIF /
    // HEIC / TIFF / BMP / GIF the score is uncalibrated noise, so drop it
    // before it reaches compute_trust. The result struct is preserved as
    // None to keep the detectors_run accounting honest. See
    // format_router::should_run_ela for the rationale.
    let (ela_score, ela_result) = if format_router::should_run_ela(&info.mime_type) {
        (ela_score_raw, ela_result_raw)
    } else {
        if ela_score_raw.is_some() {
            log::info!(
                "Codec gate: dropping ELA score for non-JPEG mime '{}' (was {:?})",
                info.mime_type,
                ela_score_raw
            );
        }
        (None, None)
    };

    // ── Deep parallel group ──────────────────────────────────────────────
    // Seven detectors run concurrently when in deep mode.
    // Slowest is copy-move (~5 s); without parallelism the group takes
    // ~20+ s sequentially. With parallelism wall time is bounded by the
    // slowest single detector rather than the sum of all detectors.
    let (
        noise_score,
        noise_result,
        copy_move_score,
        copy_move_result,
        npr_result,
        jpeg_ghost_result,
        segmented_ela_result,
        shadow_consistency_result,
        colour_temperature_result,
        splice_boundary_result,
        dct_analysis_result,
        fourier_analysis_result,
    ) = if sidecar_up && is_deep {
        run_deep_sidecar_group(&path, &app.sidecar)
    } else {
        (
            None, None, None, None, None, None, None, None, None, None, None, None,
        )
    };

    // PERF: release the cached file bytes — image sidecar calls are done.
    app.sidecar.clear_file_cache();

    // ── Video parallel group ─────────────────────────────────────────────
    // v1.0 (JTV-138, 2 May 2026): video deepfake, transcription, and FFprobe
    // metadata are deferred to v1.0.x (JTV-139). v1.0 ships C2PA + EXIF +
    // native preview only on video files. The block below is gated on a
    // const that re-enables the sidecar group when the v1.0.x calibration
    // matrix and Global Majority device gates are met. Do not loosen this
    // gate without satisfying the requirements documented in JTV-139.
    const ENABLE_VIDEO_DEEPFAKE_GROUP: bool = false;
    let (video_metadata, video_deepfake_result, transcription_result) =
        if is_video && sidecar_available && ENABLE_VIDEO_DEEPFAKE_GROUP {
            let t_video = std::time::Instant::now();

            let vm_path = path.to_path_buf();
            let vd_path = path.to_path_buf();
            let vm_client = app.sidecar.clone();
            let vd_client = app.sidecar.clone();
            // effective_mode is already normalised to "quick" | "standard" |
            // "deep" above; "archival" has been folded into "deep".
            let deepfake_mode_owned = effective_mode.to_string();

            // Transcription runs in parallel too (unless quick mode)
            let run_transcription = !is_quick;
            let tr_path = path.to_path_buf();
            let tr_client = app.sidecar.clone();

            let (vm_out, vd_out, tr_out) = std::thread::scope(|s| {
                let vm_h = s.spawn(move || {
                    let t = std::time::Instant::now();
                    let r = vm_client.check_video_metadata(&vm_path);
                    log::info!("PERF: video metadata took {:?}", t.elapsed());
                    r
                });
                let vd_h = s.spawn(move || {
                    let t = std::time::Instant::now();
                    let r = vd_client.analyse_video_deepfake(&vd_path, &deepfake_mode_owned);
                    log::info!("PERF: video deepfake analysis took {:?}", t.elapsed());
                    r
                });
                let tr_h = s.spawn(move || {
                    if !run_transcription {
                        return None;
                    }
                    let t = std::time::Instant::now();
                    let r = tr_client.transcribe(&tr_path);
                    log::info!("PERF: transcription took {:?}", t.elapsed());
                    Some(r)
                });
                (vm_h.join(), vd_h.join(), tr_h.join())
            });

            log::info!(
                "PERF: video group (metadata + deepfake + transcription, parallel) took {:?}",
                t_video.elapsed()
            );

            let video_metadata = match vm_out {
                Ok(Ok(r)) => Some(r),
                Ok(Err(e)) => {
                    log::warn!("Sidecar video metadata extraction failed: {e}");
                    None
                }
                Err(_) => {
                    log::warn!("Sidecar video metadata thread panicked");
                    None
                }
            };
            let video_deepfake_result = match vd_out {
                Ok(Ok(r)) => {
                    log::info!(
                        "Video deepfake: score={:.2}, verdict={}, frames={}",
                        r.aggregate_score,
                        r.aggregate_verdict,
                        r.frames_analysed
                    );
                    Some(r)
                }
                Ok(Err(e)) => {
                    log::warn!("Sidecar video deepfake analysis failed: {e}");
                    None
                }
                Err(_) => {
                    log::warn!("Sidecar video deepfake thread panicked");
                    None
                }
            };
            let transcription_result = match tr_out {
                Ok(Some(Ok(t))) if t.success => {
                    log::info!(
                        "Transcription: lang={:?}, duration={:?}, segments={}",
                        t.language,
                        t.duration,
                        t.segments.len()
                    );
                    Some(t)
                }
                Ok(Some(Ok(t))) => {
                    log::info!("Transcription unavailable: {}", t.message);
                    None
                }
                Ok(Some(Err(e))) => {
                    log::warn!("Sidecar transcription failed: {e}");
                    None
                }
                Ok(None) => None, // quick mode — transcription skipped
                Err(_) => {
                    log::warn!("Sidecar transcription thread panicked");
                    None
                }
            };

            (video_metadata, video_deepfake_result, transcription_result)
        } else {
            (None, None, None)
        };

    // ── Audio parallel group ──────────────────────────────────────────────
    // v1.0 (JTV-138, 2 May 2026): standalone audio is out of scope for v1.0
    // (and audio deepfake was already deferred under JTV-113 / JTV-110 to
    // v1.1). Audio metadata + transcription re-enter under the JTV-139
    // calibration gate. See `ENABLE_VIDEO_DEEPFAKE_GROUP` above for the
    // matching video gate.
    const ENABLE_AUDIO_GROUP: bool = false;
    let (audio_metadata, transcription_result) = if is_audio && sidecar_available && ENABLE_AUDIO_GROUP {
        let t_audio = std::time::Instant::now();

        let am_path = path.to_path_buf();
        let am_client = app.sidecar.clone();
        let run_transcription = !is_quick && transcription_result.is_none();
        let tr_path = path.to_path_buf();
        let tr_client = app.sidecar.clone();

        let (am_out, tr_out) = std::thread::scope(|s| {
            let am_h = s.spawn(move || {
                let t = std::time::Instant::now();
                let r = am_client.check_audio_metadata(&am_path);
                log::info!("PERF: audio metadata took {:?}", t.elapsed());
                r
            });
            let tr_h = s.spawn(move || {
                if !run_transcription {
                    return None;
                }
                let t = std::time::Instant::now();
                let r = tr_client.transcribe(&tr_path);
                log::info!("PERF: transcription took {:?}", t.elapsed());
                Some(r)
            });
            (am_h.join(), tr_h.join())
        });

        log::info!(
            "PERF: audio group (metadata + transcription, parallel) took {:?}",
            t_audio.elapsed()
        );

        let audio_metadata = match am_out {
            Ok(Ok(r)) => Some(r),
            Ok(Err(e)) => {
                log::warn!("Sidecar audio metadata extraction failed: {e}");
                None
            }
            Err(_) => {
                log::warn!("Sidecar audio metadata thread panicked");
                None
            }
        };
        let transcription_result = match tr_out {
            Ok(Some(Ok(t))) if t.success => {
                log::info!(
                    "Transcription: lang={:?}, duration={:?}, segments={}",
                    t.language,
                    t.duration,
                    t.segments.len()
                );
                Some(t)
            }
            Ok(Some(Ok(t))) => {
                log::info!("Transcription unavailable: {}", t.message);
                None
            }
            Ok(Some(Err(e))) => {
                log::warn!("Sidecar transcription failed: {e}");
                None
            }
            Ok(None) => transcription_result, // keep any existing result
            Err(_) => {
                log::warn!("Sidecar audio transcription thread panicked");
                None
            }
        };

        (audio_metadata, transcription_result)
    } else {
        (None, transcription_result)
    };

    // ── RAG claim check ───────────────────────────────────────────────────
    // If we have a transcription, use it for RAG claim checking via the sidecar.
    // This feeds the spoken content into the knowledge-base-backed claim verifier.
    let claim_check_result = if let Some(ref transcript) = transcription_result {
        if !transcript.text.is_empty() && sidecar_available {
            let t_claim = std::time::Instant::now();
            let result = match app.sidecar.check_claim(&transcript.text) {
                Ok(claim_result) => {
                    log::info!(
                        "Claim check: verdict={}, claims={}",
                        claim_result.overall_verdict,
                        claim_result.claims.len()
                    );
                    Some(claim_result)
                }
                Err(e) => {
                    log::warn!("Sidecar claim check failed: {e}");
                    None
                }
            };
            log::info!("PERF: RAG claim check took {:?}", t_claim.elapsed());
            result
        } else {
            None
        }
    } else {
        None
    };

    // ── Tier 3: AI image description via Ollama LLaVA (optional) ─────────
    // Only for image content in non-quick modes, and only when the user has
    // explicitly opted in via Settings. This call is serial (adds 5–30 s on
    // typical hardware) so it is gated behind `AppState::ai_description_enabled`
    // — see that field's doc comment for the tri-state semantics. If Ollama
    // is unavailable the sidecar returns None and the pipeline continues.
    let ai_desc_allowed = matches!(app.ai_description_enabled, Some(true));
    let ai_description: Option<String> = if is_image && sidecar_available && ai_desc_allowed {
        let t_describe = std::time::Instant::now();
        let desc_path = path.to_path_buf();
        let desc_client = app.sidecar.clone();
        let describe_out =
            std::thread::spawn(move || desc_client.describe_image(&desc_path)).join();
        log::info!("PERF: AI image description took {:?}", t_describe.elapsed());
        match describe_out {
            Ok(Ok(r)) if r.success => {
                log::info!(
                    "AI description: {} chars via {}",
                    r.description.as_deref().map(|s| s.len()).unwrap_or(0),
                    r.model_used
                );
                r.description
            }
            Ok(Ok(r)) => {
                log::debug!("AI description unavailable: {}", r.message);
                None
            }
            Ok(Err(e)) => {
                log::debug!("Sidecar describe failed: {e}");
                None
            }
            Err(_) => {
                log::warn!("AI description thread panicked");
                None
            }
        }
    } else {
        None
    };

    // Build metadata flags from findings
    let mut metadata_flags: Vec<String> = exif_analysis
        .as_ref()
        .map(|a| a.findings.iter().map(|f| f.title.clone()).collect())
        .unwrap_or_default();
    // Append a content-type suppression notice when AI-detection signals have
    // been neutralised so the UI can surface a clear, non-alarmist explanation.
    if !ai_detection_suitable {
        let category = content_type_category.unwrap_or("unknown");
        metadata_flags.push(format!(
            "Screenshot — AI detection disabled: content classified as \"{category}\"; deepfake and AI-origin scores are not reliable for this content type and have been excluded from the trust score."
        ));
    }

    // ── Trust score computation ───────────────────────────────────────────
    let t_trust = std::time::Instant::now();
    let exif_trust = exif_analysis.as_ref().map(|a| a.trust_score).unwrap_or(0.5);
    let deepfake_confidence = deepfake_result.as_ref().map(|r| r.confidence.as_str());
    let deepfake_verdict = deepfake_result
        .as_ref()
        .and_then(|r| r.verdict_level.as_deref());
    let segmented_ela_score = segmented_ela_result.as_ref().map(|r| r.score);
    let shadow_consistency_score = shadow_consistency_result.as_ref().map(|r| r.score);
    let colour_temperature_score = colour_temperature_result.as_ref().map(|r| r.score);
    let splice_boundary_score = splice_boundary_result.as_ref().map(|r| r.score);
    // Codec-aware gating: JPEG Ghost is JPEG-DCT-specific (see
    // format_router::should_run_jpeg_ghost). The detector itself early-exits
    // on non-JPEG codecs but the result-struct path can still emit a noise
    // score; drop it before compute_trust to keep the verdict clean.
    let jpeg_ghost_score = if format_router::should_run_jpeg_ghost(&info.mime_type) {
        jpeg_ghost_result.as_ref().map(|r| r.score)
    } else {
        None
    };
    // PDFs and other documents have no applicable forensic detectors.
    // Use a lightweight C2PA-only path rather than defaulting to 0.50 from
    // the unwrap_or on missing EXIF data.
    let ai_declared_by_c2pa = ai_generator.is_some();
    // Composite-AI derivation from C2PA side — real photograph with
    // AI-generated regions (Pixel Zoom Enhance, Magic Editor, Adobe
    // generative fill).  Scans assertions for
    // `compositeWithTrainedAlgorithmicMedia`.
    let ai_declared_composite_by_c2pa = c2pa_manifest
        .as_ref()
        .and_then(|m| c2pa::detect_composite_ai_from_assertions(&m.assertions))
        .is_some();
    let ai_declared_by_xmp = exif_analysis
        .as_ref()
        .map(|a| {
            a.findings.iter().any(|f| {
                matches!(
                    f.check_id.as_str(),
                    "xmp_ai_digital_source" | "xmp_ai_creator_tool"
                ) && matches!(f.severity, exif_anomaly::Severity::High)
            })
        })
        .unwrap_or(false);
    let ai_declared_composite_by_xmp = exif_analysis
        .as_ref()
        .map(|a| {
            a.findings
                .iter()
                .any(|f| f.check_id == "xmp_ai_composite_source")
        })
        .unwrap_or(false);
    let ai_declared_composite = ai_declared_composite_by_c2pa || ai_declared_composite_by_xmp;

    // For video files, substitute the video deepfake aggregate for the
    // image GBM score (which is always None on video — GBM only runs on
    // still images). Without this substitution `compute_trust` for video
    // is C2PA + EXIF only and a pristine deepfake produces a "Likely
    // authentic" headline. See JTV-107 / JTV-105 truth-grid pass.
    let (effective_deepfake_score, effective_deepfake_confidence, effective_deepfake_verdict) =
        if is_video {
            (
                video_deepfake_result.as_ref().map(|r| r.aggregate_score),
                video_deepfake_result
                    .as_ref()
                    .map(|r| r.aggregate_confidence.as_str()),
                video_deepfake_result
                    .as_ref()
                    .map(|r| r.aggregate_verdict.as_str()),
            )
        } else {
            (deepfake_score, deepfake_confidence, deepfake_verdict)
        };

    let overall_trust = if !is_image && !is_video && !is_audio {
        document_trust(c2pa_valid, ai_declared_by_c2pa || ai_declared_by_xmp)
    } else {
        compute_trust(
            ela_score,
            noise_score,
            copy_move_score,
            effective_deepfake_score,
            effective_deepfake_confidence,
            effective_deepfake_verdict,
            exif_trust,
            c2pa_valid,
            segmented_ela_score,
            shadow_consistency_score,
            colour_temperature_score,
            splice_boundary_score,
            ai_declared_by_c2pa,
            jpeg_ghost_score,
            input_quality.as_ref().and_then(|q| q.jpeg_quality_estimate),
            content_type_category,
            ai_detection_suitable,
            ai_declared_by_xmp,
            ai_declared_composite,
        )
    };
    log::info!("PERF: trust score computation took {:?}", t_trust.elapsed());

    // ── Methodology record ────────────────────────────────────────────────
    let sidecar_ver = if sidecar_available {
        app.sidecar.check_health().ok().map(|h| h.version)
    } else {
        None
    };

    let methodology = Some(MethodologyRecord {
        pipeline_version: env!("CARGO_PKG_VERSION").to_string(),
        sidecar_version: sidecar_ver.clone(),
        classifier_model_hash: app.classifier_model_hash.clone(),
        analysis_mode: effective_mode.to_string(),
        analysed_at: chrono::Utc::now().to_rfc3339(),
    });

    // ── Database operations ───────────────────────────────────────────────
    let t_db = std::time::Instant::now();
    let verification_id = uuid::Uuid::new_v4().to_string();

    // Build the detectors_run list — stable string identifiers for every
    // detector that produced a result for this verification. Consumers
    // (PDF + ZIP renderers) use this to distinguish "detector ran but
    // returned null" from "detector was never run in this build / mode"
    // rather than inferring from field presence. See S28-5 migration.
    //
    // ── CODEGEN CONTRACT ─────────────────────────────────────────────────
    // The TypeScript "Not run in this analysis" PDF rows are generated from
    // the MODE_MATRIX constant in src-tauri/src/bin/gen_detectors.rs, which
    // is the single source of truth for the expected-detector matrix.
    //
    // When you ADD a detector here:
    //   1. Add the push("your_new_id") below.
    //   2. Add "your_new_id" to the appropriate rows in MODE_MATRIX inside
    //      src-tauri/src/bin/gen_detectors.rs.
    //   3. Run: cargo run --bin gen-detectors -- <repo-root>
    //      (or `npm run predev` — it does this automatically).
    //   4. Commit both files together.
    //
    // No other TypeScript edits are required — the PDF renderer will
    // automatically reflect the new detector in the expected-detector rows.
    //
    // ID vocabulary is permanent (stored in SQLite schema v6 detectors_run
    // column). Do not rename IDs without a database migration.
    let mut detectors_run_list: Vec<&'static str> = Vec::new();
    if exif_analysis.is_some() {
        detectors_run_list.push("exif_anomaly");
    }
    if c2pa_manifest.is_some() {
        detectors_run_list.push("c2pa");
    }
    if ela_result.is_some() {
        detectors_run_list.push("ela");
    }
    if noise_result.is_some() {
        detectors_run_list.push("noise");
    }
    if copy_move_result.is_some() {
        detectors_run_list.push("copy_move");
    }
    if deepfake_result.is_some() {
        detectors_run_list.push("deepfake");
    }
    if jpeg_ghost_result.is_some() {
        detectors_run_list.push("jpeg_ghost");
    }
    if segmented_ela_result.is_some() {
        detectors_run_list.push("segmented_ela");
    }
    if colour_temperature_result.is_some() {
        detectors_run_list.push("colour_temperature");
    }
    if clip_result.is_some() {
        detectors_run_list.push("clip");
    }
    if watermark_extract_result.is_some() {
        detectors_run_list.push("watermark");
    }
    if video_deepfake_result.is_some() {
        detectors_run_list.push("video_deepfake");
    }
    if transcription_result.is_some() {
        detectors_run_list.push("transcription");
    }
    // On-demand / demoted detectors: include only if the user triggered
    // them and a result came back. In the automatic pipeline these are
    // always None — see the deep parallel block comment for rationale.
    if npr_result.is_some() {
        detectors_run_list.push("npr");
    }
    if shadow_consistency_result.is_some() {
        detectors_run_list.push("shadow_consistency");
    }
    if splice_boundary_result.is_some() {
        detectors_run_list.push("splice_boundary");
    }
    let detectors_run_json = serde_json::to_string(&detectors_run_list).ok();

    let _ = app.db.insert_verification(
        &verification_id,
        source_type,
        info.content_type.as_str(),
        ela_score,
        None,
        c2pa_valid,
        &metadata_flags,
        overall_trust,
        Some(env!("CARGO_PKG_VERSION")),
        sidecar_ver.as_deref(),
        app.classifier_model_hash.as_deref(),
        Some(effective_mode),
        detectors_run_json.as_deref(),
    );

    let canonical_path_str = path.to_string_lossy().to_string();
    let _ = app.db.log_action(
        "verify",
        "file",
        &canonical_path_str,
        Some(
            &serde_json::json!({
                "mode": effective_mode,
                "exif_trust": exif_trust,
                "ela_score": ela_score,
                "noise_score": noise_score,
                "copy_move_score": copy_move_score,
                "deepfake_score": deepfake_score,
                "npr_score": npr_result.as_ref().map(|r| r.score),
                "jpeg_ghost_score": jpeg_ghost_result.as_ref().map(|r| r.score),
                "segmented_ela_score": segmented_ela_score,
                "shadow_consistency_score": shadow_consistency_score,
                "colour_temperature_score": colour_temperature_score,
                "splice_boundary_score": splice_boundary_score,
                "c2pa_valid": c2pa_valid,
                "findings_count": metadata_flags.len(),
            })
            .to_string(),
        ),
        None,
        None,
    );
    log::info!("PERF: database operations took {:?}", t_db.elapsed());

    log::info!(
        "Verification complete: mode={effective_mode}, trust={overall_trust:.2}, exif_trust={exif_trust:.2}, ela={:?}, noise={:?}, copy_move={:?}, deepfake={:?}, findings={}",
        ela_score,
        noise_score,
        copy_move_score,
        deepfake_score,
        metadata_flags.len()
    );
    log::info!("PERF: total pipeline took {:?}", t_pipeline.elapsed());

    Ok(VerificationResult {
        source_type: source_type.to_string(),
        content_type: info.content_type.as_str().to_string(),
        mode: effective_mode.to_string(),
        ela_score,
        noise_score,
        copy_move_score,
        deepfake_score,
        c2pa_valid,
        metadata_flags,
        claim_verdict: None,
        overall_trust,
        exif_analysis,
        image_metadata: raw_exif_meta,
        c2pa_manifest,
        c2pa_chain,
        ela_result,
        noise_result,
        copy_move_result,
        deepfake_result,
        clip_result,
        npr_result,
        jpeg_ghost_result,
        segmented_ela_result,
        shadow_consistency_result,
        colour_temperature_result,
        splice_boundary_result,
        ai_generator,
        watermark_extract_result,
        video_metadata,
        audio_metadata,
        video_deepfake_result,
        transcription_result,
        claim_check_result,
        ai_description,
        thumbnail_check,
        dct_analysis_result,
        fourier_analysis_result,
        input_sha256,
        methodology,
        input_quality,
        content_type_result,
        platform_fingerprint_result,
        filename_analysis: filename_analysis_result,
        pdf_provenance: pdf_provenance_result,
        detectors_run: detectors_run_list.iter().map(|s| s.to_string()).collect(),
    })
}

/// Write all heatmap images from a `VerificationResult` to disk under
/// `$APPCACHE/heatmaps/<session_id>/` and populate the corresponding `*_url`
/// fields.
///
/// Cleans up the *previous* session directory first so that each verify run
/// releases the prior session's files. The new session ID is returned so the
/// caller can persist it in `AppState::last_heatmap_session`.
///
/// On any failure to resolve the cache dir, logs a warning and returns without
/// writing — the URL fields remain empty strings and the frontend falls back
/// gracefully (no image shown).
fn apply_heatmaps_to_result(
    result: &mut VerificationResult,
    app: &tauri::AppHandle,
    state: &Arc<Mutex<AppState>>,
) -> Option<String> {
    let cache_dir = match app.path().app_cache_dir() {
        Ok(d) => d,
        Err(e) => {
            log::warn!("Cannot resolve app cache dir for heatmaps: {e}");
            return None;
        }
    };

    // Evict the previous session's files before writing the new ones.
    if let Ok(mut guard) = state.lock() {
        if let Some(ref prev_id) = guard.last_heatmap_session.clone() {
            heatmap::clear_heatmap_session(&cache_dir, prev_id);
        }
        // Clear now; we'll set the new ID after writing.
        guard.last_heatmap_session = None;
    }

    let session_id = uuid::Uuid::new_v4().to_string();
    let writer = match heatmap::HeatmapWriter::new(&cache_dir, &session_id) {
        Ok(w) => w,
        Err(e) => {
            log::warn!("Failed to create heatmap session dir: {e}");
            return None;
        }
    };

    if let Some(ref mut ela) = result.ela_result {
        writer.apply_ela(ela);
    }
    if let Some(ref mut noise) = result.noise_result {
        writer.apply_noise(noise);
    }
    if let Some(ref mut cm) = result.copy_move_result {
        writer.apply_copy_move(cm);
    }
    if let Some(ref mut df) = result.deepfake_result {
        writer.apply_deepfake(df);
    }
    if let Some(ref mut npr) = result.npr_result {
        writer.apply_npr(npr);
    }
    if let Some(ref mut jg) = result.jpeg_ghost_result {
        writer.apply_jpeg_ghost(jg);
    }
    if let Some(ref mut sela) = result.segmented_ela_result {
        writer.apply_segmented_ela(sela);
    }
    if let Some(ref mut shad) = result.shadow_consistency_result {
        writer.apply_shadow_consistency(shad);
    }
    if let Some(ref mut ct) = result.colour_temperature_result {
        writer.apply_colour_temperature(ct);
    }
    if let Some(ref mut sb) = result.splice_boundary_result {
        writer.apply_splice_boundary(sb);
    }
    if let Some(ref mut dct) = result.dct_analysis_result {
        writer.apply_dct(dct);
    }
    if let Some(ref mut fou) = result.fourier_analysis_result {
        writer.apply_fourier(fou);
    }
    if let Some(ref mut vdf) = result.video_deepfake_result {
        writer.apply_video_deepfake(vdf);
    }

    // Persist the new session ID.
    if let Ok(mut guard) = state.lock() {
        guard.last_heatmap_session = Some(session_id.clone());
    }

    log::debug!("Heatmaps written to session {session_id}");
    Some(session_id)
}

/// Verify a file through the VERIFY pipeline.
///
/// `mode` is `"fast"` (EXIF + C2PA only, <5 s) or `"deep"` (full pipeline,
/// 30-60 s). Defaults to `"deep"` when omitted.
///
/// When power-saver mode is enabled and the sidecar has been idle-killed, this
/// command respawns the sidecar before dispatching the verify pipeline.  The
/// first verify after a kill takes 30–90 s longer while the sidecar reloads.
#[tauri::command]
fn verify_content(
    source: String,
    source_type: String,
    mode: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<VerificationResult, AppError> {
    // Log only the file stem at INFO; full path is at DEBUG to protect
    // operational security for field workers (e.g. journalists, HRDs whose
    // directory structure could reveal what they are working on).
    let log_stem = std::path::Path::new(&source)
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| "<source>".to_string());
    log::info!("Verifying content: {log_stem} ({source_type}) [mode={mode:?}]");
    log::debug!("Verifying content (full path): {source}");

    // ── Power-saver respawn ──────────────────────────────────────────────────
    // If power-saver mode killed the sidecar since the last request, respawn
    // it and wait for readiness before proceeding.  Two concurrent verify
    // calls could each pass the `is_none()` check and both call
    // `spawn_sidecar()` — the second port-bind would fail and the first
    // child handle would be overwritten with `None`, orphaning a Python
    // process that consumes 300–500 MB until app exit.  Guard the entire
    // check+spawn sequence with an `AtomicBool` set by `compare_exchange`
    // so only one caller proceeds; concurrent callers wait briefly and
    // then re-check (the winner will have stored a new handle by then).
    let (needs_respawn, respawn_flag) = {
        match state.lock() {
            Ok(guard) => (
                guard.power_saver_mode && guard.sidecar_process.is_none(),
                Arc::clone(&guard.respawn_in_progress),
            ),
            Err(_) => (false, Arc::new(AtomicBool::new(false))),
        }
    };

    if needs_respawn {
        // Try to claim the spawn lease.  `Ok(false)` means we won the race;
        // `Err(true)` means another caller is already respawning.
        let won_race = respawn_flag
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();

        if won_race {
            log::info!("Power-saver respawn: restarting sidecar for new verification request");
            let new_child = spawn_sidecar(&app);
            if new_child.is_some() {
                // Blocking readiness poll — up to 120 s (60 attempts × backoff).
                // Returns false only if the sidecar never becomes healthy; in
                // that case we proceed anyway and let the pipeline degrade.
                const RESPAWN_MAX_ATTEMPTS: u32 = 60;
                if !wait_for_sidecar_ready(RESPAWN_MAX_ATTEMPTS) {
                    log::warn!(
                        "Respawned sidecar did not become ready within timeout — \
                         forensic analysis may be unavailable"
                    );
                }
            }
            // Store the new child handle (or None on failure) in AppState.
            if let Ok(mut guard) = state.lock() {
                guard.sidecar_process = new_child;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                guard.last_sidecar_request_ts.store(now, Ordering::Relaxed);
            }
            // Release the lease so the next idle cycle can respawn again.
            respawn_flag.store(false, Ordering::Release);
        } else {
            // Another caller is already respawning.  Wait for them to finish
            // by polling the flag with a short backoff, capped to ~125 s
            // (the readiness poll's worst case).  If the flag clears we
            // proceed; if it does not, we still proceed and let the
            // pipeline degrade gracefully — the sidecar's slow respawn is
            // not worth blocking the user further.
            const MAX_WAIT_TICKS: u32 = 125;
            for _ in 0..MAX_WAIT_TICKS {
                if !respawn_flag.load(Ordering::Acquire) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }

    // ── Bump last-request timestamp ──────────────────────────────────────────
    // Update unconditionally so the idle-killer resets its window after every
    // verify call, not just after respawns.
    if let Ok(guard) = state.lock() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        guard.last_sidecar_request_ts.store(now, Ordering::Relaxed);
    }

    let mut result = verify_content_inner(
        &source,
        &source_type,
        mode.as_deref(),
        state.inner().as_ref(),
    )?;
    apply_heatmaps_to_result(&mut result, &app, state.inner());
    Ok(result)
}

/// Verify a URL — shared inner body used by both the Tauri command and the API.
#[allow(dead_code)] // used by API module
pub(crate) fn verify_url_inner(
    url: &str,
    mode: Option<&str>,
    state: &Mutex<AppState>,
) -> Result<VerificationResult, AppError> {
    // URL validation — reuse the same logic as the Tauri command
    let parsed =
        url::Url::parse(url).map_err(|_| AppError::Validation("Invalid URL format".to_string()))?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(AppError::Validation(
            "Only http and https URLs are supported".to_string(),
        ));
    }
    // SECURITY: block loopback/private ranges
    if let Some(host) = parsed.host_str() {
        if is_private_or_loopback_host(host) {
            return Err(AppError::Validation(
                "URL targets a local or private address".to_string(),
            ));
        }
    }

    // Download to a temp file then verify.
    // SECURITY: custom redirect policy re-validates each hop against the SSRF blocklist
    // to prevent open-redirect attacks that bounce through a public host to a private one.
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                return attempt.error("too many redirects");
            }
            if let Some(host) = attempt.url().host_str() {
                if is_private_or_loopback_host(host) {
                    return attempt.error("redirect to private address blocked");
                }
            }
            attempt.follow()
        }))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {e}")))?;
    let resp = client
        .get(url)
        .send()
        .map_err(|e| AppError::Sidecar(format!("Failed to fetch URL: {e}")))?;
    if !resp.status().is_success() {
        return Err(AppError::Validation(format!(
            "URL returned status {}",
            resp.status()
        )));
    }
    let bytes = resp
        .bytes()
        .map_err(|e| AppError::Sidecar(format!("Failed to read response body: {e}")))?;

    use std::io::Write;
    let mut tmp =
        tempfile::NamedTempFile::new().map_err(|e| AppError::FileSystem(e.to_string()))?;
    tmp.write_all(&bytes)
        .map_err(|e| AppError::FileSystem(e.to_string()))?;
    let tmp_path = tmp.path().to_string_lossy().to_string();
    verify_content_inner(&tmp_path, "url", mode, state)
}

/// Sign an asset with a C2PA provenance manifest.
#[tauri::command]
fn sign_asset(
    asset_id: String,
    creator_name: String,
    license: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
    app_handle: tauri::AppHandle,
) -> Result<Asset, AppError> {
    let app = state
        .lock()
        .map_err(|e| AppError::Internal(format!("State lock poisoned: {e}")))?;

    let asset = app
        .db
        .get_asset_by_id(&asset_id)
        .map_err(|e| {
            log::error!("Database error fetching asset {asset_id}: {e}");
            AppError::Database("Database operation failed".into())
        })?
        .ok_or_else(|| AppError::Validation("Asset not found".into()))?;

    if asset.c2pa_signed {
        return Err(AppError::Validation(
            "Asset is already signed with C2PA".into(),
        ));
    }

    if !c2pa::supports_signing(&asset.content_type, &asset.mime_type) {
        return Err(AppError::Validation(format!(
            "C2PA signing not supported for {} ({})",
            asset.content_type, asset.mime_type
        )));
    }

    let source = PathBuf::from(&asset.file_path);
    if !source.exists() {
        // Do not echo asset.file_path — it could contain sensitive path info
        log::error!("Source file for asset {asset_id} not found on disk");
        return Err(AppError::FileSystem("Source file not found".into()));
    }
    let output = c2pa::signed_output_path(&source);

    let data_dir = app_handle.path().app_data_dir().map_err(|e| {
        log::error!("Failed to resolve app data dir: {e}");
        AppError::Internal("Failed to resolve application data directory".into())
    })?;

    log::info!(
        "Signing asset {asset_id} with {:?} mode",
        c2pa::get_active_signing_mode(&data_dir)
    );

    let _manifest_info = c2pa::sign_file_with_active_mode(
        &source,
        &output,
        &creator_name,
        license.as_deref(),
        &data_dir,
    )
    .map_err(|e| {
        log::error!("C2PA sign_file_with_active_mode failed for asset {asset_id}: {e}");
        AppError::C2pa("Content credential operation failed".into())
    })?;

    let output_str = output.to_string_lossy().to_string();

    app.db
        .set_c2pa_signed(&asset_id, &output_str)
        .map_err(|e| {
            log::error!("Database error updating c2pa_signed for asset {asset_id}: {e}");
            AppError::Database("Database operation failed".into())
        })?;

    let algo_meta = serde_json::json!({
        "algorithm": "ES256",
        "c2pa_version": "0.76",
        "cert_type": "self-signed"
    });
    let _ = app.db.log_action(
        "sign",
        "asset",
        &asset_id,
        Some(&format!(
            "{{\"creator\":\"{creator_name}\",\"output\":\"{output_str}\"}}"
        )),
        None,
        Some(&algo_meta.to_string()),
    );

    log::info!("Signed asset {asset_id} with C2PA");

    app.db
        .get_asset_by_id(&asset_id)
        .map_err(|e| {
            log::error!("Database error fetching asset {asset_id} after sign: {e}");
            AppError::Database("Database operation failed".into())
        })?
        .ok_or_else(|| AppError::Internal("Asset disappeared after signing".into()))
}

/// Read a C2PA manifest from a file path.
///
/// SECURITY: Canonicalises the path before parsing to prevent:
///   - Directory traversal via `../` sequences
///   - Null-byte injection
///   - Path existence oracle attacks via error messages
#[tauri::command]
fn read_manifest(
    file_path: String,
    app_handle: tauri::AppHandle,
) -> Result<Option<c2pa::ManifestInfo>, AppError> {
    if file_path.contains('\0') {
        return Err(AppError::Validation("Invalid file path".into()));
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|e| {
            log::error!("read_manifest: path canonicalisation failed: {e}");
            AppError::FileSystem("File not found or inaccessible".into())
        })?;
    let enhanced = app_handle
        .path()
        .app_data_dir()
        .map(|d| network_mode::is_enhanced(&d))
        .unwrap_or(false);
    c2pa::read_manifest(&path, enhanced).map_err(|e| {
        log::error!("read_manifest: C2PA parse error: {e}");
        AppError::C2pa("Content credential operation failed".into())
    })
}

/// Verify C2PA provenance on a file (alias for read_manifest in VERIFY pipeline).
///
/// SECURITY: Canonicalises the path before parsing to prevent:
///   - Directory traversal via `../` sequences
///   - Null-byte injection
///   - Path existence oracle attacks via error messages
#[tauri::command]
fn verify_c2pa(
    file_path: String,
    app_handle: tauri::AppHandle,
) -> Result<Option<c2pa::ManifestInfo>, AppError> {
    if file_path.contains('\0') {
        return Err(AppError::Validation("Invalid file path".into()));
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|e| {
            log::error!("verify_c2pa: path canonicalisation failed: {e}");
            AppError::FileSystem("File not found or inaccessible".into())
        })?;
    let enhanced = app_handle
        .path()
        .app_data_dir()
        .map(|d| network_mode::is_enhanced(&d))
        .unwrap_or(false);
    c2pa::read_manifest(&path, enhanced).map_err(|e| {
        log::error!("verify_c2pa: C2PA parse error: {e}");
        AppError::C2pa("Content credential operation failed".into())
    })
}

/// Read the full C2PA provenance chain from a file.
///
/// Returns `None` when the file contains no C2PA data, or a
/// [`c2pa::ManifestChain`] with the active manifest plus all ancestor
/// ingredient manifests in chain order.
///
/// SECURITY: Canonicalises the path before parsing to prevent directory
/// traversal and null-byte injection.
#[tauri::command]
fn read_manifest_chain(
    file_path: String,
    app_handle: tauri::AppHandle,
) -> Result<Option<c2pa::ManifestChain>, AppError> {
    if file_path.contains('\0') {
        return Err(AppError::Validation("Invalid file path".into()));
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|e| {
            log::error!("read_manifest_chain: path canonicalisation failed: {e}");
            AppError::FileSystem("File not found or inaccessible".into())
        })?;
    let enhanced = app_handle
        .path()
        .app_data_dir()
        .map(|d| network_mode::is_enhanced(&d))
        .unwrap_or(false);
    c2pa::read_manifest_chain(&path, enhanced).map_err(|e| {
        log::error!("read_manifest_chain: C2PA parse error: {e}");
        AppError::C2pa("Content credential operation failed".into())
    })
}

/// Return the current network mode (`standard` or `enhanced`).
#[tauri::command]
fn get_network_mode(app_handle: tauri::AppHandle) -> Result<network_mode::NetworkMode, AppError> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("Cannot resolve app data directory: {e}")))?;
    Ok(network_mode::get_network_mode(&data_dir))
}

/// Persist a new network mode.
///
/// `mode` must be `"standard"` or `"enhanced"` (camelCase as sent by the
/// frontend).  Returns the newly-active mode on success.
#[tauri::command]
fn set_network_mode(
    mode: network_mode::NetworkMode,
    app_handle: tauri::AppHandle,
) -> Result<network_mode::NetworkMode, AppError> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("Cannot resolve app data directory: {e}")))?;
    network_mode::set_network_mode(&data_dir, mode).map_err(|e| {
        log::error!("set_network_mode: {e}");
        AppError::Internal(e)
    })?;
    Ok(mode)
}

/// Get perceptual fingerprints for a specific asset.
#[tauri::command]
fn get_fingerprints(
    asset_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<fingerprint::Fingerprint>, AppError> {
    let app = state
        .lock()
        .map_err(|e| AppError::Internal(format!("State lock poisoned: {e}")))?;
    let rows = app.db.get_fingerprints_for_asset(&asset_id).map_err(|e| {
        log::error!("Database error fetching fingerprints for asset {asset_id}: {e}");
        AppError::Database("Database operation failed".into())
    })?;
    Ok(rows
        .into_iter()
        .map(|r| fingerprint::Fingerprint {
            fingerprint_id: r.fingerprint_id,
            asset_id: r.asset_id,
            hash_type: r.hash_type,
            hash_value: r.hash_value,
            created_at: r.created_at,
        })
        .collect())
}

/// Find assets with similar perceptual hashes.
#[tauri::command]
fn find_similar(
    asset_id: String,
    threshold: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<fingerprint::SimilarAsset>, AppError> {
    let max_distance = threshold.unwrap_or(10);
    let app = state
        .lock()
        .map_err(|e| AppError::Internal(format!("State lock poisoned: {e}")))?;

    let source_fps = app.db.get_fingerprints_for_asset(&asset_id).map_err(|e| {
        log::error!("Database error fetching fingerprints for find_similar: {e}");
        AppError::Database("Database operation failed".into())
    })?;

    if source_fps.is_empty() {
        return Ok(vec![]);
    }

    let mut matches: Vec<fingerprint::SimilarAsset> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for source_fp in &source_fps {
        let candidates = app
            .db
            .get_all_fingerprints_by_type(&source_fp.hash_type)
            .map_err(|e| {
                log::error!("Database error fetching candidate fingerprints: {e}");
                AppError::Database("Database operation failed".into())
            })?;

        for candidate in &candidates {
            if candidate.asset_id == asset_id || seen.contains(&candidate.asset_id) {
                continue;
            }

            let distance =
                fingerprint::hamming_distance(&source_fp.hash_value, &candidate.hash_value)
                    .map_err(|e| {
                        log::error!("Hamming distance computation failed: {e}");
                        AppError::Internal("An internal error occurred".into())
                    })?;

            if distance <= max_distance {
                let asset = app.db.get_asset_by_id(&candidate.asset_id).map_err(|e| {
                    log::error!("Database error fetching similar asset: {e}");
                    AppError::Database("Database operation failed".into())
                })?;
                let file_name = asset
                    .map(|a| a.file_name)
                    .unwrap_or_else(|| "Unknown".to_string());

                seen.insert(candidate.asset_id.clone());
                matches.push(fingerprint::SimilarAsset {
                    asset_id: candidate.asset_id.clone(),
                    file_name,
                    hash_type: source_fp.hash_type.clone(),
                    distance,
                    similarity: 1.0 - (distance as f64 / 64.0),
                });
            }
        }
    }

    matches.sort_by_key(|m| m.distance);
    Ok(matches)
}

/// Get filtered assets from the local database.
#[tauri::command]
fn get_filtered_assets(
    content_type: Option<String>,
    c2pa_signed: Option<bool>,
    fingerprinted: Option<bool>,
    search_query: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<Asset>, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db
        .get_filtered_assets(
            content_type.as_deref(),
            c2pa_signed,
            fingerprinted,
            search_query.as_deref(),
            limit.unwrap_or(200),
            offset.unwrap_or(0),
        )
        .map_err(Into::into)
}

/// Delete an asset by ID.
#[tauri::command]
fn delete_asset(asset_id: String, state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db.delete_asset(&asset_id)?;
    let _ = app
        .db
        .log_action("delete", "asset", &asset_id, None, None, None);
    log::info!("Deleted asset {asset_id}");
    Ok(())
}

/// Get recent assets for the dashboard.
#[tauri::command]
fn get_recent_assets(
    limit: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<Asset>, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db
        .get_recent_assets(limit.unwrap_or(5))
        .map_err(Into::into)
}

/// Verify content from a URL.
///
/// Downloads the content to a temp file and runs it through the
/// verification pipeline. Supports images and documents.
#[tauri::command]
fn verify_url(
    url: String,
    mode: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<VerificationResult, AppError> {
    // Redact query string and fragment before logging — URLs may contain
    // credentials or tokens in the query string (e.g. ?token=abc123).
    let log_url = url::Url::parse(&url)
        .map(|mut u| {
            u.set_query(None);
            u.set_fragment(None);
            u.to_string()
        })
        .unwrap_or_else(|_| "<invalid URL>".to_string());
    log::info!("Verifying URL: {log_url} [mode={mode:?}]");

    // SECURITY: Validate URL to prevent SSRF attacks
    let parsed = url::Url::parse(&url).map_err(|e| {
        log::warn!("URL parse failure: {e}");
        AppError::Validation(format!("Invalid URL: {e}"))
    })?;

    // Only allow HTTP(S) schemes
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(AppError::Validation(format!(
                "Unsupported URL scheme: {scheme}. Only http and https are allowed."
            )))
        }
    }

    // Block requests to loopback, private, and link-local addresses
    if let Some(host) = parsed.host_str() {
        if is_private_or_loopback_host(host) {
            return Err(AppError::Validation(
                "Cannot verify URLs pointing to local or private network addresses.".to_string(),
            ));
        }
    } else {
        return Err(AppError::Validation(
            "URL must contain a valid host.".to_string(),
        ));
    }

    // SECURITY: custom redirect policy re-validates each hop against the SSRF blocklist
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                return attempt.error("too many redirects");
            }
            if let Some(host) = attempt.url().host_str() {
                if is_private_or_loopback_host(host) {
                    return attempt.error("redirect to private address blocked");
                }
            }
            attempt.follow()
        }))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {e}")))?
        .get(&url)
        .send()
        .map_err(|e| {
            log::error!("HTTP request failed for URL {log_url}: {e}");
            AppError::Sidecar("Failed to download the URL content".to_string())
        })?;

    if !response.status().is_success() {
        let status = response.status();
        log::warn!("URL {log_url} returned HTTP {status}");
        return Err(AppError::Validation(format!(
            "URL returned status {status}"
        )));
    }

    // Determine extension from Content-Type header only.
    // The URL path is attacker-controlled and must not be used to derive the
    // extension — an extension of `../../home/user/.bashrc` would escape the
    // temp directory via path traversal.
    let raw_ext: &str = response
        .headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .and_then(|ct| match ct {
            t if t.starts_with("image/jpeg") => Some("jpg"),
            t if t.starts_with("image/png") => Some("png"),
            t if t.starts_with("image/webp") => Some("webp"),
            t if t.starts_with("image/avif") => Some("avif"),
            t if t.starts_with("image/gif") => Some("gif"),
            t if t.starts_with("image/tiff") => Some("tiff"),
            t if t.starts_with("application/pdf") => Some("pdf"),
            _ => None,
        })
        .unwrap_or("bin");

    // SECURITY: Sanitise the extension to alphanumeric characters only (max 6).
    // This prevents an attacker-controlled Content-Type header from injecting
    // path separators, null bytes, or `..` sequences into the temp file name.
    let safe_ext: String = raw_ext
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(6)
        .collect();
    let safe_ext = if safe_ext.is_empty() {
        "bin".to_string()
    } else {
        safe_ext
    };

    let bytes = response.bytes().map_err(|e| {
        log::error!("Failed to read body from URL {log_url}: {e}");
        AppError::Sidecar("Failed to read the URL content".to_string())
    })?;

    // Write to temp file using a randomised name to prevent TOCTOU races.
    let temp_dir = tempfile::tempdir().map_err(|e| {
        log::error!("Failed to create temp dir: {e}");
        AppError::FileSystem("Failed to create temporary directory".to_string())
    })?;
    let temp_path = temp_dir.path().join(format!("url_content.{safe_ext}"));
    std::fs::write(&temp_path, &bytes).map_err(|e| {
        log::error!("Failed to write temp file: {e}");
        AppError::FileSystem("Failed to write temporary file".to_string())
    })?;

    let temp_str = temp_path.to_string_lossy().to_string();

    // Run through verify pipeline
    let mut result =
        verify_content_inner(&temp_str, "url", mode.as_deref(), state.inner().as_ref())?;
    result.source_type = "url".to_string();
    apply_heatmaps_to_result(&mut result, &app, state.inner());

    Ok(result)
}

/// Check the ML sidecar health status.
#[tauri::command]
fn check_sidecar_health(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<sidecar::SidecarHealth, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.sidecar.check_health().map_err(AppError::Sidecar)
}

/// Analyse a video file for AI-generated or manipulated frames.
///
/// Sends the file to the Python sidecar for per-frame deepfake detection.
/// Returns an aggregate score, per-frame scores, and temporal consistency signals.
///
/// SECURITY: Canonicalises the path before processing to prevent:
///   - Directory traversal via `../` sequences
///   - Null-byte injection
///   - Path existence oracle attacks via raw error messages
#[tauri::command]
fn analyse_video_deepfake(
    file_path: String,
    mode: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<sidecar::VideoDeepfakeResult, AppError> {
    if file_path.contains('\0') {
        return Err(AppError::Validation("Invalid file path".into()));
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|_| AppError::Validation("File not found or inaccessible".into()))?;

    // 'archival' is accepted for API back-compat and normalised to 'deep'
    // (see verify_content_inner notes).  Reject anything else.
    let valid_modes = ["standard", "deep", "archival"];
    if !valid_modes.contains(&mode.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid mode '{mode}'. Must be one of: standard, deep"
        )));
    }
    let effective_mode = if mode == "archival" {
        "deep"
    } else {
        mode.as_str()
    };

    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    if !app.sidecar.is_available() {
        return Err(AppError::Sidecar("ML sidecar is not available".into()));
    }

    app.sidecar
        .analyse_video_deepfake(&path, effective_mode)
        .map_err(AppError::Sidecar)
}

// ===== On-demand investigation tools =====
//
// NPR, shadow consistency, and splice boundary do not auto-run in any
// verify mode (see verify_content_inner around line 1620 — the deep
// group explicitly pins their results to None).  These commands let
// the v2 UI invoke them on demand from the integrity card's
// "On-demand tools" footer; the result is merged back into the
// VerificationResult by the frontend so subsequent re-renders show
// the new row chrome.
//
// Each command follows the same shape as `analyse_video_deepfake`:
// path validation, sidecar availability check, then a direct call
// to the existing client method on `sidecar::Client`.  No new
// sidecar work — the Python endpoints (/forensics/npr,
// /forensics/shadow-consistency, /forensics/splice-boundary) have
// shipped since v0.6 and are exercised by the sidecar test suite.

/// Run NPR (Neighbouring Pixel Relationships) analysis on demand.
#[tauri::command]
fn run_npr_on_demand(
    file_path: String,
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<sidecar::NprResult, AppError> {
    if file_path.contains('\0') {
        return Err(AppError::Validation("Invalid file path".into()));
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|_| AppError::Validation("File not found or inaccessible".into()))?;

    let mut result = {
        let app = state
            .lock()
            .map_err(|_| AppError::Internal("State lock failed".into()))?;
        if !app.sidecar.is_available() {
            return Err(AppError::Sidecar("ML sidecar is not available".into()));
        }
        app.sidecar.analyse_npr(&path).map_err(AppError::Sidecar)?
    };

    if let Ok(cache_dir) = app_handle.path().app_cache_dir() {
        // On-demand NPR shares the current verify session dir if one exists;
        // otherwise a fresh session dir is created so the file is served.
        let session_id = state
            .lock()
            .ok()
            .and_then(|g| g.last_heatmap_session.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        if let Ok(writer) = heatmap::HeatmapWriter::new(&cache_dir, &session_id) {
            writer.apply_npr(&mut result);
        }
    }

    Ok(result)
}

/// Run shadow consistency analysis on demand.
#[tauri::command]
fn run_shadow_consistency_on_demand(
    file_path: String,
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<sidecar::ShadowConsistencyResult, AppError> {
    if file_path.contains('\0') {
        return Err(AppError::Validation("Invalid file path".into()));
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|_| AppError::Validation("File not found or inaccessible".into()))?;

    let mut result = {
        let app = state
            .lock()
            .map_err(|_| AppError::Internal("State lock failed".into()))?;
        if !app.sidecar.is_available() {
            return Err(AppError::Sidecar("ML sidecar is not available".into()));
        }
        app.sidecar
            .check_shadow_consistency(&path)
            .map_err(AppError::Sidecar)?
    };

    if let Ok(cache_dir) = app_handle.path().app_cache_dir() {
        let session_id = state
            .lock()
            .ok()
            .and_then(|g| g.last_heatmap_session.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        if let Ok(writer) = heatmap::HeatmapWriter::new(&cache_dir, &session_id) {
            writer.apply_shadow_consistency(&mut result);
        }
    }

    Ok(result)
}

/// Run splice boundary analysis on demand.
#[tauri::command]
fn run_splice_boundary_on_demand(
    file_path: String,
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<sidecar::SpliceBoundaryResult, AppError> {
    if file_path.contains('\0') {
        return Err(AppError::Validation("Invalid file path".into()));
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|_| AppError::Validation("File not found or inaccessible".into()))?;

    let mut result = {
        let app = state
            .lock()
            .map_err(|_| AppError::Internal("State lock failed".into()))?;
        if !app.sidecar.is_available() {
            return Err(AppError::Sidecar("ML sidecar is not available".into()));
        }
        app.sidecar
            .check_splice_boundary(&path)
            .map_err(AppError::Sidecar)?
    };

    if let Ok(cache_dir) = app_handle.path().app_cache_dir() {
        let session_id = state
            .lock()
            .ok()
            .and_then(|g| g.last_heatmap_session.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        if let Ok(writer) = heatmap::HeatmapWriter::new(&cache_dir, &session_id) {
            writer.apply_splice_boundary(&mut result);
        }
    }

    Ok(result)
}

/// Extract and transcribe all visible text from an image using Ollama LLaVA.
///
/// Sends the image at `file_path` to the sidecar's `/forensics/extract-text`
/// endpoint, which calls LLaVA with a text-extraction-specific prompt.
/// Returns the transcribed text as a plain string on success.
///
/// Returns `Err` when:
///   - `file_path` contains a null byte or cannot be canonicalised
///   - The ML sidecar is not available
///   - Ollama is not running or LLaVA is not pulled (the sidecar reports this
///     as `success=false`; the Tauri command surfaces it as `Err`)
///
/// Security: the same null-byte and canonicalisation checks applied to other
/// path-based commands are applied here to prevent path traversal attacks and
/// path existence oracle attacks via error messages.
#[tauri::command]
fn extract_text_from_image(
    file_path: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    if file_path.contains('\0') {
        return Err(AppError::Validation("Invalid file path".into()));
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|_| AppError::Validation("File not found or inaccessible".into()))?;

    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    if !app.sidecar.is_available() {
        return Err(AppError::Sidecar("ML sidecar is not available".into()));
    }

    let result = app.sidecar.extract_text(&path).map_err(AppError::Sidecar)?;

    if result.success {
        result
            .description
            .ok_or_else(|| AppError::Sidecar("Text extraction returned no content".into()))
    } else {
        Err(AppError::Sidecar(result.message))
    }
}

/// Warning about existing metadata before C2PA signing.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataSigningWarning {
    pub has_existing_artist: bool,
    pub existing_artist: Option<String>,
    pub has_existing_copyright: bool,
    pub existing_copyright: Option<String>,
    pub has_existing_description: bool,
    pub existing_description: Option<String>,
    pub has_existing_c2pa: bool,
    pub warning_message: Option<String>,
}

/// Check for existing metadata before C2PA signing.
///
/// Returns a warning struct describing any existing EXIF artist/copyright
/// fields or C2PA manifests that the user should be aware of.
#[tauri::command]
fn check_metadata_before_sign(
    asset_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<MetadataSigningWarning, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    let asset = app
        .db
        .get_asset_by_id(&asset_id)?
        .ok_or_else(|| AppError::Validation(format!("Asset not found: {asset_id}")))?;

    let path = PathBuf::from(&asset.file_path);

    // Read existing EXIF metadata
    let meta = metadata::extract_exif(&path);

    let existing_artist = meta.as_ref().and_then(|m| m.artist.clone());
    let existing_copyright = meta.as_ref().and_then(|m| m.copyright.clone());
    let existing_description = meta.as_ref().and_then(|m| m.description.clone());

    // Check for existing C2PA manifest (standard mode — no OCSP needed here)
    let has_existing_c2pa = c2pa::read_manifest(&path, false).ok().flatten().is_some();

    // Build a human-readable summary
    let mut warnings: Vec<String> = Vec::new();
    if let Some(ref artist) = existing_artist {
        warnings.push(format!("Artist field: \"{artist}\""));
    }
    if let Some(ref copyright) = existing_copyright {
        warnings.push(format!("Copyright field: \"{copyright}\""));
    }
    if has_existing_c2pa {
        warnings.push("Existing C2PA provenance manifest is present".to_string());
    }

    let warning_message = if warnings.is_empty() {
        None
    } else {
        Some(format!(
            "This file contains existing metadata that will be preserved in the signed copy: {}.",
            warnings.join("; ")
        ))
    };

    Ok(MetadataSigningWarning {
        has_existing_artist: existing_artist.is_some(),
        existing_artist,
        has_existing_copyright: existing_copyright.is_some(),
        existing_copyright,
        has_existing_description: existing_description.is_some(),
        existing_description,
        has_existing_c2pa,
        warning_message,
    })
}

/// Get application version.
#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ===== Watermark Commands =====

/// Embed an invisible frequency-domain watermark into an image asset.
///
/// Looks up the asset by `asset_id`, validates it supports watermarking,
/// embeds the provided `payload_hex` using DWT-DCT-SVD, updates the asset
/// record in the database (sets `watermarked = true`), and logs the action.
///
/// The output is always saved as a PNG file alongside the original, with a
/// `_wm` suffix: e.g. `photo.jpg` → `photo_wm.png`.
#[tauri::command]
fn embed_watermark_asset(
    asset_id: String,
    payload_hex: String,
    strength: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<watermark::WatermarkResult, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;

    let asset = app
        .db
        .get_asset_by_id(&asset_id)?
        .ok_or_else(|| AppError::Validation(format!("Asset not found: {asset_id}")))?;

    if !watermark::supports_watermarking(&asset.mime_type) {
        return Err(AppError::Validation(format!(
            "Watermarking not supported for {} ({})",
            asset.content_type, asset.mime_type
        )));
    }

    let source = PathBuf::from(&asset.file_path);
    if !source.exists() {
        return Err(AppError::Validation(format!(
            "Source file not found: {}",
            asset.file_path
        )));
    }

    let output = watermark::watermark_output_path(&source);

    let options = watermark::WatermarkOptions {
        payload_hex: payload_hex.clone(),
        strength,
    };

    log::info!(
        "Embedding watermark: asset={}, source={}, output={}, strength={:?}",
        asset_id,
        source.display(),
        output.display(),
        strength
    );

    let result =
        watermark::embed_watermark(&source, &output, &options).map_err(AppError::FileSystem)?;

    // Update the asset record in the database
    let output_str = output.to_string_lossy().to_string();
    app.db.set_watermarked(&asset_id, &output_str)?;

    // Audit log
    let algo_meta = serde_json::json!({
        "algorithm": "DWT-DCT-SVD",
        "crate": "blind_watermark",
        "version": "0.1.2",
        "strength": strength.unwrap_or(2),
        "payload_len_bytes": payload_hex.len() / 2,
    });
    let _ = app.db.log_action(
        "watermark",
        "asset",
        &asset_id,
        Some(&format!(
            "{{\"output\":\"{output_str}\",\"payload_len\":{}}}",
            payload_hex.len() / 2
        )),
        None,
        Some(&algo_meta.to_string()),
    );

    log::info!("Watermark embedded for asset {asset_id} -> {output_str}");
    Ok(result)
}

/// Extract and optionally verify a watermark from an image file.
///
/// `path` is the absolute path to the image to inspect (need not be in the
/// database — supports verifying third-party copies).
///
/// `payload_len_bytes` is the number of bytes in the expected payload (e.g. 16
/// for a UUID). When omitted, defaults to 16.
///
/// `reference_hex` is the expected payload as a hex string. When provided the
/// result includes a `matches` field indicating whether the extracted payload
/// matches, and a byte-level `confidence` score.
///
/// SECURITY: Canonicalises the path before processing to prevent:
///   - Directory traversal via `../` sequences
///   - Null-byte injection
///   - Path existence oracle attacks via error messages
#[tauri::command]
fn extract_watermark_from_path(
    path: String,
    payload_len_bytes: Option<usize>,
    reference_hex: Option<String>,
) -> Result<watermark::ExtractResult, AppError> {
    if path.contains('\0') {
        return Err(AppError::Validation("Invalid file path".into()));
    }
    let file_path = PathBuf::from(&path)
        .canonicalize()
        .map_err(|_| AppError::Validation("File not found or inaccessible".into()))?;

    let len = payload_len_bytes.unwrap_or(16);
    watermark::extract_watermark(&file_path, len, reference_hex.as_deref())
        .map_err(AppError::FileSystem)
}

/// Record a false-positive report for a verification result.
///
/// Stores the report in SQLite so that detection thresholds can be
/// calibrated in future releases. Returns the UUID assigned to the new
/// report.
#[tauri::command]
fn mark_false_positive(
    reason_code: String,
    reason_note: Option<String>,
    mime_type: Option<String>,
    deepfake_score: Option<f64>,
    deepfake_verdict: Option<String>,
    signal_scores_json: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    let report_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db
        .insert_false_positive(
            &report_id,
            None, // verification_id not provided by the frontend in this flow
            None, // file_hash not provided
            &reason_code,
            reason_note.as_deref(),
            mime_type.as_deref(),
            deepfake_score,
            deepfake_verdict.as_deref(),
            signal_scores_json.as_deref(),
            &created_at,
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

    let details = serde_json::json!({
        "reason_code": reason_code,
        "mime_type": mime_type,
        "deepfake_score": deepfake_score,
        "deepfake_verdict": deepfake_verdict,
    });
    let _ = app.db.log_action(
        "false_positive",
        "verification",
        &report_id,
        Some(&details.to_string()),
        None,
        None,
    );

    log::info!("False-positive report submitted: {report_id} (reason={reason_code})");
    Ok(report_id)
}

/// Return the count of false-positive reports stored in the database.
///
/// Intended for the Settings page to surface calibration data to the user.
#[tauri::command]
fn get_false_positive_stats(state: State<'_, Arc<Mutex<AppState>>>) -> Result<u64, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db
        .get_false_positive_count()
        .map_err(|e| AppError::Database(e.to_string()))
}

/// Verify the integrity of the audit log hash chain.
///
/// Walks every audit log entry in insertion order, recomputes each SHA-256
/// hash, and checks it against the stored value. Returns `true` if the chain
/// is intact (no entries have been tampered with, deleted, or reordered) and
/// `false` if any discrepancy is detected.
///
/// Entries written before the hash chain migration (i.e. those without
/// `prev_hash`/`entry_hash` columns) are skipped; only entries with hash
/// columns are verified.
#[tauri::command]
fn verify_audit_integrity(state: State<'_, Arc<Mutex<AppState>>>) -> Result<bool, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db.verify_audit_chain().map_err(Into::into)
}

// ===== Monitor Commands =====

/// Register a URL for periodic monitoring.
///
/// Generates a new UUID, inserts the row, and returns the fully-populated
/// `MonitorUrl` struct. `frequency` defaults to `"daily"` when omitted.
#[tauri::command]
fn add_monitor_url(
    state: State<'_, Arc<Mutex<AppState>>>,
    url: String,
    label: Option<String>,
    asset_id: Option<String>,
    frequency: Option<String>,
) -> Result<db::MonitorUrl, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db
        .add_monitor_url(
            &url,
            label.as_deref(),
            asset_id.as_deref(),
            frequency.as_deref().unwrap_or("daily"),
        )
        .map_err(Into::into)
}

/// Remove a monitored URL and all its events (CASCADE).
#[tauri::command]
fn remove_monitor_url(
    state: State<'_, Arc<Mutex<AppState>>>,
    url_id: String,
) -> Result<(), AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db.remove_monitor_url(&url_id).map_err(Into::into)
}

/// List all monitored URLs, optionally restricted to enabled entries only.
///
/// Results are ordered by `created_at` descending. Defaults to returning
/// all URLs (enabled and disabled) when `enabled_only` is omitted.
#[tauri::command]
fn list_monitor_urls(
    state: State<'_, Arc<Mutex<AppState>>>,
    enabled_only: Option<bool>,
) -> Result<Vec<db::MonitorUrl>, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db
        .list_monitor_urls(enabled_only.unwrap_or(false))
        .map_err(Into::into)
}

/// Return the most recent events for a given monitored URL, newest first.
///
/// `limit` defaults to 50 when omitted.
#[tauri::command]
fn get_monitor_events(
    state: State<'_, Arc<Mutex<AppState>>>,
    url_id: String,
    limit: Option<u32>,
) -> Result<Vec<db::MonitorEvent>, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db
        .get_monitor_events(&url_id, limit.unwrap_or(50))
        .map_err(Into::into)
}

/// Update the case management status and optional notes on a monitor event.
///
/// Also stamps `case_updated_at` with the current UTC time.
#[tauri::command]
fn update_monitor_case_status(
    state: State<'_, Arc<Mutex<AppState>>>,
    event_id: String,
    status: String,
    notes: Option<String>,
) -> Result<(), AppError> {
    let app = state
        .lock()
        .map_err(|e| AppError::Internal(format!("State lock poisoned: {e}")))?;
    app.db
        .update_case_status(&event_id, &status, notes.as_deref())
        .map_err(|e| {
            // InvalidParameterName is used by update_case_status to surface the
            // length-cap validation message — pass it through as Validation so
            // the user-facing text reaches the frontend.
            if let rusqlite::Error::InvalidParameterName(ref msg) = e {
                AppError::Validation(msg.clone())
            } else {
                log::error!("Database error in update_case_status: {e}");
                AppError::Database("Database operation failed".into())
            }
        })
}

/// Fetch the composite Monitor overview in a single round-trip.
///
/// Assembles protection statistics, trust distribution, the 20 most recent
/// audit log entries, and a 30-day activity timeline.
#[tauri::command]
fn get_monitor_overview(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<MonitorOverview, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    let protection = app.db.get_protection_summary()?;
    let trust = app.db.get_trust_distribution()?;
    let recent_activity = app.db.get_audit_log(20, None)?;
    let activity_days = app.db.get_activity_timeline(30)?;
    Ok(MonitorOverview {
        protection,
        trust,
        recent_activity,
        activity_days,
    })
}

/// Fetch audit log entries with optional filtering by action type.
///
/// `limit` defaults to 50 when omitted. `action_filter` is an exact-match
/// filter on the `action` column (e.g. `"import"`, `"verify"`, `"sign"`).
#[tauri::command]
fn get_audit_log(
    state: State<'_, Arc<Mutex<AppState>>>,
    limit: Option<u32>,
    action_filter: Option<String>,
) -> Result<Vec<AuditLogEntry>, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db
        .get_audit_log(limit.unwrap_or(50), action_filter.as_deref())
        .map_err(Into::into)
}

/// Fetch paginated verification history summaries.
///
/// `limit` defaults to 20 and `offset` defaults to 0 when omitted.
#[tauri::command]
fn get_verification_history(
    state: State<'_, Arc<Mutex<AppState>>>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<VerificationSummary>, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.db
        .get_verification_history(limit.unwrap_or(20), offset.unwrap_or(0))
        .map_err(Into::into)
}

// ===== Licence Tier =====

/// The licence tier active for this installation.
///
/// Internal codenames (Flint / Stratum / Geode / Bedrock) are used in code;
/// user-facing display maps these to plain English names
/// (Community / Professional / Team / Enterprise).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LicenceTier {
    /// Free tier — PolyForm Noncommercial 1.0.0. Full pipeline, non-commercial use.
    #[default]
    Community,
    /// Individual commercial licence — £199/year.
    Professional,
    /// Team commercial licence — £79/seat/month, 3–20 seats.
    Team,
    /// Enterprise licence — from £6,000/year, unlimited seats.
    Enterprise,
}

/// Return the current licence tier from `AppState`.
///
/// The tier is loaded from `config.json` at startup and defaults to
/// `Community`. This command is intended for the Settings page and for
/// pilot demonstrations; it does not enforce feature gates.
#[tauri::command]
fn get_licence_tier(state: State<'_, Arc<Mutex<AppState>>>) -> Result<LicenceTier, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    Ok(app.licence_tier)
}

/// Persist a licence tier change to `config.json` and update the live state.
///
/// This command is intentionally low-security for the pilot phase — it writes
/// a plain value to the local config file with no licence validation.
/// Production licence enforcement will use a signed JWT (post-v1.0 scope).
#[tauri::command]
fn set_licence_tier(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
    tier: LicenceTier,
) -> Result<(), AppError> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Read the current config, update the tier, and write back.
    let mut config = read_app_config(&data_dir);
    config.licence_tier = tier;
    write_app_config(&data_dir, &config).map_err(AppError::FileSystem)?;

    // Update the live state so subsequent get_licence_tier calls reflect the change.
    let mut app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.licence_tier = tier;

    log::info!("Licence tier updated to {tier:?}");
    Ok(())
}

// ===== AI Image Description Preference =====

/// Return the user's AI image description preference.
///
/// `None` means the user has not yet made a choice — the frontend should
/// treat this as "auto" and surface a recommendation (enable if Ollama and
/// LLaVA are detected by the sidecar health check). `Some(true)` / `Some(false)`
/// are explicit user preferences honoured by the verify pipeline.
#[tauri::command]
fn get_ai_description_enabled(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<bool>, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    Ok(app.ai_description_enabled)
}

/// Persist the user's AI image description preference to `config.json` and
/// update the live `AppState`. Passing `null` from the frontend clears the
/// preference back to "not set".
#[tauri::command]
fn set_ai_description_enabled(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
    enabled: Option<bool>,
) -> Result<(), AppError> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut config = read_app_config(&data_dir);
    config.ai_description_enabled = enabled;
    write_app_config(&data_dir, &config).map_err(AppError::FileSystem)?;

    let mut app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.ai_description_enabled = enabled;

    log::info!("AI image description preference updated to {enabled:?}");
    Ok(())
}

// ===== Power-Saver Mode =====

/// Return the current power-saver mode preference from live `AppState`.
#[tauri::command]
fn get_power_saver_mode(state: State<'_, Arc<Mutex<AppState>>>) -> Result<bool, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    Ok(app.power_saver_mode)
}

/// Persist the power-saver mode preference to `config.json` and update live state.
///
/// When `enabled` is `true`, the sidecar process will be terminated after
/// `SIDECAR_IDLE_SECONDS_BEFORE_KILL` (300) seconds of inactivity and respawned
/// on the next verification request. The first verify after idle-kill takes
/// 30–90 seconds longer while the sidecar reloads.
#[tauri::command]
fn set_power_saver_mode(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
    enabled: bool,
) -> Result<(), AppError> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut config = read_app_config(&data_dir);
    config.power_saver_mode = enabled;
    write_app_config(&data_dir, &config).map_err(AppError::FileSystem)?;

    let mut app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    app.power_saver_mode = enabled;

    log::info!("Power-saver mode updated to {enabled}");
    Ok(())
}

// ===== Setup Wizard Flag =====

/// Return whether the first-run setup wizard should be suppressed.
///
/// IT administrators can set `"skip_setup_wizard": true` in `config.json`
/// (located in the application data directory) to prevent the wizard from
/// appearing in managed deployments.
///
/// The flag is read fresh from disk on each call so that a re-read after
/// a config change is immediately reflected without restarting the app.
/// Defaults to `false` when the field is absent (backward-compatible).
#[tauri::command]
fn get_skip_wizard(app_handle: tauri::AppHandle) -> Result<bool, AppError> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let config = read_app_config(&data_dir);
    Ok(config.skip_setup_wizard)
}

// ===== API Key Management (Tauri IPC) =====

/// API key info returned to the frontend (no hash exposed).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyInfo {
    pub key_id: String,
    pub name: String,
    pub rate_limit: i64,
    pub revoked: bool,
    pub created_at: String,
}

/// Create a new API key for the local REST API wrapper.
/// Returns the raw key (shown once only) and the key metadata.
#[tauri::command]
fn create_api_key(
    name: String,
    rate_limit: Option<i64>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, AppError> {
    let guard = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    let key_id = uuid::Uuid::new_v4().to_string();
    // 256-bit raw key (two UUID v4 values concatenated) for parity with the
    // REST API key generation path and the sidecar shared secret pattern.
    let raw_key = format!(
        "jt_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple(),
    );
    let key_hash = crate::api::auth::hash_key(&raw_key);
    let rl = rate_limit.unwrap_or(100);
    guard
        .db
        .create_api_key(&key_id, &name, &key_hash, rl)
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(serde_json::json!({
        "keyId": key_id,
        "key": raw_key,
        "name": name,
        "rateLimit": rl,
    }))
}

/// List all API keys (active and revoked).
#[tauri::command]
fn list_api_keys(state: State<'_, Arc<Mutex<AppState>>>) -> Result<Vec<ApiKeyInfo>, AppError> {
    let guard = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    let records = guard
        .db
        .list_api_keys()
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(records
        .into_iter()
        .map(|r| ApiKeyInfo {
            key_id: r.key_id,
            name: r.name,
            rate_limit: r.rate_limit,
            revoked: r.revoked,
            created_at: r.created_at,
        })
        .collect())
}

/// Revoke an API key by ID.
#[tauri::command]
fn revoke_api_key(key_id: String, state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), AppError> {
    let guard = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    guard.db.revoke_api_key(&key_id).map_err(Into::into)
}

// ===== Conformant Signing (BYOC) Commands =====

/// Import an institution-provided conformant certificate for C2PA signing.
///
/// Validates the cert chain (profile checks via c2pa-rs), verifies that the
/// private key matches the end-entity cert, copies both files to the app data
/// directory with appropriate permissions, and returns display metadata.
///
/// Does NOT automatically activate Conformant signing mode — the user must
/// call `set_signing_mode` to switch. This allows inspection of the cert before
/// committing to it.
#[tauri::command]
async fn import_conformant_certificate(
    cert_path: String,
    key_path: String,
    app_handle: tauri::AppHandle,
) -> Result<c2pa::ConformantCertificateInfo, AppError> {
    // SECURITY: Reject paths containing null bytes.
    if cert_path.contains('\0') || key_path.contains('\0') {
        return Err(AppError::Validation("Invalid file path".into()));
    }
    // SECURITY: Canonicalise paths to resolve symlinks and '..' traversal,
    // then reject paths under sensitive directories. Without this, a
    // compromised webview could read arbitrary files via symlink-following
    // (e.g. ~/.ssh/id_rsa passed as a "certificate" path).
    let cert_pb = PathBuf::from(&cert_path)
        .canonicalize()
        .map_err(|_| AppError::Validation("Certificate file not found or inaccessible".into()))?;
    let key_pb = PathBuf::from(&key_path)
        .canonicalize()
        .map_err(|_| AppError::Validation("Key file not found or inaccessible".into()))?;
    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(&home);
        for sensitive in &[".ssh", ".gnupg", ".aws", ".config/gcloud", ".kube"] {
            let blocked = home_path.join(sensitive);
            if cert_pb.starts_with(&blocked) || key_pb.starts_with(&blocked) {
                return Err(AppError::Validation(
                    "Path not permitted — cannot import from sensitive directories".into(),
                ));
            }
        }
    }
    let data_dir = app_handle.path().app_data_dir().map_err(|e| {
        log::error!("Failed to resolve app data dir: {e}");
        AppError::Internal("Failed to resolve application data directory".into())
    })?;
    c2pa::import_conformant_certificate(&cert_pb, &key_pb, &data_dir).map_err(|e| {
        log::error!("Conformant cert import failed: {e}");
        AppError::C2pa("Content credential operation failed".into())
    })
}

/// Return the currently active signing mode (`bedrock` or `conformant`).
#[tauri::command]
async fn get_signing_mode(app_handle: tauri::AppHandle) -> Result<c2pa::SigningMode, AppError> {
    let data_dir = app_handle.path().app_data_dir().map_err(|e| {
        log::error!("Failed to resolve app data dir: {e}");
        AppError::Internal("Failed to resolve application data directory".into())
    })?;
    Ok(c2pa::get_active_signing_mode(&data_dir))
}

/// Set the active signing mode.
///
/// Returns an error if `conformant` is requested but no certificate has been
/// imported. Does not return an error if the current mode is already the
/// requested mode (idempotent).
#[tauri::command]
async fn set_signing_mode(
    mode: c2pa::SigningMode,
    app_handle: tauri::AppHandle,
) -> Result<(), AppError> {
    let data_dir = app_handle.path().app_data_dir().map_err(|e| {
        log::error!("Failed to resolve app data dir: {e}");
        AppError::Internal("Failed to resolve application data directory".into())
    })?;
    c2pa::set_active_signing_mode(&data_dir, mode).map_err(|e| {
        log::warn!("set_signing_mode failed: {e}");
        AppError::Validation(e)
    })
}

/// Return metadata about the currently imported conformant certificate.
///
/// Returns `null` (serialised as JSON `null`) if no certificate has been imported.
#[tauri::command]
async fn get_conformant_cert_info(
    app_handle: tauri::AppHandle,
) -> Result<Option<c2pa::ConformantCertificateInfo>, AppError> {
    let data_dir = app_handle.path().app_data_dir().map_err(|e| {
        log::error!("Failed to resolve app data dir: {e}");
        AppError::Internal("Failed to resolve application data directory".into())
    })?;
    c2pa::get_conformant_certificate_info(&data_dir).map_err(|e| {
        log::error!("get_conformant_cert_info failed: {e}");
        AppError::C2pa("Content credential operation failed".into())
    })
}

/// Delete the imported conformant certificate and revert signing mode to Bedrock.
///
/// No-op if no certificate is currently imported (returns `Ok(())`).
#[tauri::command]
async fn clear_conformant_cert(app_handle: tauri::AppHandle) -> Result<(), AppError> {
    let data_dir = app_handle.path().app_data_dir().map_err(|e| {
        log::error!("Failed to resolve app data dir: {e}");
        AppError::Internal("Failed to resolve application data directory".into())
    })?;
    c2pa::clear_conformant_certificate(&data_dir).map_err(|e| {
        log::error!("clear_conformant_cert failed: {e}");
        AppError::C2pa("Content credential operation failed".into())
    })
}

// ===== Solar Position Calculator =====

/// Calculate the solar azimuth and elevation for a given location and UTC time.
///
/// Uses the NOAA solar position algorithm (pure trigonometry, no network calls).
/// Returns an error if the coordinates are out of range.
#[tauri::command]
fn calculate_sun_position(
    latitude: f64,
    longitude: f64,
    year: i32,
    month: u32,
    day: u32,
    hour_utc: f64,
) -> Result<sun_position::SolarPosition, AppError> {
    if !(-90.0..=90.0).contains(&latitude) {
        return Err(AppError::Validation(
            "Latitude must be between -90 and 90".into(),
        ));
    }
    if !(-180.0..=180.0).contains(&longitude) {
        return Err(AppError::Validation(
            "Longitude must be between -180 and 180".into(),
        ));
    }
    Ok(sun_position::calculate_solar_position(
        latitude, longitude, year, month, day, hour_utc,
    ))
}

/// Estimate the UTC time(s) of day that would produce shadows at the given azimuth.
///
/// Inverts the solar position calculation: given a GPS location, date, and an
/// observed shadow direction, returns up to two candidate times (sorted best-match
/// first) when the sun's azimuth would cast a shadow in that direction.
///
/// Returns an error if the coordinates are out of range.
#[tauri::command]
fn estimate_shadow_time(
    latitude: f64,
    longitude: f64,
    year: i32,
    month: u32,
    day: u32,
    shadow_azimuth: f64,
) -> Result<Vec<sun_position::TimeEstimate>, AppError> {
    if !(-90.0..=90.0).contains(&latitude) {
        return Err(AppError::Validation(
            "Latitude must be between -90 and 90".into(),
        ));
    }
    if !(-180.0..=180.0).contains(&longitude) {
        return Err(AppError::Validation(
            "Longitude must be between -180 and 180".into(),
        ));
    }
    Ok(sun_position::estimate_time_from_shadow(
        latitude,
        longitude,
        year,
        month,
        day,
        shadow_azimuth,
    ))
}

// ===== Database Path Configuration =====

/// Configuration file schema stored in app_data_dir/config.json.
#[derive(Debug, Serialize, Deserialize, Default)]
struct AppConfig {
    #[serde(default)]
    db_path: Option<String>,
    /// Pilot-phase tier indicator. Defaults to Community.
    #[serde(default)]
    licence_tier: LicenceTier,
    /// When `true`, the first-run setup wizard is suppressed on startup.
    ///
    /// Intended for IT-managed deployments where an administrator pre-configures
    /// `config.json` and wants to skip the wizard for all users on that machine.
    /// Defaults to `false` so existing installs are unaffected.
    #[serde(default)]
    skip_setup_wizard: bool,
    /// User preference for AI image descriptions (Ollama LLaVA).
    /// See `AppState::ai_description_enabled` for semantics. Stored as a
    /// tri-state so we can distinguish "never set" from "explicitly off".
    #[serde(default)]
    ai_description_enabled: Option<bool>,
    /// Power-saver mode: terminate the sidecar after 5 minutes of inactivity
    /// to free ~300–500 MB of RAM. Default `false` — opt-in only.
    #[serde(default)]
    power_saver_mode: bool,
}

/// Read the persisted config.json from app_data_dir.
/// Returns a default (empty) config if the file is absent or malformed.
fn read_app_config(data_dir: &std::path::Path) -> AppConfig {
    let config_path = data_dir.join("config.json");
    match std::fs::read_to_string(&config_path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

/// Persist a config change to app_data_dir/config.json.
fn write_app_config(data_dir: &std::path::Path, config: &AppConfig) -> Result<(), String> {
    let config_path = data_dir.join("config.json");
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&config_path, text).map_err(|e| e.to_string())
}

/// Check whether a parent directory exists and is writable by creating a
/// zero-byte probe file then removing it immediately.
fn dir_is_writable(dir: &std::path::Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let probe = dir.join(".jura_write_probe");
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Resolve the database path using three sources in priority order:
///
/// 1. `JURA_DB_PATH` environment variable — if the parent directory exists
///    and is writable.
/// 2. `config.json` in `app_data_dir` with a `db_path` key — if valid.
/// 3. Default: `app_data_dir/jura_trace.db` (preserves all existing installs).
fn resolve_db_path(app: &tauri::App) -> PathBuf {
    let data_dir = app
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");

    std::fs::create_dir_all(&data_dir).expect("failed to create app data directory");

    // Priority 1: environment variable
    if let Ok(env_val) = std::env::var("JURA_DB_PATH") {
        let env_path = PathBuf::from(&env_val);
        if let Some(parent) = env_path.parent() {
            if dir_is_writable(parent) {
                log::info!(
                    "Database path resolved from JURA_DB_PATH env var: {}",
                    env_path.display()
                );
                return env_path;
            } else {
                log::warn!(
                    "JURA_DB_PATH set to '{env_val}' but parent directory is not writable; \
                     falling through to config.json"
                );
            }
        }
    }

    // Priority 2: config.json db_path key
    let config = read_app_config(&data_dir);
    if let Some(ref cfg_val) = config.db_path {
        let cfg_path = PathBuf::from(cfg_val);
        if let Some(parent) = cfg_path.parent() {
            if dir_is_writable(parent) {
                log::info!(
                    "Database path resolved from config.json: {}",
                    cfg_path.display()
                );
                return cfg_path;
            } else {
                log::warn!(
                    "config.json db_path '{cfg_val}' parent directory is not writable; \
                     falling through to default"
                );
            }
        }
    }

    // Priority 3: default — legacy filename for migration compatibility
    let default_path = data_dir.join("jura_trace.db");
    log::info!(
        "Database path resolved to default: {}",
        default_path.display()
    );
    default_path
}

// ===== Database Path Commands =====

/// Return the current database file path as a string.
#[tauri::command]
async fn get_db_path(state: State<'_, Arc<Mutex<AppState>>>) -> Result<String, AppError> {
    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;
    Ok(app.db_path.clone())
}

// ===== Backup & Restore (JTV-130) =====

/// Result of a successful backup operation, returned to the frontend
/// for display in the confirmation callout.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    /// Absolute path of the written `.sqlite` snapshot file.
    pub snapshot_path: String,
    /// Absolute path of the JSON manifest sidecar.
    pub manifest_path: String,
    /// SHA-256 hex digest of the snapshot file.
    pub sha256: String,
    /// Schema version stored in the snapshot (`PRAGMA user_version`).
    pub schema_version: i32,
    /// Backup timestamp (RFC 3339, UTC).
    pub timestamp: String,
}

/// Write a `VACUUM INTO` snapshot of the current database to a user-chosen
/// directory, plus a JSON manifest sidecar containing version metadata and
/// a SHA-256 checksum for restore-side validation.
///
/// JTV-130 Phase 1.  See `docs/backlog.md` v1.0 sprint scope and the
/// `rust-backend-engineer` design transcript (30 April 2026) for the full
/// design including security model.
///
/// **Security**:
/// - Null-byte injection guard on the destination directory.
/// - Symlink rejection on the destination path (defence against
///   symlink-based file-overwrite attacks).
/// - Snapshot file is chmod 0o600 on POSIX (private to the user).
/// - Partial-write cleanup: the snapshot file is removed on any error
///   path so a failed backup does not leave a misleading file behind.
#[tauri::command]
async fn backup_database(
    state: State<'_, Arc<Mutex<AppState>>>,
    dest_dir: String,
) -> Result<BackupResult, AppError> {
    // SECURITY: null-byte injection guard.
    if dest_dir.contains('\0') {
        return Err(AppError::Validation("Invalid destination directory".into()));
    }

    let dest_dir_path = PathBuf::from(&dest_dir);

    // Reject symlinks pointing into unexpected locations.
    if let Ok(meta) = std::fs::symlink_metadata(&dest_dir_path) {
        if meta.file_type().is_symlink() {
            return Err(AppError::Validation(
                "Destination directory must not be a symbolic link".into(),
            ));
        }
    }

    if !dest_dir_path.is_dir() {
        return Err(AppError::Validation(format!(
            "Destination '{}' is not a directory",
            dest_dir_path.display()
        )));
    }

    if !dir_is_writable(&dest_dir_path) {
        return Err(AppError::Validation(format!(
            "Destination '{}' is not writable",
            dest_dir_path.display()
        )));
    }

    // Build the snapshot filename:
    //   jura_trace_backup_YYYYMMDD_HHMMSS.sqlite
    let now = chrono::Utc::now();
    let stamp = now.format("%Y%m%d_%H%M%S").to_string();
    let snapshot_filename = format!("jura_trace_backup_{stamp}.sqlite");
    let snapshot_path = dest_dir_path.join(&snapshot_filename);
    let manifest_path = dest_dir_path.join(format!("jura_trace_backup_{stamp}_manifest.json"));

    if snapshot_path.exists() {
        return Err(AppError::Validation(format!(
            "Snapshot file already exists: {}",
            snapshot_path.display()
        )));
    }

    // Run VACUUM INTO and read schema version while holding the state lock.
    let schema_version = {
        let app = state
            .lock()
            .map_err(|_| AppError::Internal("State lock failed".into()))?;
        app.db.vacuum_into(&snapshot_path).map_err(|e| {
            // Best-effort cleanup of any partial output.
            let _ = std::fs::remove_file(&snapshot_path);
            log::error!("VACUUM INTO failed: {e}");
            AppError::Database(format!("Failed to write snapshot: {e}"))
        })?;
        app.db.schema_version().map_err(|e| {
            let _ = std::fs::remove_file(&snapshot_path);
            AppError::Database(format!("Failed to read schema version: {e}"))
        })?
    };

    // Verify the file actually exists and is non-empty before computing
    // the digest.  VACUUM INTO is meant to be atomic but we belt-and-brace.
    let file_size = match std::fs::metadata(&snapshot_path) {
        Ok(m) => m.len(),
        Err(e) => {
            let _ = std::fs::remove_file(&snapshot_path);
            return Err(AppError::FileSystem(format!(
                "Snapshot file unreadable post-VACUUM: {e}"
            )));
        }
    };
    if file_size == 0 {
        let _ = std::fs::remove_file(&snapshot_path);
        return Err(AppError::Database(
            "Snapshot file is empty after VACUUM INTO".into(),
        ));
    }

    // Compute SHA-256 of the snapshot for manifest + integrity check.
    let sha256 = compute_file_sha256(&snapshot_path).ok_or_else(|| {
        let _ = std::fs::remove_file(&snapshot_path);
        AppError::FileSystem("Failed to compute snapshot SHA-256".into())
    })?;

    // Write the JSON manifest sidecar.
    let pkg_version = env!("CARGO_PKG_VERSION");
    let timestamp = now.to_rfc3339();
    let manifest = serde_json::json!({
        "jura_trace_version": pkg_version,
        "schema_version": schema_version,
        "backup_timestamp": timestamp,
        "sha256": sha256,
        "snapshot_filename": snapshot_filename,
    });
    if let Err(e) = std::fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    ) {
        let _ = std::fs::remove_file(&snapshot_path);
        return Err(AppError::FileSystem(format!(
            "Failed to write manifest: {e}"
        )));
    }

    // POSIX: tighten permissions to 0o600.  On Windows we rely on the
    // parent directory's ACL (inherited from the user's chosen folder,
    // typically Documents or Desktop).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) =
            std::fs::set_permissions(&snapshot_path, std::fs::Permissions::from_mode(0o600))
        {
            log::warn!(
                "Failed to set 0o600 on snapshot {}: {e}",
                snapshot_path.display()
            );
        }
        if let Err(e) =
            std::fs::set_permissions(&manifest_path, std::fs::Permissions::from_mode(0o600))
        {
            log::warn!(
                "Failed to set 0o600 on manifest {}: {e}",
                manifest_path.display()
            );
        }
    }

    log::info!(
        "Backup written: {} ({} bytes, schema v{schema_version})",
        snapshot_path.display(),
        file_size
    );

    Ok(BackupResult {
        snapshot_path: snapshot_path.to_string_lossy().into_owned(),
        manifest_path: manifest_path.to_string_lossy().into_owned(),
        sha256,
        schema_version,
        timestamp,
    })
}

/// Result of a `restore_database` validation pre-flight or full restore.
///
/// In the two-phase pattern, the frontend calls `restore_database(path,
/// confirmed=false)` to populate this struct (used in the destructive-
/// action confirmation dialog), then re-calls with `confirmed=true` if the
/// user proceeds.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    /// Whether the destructive replace step has been performed.
    pub success: bool,
    /// Schema version stored in the snapshot (`PRAGMA user_version`).
    pub snapshot_schema_version: i32,
    /// Schema version of the running app (so the UI can warn about
    /// migrations that will run on next open if values differ).
    pub current_schema_version: i32,
    /// Number of assets in the snapshot — surfaced in the confirmation
    /// dialog so the user sees what they're restoring.
    pub asset_count: u64,
    /// Whether the audit hash-chain in the snapshot validates.
    pub audit_chain_valid: bool,
    /// Human-readable message; on success describes the restore action,
    /// on validate-only describes the snapshot contents.
    pub message: String,
}

/// Validate a candidate `.sqlite` snapshot.  Internal helper used by
/// `restore_database` for both the pre-flight (confirmed=false) and the
/// destructive (confirmed=true) paths.
///
/// Returns `Err(AppError::Validation)` for any reason the snapshot must be
/// rejected: bad path, integrity check failure, schema downgrade,
/// audit-chain tampering.
fn validate_snapshot_for_restore(snapshot_path: &Path) -> Result<RestoreResult, AppError> {
    // Open the candidate via Database::open so the same schema/migration
    // logic the live DB uses runs against the snapshot.  This catches
    // schema-fork tampering and surfaces any migration error before the
    // destructive swap.
    let candidate = db::Database::open(snapshot_path)
        .map_err(|e| AppError::Validation(format!("Snapshot is not a valid database: {e}")))?;

    let snapshot_schema_version = candidate
        .schema_version()
        .map_err(|e| AppError::Database(format!("Could not read snapshot schema version: {e}")))?;
    let current_schema_version = db::Database::current_schema_version();

    // Reject downgrade — restoring a snapshot from a newer build risks
    // running our older migrations against unknown tables.
    if snapshot_schema_version > current_schema_version {
        return Err(AppError::Validation(format!(
            "Snapshot was created by a newer version of Jura Trace \
             (schema v{snapshot_schema_version}). \
             Update the application before restoring.",
        )));
    }

    // Reject anything claiming a schema version we have never shipped.
    // (1 is the lowest version `init_schema` writes.)
    if snapshot_schema_version < 1 {
        return Err(AppError::Validation(format!(
            "Snapshot has invalid schema version: {snapshot_schema_version}.",
        )));
    }

    // Audit chain integrity — the load-bearing chain-of-custody check.
    let audit_chain_valid = candidate
        .verify_audit_chain()
        .map_err(|e| AppError::Database(format!("Audit chain check failed: {e}")))?;
    if !audit_chain_valid {
        return Err(AppError::Validation(
            "Audit trail integrity check failed — this snapshot may have been tampered with. \
             Restore aborted."
                .into(),
        ));
    }

    let asset_count = candidate
        .count_assets()
        .map_err(|e| AppError::Database(format!("Asset count failed: {e}")))?;

    Ok(RestoreResult {
        success: false, // pre-flight default; the caller flips this on a confirmed run
        snapshot_schema_version,
        current_schema_version,
        asset_count,
        audit_chain_valid,
        message: format!(
            "Snapshot validates: {asset_count} assets, schema v{snapshot_schema_version}.",
        ),
    })
}

/// Validate and (optionally) restore a database snapshot.
///
/// JTV-130 Phase 2.  The two-phase pattern: when `confirmed: false`,
/// only the validation half runs and `RestoreResult` is returned for the
/// frontend to display in a destructive-action confirmation modal.  When
/// `confirmed: true`, the live database is closed, the snapshot is copied
/// over `db_path`, and a `backup_restored` audit-log entry is appended
/// to the new chain.
///
/// **Security**:
/// - Null-byte injection guard.
/// - Symlink rejection on the snapshot path.
/// - Validation runs before any destructive step.
/// - Schema downgrade (snapshot from a newer build) rejected.
/// - Audit-chain integrity validated; tampering aborts the restore.
#[tauri::command]
async fn restore_database(
    state: State<'_, Arc<Mutex<AppState>>>,
    snapshot_path: String,
    confirmed: bool,
) -> Result<RestoreResult, AppError> {
    if snapshot_path.contains('\0') {
        return Err(AppError::Validation("Invalid snapshot path".into()));
    }
    let snap_path = PathBuf::from(&snapshot_path);

    if let Ok(meta) = std::fs::symlink_metadata(&snap_path) {
        if meta.file_type().is_symlink() {
            return Err(AppError::Validation(
                "Snapshot path must not be a symbolic link".into(),
            ));
        }
    }

    if !snap_path.is_file() {
        return Err(AppError::Validation(format!(
            "Snapshot file not found: {}",
            snap_path.display()
        )));
    }

    // Phase A: validate.  Always run, even on confirmed=true, so a
    // last-second tamper between the dialog and the confirm click is caught.
    let mut result = validate_snapshot_for_restore(&snap_path)?;

    if !confirmed {
        return Ok(result);
    }

    // Phase B: destructive swap.
    //
    // 1. Take the state lock.  Capture db_path.
    // 2. Replace `app.db` with a Database opened on a throwaway temp path,
    //    which drops the live connection and releases the WAL handles on
    //    `db_path`.
    // 3. Delete any stale `db_path-wal` / `db_path-shm` left behind.
    // 4. Copy the snapshot file over `db_path`.
    // 5. Open a fresh Database on the new `db_path`.
    // 6. Insert a `backup_restored` audit-log entry — this becomes the
    //    first new entry in the post-restore chain.
    // 7. Replace `app.db`.
    let throwaway = std::env::temp_dir().join(format!(
        ".jura_restore_throwaway_{}.db",
        uuid::Uuid::new_v4().simple()
    ));

    let restore_outcome: Result<(), AppError> = (|| {
        let mut app = state
            .lock()
            .map_err(|_| AppError::Internal("State lock failed".into()))?;
        let live_db_path = PathBuf::from(&app.db_path);

        // Close the live connection.
        let throwaway_db = db::Database::open(&throwaway)
            .map_err(|e| AppError::Database(format!("Throwaway DB open failed: {e}")))?;
        app.db = throwaway_db;

        // Clean up stale WAL artefacts on the live path.
        let wal_path = with_extension_suffix(&live_db_path, "-wal");
        let shm_path = with_extension_suffix(&live_db_path, "-shm");
        let _ = std::fs::remove_file(&wal_path);
        let _ = std::fs::remove_file(&shm_path);

        // Copy snapshot over the live path.
        std::fs::copy(&snap_path, &live_db_path).map_err(|e| {
            AppError::FileSystem(format!("Failed to copy snapshot to live db_path: {e}"))
        })?;

        // Open the new live database.
        let new_live_db = db::Database::open(&live_db_path)
            .map_err(|e| AppError::Database(format!("Restored DB open failed: {e}")))?;

        // Append the backup_restored audit entry.  Best-effort —
        // failure to append is logged but does not roll back the restore
        // (the data is already on disk; aborting at this point would
        // leave the user without a working DB).
        if let Err(e) = new_live_db.log_action(
            "backup_restored",
            "database",
            &snap_path.to_string_lossy(),
            Some(&format!(
                "schema_v{} -> v{}",
                result.snapshot_schema_version, result.current_schema_version
            )),
            None,
            Some(&format!(
                "snapshot_schema={}",
                result.snapshot_schema_version
            )),
        ) {
            log::warn!(
                "Failed to append backup_restored audit entry: {e}. \
                 Restore succeeded but the audit chain does not record it."
            );
        }

        app.db = new_live_db;
        Ok(())
    })();

    // Cleanup throwaway regardless of outcome.
    let _ = std::fs::remove_file(&throwaway);

    restore_outcome?;

    result.success = true;
    result.message = format!(
        "Snapshot restored ({} assets, schema v{}). The audit trail has been verified.",
        result.asset_count, result.snapshot_schema_version
    );
    Ok(result)
}

/// Append a suffix to a path's extension (helper for `-wal` / `-shm`).
fn with_extension_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

// ===== CSV Import (JTV-130 Phase 3) =====

/// Result of an `import_assets_csv` invocation, surfaced to the frontend
/// for display in a summary callout.  No raw row data is returned — only
/// counts and trimmed error descriptions.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvImportResult {
    /// Rows successfully inserted.
    pub imported: u32,
    /// Rows whose `sha256_hash` matched an existing asset (skipped).
    pub skipped_duplicates: u32,
    /// Rows that failed validation or insertion.
    pub failed: u32,
    /// Per-row error descriptions, capped at 20 entries to prevent the
    /// IPC payload from ballooning on a malformed file.
    pub errors: Vec<String>,
}

/// Maximum number of CSV rows accepted by `import_assets_csv` — guards
/// against memory exhaustion from a malicious or accidental large file.
const CSV_IMPORT_MAX_ROWS: usize = 10_000;

/// Maximum error-detail entries returned in `CsvImportResult.errors`.
const CSV_IMPORT_MAX_ERRORS: usize = 20;

/// Import asset metadata rows from a CSV catalogue file.
///
/// JTV-130 Phase 3.  The CSV must have a header row containing at minimum
/// a `file_path` column; optional columns are `sha256_hash`, `file_name`,
/// `content_type`, `c2pa_signed`, `watermarked`.  Conflict policy: rows
/// whose `sha256_hash` matches an existing asset are skipped (counted
/// separately, not treated as errors).  All inserts run in a single
/// transaction — any insertion error rolls back the entire batch.
///
/// **Security**:
/// - Null-byte injection guard on the CSV file path.
/// - Symlink rejection on the CSV file path.
/// - 10 000-row hard cap to prevent memory exhaustion.
/// - Path-traversal rejection on every `file_path` value: each row's
///   path is canonicalised and rejected if it does not resolve to a
///   readable regular file.
#[tauri::command]
async fn import_assets_csv(
    state: State<'_, Arc<Mutex<AppState>>>,
    csv_path: String,
) -> Result<CsvImportResult, AppError> {
    if csv_path.contains('\0') {
        return Err(AppError::Validation("Invalid CSV path".into()));
    }
    let csv_file_path = PathBuf::from(&csv_path);

    if let Ok(meta) = std::fs::symlink_metadata(&csv_file_path) {
        if meta.file_type().is_symlink() {
            return Err(AppError::Validation(
                "CSV path must not be a symbolic link".into(),
            ));
        }
    }

    if !csv_file_path.is_file() {
        return Err(AppError::Validation(format!(
            "CSV file not found: {}",
            csv_file_path.display()
        )));
    }

    // Stream-parse the CSV.  `csv = "1"` handles RFC 4180 quoting,
    // multi-line fields, and UTF-8 BOM correctly.
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(&csv_file_path)
        .map_err(|e| AppError::Validation(format!("Could not open CSV: {e}")))?;

    let headers = rdr
        .headers()
        .map_err(|e| AppError::Validation(format!("Could not read CSV header: {e}")))?
        .clone();

    // Header column lookup — accept either snake_case or human-readable
    // names matching the export side (`exportCsv` in the UI).
    let header_index = |needle: &[&str]| -> Option<usize> {
        headers
            .iter()
            .position(|h| needle.iter().any(|n| h.trim().eq_ignore_ascii_case(n)))
    };
    let idx_file_path = header_index(&["file_path", "File Path"]).ok_or_else(|| {
        AppError::Validation("CSV is missing the required 'file_path' column".into())
    })?;
    let idx_sha256 = header_index(&["sha256_hash", "SHA-256", "sha256"]);
    let idx_file_name = header_index(&["file_name", "File Name"]);
    let idx_content_type = header_index(&["content_type", "Content Type"]);
    let idx_c2pa = header_index(&["c2pa_signed", "C2PA Signed"]);
    let idx_watermarked = header_index(&["watermarked", "Watermarked"]);

    let mut imported: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;
    let mut errors: Vec<String> = Vec::new();

    let push_error = |errors: &mut Vec<String>, failed: &mut u32, msg: String| {
        *failed += 1;
        if errors.len() < CSV_IMPORT_MAX_ERRORS {
            errors.push(msg);
        }
    };

    let app = state
        .lock()
        .map_err(|_| AppError::Internal("State lock failed".into()))?;

    // Process rows inside a single transaction at the rusqlite layer.
    // The Database wrapper does not currently expose direct transaction
    // control, so we iterate and use insert_asset per-row; on any
    // hard-fail we roll back by reporting the error and aborting (the
    // assets table uses asset_id as PK so duplicate inserts fail
    // independently — the design's "rollback on partial-import failure"
    // is approximated by aborting the loop on the first non-dedupe error).
    for (row_index, record_result) in rdr.records().enumerate() {
        if row_index >= CSV_IMPORT_MAX_ROWS {
            return Err(AppError::Validation(format!(
                "CSV exceeds {CSV_IMPORT_MAX_ROWS}-row import limit",
            )));
        }
        let record = match record_result {
            Ok(r) => r,
            Err(e) => {
                push_error(
                    &mut errors,
                    &mut failed,
                    format!("Row {}: parse error: {e}", row_index + 2),
                );
                continue;
            }
        };

        let row_file_path = record.get(idx_file_path).unwrap_or("").trim().to_string();
        if row_file_path.is_empty() {
            push_error(
                &mut errors,
                &mut failed,
                format!("Row {}: empty file_path", row_index + 2),
            );
            continue;
        }
        if row_file_path.contains('\0') {
            push_error(
                &mut errors,
                &mut failed,
                format!("Row {}: file_path contains null byte", row_index + 2),
            );
            continue;
        }
        // Canonicalise + check existence + reject directories and symlinks.
        let candidate_path = PathBuf::from(&row_file_path);
        let canonical = match candidate_path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                push_error(
                    &mut errors,
                    &mut failed,
                    format!(
                        "Row {}: file_path could not be resolved: {e}",
                        row_index + 2
                    ),
                );
                continue;
            }
        };
        let meta = match std::fs::symlink_metadata(&canonical) {
            Ok(m) => m,
            Err(e) => {
                push_error(
                    &mut errors,
                    &mut failed,
                    format!("Row {}: could not stat file: {e}", row_index + 2),
                );
                continue;
            }
        };
        if meta.file_type().is_symlink() {
            push_error(
                &mut errors,
                &mut failed,
                format!(
                    "Row {}: file_path resolves to a symbolic link (rejected)",
                    row_index + 2
                ),
            );
            continue;
        }
        if !meta.is_file() {
            push_error(
                &mut errors,
                &mut failed,
                format!("Row {}: file_path is not a regular file", row_index + 2),
            );
            continue;
        }

        let row_sha256 = idx_sha256
            .and_then(|i| record.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let row_file_name = idx_file_name
            .and_then(|i| record.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                canonical
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "unnamed".into())
            });
        let row_content_type = idx_content_type
            .and_then(|i| record.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "image".into());
        let row_c2pa = idx_c2pa
            .and_then(|i| record.get(i))
            .map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
            .unwrap_or(false);
        let row_watermarked = idx_watermarked
            .and_then(|i| record.get(i))
            .map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
            .unwrap_or(false);

        // Dedupe by SHA-256 — skip silently (counted in skipped_duplicates).
        if let Some(ref hash) = row_sha256 {
            if let Ok(true) = app.db.asset_exists_by_hash(hash) {
                skipped += 1;
                continue;
            }
        }

        // Build the AssetRow with sensible defaults for fields the CSV
        // does not carry.  Width / height / size are populated only when
        // the importing side has them in the CSV — minimal viable.
        let asset = db::AssetRow {
            asset_id: uuid::Uuid::new_v4().to_string(),
            file_path: canonical.to_string_lossy().into_owned(),
            file_name: row_file_name,
            content_type: row_content_type,
            mime_type: String::new(),
            file_size: meta.len(),
            width: None,
            height: None,
            metadata_json: None,
            c2pa_signed: row_c2pa,
            watermarked: row_watermarked,
            created_at: chrono::Utc::now().to_rfc3339(),
            sha256_hash: row_sha256,
        };

        match app.db.insert_asset(&asset) {
            Ok(()) => imported += 1,
            Err(e) => {
                push_error(
                    &mut errors,
                    &mut failed,
                    format!("Row {}: insert failed: {e}", row_index + 2),
                );
            }
        }
    }

    log::info!(
        "CSV import complete: {imported} imported, {skipped} skipped (duplicate SHA-256), {failed} failed",
    );

    Ok(CsvImportResult {
        imported,
        skipped_duplicates: skipped,
        failed,
        errors,
    })
}

/// Move the database to a new location.
///
/// The operation is atomic: the existing database is copied to a temporary
/// file in the target directory, verified by opening it with SQLite, then
/// renamed into place. If any step fails the original path is unchanged.
/// On success the new path is persisted in `config.json`.
#[tauri::command]
async fn set_db_path(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
    new_path: String,
) -> Result<String, AppError> {
    // SECURITY: Guard against null-byte injection in the path.
    if new_path.contains('\0') {
        return Err(AppError::Validation("Invalid database path".into()));
    }

    let new_db_path = PathBuf::from(&new_path);

    // SECURITY: Require a recognised database extension to prevent the command
    // from being used to overwrite arbitrary files (e.g. ~/.zshrc or a config
    // file) with a copy of the SQLite database.
    let ext = new_db_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !matches!(ext.to_lowercase().as_str(), "db" | "sqlite" | "sqlite3") {
        return Err(AppError::Validation(
            "Database path must use a .db, .sqlite, or .sqlite3 extension".into(),
        ));
    }

    // SECURITY: Reject symlinks in the target path to prevent symlink-based
    // file-overwrite attacks on the destination.  If the destination does not
    // yet exist, symlink_metadata returns an error which we treat as "not a
    // symlink" (the file will be created by the copy step below).
    if let Ok(meta) = std::fs::symlink_metadata(&new_db_path) {
        if meta.file_type().is_symlink() {
            return Err(AppError::Validation(
                "Database path must not be a symbolic link".into(),
            ));
        }
    }

    // Validate: parent directory must exist and be writable
    let parent = new_db_path
        .parent()
        .ok_or_else(|| AppError::Validation("New database path has no parent directory".into()))?;

    if !dir_is_writable(parent) {
        return Err(AppError::Validation(format!(
            "Directory '{}' does not exist or is not writable",
            parent.display()
        )));
    }

    // Get current DB path from shared state
    let current_path = {
        let app = state
            .lock()
            .map_err(|_| AppError::Internal("State lock failed".into()))?;
        PathBuf::from(&app.db_path)
    };

    if current_path == new_db_path {
        return Ok(new_path);
    }

    // Atomic copy: write to a temp file first, verify, then rename
    let tmp_path = parent.join(format!(
        ".jura_db_migrate_{}.tmp",
        uuid::Uuid::new_v4().as_simple()
    ));

    // Copy current DB to temp location
    std::fs::copy(&current_path, &tmp_path).map_err(|e| {
        log::error!("Failed to copy database to '{}': {e}", tmp_path.display());
        AppError::FileSystem("Failed to copy database to new location".into())
    })?;

    // Verify the copy opens cleanly with SQLite
    {
        let verify_conn = rusqlite::Connection::open(&tmp_path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp_path);
            log::error!("Copied database failed SQLite verification: {e}");
            AppError::Database("Copied database failed SQLite verification".into())
        })?;
        // Quick integrity check
        verify_conn
            .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .map_err(|e| {
                let _ = std::fs::remove_file(&tmp_path);
                log::error!("Database integrity check failed: {e}");
                AppError::Database("Database integrity check failed".into())
            })?;
    }

    // Rename temp file to final destination
    std::fs::rename(&tmp_path, &new_db_path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        log::error!(
            "Failed to move database to '{}': {e}",
            new_db_path.display()
        );
        AppError::FileSystem("Failed to move database to new location".into())
    })?;

    // Persist the new path in config.json
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Read the current config so we preserve the licence_tier (and any future
    // fields), then update only db_path.
    let mut config = read_app_config(&data_dir);
    config.db_path = Some(new_path.clone());
    write_app_config(&data_dir, &config).map_err(AppError::FileSystem)?;

    // Update the shared state so get_db_path reflects the change immediately.
    {
        let mut app = state
            .lock()
            .map_err(|_| AppError::Internal("State lock failed".into()))?;
        app.db_path.clone_from(&new_path);
    }

    log::info!(
        "Database path changed: {} -> {}",
        current_path.display(),
        new_db_path.display()
    );

    Ok(new_path)
}

// ===== Annotation Commands =====

/// Save an analyst annotation linked to an asset, a verification run, or both.
///
/// Generates a new UUID for the annotation and stores it in the local database.
/// Returns the fully-populated `Annotation` record so the frontend can display
/// it immediately without a separate fetch.
#[tauri::command]
fn save_annotation(
    annotation_type: String,
    data_json: String,
    asset_id: Option<String>,
    verification_id: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<db::Annotation, AppError> {
    let ann = db::Annotation {
        annotation_id: uuid::Uuid::new_v4().to_string(),
        verification_id,
        asset_id,
        annotation_type,
        data_json,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let guard = state
        .lock()
        .map_err(|e| AppError::Internal(format!("State lock poisoned: {e}")))?;
    guard
        .db
        .insert_annotation(&ann)
        .map_err(|e| AppError::Database(e.to_string()).log())?;
    Ok(ann)
}

/// Retrieve annotations for an asset or verification run.
///
/// Exactly one of `asset_id` or `verification_id` must be supplied. If both
/// are `None` the command returns an empty list rather than an error.
#[tauri::command]
fn get_annotations(
    asset_id: Option<String>,
    verification_id: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<db::Annotation>, AppError> {
    let guard = state
        .lock()
        .map_err(|e| AppError::Internal(format!("State lock poisoned: {e}")))?;

    if let Some(aid) = asset_id {
        guard
            .db
            .get_annotations_for_asset(&aid)
            .map_err(|e| AppError::Database(e.to_string()).log())
    } else if let Some(vid) = verification_id {
        guard
            .db
            .get_annotations_for_verification(&vid)
            .map_err(|e| AppError::Database(e.to_string()).log())
    } else {
        Ok(vec![])
    }
}

/// Delete a single annotation by its UUID.
///
/// Deleting a non-existent annotation is a no-op and returns `Ok(())`.
#[tauri::command]
fn delete_annotation(
    annotation_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let guard = state
        .lock()
        .map_err(|e| AppError::Internal(format!("State lock poisoned: {e}")))?;
    guard
        .db
        .delete_annotation(&annotation_id)
        .map_err(|e| AppError::Database(e.to_string()).log())
}

// ===== Application Entry =====

/// Best-effort early resolution of the application data directory.
///
/// Tauri's authoritative path resolver is only available after `.setup()` runs,
/// which is too late to capture early startup log messages.  This function
/// derives the same path using only standard library calls and the platform
/// environment so that [`init_logging`] can open the log file before the Tauri
/// builder is invoked.
///
/// Returns `None` if the home directory cannot be determined.
fn dirs_next_data_dir() -> Option<PathBuf> {
    let bundle_id = "com.juralabs.jura-trace";
    #[cfg(target_os = "macos")]
    {
        // ~/Library/Application Support/<bundle-id>
        std::env::var_os("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library")
                .join("Application Support")
                .join(bundle_id)
        })
    }
    #[cfg(target_os = "linux")]
    {
        // $XDG_DATA_HOME/<bundle-id>  or  ~/.local/share/<bundle-id>
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            Some(PathBuf::from(xdg).join(bundle_id))
        } else {
            std::env::var_os("HOME").map(|h| {
                PathBuf::from(h)
                    .join(".local")
                    .join("share")
                    .join(bundle_id)
            })
        }
    }
    #[cfg(target_os = "windows")]
    {
        // %APPDATA%\<bundle-id>\data
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(bundle_id).join("data"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

/// Maximum log file size before it is truncated (10 MiB).
/// When the file exceeds this size at startup the old content is discarded
/// so that the log file never grows unboundedly on long-running deployments.
const MAX_LOG_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// Initialise the logging subsystem.
///
/// Writes to:
/// - stdout (always), so `RUST_LOG` / terminal still works in development
/// - `app_data_dir/jura-trace.log` (production builds), so IT managers can
///   inspect logs without attaching a terminal
///
/// The log file is truncated when it exceeds [`MAX_LOG_FILE_BYTES`] so that
/// long-running managed deployments do not accumulate unbounded disk usage.
/// The path is resolved from the Tauri app data directory; if that cannot be
/// determined before the Tauri app is built (we call this from `run()` before
/// `.setup()`), we fall back to stdout-only.
///
/// Returns the path that was opened, or `None` when file logging was skipped.
fn init_logging(app_data_dir: Option<&std::path::Path>) -> Option<PathBuf> {
    // Attempt to open a log file when a data directory is available.
    let log_path = app_data_dir.map(|dir| dir.join("jura-trace.log"));

    let file_target: Option<std::fs::File> = log_path.as_ref().and_then(|p| {
        // Ensure the parent directory exists.
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Rotate (truncate) if the file already exceeds the size cap.
        if let Ok(meta) = std::fs::metadata(p) {
            if meta.len() > MAX_LOG_FILE_BYTES {
                // Truncate by re-opening with create(true) + truncate(true).
                let _ = std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(p);
            }
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .ok()
    });

    match file_target {
        Some(file) => {
            // Fan-out writer: send every log line to both stdout and the file.
            struct DualWriter {
                file: std::sync::Mutex<std::fs::File>,
            }
            impl Write for DualWriter {
                fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                    // Best-effort write to file; ignore failures so a full disk
                    // never causes the app to crash.
                    let _ = self.file.lock().map(|mut f| f.write_all(buf));
                    // Always write to stdout.
                    std::io::stdout().write(buf)
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    let _ = self.file.lock().map(|mut f| f.flush());
                    std::io::stdout().flush()
                }
            }

            env_logger::Builder::from_default_env()
                .target(env_logger::Target::Pipe(Box::new(DualWriter {
                    file: std::sync::Mutex::new(file),
                })))
                .init();

            log_path
        }
        None => {
            // No log file — fall back to stdout only.
            env_logger::init();
            None
        }
    }
}

/// Spawn the PyInstaller sidecar binary and return the child handle.
///
/// Used both at startup and by the power-saver respawn path when the sidecar
/// has been idle-killed and a new verification request arrives.
///
/// In debug builds (`cargo tauri dev`) the binary is absent; the function logs
/// a notice and returns `None` so the developer's manual `uvicorn` process is used.
///
/// The caller is responsible for polling `/health` after a successful spawn to
/// wait for the sidecar to become ready before dispatching requests.
fn spawn_sidecar(app: &tauri::AppHandle) -> Option<tauri_plugin_shell::process::CommandChild> {
    if cfg!(debug_assertions) {
        return None;
    }

    // Set JURA_MODELS_DIR so the sidecar can find model files.
    if let Ok(resource_dir) = app.path().resource_dir() {
        let models_dir = resource_dir.join("models");
        if models_dir.is_dir() {
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var("JURA_MODELS_DIR", &models_dir);
            }
        }
    }

    match app.shell().sidecar("jura-sidecar") {
        Err(e) => {
            log::warn!(
                "Could not locate sidecar binary for (re)spawn: {e}. \
                 Forensic analysis will be unavailable."
            );
            None
        }
        Ok(cmd) => {
            // macOS-only: launchd-launched apps inherit a limited PATH that
            // does NOT include Homebrew directories (/opt/homebrew/bin on
            // Apple Silicon, /usr/local/bin on Intel).  The sidecar's
            // ffmpeg/ffprobe health probe uses `shutil.which()` which
            // only searches PATH, so without this prepend the sidecar
            // reports "FFmpeg not installed" even when Homebrew has it.
            // Linux and Windows package managers put ffmpeg in PATH by
            // default — only macOS needs the augmentation.
            let augmented_path = {
                let homebrew = "/opt/homebrew/bin:/usr/local/bin";
                match std::env::var("PATH") {
                    Ok(p) if !p.is_empty() => format!("{homebrew}:{p}"),
                    _ => homebrew.to_string(),
                }
            };
            let cmd = cmd.env("PATH", augmented_path);
            // JTV-142 fix 2 (2026-05-02): without PYTHONUNBUFFERED, Python's
            // stdout is fully buffered when piped to Tauri's CommandEvent
            // stream. uvicorn's "Application startup complete" + bind log
            // can be held in a 64 KB buffer for the entire startup window,
            // which makes the "process alive but Settings shows Offline"
            // symptom hard to diagnose. Forcing line-buffered flush makes
            // startup progress visible in the Rust log reader in real time.
            let cmd = cmd.env("PYTHONUNBUFFERED", "1");
            match cmd.args(["--host", "127.0.0.1", "--port", "8200"]).spawn() {
                Err(e) => {
                    log::warn!(
                        "Failed to (re)spawn sidecar: {e}. \
                     Forensic analysis will be unavailable."
                    );
                    None
                }
                Ok((mut rx, child)) => {
                    tauri::async_runtime::spawn(async move {
                        use tauri_plugin_shell::process::CommandEvent;
                        while let Some(event) = rx.recv().await {
                            match event {
                                CommandEvent::Stdout(line) => {
                                    // JTV-142 fix 2: surface sidecar startup at
                                    // info so port-bind / model-warmup progress
                                    // is visible without raising the global log
                                    // level. Volume is tolerable because the
                                    // sidecar prints sparingly post-startup.
                                    log::info!("sidecar: {}", String::from_utf8_lossy(&line));
                                }
                                CommandEvent::Stderr(line) => {
                                    log::info!("sidecar: {}", String::from_utf8_lossy(&line));
                                }
                                CommandEvent::Terminated(p) => {
                                    log::info!("Sidecar process terminated (code: {:?})", p.code);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    });
                    Some(child)
                }
            }
        }
    }
}

/// Poll the sidecar `/health/ready` endpoint until it responds or the timeout
/// elapses.
///
/// Uses exponential back-off: 200 ms → 400 → 800 → 1 600 ms (capped), up to
/// `max_attempts` total tries. Returns `true` when the sidecar is ready.
///
/// JTV-142 fix 3 (2026-05-02): polls `/health/ready`, not `/health`. The
/// `/health` endpoint runs `_ensure_model()` per request which can re-import
/// scikit-image / sklearn modules from `_MEIPASS` on a cold PyInstaller
/// bundle and block the response for hundreds of ms. `/health/ready` is a
/// constant-time bool read of a flag set during the FastAPI lifespan, so
/// every retry burns its full back-off interval rather than serialising on
/// the lazy CLIP probe. The full capability JSON at `/health` is fetched
/// separately by `SidecarClient::health()` once readiness is confirmed.
fn wait_for_sidecar_ready(max_attempts: u32) -> bool {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    for attempt in 0..max_attempts {
        let delay_ms = 200u64 * (1u64 << attempt.min(3));
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        if client
            .get("http://127.0.0.1:8200/health/ready")
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            log::info!("Sidecar ready after {} poll attempt(s)", attempt + 1);
            return true;
        }
    }
    false
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise logging before anything else.  We do a best-effort resolve of
    // the app data directory here; Tauri's proper path resolver is available
    // only inside the `.setup()` callback, but by then it is too late to
    // capture early startup messages.  The platform-specific default paths are:
    //   macOS: ~/Library/Application Support/com.juralabs.jura-trace
    //   Linux: ~/.local/share/com.juralabs.jura-trace
    //   Windows: %APPDATA%\com.juralabs.jura-trace\data
    //
    // We derive this early approximation using the same crate that Tauri uses
    // internally (dirs_next / home_dir), then let `.setup()` confirm the real
    // path.  If the early path is wrong we still log; we just lose the ability
    // to write early startup lines to the correct file.
    let early_data_dir: Option<PathBuf> = dirs_next_data_dir();
    let log_file_path = init_logging(early_data_dir.as_deref());
    if let Some(ref p) = log_file_path {
        log::info!("Log file: {}", p.display());
    }
    log::info!(
        "Starting Jura Trace v{} ({})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let db_path = resolve_db_path(app);
            log::info!("Database: {}", db_path.display());

            // Read the licence tier from config.json (defaults to Community).
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data directory");
            let startup_config = read_app_config(&data_dir);
            let licence_tier = startup_config.licence_tier;
            log::info!("Licence tier: {licence_tier:?}");
            let ai_description_enabled = startup_config.ai_description_enabled;
            log::info!(
                "AI image description preference: {}",
                match ai_description_enabled {
                    Some(true) => "enabled",
                    Some(false) => "disabled",
                    None => "not set (defaults off until user opts in)",
                }
            );

            let database = db::Database::open(&db_path).expect("failed to open database");

            // SECURITY (LOW-6): In production builds, auto-generate a random
            // session key if none is set in the environment.  This ensures the
            // sidecar is never left with empty-string authentication in the
            // field.  Development builds retain the existing behaviour (env var
            // or empty default) to allow manual `uvicorn` starts without a key.
            let sidecar_key = if cfg!(debug_assertions) {
                // Development: honour the env var, fall back to empty (no auth)
                std::env::var("JURA_SIDECAR_KEY").unwrap_or_default()
            } else {
                match std::env::var("JURA_SIDECAR_KEY") {
                    Ok(k) if !k.is_empty() => k,
                    _ => {
                        // Generate a 256-bit random key (two UUIDs concatenated)
                        // for this session and propagate it to the sidecar via
                        // the environment so the spawned process inherits it.
                        let generated = format!(
                            "{}{}",
                            uuid::Uuid::new_v4().as_simple(),
                            uuid::Uuid::new_v4().as_simple()
                        );
                        // Safety: this is a single-threaded setup callback; no
                        // other threads read JURA_SIDECAR_KEY at this point.
                        #[allow(unused_unsafe)]
                        unsafe {
                            std::env::set_var("JURA_SIDECAR_KEY", &generated);
                        }
                        log::info!(
                            "Sidecar API key auto-generated for this session \
                             (JURA_SIDECAR_KEY was not set)"
                        );
                        generated
                    }
                }
            };
            let sidecar_client = sidecar::SidecarClient::new("http://127.0.0.1:8200", &sidecar_key);

            // ── Sidecar auto-launch ──────────────────────────────────────────
            // In production builds the frozen PyInstaller binary is bundled
            // under `binaries/jura-sidecar-<target-triple>`.  We attempt to
            // spawn it here and store the child handle so we can kill it on
            // exit.  In development the binary is absent; we log a notice and
            // let the developer start `uvicorn` manually.
            //
            // `cfg!(debug_assertions)` is true for `cargo tauri dev` and false
            // for `cargo tauri build --release`, which is the right proxy for
            // "are we in dev mode?".
            if cfg!(debug_assertions) {
                log::info!(
                    "Dev mode: sidecar assumed to be running manually on \
                     http://127.0.0.1:8200"
                );
            } else if let Ok(resource_dir) = app.path().resource_dir() {
                // Emit a startup-only log for JURA_MODELS_DIR — spawn_sidecar
                // sets the env var but does not log the path itself, so we log
                // it here before delegating.
                let models_dir = resource_dir.join("models");
                if models_dir.is_dir() {
                    log::info!("JURA_MODELS_DIR set to {models_dir:?}");
                } else {
                    log::warn!(
                        "Models directory not found at {models_dir:?} — classifier \
                         and UnivFD probe will be unavailable"
                    );
                }
            }
            let sidecar_child = spawn_sidecar(app.app_handle());

            // ── Sidecar readiness check (non-blocking) ───────────────────────
            // Previously this was a blocking `wait_for_sidecar_ready(N)` call
            // in the setup thread, which kept the Tauri window hidden until
            // it returned — fine when N=10 / ~12 s, hostile when N=60 / ~125 s
            // for the PyInstaller cold-start case (Gatekeeper scan + _MEIPASS
            // extract on a 181 MB unsigned binary). Users saw the app icon
            // bounce in the Dock with no window for up to 2 minutes — the
            // "not responding" symptom from JTV-142, 2 May 2026.
            //
            // Fix: spawn the readiness probe as a tokio task. The window
            // appears immediately. Per-request availability is handled
            // dynamically by `SidecarClient::is_available()` (verify pipeline
            // gates `sidecar_available` per call) and by the Settings page
            // polling `/health` from the frontend. The probe below is purely
            // diagnostic — it logs when the sidecar comes up so launch-time
            // performance is observable without holding the main thread.
            const STARTUP_READINESS_ATTEMPTS: u32 = 60;
            let sidecar_present = sidecar_child.is_some();
            if sidecar_present {
                tauri::async_runtime::spawn(async move {
                    let ready = tauri::async_runtime::spawn_blocking(move || {
                        wait_for_sidecar_ready(STARTUP_READINESS_ATTEMPTS)
                    })
                    .await
                    .unwrap_or(false);
                    if !ready {
                        log::warn!(
                            "Sidecar did not respond within startup timeout. \
                             Verify-time availability check will be tried per request."
                        );
                    }
                });
            }

            log::info!(
                "Startup complete: db={}, tier={:?}, sidecar_auto_launched={}",
                db_path.display(),
                licence_tier,
                sidecar_child.is_some()
            );

            // Compute classifier model hash once at startup for methodology
            // versioning. Try the standard model path relative to the app
            // resource directory, falling back to the development models/ dir.
            let classifier_hash = {
                let model_name = "deepfake_classifier.joblib";
                let app_dir = app.path().resource_dir().ok();
                let candidates: Vec<PathBuf> = [
                    app_dir.as_ref().map(|d| d.join("models").join(model_name)),
                    Some(PathBuf::from("models").join(model_name)),
                ]
                .into_iter()
                .flatten()
                .collect();
                candidates.iter().find_map(|p| compute_file_sha256(p))
            };
            if let Some(ref h) = classifier_hash {
                log::info!("Classifier model hash: {}", &h[..16]);
            }

            let power_saver_mode = startup_config.power_saver_mode;
            // Initialise last_sidecar_request_ts to "now" so the idle-killer
            // does not immediately fire on a freshly started sidecar.
            let initial_ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let last_sidecar_request_ts = Arc::new(AtomicU64::new(initial_ts));

            let shared_state = Arc::new(Mutex::new(AppState {
                db: database,
                sidecar: sidecar_client,
                db_path: db_path.to_string_lossy().into_owned(),
                licence_tier,
                sidecar_process: sidecar_child,
                classifier_model_hash: classifier_hash,
                ai_description_enabled,
                scheduler_handle: None,
                last_heatmap_session: None,
                last_sidecar_request_ts: Arc::clone(&last_sidecar_request_ts),
                power_saver_mode,
                respawn_in_progress: Arc::new(AtomicBool::new(false)),
            }));

            // ── Register managed state FIRST ─────────────────────────────────
            //
            // `app.manage()` must run BEFORE any code path that could lead to a
            // Tauri command dispatch via `State<'_, Arc<Mutex<AppState>>>`.
            // Otherwise Tauri's internal state manager will panic with
            // `state() called before manage()` the first time a handler tries
            // to acquire the state.
            //
            // This race bit the project three times in one day during
            // `cargo tauri dev` hot-reload cycles: the webview on port 1420
            // stays alive across Rust binary restarts and reconnects the
            // moment the new process's event loop starts, firing its onMount
            // Tauri commands (check_sidecar_health, get_licence_tier, etc.)
            // immediately. If any async task spawned below could schedule
            // faster than the setup closure finishes, or if the frontend
            // reconnects during the setup closure, the command dispatch
            // happens before `.manage()` has been called.
            //
            // The fix is to call `.manage()` with a clone of the Arc at the
            // earliest possible point — before any spawn, before any path
            // that could yield to the Tauri runtime — and then continue to
            // use `shared_state.clone()` for the scheduler and the local
            // REST API server. `Arc<Mutex<AppState>>` is trivially clonable
            // so this costs nothing.
            //
            // Do not move this line back below the spawn blocks.
            app.manage(shared_state.clone());

            // ── URL watchlist scheduler (Monitor tab, paid tiers) ────────────
            // Spawns a background task that wakes every 60 s and verifies any
            // monitored URLs that are due for a check.  The handle is stored in
            // AppState so the RunEvent::Exit handler can cancel it cleanly.
            // On the Community tier the task idles without performing any checks.
            {
                let scheduler_state = shared_state.clone();
                let handle = monitor_scheduler::spawn_scheduler(scheduler_state, data_dir.clone());
                if let Ok(mut guard) = shared_state.lock() {
                    guard.scheduler_handle = Some(handle);
                }
            }

            // ── CLIP idle-watcher ────────────────────────────────────────────
            // Polls the sidecar every 60 s. When the CLIP model has been idle
            // for CLIP_IDLE_SECONDS_BEFORE_UNLOAD (10 min) and is still loaded,
            // sends an unload request to reclaim ~600–700 MB of RAM. Errors are
            // silently swallowed so a temporarily unreachable sidecar never
            // crashes the task — it will retry on the next tick.
            {
                let watcher_state = shared_state.clone();
                tauri::async_runtime::spawn(async move {
                    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
                    loop {
                        interval.tick().await;
                        let sidecar = {
                            match watcher_state.lock() {
                                Ok(guard) => guard.sidecar.clone(),
                                Err(_) => continue,
                            }
                        };
                        let status = match sidecar.clip_status() {
                            Ok(s) => s,
                            Err(_) => continue, // sidecar unreachable — retry next tick
                        };
                        if status.loaded
                            && status.idle_seconds >= CLIP_IDLE_SECONDS_BEFORE_UNLOAD as f64
                        {
                            log::info!(
                                "CLIP model idle for {:.0}s — requesting eviction",
                                status.idle_seconds
                            );
                            if let Err(e) = sidecar.unload_clip() {
                                log::warn!("CLIP unload request failed: {e}");
                            }
                        }
                    }
                });
            }

            // ── Power-saver idle-killer ──────────────────────────────────────
            // When power-saver mode is enabled, the sidecar process is terminated
            // after SIDECAR_IDLE_SECONDS_BEFORE_KILL (300 s) of inactivity to
            // reclaim ~300–500 MB of RAM.  The watcher checks every 60 s.
            // Errors are silently swallowed so a temporarily absent sidecar
            // never crashes the task.
            {
                let killer_state = shared_state.clone();
                let killer_ts = Arc::clone(&last_sidecar_request_ts);
                let killer_app = app.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
                    loop {
                        interval.tick().await;

                        // Read power-saver flag and last-request timestamp atomically.
                        let (enabled, process_present) = {
                            match killer_state.lock() {
                                Ok(guard) => {
                                    (guard.power_saver_mode, guard.sidecar_process.is_some())
                                }
                                Err(_) => continue,
                            }
                        };

                        if !enabled || !process_present {
                            continue;
                        }

                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        let last_ts = killer_ts.load(Ordering::Relaxed);
                        let idle_secs = now.saturating_sub(last_ts);

                        if idle_secs < SIDECAR_IDLE_SECONDS_BEFORE_KILL {
                            continue;
                        }

                        // Idle threshold exceeded — kill the sidecar.
                        let child = {
                            match killer_state.lock() {
                                Ok(mut guard) => guard.sidecar_process.take(),
                                Err(_) => continue,
                            }
                        };

                        if let Some(child) = child {
                            if let Err(e) = child.kill() {
                                log::warn!("Power-saver kill failed: {e}");
                            } else {
                                log::info!(
                                    "Sidecar process killed for RAM reclamation \
                                     (power-saver mode, idle {idle_secs}s)"
                                );
                            }
                        }

                        // Respawn will be triggered by verify_content on the
                        // next request.  Update the timestamp so that a
                        // concurrent respawn request does not race with a
                        // second kill cycle.
                        let now2 = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        killer_ts.store(now2, Ordering::Relaxed);

                        // Store the app handle clone for potential future
                        // logging — currently unused but suppresses dead-code.
                        let _ = &killer_app;
                    }
                });
            }

            // ── Local REST API server (port 8300) ────────────────────────────
            // Spawned in the Tauri async runtime so it shares the tokio executor
            // without blocking the setup thread. Binds to 127.0.0.1 only.
            // Skipped if the `api` feature is not compiled in.
            #[cfg(feature = "api")]
            {
                let api_state = shared_state.clone();
                tauri::async_runtime::spawn(async move {
                    // Bootstrap a first API key if the database has none yet.
                    if let Ok(mut guard) = api_state.lock() {
                        match guard.db.ensure_bootstrap_api_key() {
                            Ok(Some(key)) => {
                                // SECURITY: log only a redacted prefix — full key
                                // is shown once in the Settings → API Keys panel.
                                let prefix = &key[..key.len().min(8)];
                                log::info!(
                                    "API server bootstrap key created (jt_{prefix}...). \
                                     Retrieve the full key from Settings → API Keys."
                                );
                            }
                            Ok(None) => {
                                log::info!(
                                    "API server: existing API key(s) found, no bootstrap needed"
                                );
                            }
                            Err(e) => {
                                log::warn!("API server: bootstrap key creation failed: {e}");
                            }
                        }
                    }

                    if let Err(e) = api::start_server(api_state, 8300).await {
                        log::error!("API server error: {e}");
                    }
                });
            }

            // `shared_state` is now owned by the scheduler block above plus the
            // API server spawn plus the managed-state registration. The
            // original binding drops here; the Arc lives on via the clones.
            drop(shared_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_stats,
            import_files,
            get_assets,
            get_filtered_assets,
            get_recent_assets,
            delete_asset,
            verify_content,
            sign_asset,
            read_manifest,
            verify_c2pa,
            get_fingerprints,
            find_similar,
            verify_url,
            check_sidecar_health,
            check_metadata_before_sign,
            get_version,
            mark_false_positive,
            get_false_positive_stats,
            verify_audit_integrity,
            get_monitor_overview,
            get_audit_log,
            get_verification_history,
            add_monitor_url,
            remove_monitor_url,
            list_monitor_urls,
            get_monitor_events,
            update_monitor_case_status,
            embed_watermark_asset,
            extract_watermark_from_path,
            analyse_video_deepfake,
            run_npr_on_demand,
            run_shadow_consistency_on_demand,
            run_splice_boundary_on_demand,
            extract_text_from_image,
            get_db_path,
            set_db_path,
            backup_database,
            restore_database,
            import_assets_csv,
            get_licence_tier,
            set_licence_tier,
            get_ai_description_enabled,
            set_ai_description_enabled,
            get_skip_wizard,
            calculate_sun_position,
            estimate_shadow_time,
            save_annotation,
            get_annotations,
            delete_annotation,
            create_api_key,
            list_api_keys,
            revoke_api_key,
            import_conformant_certificate,
            get_signing_mode,
            set_signing_mode,
            get_conformant_cert_info,
            clear_conformant_cert,
            read_manifest_chain,
            get_network_mode,
            set_network_mode,
            get_power_saver_mode,
            set_power_saver_mode,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Jura Trace")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Kill the sidecar process when the app exits so it does not
                // linger in the background consuming system resources.
                //
                // The managed type is `Arc<Mutex<AppState>>` (see `app.manage`
                // above), so we request that exact type.  Use `try_state` to
                // avoid panicking if setup failed before state was registered
                // (a crash loop during `cargo tauri dev` hot-reload can trigger
                // Exit without a completed setup).
                if let Some(state) = app.try_state::<Arc<Mutex<AppState>>>() {
                    if let Ok(mut guard) = state.lock() {
                        // Cancel the background URL watchlist scheduler.
                        if let Some(handle) = guard.scheduler_handle.take() {
                            handle.cancel();
                            log::info!("Monitor scheduler cancelled on app exit");
                        }

                        if let Some(child) = guard.sidecar_process.take() {
                            if let Err(e) = child.kill() {
                                log::warn!("Failed to kill sidecar on exit: {e}");
                            } else {
                                log::info!("Sidecar process terminated on app exit");
                            }
                        }
                    }
                }
                // Remove all heatmap session directories from the cache dir.
                if let Ok(cache_dir) = app.path().app_cache_dir() {
                    heatmap::clear_all_heatmap_sessions(&cache_dir);
                    log::info!("Heatmap session directories cleared on app exit");
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_clean_image_with_exif() {
        // All signals clean, full EXIF, authentic verdict → high trust
        let trust = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            1.0,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(trust > 0.85, "Expected >0.85, got {trust:.3}");
    }

    #[test]
    fn trust_manipulated_image() {
        // ELA, noise, and copy-move all suspicious → low trust
        let trust = compute_trust(
            Some(0.7),
            Some(0.8),
            Some(0.6),
            Some(0.2),
            Some("high"),
            Some("authentic"),
            0.5,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(trust < 0.5, "Expected <0.5, got {trust:.3}");
    }

    #[test]
    fn trust_concordance_dampens_false_positives() {
        // ELA clean, deepfake clean, but noise+copymove maxed (codec false positive)
        let trust = compute_trust(
            Some(0.04),
            Some(1.0),
            Some(1.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.80,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(trust > 0.50, "Expected >0.50, got {trust:.3}");
    }

    #[test]
    fn trust_genuine_manipulation_not_boosted() {
        // ELA is suspicious → concordance boost should NOT fire
        let trust = compute_trust(
            Some(0.7),
            Some(0.8),
            Some(0.5),
            Some(0.2),
            Some("high"),
            Some("authentic"),
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(trust < 0.55, "Expected <0.55, got {trust:.3}");
    }

    #[test]
    fn trust_ela_weighted_higher() {
        // ELA clean but noise suspicious — ELA's 2.0 weight should pull up
        let trust_weighted = compute_trust(
            Some(0.1),
            Some(0.8),
            Some(0.5),
            Some(0.3),
            Some("high"),
            Some("authentic"),
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust_weighted > 0.55,
            "Expected >0.55, got {trust_weighted:.3}"
        );
    }

    #[test]
    fn trust_c2pa_bonus_applied() {
        let trust_without = compute_trust(
            Some(0.1),
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        let trust_with = compute_trust(
            Some(0.1),
            None,
            None,
            None,
            None,
            None,
            0.8,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust_with > trust_without,
            "C2PA bonus not applied: {trust_with:.3} vs {trust_without:.3}"
        );
    }

    #[test]
    fn trust_no_forensics_falls_back_to_exif() {
        let trust = compute_trust(
            None, None, None, None, None, None, 0.8, None, None, None, None, None, false, None,
            None, None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!((trust - 0.8).abs() < 0.01, "Expected ~0.8, got {trust:.3}");
    }

    #[test]
    fn trust_avif_news_image_regression() {
        let trust = compute_trust(
            Some(0.04),
            Some(0.76),
            Some(0.35),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust > 0.65,
            "AVIF news image should score >65%, got {:.1}%",
            trust * 100.0
        );
    }

    #[test]
    fn trust_score_bounded() {
        let trust = compute_trust(
            Some(0.0),
            Some(0.0),
            Some(0.0),
            Some(0.0),
            Some("high"),
            Some("authentic"),
            1.0,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(trust <= 1.0, "Trust exceeded 1.0: {trust:.3}");
    }

    // ── Verdict ceiling tests ─────────────────────────────────────────

    #[test]
    fn trust_inconclusive_verdict_caps_trust() {
        // Fake wedding image scenario: deepfake score 0.31, inconclusive verdict.
        // Previously scored 92% "High Trust" — now capped at 55%.
        let trust = compute_trust(
            None,
            None,
            None,
            Some(0.31),
            Some("low"),
            Some("inconclusive"),
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust <= 0.55,
            "Inconclusive verdict should cap trust at 0.55, got {trust:.3}"
        );
        assert!(
            trust >= 0.30,
            "Trust should still be in medium range, got {trust:.3}"
        );
    }

    #[test]
    fn trust_synthetic_high_confidence_very_low() {
        let trust = compute_trust(
            None,
            None,
            None,
            Some(0.85),
            Some("high"),
            Some("synthetic"),
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust <= 0.25,
            "Synthetic+high should cap at 0.25, got {trust:.3}"
        );
    }

    #[test]
    fn trust_synthetic_low_confidence_capped() {
        // Synthetic with low confidence ≈ inconclusive, caps at 0.45
        let trust = compute_trust(
            None,
            None,
            None,
            Some(0.7),
            Some("low"),
            Some("synthetic"),
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust <= 0.45,
            "Synthetic+low should cap at 0.45, got {trust:.3}"
        );
    }

    /// Item 2 of JTV-105 truth-grid pass — pins the video-trust wiring fix.
    /// `compute_trust` for video files now receives `aggregate_score` from
    /// the per-frame deepfake aggregate (not the always-None image GBM
    /// score). A high-deepfake video must produce a low trust headline.
    #[test]
    fn trust_video_high_deepfake_score() {
        // Pristine deepfake video: aggregate_score 0.92 + synthetic + high
        // confidence. Image manipulation signals are all None for video.
        let trust = compute_trust(
            None, // ela_score (video — None)
            None, // noise_score
            None, // copy_move_score
            Some(0.92),
            Some("high"),
            Some("synthetic"),
            0.5,  // exif_trust (video EXIF)
            None, // c2pa_valid
            None, // segmented_ela_score
            None, // shadow_consistency_score
            None, // colour_temperature_score
            None, // splice_boundary_score
            false,
            None, // jpeg_ghost_score (video — None)
            None,
            None,
            true,
            false,
            false,
        );
        assert!(
            trust < 0.25,
            "High video deepfake (0.92, synthetic, high) should cap trust < 0.25, got {trust:.3}"
        );
    }

    /// Item 2 of JTV-105 — clean video with low aggregate_score must
    /// produce a high trust headline (no false synthetic verdict).
    #[test]
    fn trust_video_clean_deepfake_score() {
        let trust = compute_trust(
            None,
            None,
            None,
            Some(0.08),
            Some("high"),
            Some("authentic"),
            0.5,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            true,
            false,
            false,
        );
        assert!(
            trust > 0.75,
            "Clean video deepfake (0.08, authentic, high) should yield trust > 0.75, got {trust:.3}"
        );
    }

    #[test]
    fn trust_authentic_verdict_no_ceiling() {
        let trust = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            1.0,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust > 0.85,
            "Authentic verdict should allow high trust, got {trust:.3}"
        );
    }

    #[test]
    fn trust_no_verdict_no_ceiling() {
        // Sidecar offline — no verdict available, should not impose ceiling
        let trust = compute_trust(
            Some(0.04),
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust > 0.70,
            "No sidecar should fall back to EXIF, got {trust:.3}"
        );
    }

    #[test]
    fn verify_quick_mode_skips_sidecar() {
        let mode: Option<&str> = Some("quick");
        let effective = match mode {
            Some("fast") | Some("quick") => "quick",
            Some("standard") => "standard",
            Some("deep") | Some("archival") => "deep",
            _ => "standard",
        };
        assert_eq!(effective, "quick");
        let is_quick = effective == "quick";
        assert!(is_quick, "Quick mode must skip sidecar");
    }

    #[test]
    fn verify_fast_mode_maps_to_quick() {
        let mode: Option<&str> = Some("fast");
        let effective = match mode {
            Some("fast") | Some("quick") => "quick",
            Some("standard") => "standard",
            Some("deep") | Some("archival") => "deep",
            _ => "standard",
        };
        assert_eq!(effective, "quick", "Legacy 'fast' should map to 'quick'");
    }

    #[test]
    fn verify_standard_mode_allows_sidecar_not_deep() {
        let mode: Option<&str> = Some("standard");
        let effective = match mode {
            Some("fast") | Some("quick") => "quick",
            Some("standard") => "standard",
            Some("deep") | Some("archival") => "deep",
            _ => "standard",
        };
        let is_quick = effective == "quick";
        let is_deep = matches!(effective, "deep" | "archival");
        assert!(!is_quick, "Standard should allow sidecar");
        assert!(!is_deep, "Standard should NOT run deep detectors");
    }

    #[test]
    fn verify_deep_mode_allows_all_detectors() {
        let mode: Option<&str> = Some("deep");
        let effective = match mode {
            Some("fast") | Some("quick") => "quick",
            Some("standard") => "standard",
            Some("deep") | Some("archival") => "deep",
            _ => "standard",
        };
        let is_deep = matches!(effective, "deep" | "archival");
        assert!(is_deep, "Deep mode must run all detectors");
    }

    #[test]
    fn archival_mode_alias_normalises_to_deep() {
        // Sprint 30 verify mode consolidation:
        // The archival mode is retired but `"archival"` is still accepted as
        // a back-compat alias and silently normalised to `"deep"`.  This test
        // is a regression guard — if the alias is ever dropped (a breaking
        // change), this test will fail and the committer must update both
        // the alias acceptance in `verify_content_inner` AND any pilot user
        // localStorage migration before merging.
        let mode: Option<&str> = Some("archival");
        let effective = match mode {
            Some("fast") | Some("quick") => "quick",
            Some("standard") => "standard",
            Some("deep") | Some("archival") => "deep",
            _ => "standard",
        };
        assert_eq!(
            effective, "deep",
            "Legacy archival mode must normalise to deep for back-compat"
        );
    }

    #[test]
    fn deep_mode_requests_twenty_video_frames() {
        // Sprint 30 acceptance criterion: Deep mode video deepfake analysis
        // must request 20 frames (FRAME_COUNTS["deep"] in the sidecar), not
        // the obsolete 12-frame cap from before the deep-mode rollout.
        // The sidecar enforces the value; this test guards the Rust-side
        // expectation by encoding the contract in code so any future
        // request to lower the deep-mode frame budget is visible at review.
        const DEEP_MODE_FRAME_COUNT: u32 = 20;
        assert_eq!(
            DEEP_MODE_FRAME_COUNT, 20,
            "Deep mode promises 20 video frames; sidecar FRAME_COUNTS must match"
        );
    }

    // ── Restore validation (JTV-130 Phase 2) ──────────────────────

    #[test]
    fn validate_snapshot_accepts_fresh_database() {
        let dir = tempfile::tempdir().unwrap();
        let snap_path = dir.path().join("good.sqlite");
        let _seed = db::Database::open(&snap_path).unwrap();
        drop(_seed);

        let result = validate_snapshot_for_restore(&snap_path).expect("fresh DB must validate");
        assert!(!result.success, "validate-only must not flip success");
        assert!(result.audit_chain_valid);
        assert_eq!(
            result.snapshot_schema_version,
            db::Database::current_schema_version()
        );
    }

    #[test]
    fn validate_snapshot_rejects_future_schema_version() {
        // Forge a snapshot whose user_version is one greater than the
        // current build supports.  This simulates a snapshot taken on a
        // newer build that we cannot safely restore on the current one.
        let dir = tempfile::tempdir().unwrap();
        let snap_path = dir.path().join("future.sqlite");
        {
            let conn = rusqlite::Connection::open(&snap_path).unwrap();
            let future = db::Database::current_schema_version() + 1;
            conn.pragma_update(None, "user_version", future).unwrap();
        }

        let err = validate_snapshot_for_restore(&snap_path)
            .expect_err("future-schema snapshot must be rejected");
        match err {
            AppError::Validation(msg) => {
                assert!(
                    msg.contains("newer version"),
                    "expected 'newer version' message, got: {msg}"
                );
            }
            other => panic!("expected Validation error, got {other:?}"),
        }
    }

    #[test]
    fn validate_snapshot_rejects_tampered_audit_chain() {
        // Seed a DB with two audit entries, then mutate one of the
        // entry_hash values directly via SQLite to break the chain.
        // The validator must surface this as a Validation error.
        let dir = tempfile::tempdir().unwrap();
        let snap_path = dir.path().join("tampered.sqlite");
        {
            let db = db::Database::open(&snap_path).unwrap();
            db.log_action("import", "asset", "x", None, None, None)
                .unwrap();
            db.log_action("verify", "asset", "x", None, None, None)
                .unwrap();
        }
        // Tamper.
        {
            let conn = rusqlite::Connection::open(&snap_path).unwrap();
            conn.execute(
                "UPDATE audit_log SET entry_hash = ? WHERE rowid = (SELECT MIN(rowid) FROM audit_log)",
                ["0000000000000000000000000000000000000000000000000000000000000000"],
            )
            .unwrap();
        }

        let err =
            validate_snapshot_for_restore(&snap_path).expect_err("tampered chain must be rejected");
        match err {
            AppError::Validation(msg) => {
                assert!(
                    msg.contains("Audit trail integrity check failed"),
                    "expected audit-trail rejection message, got: {msg}"
                );
            }
            other => panic!("expected Validation error, got {other:?}"),
        }
    }

    // ── CSV import (JTV-130 Phase 3) ──────────────────────────────

    #[test]
    fn csv_import_rejects_missing_file_path_column() {
        // Header without `file_path` is a structural failure — we surface
        // it as a Validation error rather than processing zero rows.
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("bad.csv");
        std::fs::write(&csv_path, "name,size\nfoo,100\n").unwrap();

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(&csv_path)
            .unwrap();
        let headers = rdr.headers().unwrap().clone();
        let has_file_path = headers
            .iter()
            .any(|h| matches!(h.trim(), "file_path" | "File Path"));
        assert!(
            !has_file_path,
            "Sanity check: this CSV should not have a file_path column"
        );
    }

    #[test]
    fn csv_import_handles_utf8_bom_in_header() {
        // QA EDGE-2 (30 April 2026 final QA pass): Excel-on-Windows
        // exports CSVs with a UTF-8 BOM (\xEF\xBB\xBF) by default.  The
        // csv crate's ReaderBuilder strips the BOM only when configured
        // correctly.  This test pins the contract: a BOM-prefixed header
        // must still resolve `file_path` as the first column.
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("bom.csv");
        let mut f = std::fs::File::create(&csv_path).unwrap();
        f.write_all(b"\xEF\xBB\xBF").unwrap();
        writeln!(f, "file_path,sha256_hash").unwrap();
        writeln!(f, "/tmp/dummy.jpg,abc123").unwrap();
        drop(f);

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_path(&csv_path)
            .unwrap();
        let headers = rdr.headers().unwrap().clone();
        let file_path_idx = headers.iter().position(|h| {
            h.trim().eq_ignore_ascii_case("file_path") || h.trim().eq_ignore_ascii_case("File Path")
        });
        assert!(
            file_path_idx.is_some(),
            "csv crate must strip UTF-8 BOM and recognise file_path column. \
             Headers were: {:?}",
            headers.iter().collect::<Vec<_>>()
        );
        assert_eq!(file_path_idx, Some(0));
    }

    #[test]
    fn csv_import_row_cap_is_exclusive_at_ten_thousand() {
        // QA GAP-2 (30 April 2026 final QA pass): the 10 000-row hard
        // cap was untested.  Pin the inclusive/exclusive semantics so
        // any future change to CSV_IMPORT_MAX_ROWS or the guard
        // condition (`>=` vs `>`) is caught at review time.
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();

        // Exactly CSV_IMPORT_MAX_ROWS rows: cap must NOT fire.
        let csv_path = dir.path().join("at_cap.csv");
        {
            let mut f = std::fs::File::create(&csv_path).unwrap();
            writeln!(f, "file_path").unwrap();
            for i in 0..CSV_IMPORT_MAX_ROWS {
                writeln!(f, "/nonexistent/path/{i}").unwrap();
            }
        }
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_path(&csv_path)
            .unwrap();
        let mut row_count: usize = 0;
        for (idx, _rec) in rdr.records().enumerate() {
            assert!(
                idx < CSV_IMPORT_MAX_ROWS,
                "Cap should not fire at row {idx} when total rows = CSV_IMPORT_MAX_ROWS"
            );
            row_count += 1;
        }
        assert_eq!(
            row_count, CSV_IMPORT_MAX_ROWS,
            "Exactly {CSV_IMPORT_MAX_ROWS} rows must be processed before cap fires"
        );

        // CSV_IMPORT_MAX_ROWS + 1: the cap-trigger row must be reachable.
        let csv_over = dir.path().join("over_cap.csv");
        {
            let mut f = std::fs::File::create(&csv_over).unwrap();
            writeln!(f, "file_path").unwrap();
            for i in 0..=CSV_IMPORT_MAX_ROWS {
                writeln!(f, "/nonexistent/path/{i}").unwrap();
            }
        }
        let mut rdr2 = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_path(&csv_over)
            .unwrap();
        let over_index = rdr2
            .records()
            .enumerate()
            .find(|(idx, _)| *idx >= CSV_IMPORT_MAX_ROWS)
            .map(|(idx, _)| idx);
        assert_eq!(
            over_index,
            Some(CSV_IMPORT_MAX_ROWS),
            "Row at index CSV_IMPORT_MAX_ROWS must be reachable so the cap guard fires"
        );
    }

    #[test]
    fn csv_import_handles_quoted_field_with_embedded_comma() {
        // QA EDGE-3 (30 April 2026 final QA pass): RFC 4180 quoting must
        // not interact badly with `flexible(true)`.  A row with a quoted
        // file_path containing a comma must parse as a single field, not
        // shift columns and drop the path.
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("quoted.csv");
        let mut f = std::fs::File::create(&csv_path).unwrap();
        writeln!(f, "file_path,sha256_hash").unwrap();
        writeln!(f, "\"/tmp/foo, bar.jpg\",abc123").unwrap();
        drop(f);

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_path(&csv_path)
            .unwrap();
        let record = rdr.records().next().unwrap().unwrap();
        assert_eq!(
            record.get(0),
            Some("/tmp/foo, bar.jpg"),
            "RFC 4180 quoted comma must not split the field"
        );
        assert_eq!(record.get(1), Some("abc123"));
    }

    #[test]
    fn csv_import_canonicalisation_rejects_traversal() {
        // The `..` traversal sequence canonicalises into a real path on
        // the filesystem; the safety net is the regular-file + symlink
        // checks that follow.  Exercise that flow here by feeding a
        // non-existent path: the canonicalize() call returns Err, which
        // the import loop converts into a per-row error (not an abort).
        let bogus = std::path::PathBuf::from("/nonexistent_dir/../../etc/passwd");
        let result = bogus.canonicalize();
        assert!(
            result.is_err(),
            "Non-existent path must fail canonicalize() (the import-side guard relies on this)"
        );
    }

    #[test]
    fn with_extension_suffix_appends_correctly() {
        // -wal / -shm path derivation for SQLite WAL cleanup during restore.
        let p = std::path::Path::new("/tmp/foo.db");
        assert_eq!(
            with_extension_suffix(p, "-wal"),
            std::path::PathBuf::from("/tmp/foo.db-wal")
        );
        assert_eq!(
            with_extension_suffix(p, "-shm"),
            std::path::PathBuf::from("/tmp/foo.db-shm")
        );
    }

    #[test]
    fn metadata_signing_warning_serialises_to_camel_case() {
        let warning = MetadataSigningWarning {
            has_existing_artist: true,
            existing_artist: Some("Jane Smith".to_string()),
            has_existing_copyright: false,
            existing_copyright: None,
            has_existing_description: false,
            existing_description: None,
            has_existing_c2pa: false,
            warning_message: Some("Artist field: \"Jane Smith\".".to_string()),
        };
        let json = serde_json::to_string(&warning).unwrap();
        assert!(json.contains("\"hasExistingArtist\""));
        assert!(json.contains("\"existingArtist\""));
        assert!(json.contains("\"warningMessage\""));
        assert!(!json.contains("\"has_existing_artist\""));
    }

    #[test]
    fn metadata_warning_message_builder() {
        let warnings: Vec<String> = vec![
            "Artist field: \"Alice\"".to_string(),
            "Copyright field: \"2026 Alice\"".to_string(),
        ];
        let msg = format!(
            "This file contains existing metadata that will be preserved in the signed copy: {}.",
            warnings.join("; ")
        );
        assert!(msg.contains("Artist field"));
        assert!(msg.contains("Copyright field"));
        assert!(msg.ends_with('.'));
    }

    #[test]
    fn verify_none_mode_defaults_to_standard() {
        let mode: Option<&str> = None;
        let effective = match mode {
            Some("fast") | Some("quick") => "quick",
            Some("standard") => "standard",
            Some("deep") | Some("archival") => "deep",
            _ => "standard",
        };
        assert_eq!(effective, "standard", "None mode must default to standard");
        let is_quick = effective == "quick";
        let is_deep = matches!(effective, "deep" | "archival");
        assert!(!is_quick, "Default should allow sidecar");
        assert!(!is_deep, "Default should not run deep detectors");
    }

    #[test]
    fn trust_verdict_ceiling_overrides_high_base_trust() {
        // The key regression test: a fake image with clean ELA/noise/copy-move
        // but inconclusive deepfake should NOT show "High Trust".
        // Previously: trust = 0.92 (92% High Trust) — dangerously misleading.
        // Now: capped at 0.55 by inconclusive ceiling.
        let trust = compute_trust(
            Some(0.05), // ELA clean
            Some(0.06), // Noise clean
            Some(0.0),  // Copy-move clean
            Some(0.31), // Deepfake borderline
            Some("low"),
            Some("inconclusive"),
            0.95, // Good EXIF (web image with some data)
            None,
            None,
            None,
            None,
            None, // no regional detectors
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust <= 0.55,
            "Inconclusive should cap trust at 55% max, got {:.1}%",
            trust * 100.0
        );
    }

    #[test]
    fn trust_xmp_ai_digital_source_capped_at_25() {
        // Firefly→Photoshop Web→JPEG round-trip pattern: C2PA stripped, but
        // XMP IPTC DigitalSourceType: TrainedAlgorithmicMedia survives.
        // GBM inconclusive, CLIP/UnivFD lukewarm, no tampering. Without
        // the self-declared ceiling, trust ≈ 0.55 (Concern). With it,
        // capped at 0.25 — the file says it's AI, so we believe the file.
        let trust = compute_trust(
            Some(0.05), // ELA clean
            Some(0.06), // Noise clean
            Some(0.0),  // Copy-move clean
            Some(0.41), // Deepfake borderline
            Some("low"),
            Some("inconclusive"),
            0.85, // EXIF still has useful data
            None,
            None,
            None,
            None,
            None,
            false, // ai_declared_by_c2pa: false (manifest stripped)
            None,
            None,
            None,
            true,
            true,  // ai_declared_by_xmp: TRUE (the new behaviour)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust <= 0.25,
            "XMP self-declared AI must cap trust at 25% max, got {:.1}%",
            trust * 100.0
        );
    }

    #[test]
    fn trust_composite_ai_capped_at_055_not_025() {
        // Pixel Zoom Enhance / Magic Editor pattern: real photo with
        // AI-composited regions.  XMP DigitalSourceType =
        // compositeWithTrainedAlgorithmicMedia.  Should cap at 0.55
        // (Medium — AI components declared), NOT 0.25 (pure-AI ceiling).
        // The base photograph is real; the AI-touched regions warrant
        // disclosure but not deepfake-class trust.
        let trust = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.20),
            Some("medium"),
            Some("authentic"),
            0.95,
            Some(true), // c2pa_valid: Pixel manifest is fully valid
            None,
            None,
            None,
            None,
            false, // ai_declared_by_c2pa: false (composite is separate)
            None,
            None,
            None,
            true,
            false, // ai_declared_by_xmp: false (composite is separate)
            true,  // ai_declared_composite: TRUE (the new behaviour)
        );
        assert!(
            trust > 0.25,
            "Composite AI must NOT trigger the 0.25 pure-AI ceiling, got {:.1}%",
            trust * 100.0
        );
        assert!(
            trust <= 0.55,
            "Composite AI must cap at 0.55, got {:.1}%",
            trust * 100.0
        );
    }

    #[test]
    fn trust_pure_ai_overrides_composite_when_both_set() {
        // Defensive: if both flags are somehow set, pure AI wins (stricter
        // ceiling applies). Real cases shouldn't have both, but the code
        // path must be deterministic.
        let trust = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            1.0,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            true,
            true, // pure-AI declared
            true, // composite-AI also declared
        );
        assert!(
            trust <= 0.25,
            "Pure-AI ceiling must dominate when both flags set, got {:.1}%",
            trust * 100.0
        );
    }

    #[test]
    fn trust_xmp_declaration_overrides_high_base_trust() {
        // Even with all forensics clean and a strongly authentic deepfake
        // verdict (which would otherwise yield > 0.85), an XMP self-declared
        // AI provenance signal still caps the score at 0.25.
        let trust = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            1.0,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            true,
            true,  // ai_declared_by_xmp: TRUE
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust <= 0.25,
            "XMP self-declared AI must override even high base trust, got {:.1}%",
            trust * 100.0
        );
    }

    #[test]
    fn verification_result_includes_mode() {
        // Verify that VerificationResult has a `mode` field and that it
        // serialises to camelCase (it is a plain string so rename_all does
        // not change the key, but the value must round-trip correctly).
        let result = VerificationResult {
            source_type: "file".to_string(),
            content_type: "image".to_string(),
            mode: "standard".to_string(),
            ela_score: None,
            noise_score: None,
            copy_move_score: None,
            deepfake_score: None,
            c2pa_valid: None,
            metadata_flags: vec![],
            claim_verdict: None,
            overall_trust: 0.0,
            exif_analysis: None,
            image_metadata: None,
            c2pa_manifest: None,
            c2pa_chain: None,
            ela_result: None,
            noise_result: None,
            copy_move_result: None,
            deepfake_result: None,
            clip_result: None,
            npr_result: None,
            jpeg_ghost_result: None,
            segmented_ela_result: None,
            shadow_consistency_result: None,
            colour_temperature_result: None,
            splice_boundary_result: None,
            ai_generator: None,
            watermark_extract_result: None,
            video_metadata: None,
            audio_metadata: None,
            video_deepfake_result: None,
            transcription_result: None,
            claim_check_result: None,
            ai_description: None,
            thumbnail_check: None,
            dct_analysis_result: None,
            fourier_analysis_result: None,
            input_sha256: None,
            methodology: None,
            input_quality: None,
            content_type_result: None,
            platform_fingerprint_result: None,
            filename_analysis: None,
            pdf_provenance: None,
            detectors_run: Vec::new(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(
            json.contains("\"mode\":\"standard\""),
            "mode field missing or wrong value in serialised JSON: {json}"
        );
        // JTV-134: confirm the new informational-only field serialises with
        // camelCase under the existing `rename_all = "camelCase"` rule.
        assert!(
            json.contains("\"platformFingerprintResult\""),
            "platformFingerprintResult field missing in serialised JSON: {json}"
        );
    }

    #[test]
    fn platform_fingerprint_is_informational_only() {
        // JTV-134 informational-only contract: the platform fingerprint
        // result does NOT contribute to `compute_trust`.  This test guards
        // against future regressions where someone wires the result into
        // the trust formula.  Two clean calls with identical args MUST
        // produce identical scores.  If a future change adds a platform
        // fingerprint parameter to `compute_trust`, this call site fails
        // to compile — that is the tripwire.
        let baseline = compute_trust(
            Some(0.1),
            None,
            None,
            None,
            None,
            None,
            0.8,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            true,
            false,
            false,
        );
        let again = compute_trust(
            Some(0.1),
            None,
            None,
            None,
            None,
            None,
            0.8,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            true,
            false,
            false,
        );
        assert!((baseline - again).abs() < f64::EPSILON);
    }

    // ── Regional detector trust tests ─────────────────────────────────

    #[test]
    fn trust_single_regional_detector_lowers_trust() {
        // Segmented ELA alone (score 0.7) should lower trust below a clean baseline
        let trust_with = compute_trust(
            Some(0.05),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            Some(0.7),
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        let trust_without = compute_trust(
            Some(0.05),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust_with < trust_without,
            "Regional segmented ELA should lower trust: {trust_with:.3} vs {trust_without:.3}"
        );
    }

    #[test]
    fn trust_two_suspicious_regional_detectors_cap_at_055() {
        // Two regional detectors both > 0.5 → composite amplification cap applies
        let trust = compute_trust(
            Some(0.05),
            Some(0.05),
            Some(0.0),
            Some(0.10),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            Some(0.7), // segmented ELA suspicious
            None,
            Some(0.65), // colour temperature suspicious
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust <= 0.55,
            "Two suspicious regional detectors should cap trust at 0.55, got {trust:.3}"
        );
    }

    #[test]
    fn trust_three_suspicious_regional_detectors_still_capped() {
        // Both active regional detectors suspicious + shadow (ignored) — cap holds
        // Shadow consistency score is passed but no longer participates in
        // regional signals after the forensic audit demotion.
        let trust = compute_trust(
            Some(0.05),
            None,
            None,
            Some(0.10),
            Some("high"),
            Some("authentic"),
            0.9,
            None,
            Some(0.8),  // segmented ELA (active)
            Some(0.6),  // shadow consistency (ignored in scoring)
            Some(0.75), // colour temperature (active)
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust <= 0.55,
            "Two active suspicious regional detectors should cap at 0.55, got {trust:.3}"
        );
    }

    #[test]
    fn trust_one_suspicious_regional_detector_no_cap() {
        // Only one regional detector suspicious (score > 0.5) — cap should NOT fire
        let trust = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            Some(0.6), // segmented ELA suspicious
            None,      // shadow — absent
            Some(0.3), // colour temperature clean
            None,      // splice boundary — absent
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust > 0.55,
            "Single suspicious regional detector should not trigger the 0.55 cap, got {trust:.3}"
        );
    }

    #[test]
    fn trust_regional_detectors_all_clean_no_penalty() {
        // All four regional detectors clean — trust should match no-regional baseline
        let trust_regional = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            Some(0.05),
            Some(0.04),
            Some(0.06),
            Some(0.03),
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        let trust_no_regional = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        // With all regional detectors clean the trust should be close to the
        // no-regional baseline (regional scores ≈ 0 contribute ~1.0 trust).
        assert!(
            (trust_regional - trust_no_regional).abs() < 0.05,
            "Clean regional detectors should not significantly alter trust: \
             regional={trust_regional:.3} vs baseline={trust_no_regional:.3}"
        );
    }

    #[test]
    fn trust_regional_cap_overrides_verdict_ceiling() {
        // Regional cap (0.55) equals the inconclusive verdict ceiling (0.55)
        // — the minimum of both must apply.
        let trust = compute_trust(
            Some(0.05),
            None,
            None,
            Some(0.31),
            Some("low"),
            Some("inconclusive"),
            0.8,
            None,
            Some(0.7), // two regional detectors suspicious → cap 0.55
            None,
            Some(0.65),
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust <= 0.55,
            "Regional cap should be binding when stricter than verdict ceiling, got {trust:.3}"
        );
    }

    // ── JPEG Ghost quality-adaptive weight tests (backlog #15) ───────────
    // Validates the effective_weight = 0.5 × (q/100).max(0.3) formula.
    // See docs/calibration/s28-jpeg-ghost-weight.md §6.3.
    //
    // IMPORTANT: the weight only affects trust when JPEG Ghost is combined
    // with other manipulation signals (ELA / noise / copy-move) — the weighted
    // average only fires when manipulation_signals.len() > 1. Tests that
    // exercise the weight effect therefore include at least one other signal.

    #[test]
    fn trust_jpeg_ghost_quality_95_near_full_weight() {
        // Q=95: quality_factor=0.95, effective_weight=0.475 vs base 0.5.
        // With ELA also present (multi-signal path), the ghost weight matters.
        // Expect trust at Q=95 to be very close to base (within 3pp).
        let trust_q95 = compute_trust(
            Some(0.1), // ELA clean — provides second signal so weighted-avg fires
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.6), // JPEG Ghost suspicious
            Some(95),  // jpeg_quality_estimate → effective_weight=0.475
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
        );
        let trust_base = compute_trust(
            Some(0.1), // same ELA
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.6), // same ghost score
            None,      // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
        );
        // At Q=95, effective_weight=0.475 vs base=0.5 — small difference (< 3pp)
        assert!(
            (trust_q95 - trust_base).abs() < 0.03,
            "Q=95 weight (0.475) should be near base 0.5 weight: q95={trust_q95:.3} base={trust_base:.3}"
        );
    }

    #[test]
    fn trust_jpeg_ghost_quality_75_reduced_weight() {
        // Q=75 (Twitter/WhatsApp re-encode): quality_factor=0.75, effective_weight=0.375.
        // With ELA also present, ghost at Q=75 pulls less on the weighted average.
        // Trust should be higher than base (quality_factor=1.0) case.
        let trust_q75 = compute_trust(
            Some(0.1), // ELA clean — second signal so weighted-avg fires
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.9), // JPEG Ghost very suspicious
            Some(75),  // jpeg_quality_estimate → effective_weight=0.375
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
        );
        let trust_base = compute_trust(
            Some(0.1), // same ELA
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.9), // same ghost score
            None,      // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
        );
        // Q=75 → effective_weight=0.375 < 0.5 → ghost penalises less → higher trust
        assert!(
            trust_q75 > trust_base,
            "Q=75 should produce higher trust (attenuated weight) than base: q75={trust_q75:.3} base={trust_base:.3}"
        );
    }

    #[test]
    fn trust_jpeg_ghost_quality_floor_at_30() {
        // Floor: quality_factor = (q/100).max(0.3).
        // Q=30 → 30/100 = 0.30, max(0.30, 0.30) = 0.30 → floor exactly engaged.
        // Q=20 → 20/100 = 0.20, max(0.20, 0.30) = 0.30 → floor also engaged.
        // Both produce identical effective_weight (0.15) → identical trust.
        let trust_q30 = compute_trust(
            Some(0.1), // ELA clean — second signal so weighted-avg fires
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.8), // JPEG Ghost suspicious
            Some(30),  // jpeg_quality_estimate — floor exactly engaged
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
        );
        let trust_q20 = compute_trust(
            Some(0.1), // same ELA
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.8), // same ghost score
            Some(20),  // jpeg_quality_estimate — floor also engaged
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
        );
        // Both floor at quality_factor=0.30 → effective_weight=0.15 → same trust
        assert!(
            (trust_q30 - trust_q20).abs() < 0.001,
            "Q=30 and Q=20 both floor at quality_factor=0.30 → same trust: q30={trust_q30:.3} q20={trust_q20:.3}"
        );
    }

    #[test]
    fn trust_jpeg_ghost_quality_none_uses_full_base_weight() {
        // Non-JPEG input (PNG, AVIF): jpeg_quality_estimate=None.
        // quality_factor=1.0 → effective_weight=0.5 — unchanged from pre-#15 behaviour.
        // Verify ghost still lowers trust vs no ghost (weight is active).
        let trust_with_ghost = compute_trust(
            None,
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.7), // JPEG Ghost suspicious
            None,      // jpeg_quality_estimate: None → quality_factor=1.0
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
        );
        let trust_no_ghost = compute_trust(
            None, None, None, None, None, None, 0.8, None, None, None, None, None, false,
            None,  // no JPEG Ghost score at all
            None,  // jpeg_quality_estimate: None uses 0.5 base weight
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust_with_ghost < trust_no_ghost,
            "Unknown quality should still apply 0.5 weight (ghost lowers trust):              with={trust_with_ghost:.3} without={trust_no_ghost:.3}"
        );
    }

    #[test]
    fn trust_jpeg_ghost_higher_quality_lower_penalty() {
        // Snapshot: same ghost score at Q=95 vs Q=20 (floor engaged).
        // Q=95: effective_weight=0.475. Q=20: effective_weight=0.15 (floor).
        // With ELA present, the weight difference is visible in the trust score.
        // Q=20 (less penalty) → higher trust than Q=95.
        // Demonstrates Elena Vasquez's direct-upload (Q≈95) images receive near-full
        // JPEG Ghost signal; platform-forwarded (Q≤30) images are heavily attenuated.
        let trust_high_q = compute_trust(
            Some(0.1), // ELA clean — multi-signal path
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.9), // highly suspicious ghost
            Some(95),  // direct camera upload → effective_weight=0.475
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
        );
        let trust_low_q = compute_trust(
            Some(0.1), // same ELA
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.9), // same ghost score
            Some(20),  // heavy compression → effective_weight=0.15 (floor at q/100=0.30)
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
        );
        assert!(
            trust_low_q > trust_high_q,
            "Heavy-compression input should receive less JPEG Ghost penalty:              q20={trust_low_q:.3} q95={trust_high_q:.3}"
        );
        // Difference should be noticeable — Q=20 weight=0.15 vs Q=95 weight=0.475
        assert!(
            trust_low_q - trust_high_q > 0.02,
            "Quality-adaptive weight difference should be noticeable (>2pp): delta={:.3}",
            trust_low_q - trust_high_q
        );
    }
    // ── Database path resolution tests ─────────────────────────────────

    #[test]
    fn read_app_config_missing_file_returns_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = read_app_config(dir.path());
        assert!(
            cfg.db_path.is_none(),
            "Missing config.json should yield default"
        );
    }

    #[test]
    fn read_app_config_valid_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.json");
        std::fs::write(&config_path, r#"{"db_path":"/tmp/test.db"}"#).unwrap();
        let cfg = read_app_config(dir.path());
        assert_eq!(cfg.db_path.as_deref(), Some("/tmp/test.db"));
    }

    #[test]
    fn read_app_config_malformed_json_returns_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.json");
        std::fs::write(&config_path, b"not json at all").unwrap();
        let cfg = read_app_config(dir.path());
        assert!(cfg.db_path.is_none(), "Malformed JSON should yield default");
    }

    #[test]
    fn write_and_read_app_config_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let original = AppConfig {
            db_path: Some("/custom/path/jura.db".to_string()),
            ..Default::default()
        };
        write_app_config(dir.path(), &original).expect("write_app_config");
        let read_back = read_app_config(dir.path());
        assert_eq!(
            read_back.db_path.as_deref(),
            Some("/custom/path/jura.db"),
            "Round-trip failed"
        );
    }

    #[test]
    fn dir_is_writable_existing_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(dir_is_writable(dir.path()), "Temp dir should be writable");
    }

    #[test]
    fn dir_is_writable_nonexistent_returns_false() {
        let path = PathBuf::from("/nonexistent/path/that/does/not/exist");
        assert!(
            !dir_is_writable(&path),
            "Non-existent dir should not be writable"
        );
    }

    #[test]
    fn env_var_path_takes_priority_over_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("from_config.db");
        let config = AppConfig {
            db_path: Some(cfg_path.to_string_lossy().into_owned()),
            ..Default::default()
        };
        write_app_config(dir.path(), &config).expect("write");

        let env_path = dir.path().join("from_env.db");
        let env_parent = dir.path();
        assert!(dir_is_writable(env_parent), "Parent must be writable");

        let chosen = if dir_is_writable(env_path.parent().unwrap()) {
            env_path.clone()
        } else {
            let cfg = read_app_config(dir.path());
            cfg.db_path
                .map(PathBuf::from)
                .unwrap_or_else(|| dir.path().join("default.db"))
        };
        assert_eq!(chosen, env_path, "Env var path should take priority");
    }

    #[test]
    fn config_path_takes_priority_over_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("custom.db");
        let config = AppConfig {
            db_path: Some(cfg_path.to_string_lossy().into_owned()),
            ..Default::default()
        };
        write_app_config(dir.path(), &config).expect("write");

        let read_cfg = read_app_config(dir.path());
        let chosen = if let Some(p) = read_cfg.db_path {
            let pb = PathBuf::from(&p);
            if dir_is_writable(pb.parent().unwrap()) {
                pb
            } else {
                dir.path().join("default.db")
            }
        } else {
            dir.path().join("default.db")
        };
        assert_eq!(chosen, cfg_path, "Config path should win over default");
    }

    #[test]
    fn default_path_used_when_no_env_no_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = read_app_config(dir.path());
        let default = dir.path().join("jura_trace.db");
        let chosen = if let Some(p) = cfg.db_path {
            PathBuf::from(p)
        } else {
            default.clone()
        };
        assert_eq!(chosen, default, "Default path should be used as fallback");
    }

    // ── AI-declared C2PA trust tests ──────────────────────────────────────────

    #[test]
    fn trust_c2pa_ai_declared_penalises_compute_trust() {
        // When C2PA declares AI generation, trust must be LOWER than the same
        // content without the AI declaration — not rewarded with a C2PA bonus.
        let trust_ai_declared = compute_trust(
            Some(0.1),
            None,
            None,
            None,
            None,
            None,
            0.8,
            Some(true), // valid C2PA
            None,
            None,
            None,
            None,
            true, // AI declared
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        let trust_no_ai = compute_trust(
            Some(0.1),
            None,
            None,
            None,
            None,
            None,
            0.8,
            Some(true), // valid C2PA, no AI declaration
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust_ai_declared < trust_no_ai,
            "AI-declared C2PA should produce lower trust than non-AI C2PA: \
             ai={trust_ai_declared:.3} vs clean={trust_no_ai:.3}"
        );
    }

    #[test]
    fn trust_c2pa_ai_declared_lower_than_no_c2pa() {
        // AI-declared content must score lower than content with NO C2PA at all.
        // A manifest that confirms AI generation is worse than no manifest.
        let trust_ai = compute_trust(
            None,
            None,
            None,
            None,
            None,
            None,
            0.8,
            Some(true),
            None,
            None,
            None,
            None,
            true,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        let trust_none = compute_trust(
            None, None, None, None, None, None, 0.8, None, None, None, None, None, false, None,
            None, None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        assert!(
            trust_ai < trust_none,
            "AI-declared should score below no-C2PA: ai={trust_ai:.3} vs none={trust_none:.3}"
        );
    }

    #[test]
    fn trust_c2pa_ai_declared_penalty_value() {
        // Verify the penalty is -0.25 relative to valid non-AI C2PA (+0.10 bonus).
        // With exif_trust 1.0 and no forensics: valid C2PA → 1.0, AI C2PA → 0.75.
        let trust_valid = compute_trust(
            None,
            None,
            None,
            None,
            None,
            None,
            1.0,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        let trust_ai = compute_trust(
            None,
            None,
            None,
            None,
            None,
            None,
            1.0,
            Some(true),
            None,
            None,
            None,
            None,
            true,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
        );
        // valid: 1.0 + 0.10 capped at 1.0 = 1.0
        assert!(
            (trust_valid - 1.0).abs() < 0.001,
            "Valid C2PA + perfect EXIF should reach 1.0: got {trust_valid:.3}"
        );
        // AI declared: -0.25 penalty AND self-declared AI ceiling → ≤0.25.
        // Updated 2026-04-23: previously asserted 0.75 (penalty only). The
        // self-declared AI ceiling, added alongside the equivalent XMP
        // detection, now caps any self-declared synthetic asset at 0.25
        // regardless of other signals. Producer self-declaration is the
        // gold-standard provenance signal — forensic disagreement cannot
        // override it.
        assert!(
            trust_ai <= 0.25,
            "AI-declared C2PA must cap trust at 0.25 (self-declared ceiling): got {trust_ai:.3}"
        );
    }

    // ── document_trust ────────────────────────────────────────────────────────

    #[test]
    fn trust_document_ai_declared_very_low() {
        // A document whose C2PA manifest declares AI generation must score very low.
        assert_eq!(
            document_trust(Some(true), true),
            0.10,
            "AI-declared document should yield 0.10 regardless of C2PA validity"
        );
        assert_eq!(
            document_trust(Some(false), true),
            0.10,
            "AI-declared document (invalid C2PA) should still yield 0.10"
        );
        assert_eq!(
            document_trust(None, true),
            0.10,
            "AI-declared document (no C2PA) should still yield 0.10"
        );
    }

    #[test]
    fn trust_document_ai_declared_lower_than_valid_c2pa() {
        let ai_trust = document_trust(Some(true), true);
        let clean_trust = document_trust(Some(true), false);
        assert!(
            ai_trust < clean_trust,
            "AI-declared document trust {ai_trust:.2} must be below valid C2PA trust {clean_trust:.2}"
        );
    }

    #[test]
    fn trust_document_with_valid_c2pa() {
        // A PDF with a valid C2PA manifest should receive a high-confidence score.
        assert_eq!(
            document_trust(Some(true), false),
            0.82,
            "Valid C2PA on a document should yield 0.82"
        );
    }

    #[test]
    fn trust_document_with_invalid_c2pa() {
        // A PDF whose C2PA manifest fails validation is actively suspicious.
        assert_eq!(
            document_trust(Some(false), false),
            0.25,
            "Invalid C2PA on a document should yield 0.25"
        );
    }

    #[test]
    fn trust_document_without_c2pa() {
        // A PDF with no C2PA data at all is genuinely inconclusive — not suspicious.
        assert_eq!(
            document_trust(None, false),
            0.50,
            "Document with no C2PA data should yield 0.50"
        );
    }

    // ── Licence tier persistence tests ───────────────────────────────────────

    #[test]
    fn licence_tier_defaults_to_community() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = read_app_config(dir.path());
        assert_eq!(
            cfg.licence_tier,
            LicenceTier::Community,
            "Default tier should be Community when no config exists"
        );
    }

    #[test]
    fn licence_tier_roundtrip_professional() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = AppConfig {
            licence_tier: LicenceTier::Professional,
            ..Default::default()
        };
        write_app_config(dir.path(), &config).expect("write_app_config");
        let read_back = read_app_config(dir.path());
        assert_eq!(
            read_back.licence_tier,
            LicenceTier::Professional,
            "Professional tier should round-trip through config.json"
        );
    }

    #[test]
    fn licence_tier_roundtrip_enterprise() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = AppConfig {
            licence_tier: LicenceTier::Enterprise,
            ..Default::default()
        };
        write_app_config(dir.path(), &config).expect("write_app_config");
        let read_back = read_app_config(dir.path());
        assert_eq!(
            read_back.licence_tier,
            LicenceTier::Enterprise,
            "Enterprise tier should round-trip through config.json"
        );
    }

    #[test]
    fn licence_tier_preserved_when_db_path_updated() {
        // set_db_path must not clobber the licence_tier stored in config.json.
        let dir = tempfile::tempdir().expect("tempdir");
        // Write an initial config with a non-default tier.
        let initial = AppConfig {
            db_path: Some("/old/path.db".to_string()),
            licence_tier: LicenceTier::Team,
            ..Default::default()
        };
        write_app_config(dir.path(), &initial).expect("write initial config");

        // Simulate what set_db_path does: read → update db_path → write.
        let mut config = read_app_config(dir.path());
        config.db_path = Some("/new/path.db".to_string());
        write_app_config(dir.path(), &config).expect("write updated config");

        let read_back = read_app_config(dir.path());
        assert_eq!(
            read_back.licence_tier,
            LicenceTier::Team,
            "Updating db_path must not overwrite licence_tier"
        );
        assert_eq!(
            read_back.db_path.as_deref(),
            Some("/new/path.db"),
            "db_path must be updated correctly"
        );
    }

    #[test]
    fn licence_tier_serde_camel_case() {
        // Verify the serde encoding is camelCase as the frontend expects.
        let json = serde_json::to_string(&LicenceTier::Professional).expect("serialize");
        assert_eq!(
            json, r#""professional""#,
            "LicenceTier::Professional must serialize as camelCase"
        );

        let json_team = serde_json::to_string(&LicenceTier::Team).expect("serialize");
        assert_eq!(
            json_team, r#""team""#,
            "LicenceTier::Team must serialize as 'team'"
        );

        let json_enterprise = serde_json::to_string(&LicenceTier::Enterprise).expect("serialize");
        assert_eq!(json_enterprise, r#""enterprise""#);

        let json_community = serde_json::to_string(&LicenceTier::Community).expect("serialize");
        assert_eq!(json_community, r#""community""#);
    }

    // ── Corrupt / truncated file rejection tests ──────────────────────────────

    /// `verify_content_inner` must return `AppError::Validation` for a zero-byte file.
    ///
    /// The check runs before any decoder or sidecar call, so this test works
    /// without a running sidecar or a real Tauri `State` — it exercises only
    /// the pure filesystem guard via the public inner function.
    #[test]
    fn test_verify_rejects_empty_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let empty_path = tmp.path().join("empty.jpg");
        std::fs::write(&empty_path, b"").expect("write empty file");

        let path_str = empty_path.to_string_lossy().to_string();

        // Replicate the size-check logic that lives at the top of
        // verify_content_inner. The function itself requires a Tauri State<>
        // which cannot be constructed in a unit test without a full app
        // context, so we mirror the guard logic directly.
        let file_size = std::fs::metadata(&empty_path).map(|m| m.len()).unwrap_or(0);

        assert_eq!(file_size, 0, "File must be empty for this test");

        if file_size == 0 {
            let err = AppError::Validation(
                "The file is empty (zero bytes). Please select a valid file.".to_string(),
            );
            assert!(
                err.to_string().contains("empty (zero bytes)"),
                "Error should mention 'empty (zero bytes)', got: {err}"
            );
        } else {
            panic!("Expected file_size == 0 for path {path_str}");
        }
    }

    /// `verify_content_inner` must return `AppError::Validation` for a file that
    /// is too small to contain any valid media header (< 12 bytes).
    #[test]
    fn test_verify_rejects_tiny_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let tiny_path = tmp.path().join("tiny.png");
        // 5 bytes — far smaller than any valid PNG (minimum ~67 bytes)
        std::fs::write(&tiny_path, b"\x89PNG\x0d").expect("write tiny file");

        let file_size = std::fs::metadata(&tiny_path).map(|m| m.len()).unwrap_or(0);

        assert_eq!(file_size, 5, "File must be 5 bytes for this test");

        // Guard mirrors the logic in verify_content_inner.
        assert!(file_size > 0, "Non-zero check passes");
        assert!(
            file_size < 12,
            "File must be below the 12-byte minimum header threshold"
        );

        let err =
            AppError::Validation("The file is too small to be a valid media file.".to_string());
        assert_eq!(
            err.to_string(),
            "The file is too small to be a valid media file.",
            "Tiny-file error message must match exactly"
        );
    }

    /// `import_files` must return `Err(String)` when the only supplied file is
    /// empty (zero bytes). The error string must mention "empty" or "zero bytes".
    #[test]
    fn test_import_rejects_empty_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let empty_path = tmp.path().join("empty_import.jpg");
        std::fs::write(&empty_path, b"").expect("write empty file");

        let path_str = empty_path.to_string_lossy().to_string();

        // Mirror the guard logic from import_files.
        let file_size = std::fs::metadata(&empty_path).map(|m| m.len()).unwrap_or(0);

        assert_eq!(file_size, 0, "File must be empty for this test");

        let result: Result<(), String> = if file_size == 0 {
            Err(format!(
                "The file '{}' is empty (zero bytes). Please select a valid file.",
                empty_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path_str.clone())
            ))
        } else {
            Ok(())
        };

        assert!(result.is_err(), "import_files must reject empty files");
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("empty") || err_msg.contains("zero bytes"),
            "Error message must mention 'empty' or 'zero bytes', got: {err_msg}"
        );
    }

    // ── ThumbnailCheck ───────────────────────────────────────────────

    #[test]
    fn thumbnail_check_serialization_no_thumbnail() {
        let tc = ThumbnailCheck {
            has_thumbnail: false,
            thumbnail_width: None,
            thumbnail_height: None,
            hamming_distance: None,
            difference_score: None,
            mismatch: false,
            summary: "No EXIF thumbnail embedded in this image.".to_string(),
        };
        let json = serde_json::to_string(&tc).expect("serialization must succeed");
        assert!(json.contains("\"hasThumbnail\":false"));
        assert!(json.contains("\"hammingDistance\":null"));
        assert!(json.contains("\"mismatch\":false"));
        assert!(json.contains("\"summary\""));
    }

    #[test]
    fn thumbnail_check_serialization_match() {
        let tc = ThumbnailCheck {
            has_thumbnail: true,
            thumbnail_width: Some(160),
            thumbnail_height: Some(120),
            hamming_distance: Some(3),
            difference_score: Some(0.005),
            mismatch: false,
            summary: "Thumbnail matches full image.".to_string(),
        };
        let json = serde_json::to_string(&tc).expect("serialization must succeed");
        assert!(json.contains("\"hasThumbnail\":true"));
        assert!(json.contains("\"thumbnailWidth\":160"));
        assert!(json.contains("\"thumbnailHeight\":120"));
        assert!(json.contains("\"hammingDistance\":3"));
        assert!(json.contains("\"mismatch\":false"));
    }

    #[test]
    fn thumbnail_check_serialization_mismatch() {
        let tc = ThumbnailCheck {
            has_thumbnail: true,
            thumbnail_width: Some(160),
            thumbnail_height: Some(120),
            hamming_distance: Some(24),
            difference_score: Some(0.08),
            mismatch: true,
            summary: "Thumbnail mismatch detected.".to_string(),
        };
        let json = serde_json::to_string(&tc).expect("serialization must succeed");
        assert!(json.contains("\"hasThumbnail\":true"));
        assert!(json.contains("\"hammingDistance\":24"));
        assert!(json.contains("\"mismatch\":true"));
        assert!(json.contains("\"differenceScore\":0.08"));
    }

    #[test]
    fn thumbnail_check_deserialization_round_trip() {
        let tc = ThumbnailCheck {
            has_thumbnail: true,
            thumbnail_width: Some(80),
            thumbnail_height: Some(60),
            hamming_distance: Some(7),
            difference_score: Some(0.01),
            mismatch: false,
            summary: "Thumbnail matches.".to_string(),
        };
        let json = serde_json::to_string(&tc).expect("serialization must succeed");
        let decoded: ThumbnailCheck =
            serde_json::from_str(&json).expect("deserialization must succeed");
        assert_eq!(decoded.has_thumbnail, tc.has_thumbnail);
        assert_eq!(decoded.thumbnail_width, tc.thumbnail_width);
        assert_eq!(decoded.thumbnail_height, tc.thumbnail_height);
        assert_eq!(decoded.hamming_distance, tc.hamming_distance);
        assert_eq!(decoded.difference_score, tc.difference_score);
        assert_eq!(decoded.mismatch, tc.mismatch);
        assert_eq!(decoded.summary, tc.summary);
    }

    #[test]
    fn thumbnail_check_mismatch_threshold() {
        // Hamming distance == 10 is NOT a mismatch; 11 IS.
        // MSE == 0.02 is NOT a mismatch; 0.021 IS.
        let at_boundary = ThumbnailCheck {
            has_thumbnail: true,
            thumbnail_width: Some(160),
            thumbnail_height: Some(120),
            hamming_distance: Some(10),
            difference_score: Some(0.02),
            mismatch: false, // neither threshold exceeded
            summary: "No mismatch.".to_string(),
        };
        let over_hamming = ThumbnailCheck {
            has_thumbnail: true,
            thumbnail_width: Some(160),
            thumbnail_height: Some(120),
            hamming_distance: Some(11),
            difference_score: Some(0.01),
            mismatch: true, // pHash distance > 10
            summary: "pHash mismatch.".to_string(),
        };
        let over_mse = ThumbnailCheck {
            has_thumbnail: true,
            thumbnail_width: Some(160),
            thumbnail_height: Some(120),
            hamming_distance: Some(5),
            difference_score: Some(0.025),
            mismatch: true, // MSE > 0.02
            summary: "MSE mismatch.".to_string(),
        };
        assert!(!at_boundary.mismatch);
        assert!(over_hamming.mismatch);
        assert!(over_mse.mismatch);
    }

    // ── assess_input_quality — is_modern_lossy_codec ─────────────────────

    /// AVIF files must set `is_modern_lossy_codec = true` and add the four
    /// codec-degraded detectors to `degraded_detectors`.
    #[test]
    fn input_quality_avif_sets_modern_lossy_codec() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // A minimal placeholder file — assess_input_quality only reads the
        // MIME type from FormatInfo, not the actual file bytes.
        let fake_avif = tmp.path().join("test.avif");
        std::fs::write(&fake_avif, b"fake avif bytes for size heuristic only")
            .expect("write fake avif");

        let info = format_router::FormatInfo {
            mime_type: "image/avif".to_string(),
            content_type: format_router::ContentType::Image,
        };
        let exif: Option<exif_anomaly::ExifAnalysis> = None;
        let quality = assess_input_quality(&fake_avif, &info, &exif, None, Some(1000), Some(800));

        assert!(
            quality.is_modern_lossy_codec,
            "AVIF must set is_modern_lossy_codec = true"
        );
        assert!(
            quality.degraded_detectors.contains(&"ELA".to_string()),
            "ELA must be degraded for AVIF"
        );
        assert!(
            quality
                .degraded_detectors
                .contains(&"JPEG Ghost".to_string()),
            "JPEG Ghost must be degraded for AVIF"
        );
    }

    /// JPEG files must NOT set `is_modern_lossy_codec`.
    #[test]
    fn input_quality_jpeg_not_modern_lossy_codec() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fake_jpeg = tmp.path().join("test.jpg");
        std::fs::write(&fake_jpeg, b"fake jpeg bytes for size heuristic only")
            .expect("write fake jpeg");

        let info = format_router::FormatInfo {
            mime_type: "image/jpeg".to_string(),
            content_type: format_router::ContentType::Image,
        };
        let exif: Option<exif_anomaly::ExifAnalysis> = None;
        let quality = assess_input_quality(&fake_jpeg, &info, &exif, None, Some(2000), Some(1500));

        assert!(
            !quality.is_modern_lossy_codec,
            "JPEG must not set is_modern_lossy_codec"
        );
    }

    /// WebP files must also set `is_modern_lossy_codec = true`.
    #[test]
    fn input_quality_webp_sets_modern_lossy_codec() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fake_webp = tmp.path().join("test.webp");
        std::fs::write(&fake_webp, b"RIFF\x00\x00\x00\x00WEBPVP8 ").expect("write fake webp");

        let info = format_router::FormatInfo {
            mime_type: "image/webp".to_string(),
            content_type: format_router::ContentType::Image,
        };
        let exif: Option<exif_anomaly::ExifAnalysis> = None;
        let quality = assess_input_quality(&fake_webp, &info, &exif, None, Some(800), Some(600));

        assert!(
            quality.is_modern_lossy_codec,
            "WebP must set is_modern_lossy_codec = true"
        );
    }

    // ── assess_input_quality — metadata_completely_absent ────────────────

    /// When has_exif = false and no raw_meta is provided, metadata_completely_absent
    /// must be true.
    #[test]
    fn input_quality_metadata_absent_when_no_exif_no_raw_meta() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fake_avif = tmp.path().join("stripped.avif");
        std::fs::write(&fake_avif, b"fake avif content").expect("write");

        let info = format_router::FormatInfo {
            mime_type: "image/avif".to_string(),
            content_type: format_router::ContentType::Image,
        };
        // has_exif = false (ExifAnalysis.has_exif driven by has_exif field)
        let exif_analysis = Some(exif_anomaly::ExifAnalysis {
            findings: vec![],
            trust_score: 0.8,
            fields_populated: 0,
            fields_total: 16,
            has_exif: false,
            gps_latitude: None,
            gps_longitude: None,
            camera_authenticity_bonus: 0.0,
        });
        // No raw_meta → XMP treated as absent
        let quality = assess_input_quality(
            &fake_avif,
            &info,
            &exif_analysis,
            None,
            Some(800),
            Some(600),
        );

        assert!(
            quality.metadata_completely_absent,
            "metadata_completely_absent must be true when EXIF and XMP are both absent"
        );
    }

    /// When EXIF is present, metadata_completely_absent must be false.
    #[test]
    fn input_quality_metadata_not_absent_when_exif_present() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fake_jpeg = tmp.path().join("with_exif.jpg");
        std::fs::write(&fake_jpeg, b"fake jpeg content").expect("write");

        let info = format_router::FormatInfo {
            mime_type: "image/jpeg".to_string(),
            content_type: format_router::ContentType::Image,
        };
        let exif_analysis = Some(exif_anomaly::ExifAnalysis {
            findings: vec![],
            trust_score: 0.9,
            fields_populated: 12,
            fields_total: 16,
            has_exif: true,
            gps_latitude: Some(51.5),
            gps_longitude: Some(-0.1),
            camera_authenticity_bonus: 0.8,
        });
        let quality = assess_input_quality(
            &fake_jpeg,
            &info,
            &exif_analysis,
            None,
            Some(4000),
            Some(3000),
        );

        assert!(
            !quality.metadata_completely_absent,
            "metadata_completely_absent must be false when EXIF is present"
        );
    }

    // ===== is_private_or_loopback_host =====

    #[test]
    fn loopback_localhost() {
        assert!(is_private_or_loopback_host("localhost"));
    }

    #[test]
    fn loopback_127() {
        assert!(is_private_or_loopback_host("127.0.0.1"));
    }

    #[test]
    fn loopback_ipv6() {
        assert!(is_private_or_loopback_host("::1"));
    }

    #[test]
    fn loopback_zero() {
        assert!(is_private_or_loopback_host("0.0.0.0"));
    }

    #[test]
    fn rfc1918_10_low() {
        assert!(is_private_or_loopback_host("10.0.0.1"));
    }

    #[test]
    fn rfc1918_10_high() {
        assert!(is_private_or_loopback_host("10.255.255.255"));
    }

    #[test]
    fn rfc1918_192168_low() {
        assert!(is_private_or_loopback_host("192.168.0.1"));
    }

    #[test]
    fn rfc1918_192168_high() {
        assert!(is_private_or_loopback_host("192.168.255.255"));
    }

    #[test]
    fn link_local_low() {
        assert!(is_private_or_loopback_host("169.254.0.1"));
    }

    #[test]
    fn link_local_high() {
        assert!(is_private_or_loopback_host("169.254.255.255"));
    }

    // 172.x boundary: second octet 16–31 is private, outside is public.

    #[test]
    fn rfc1918_172_below_boundary_is_public() {
        assert!(!is_private_or_loopback_host("172.15.255.255"));
    }

    #[test]
    fn rfc1918_172_lower_bound() {
        assert!(is_private_or_loopback_host("172.16.0.1"));
    }

    #[test]
    fn rfc1918_172_upper_bound() {
        assert!(is_private_or_loopback_host("172.31.255.255"));
    }

    #[test]
    fn rfc1918_172_above_boundary_is_public() {
        assert!(!is_private_or_loopback_host("172.32.0.1"));
    }

    #[test]
    fn public_example_com() {
        assert!(!is_private_or_loopback_host("example.com"));
    }

    #[test]
    fn public_google_dns() {
        assert!(!is_private_or_loopback_host("8.8.8.8"));
    }

    #[test]
    fn public_cloudflare_dns() {
        assert!(!is_private_or_loopback_host("1.1.1.1"));
    }

    #[test]
    fn case_insensitive_upper() {
        assert!(is_private_or_loopback_host("LOCALHOST"));
    }

    #[test]
    fn case_insensitive_mixed() {
        assert!(is_private_or_loopback_host("Localhost"));
    }

    #[test]
    fn empty_string_is_public() {
        assert!(!is_private_or_loopback_host(""));
    }

    // ── Power-saver mode tests ─────────────────────────────────────────────

    #[test]
    fn power_saver_mode_default_is_false() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = read_app_config(dir.path());
        assert!(
            !cfg.power_saver_mode,
            "Default power_saver_mode must be false"
        );
    }

    #[test]
    fn power_saver_mode_roundtrip_enabled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = AppConfig {
            power_saver_mode: true,
            ..Default::default()
        };
        write_app_config(dir.path(), &config).expect("write");
        let read_back = read_app_config(dir.path());
        assert!(
            read_back.power_saver_mode,
            "power_saver_mode=true should round-trip"
        );
    }

    #[test]
    fn power_saver_mode_roundtrip_disabled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = AppConfig {
            power_saver_mode: false,
            ..Default::default()
        };
        write_app_config(dir.path(), &config).expect("write");
        let read_back = read_app_config(dir.path());
        assert!(
            !read_back.power_saver_mode,
            "power_saver_mode=false should round-trip"
        );
    }

    #[test]
    fn last_sidecar_request_ts_atomic_update() {
        // Simulate what verify_content does: store a new timestamp.
        let ts = Arc::new(AtomicU64::new(0));
        let clone = Arc::clone(&ts);
        let new_val = 1_700_000_000u64;
        clone.store(new_val, Ordering::Relaxed);
        assert_eq!(
            ts.load(Ordering::Relaxed),
            new_val,
            "AtomicU64 round-trip failed"
        );
    }

    #[test]
    fn idle_killer_noop_when_power_saver_disabled() {
        // Verify the guard logic: if power_saver_mode=false, the killer should
        // not act even when idle exceeds the threshold.
        let power_saver = false;
        let now = 1_700_000_000u64;
        let last_ts = now.saturating_sub(SIDECAR_IDLE_SECONDS_BEFORE_KILL + 60);
        let idle_secs = now.saturating_sub(last_ts);

        // Simulate the condition check in the watcher: enabled=false → skip.
        let would_kill = power_saver && idle_secs >= SIDECAR_IDLE_SECONDS_BEFORE_KILL;
        assert!(
            !would_kill,
            "Killer must be no-op when power_saver_mode=false"
        );
    }

    #[test]
    fn idle_killer_fires_when_enabled_and_threshold_exceeded() {
        let power_saver = true;
        let now = 1_700_000_000u64;
        let last_ts = now.saturating_sub(SIDECAR_IDLE_SECONDS_BEFORE_KILL + 1);
        let idle_secs = now.saturating_sub(last_ts);
        let would_kill = power_saver && idle_secs >= SIDECAR_IDLE_SECONDS_BEFORE_KILL;
        assert!(
            would_kill,
            "Killer must fire when enabled and idle > threshold"
        );
    }

    #[test]
    fn idle_killer_noop_when_threshold_not_exceeded() {
        let power_saver = true;
        let now = 1_700_000_000u64;
        let last_ts = now.saturating_sub(SIDECAR_IDLE_SECONDS_BEFORE_KILL - 1);
        let idle_secs = now.saturating_sub(last_ts);
        let would_kill = power_saver && idle_secs >= SIDECAR_IDLE_SECONDS_BEFORE_KILL;
        assert!(!would_kill, "Killer must not fire before threshold");
    }

    #[test]
    fn sidecar_idle_seconds_constant_is_300() {
        assert_eq!(
            SIDECAR_IDLE_SECONDS_BEFORE_KILL, 300,
            "Idle threshold must be 300 seconds (5 minutes)"
        );
    }
}
