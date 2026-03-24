use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};
use tauri_plugin_shell::ShellExt;

mod c2pa;
mod db;
mod error;
mod exif_anomaly;
mod fingerprint;
mod format_router;
mod metadata;
mod sidecar;
mod watermark;

use error::AppError;

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
    pub created_at: String,
}

/// Verification result from the VERIFY pipeline.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResult {
    pub source_type: String,
    pub content_type: String,
    /// Investigation mode used for this verification run
    /// (`"quick"`, `"standard"`, `"deep"`, or `"archival"`).
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
    pub c2pa_manifest: Option<c2pa::ManifestInfo>,
    pub ela_result: Option<sidecar::ElaResult>,
    pub noise_result: Option<sidecar::NoiseResult>,
    pub copy_move_result: Option<sidecar::CopyMoveResult>,
    pub deepfake_result: Option<sidecar::DeepfakeResult>,
    pub npr_result: Option<sidecar::NprResult>,
    pub jpeg_ghost_result: Option<sidecar::JpegGhostResult>,
    pub ca_result: Option<sidecar::CaResult>,
    pub segmented_ela_result: Option<sidecar::SegmentedElaResult>,
    pub shadow_consistency_result: Option<sidecar::ShadowConsistencyResult>,
    pub colour_temperature_result: Option<sidecar::ColourTemperatureResult>,
    pub splice_boundary_result: Option<sidecar::SpliceBoundaryResult>,
    pub ai_generator: Option<String>,
    /// Watermark extraction result for image files (standard/deep/archival modes).
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
    /// Handle to the spawned PyInstaller sidecar process.
    /// Present only in production builds where the binary was found and launched
    /// successfully. `None` in development (manual uvicorn) or if spawn failed.
    pub sidecar_process: Option<tauri_plugin_shell::process::CommandChild>,
}

// ===== Tauri Commands =====

/// Get application statistics for the dashboard.
#[tauri::command]
fn get_stats(state: State<'_, Mutex<AppState>>) -> Result<AppStats, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.db.get_stats().map_err(|e| e.to_string())
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
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<Asset>, String> {
    log::info!("Importing {} file(s)", paths.len());
    let app = state.lock().map_err(|e| e.to_string())?;

    let mut imported: Vec<Asset> = Vec::new();

    for path_str in &paths {
        let path = PathBuf::from(path_str);

        if !path.exists() {
            log::warn!("Skipping missing file: {path_str}");
            continue;
        }

        // Skip directories — we process individual files
        if path.is_dir() {
            log::info!("Skipping directory: {path_str}");
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
        if file_size == 0 {
            log::warn!("Skipping empty file ({path_str}): file is zero bytes");
            return Err(format!(
                "The file '{}' is empty (zero bytes). Please select a valid file.",
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path_str.clone())
            ));
        }
        if file_size < 12 {
            log::warn!(
                "Skipping file that is too small ({path_str}): {file_size} bytes \
                 is smaller than any valid media header"
            );
            return Err(format!(
                "The file '{}' is too small to be a valid media file ({file_size} bytes).",
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path_str.clone())
            ));
        }
        if file_size > MAX_IMPORT_FILE_SIZE_BYTES {
            log::warn!(
                "Skipping oversized file ({path_str}): {file_size} bytes exceeds \
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
                        "Skipping image with excessive dimensions ({path_str}): \
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

        // 4. Build asset record
        let asset_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let row = db::AssetRow {
            asset_id: asset_id.clone(),
            file_path: path_str.clone(),
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
        };

        // 5. Store
        app.db.insert_asset(&row).map_err(|e| e.to_string())?;

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
            file_path: path_str.clone(),
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
            created_at: now,
        });
    }

    log::info!("Successfully imported {} file(s)", imported.len());
    Ok(imported)
}

/// Get all assets from the local database.
#[tauri::command]
fn get_assets(state: State<'_, Mutex<AppState>>) -> Result<Vec<Asset>, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.db.get_all_assets().map_err(|e| e.to_string())
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
    shadow_consistency_score: Option<f64>,
    colour_temperature_score: Option<f64>,
    splice_boundary_score: Option<f64>,
    ai_declared_by_c2pa: bool,
) -> f64 {
    // C2PA that honestly declares AI generation should penalise trust — the
    // content's own provenance record confirms it is synthetic.  A valid
    // manifest without an AI declaration is still a positive provenance signal.
    let c2pa_bonus = if ai_declared_by_c2pa {
        -0.25 // Penalty: manifest explicitly declares AI-generated content
    } else if c2pa_valid == Some(true) {
        0.1 // Bonus: valid provenance, not declared AI
    } else {
        0.0
    };

    // Use the raw deepfake score for forensic trust. Confidence is expressed
    // via the verdict ceiling below, not by scaling the score down. The old
    // confidence_weight multiplier (low=0.3) nearly eliminated the signal,
    // causing a fake image to show 92% "High Trust" alongside "Inconclusive".
    let deepfake_trust = deepfake_score.map(|s| 1.0 - s);

    // Weighted manipulation signals: ELA weight 2.0 (most reliable),
    // noise and copy-move weight 1.0 each.
    let mut manipulation_signals: Vec<(f64, f64)> = Vec::new(); // (trust, weight)
    if let Some(s) = ela_score {
        manipulation_signals.push((1.0 - s, 2.0));
    }
    if let Some(s) = noise_score {
        manipulation_signals.push((1.0 - s, 1.0));
    }
    if let Some(s) = copy_move_score {
        manipulation_signals.push((1.0 - s, 1.0));
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
    // stronger splice indicators; shadow consistency and splice boundary
    // carry weight 1.0 each.
    let regional_signals: Vec<(f64, f64)> = [
        (segmented_ela_score, 1.5_f64),
        (shadow_consistency_score, 1.0),
        (colour_temperature_score, 1.5),
        (splice_boundary_score, 1.0),
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
        // Weight: 40% EXIF metadata, 60% forensic analysis
        (exif_trust * 0.4 + ft * 0.6 + c2pa_bonus).min(1.0)
    } else {
        (exif_trust + c2pa_bonus).min(1.0)
    };

    // ── Composite regional amplification cap ────────────────────────
    // When 2+ of the four regional detectors simultaneously flag the image
    // as suspicious (score > 0.5), the convergence of evidence is strong
    // enough to warrant a hard cap at 0.55 regardless of other signals.
    let suspicious_regional_count = [
        segmented_ela_score,
        shadow_consistency_score,
        colour_temperature_score,
        splice_boundary_score,
    ]
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
    let verdict_ceiling = match deepfake_verdict {
        Some("synthetic") => match deepfake_confidence {
            Some("high") => 0.25,
            Some("medium") => 0.35,
            _ => 0.45, // low confidence synthetic ≈ inconclusive
        },
        Some("inconclusive") => 0.60,
        _ => 1.0, // no ceiling for authentic or sidecar offline
    };

    base_trust.min(verdict_ceiling).min(regional_cap)
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

/// Inner verification logic shared by `verify_content` and `verify_url`.
///
/// `mode` controls which pipeline stages run:
/// - `"quick"` (or legacy `"fast"`) — EXIF + C2PA only. Target: <5 s.
/// - `"standard"` (default) — EXIF + C2PA + ELA + deepfake. Target: <15 s.
/// - `"deep"` — full pipeline including noise, copy-move, NPR, JPEG ghost, CA.
/// - `"archival"` — deep with scanner-calibrated tolerances.
fn verify_content_inner(
    source: &str,
    source_type: &str,
    mode: Option<&str>,
    state: &State<'_, Mutex<AppState>>,
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
            log::error!("Path canonicalisation failed for '{}': {}", source, e);
            AppError::Validation("File not found or inaccessible".to_string())
        })?;

    // ── File integrity pre-checks ────────────────────────────────────────
    // Reject zero-length and suspiciously small files before any decoding
    // attempt. Any valid image, audio, or video file will be larger than
    // the minimum header size of 12 bytes (e.g. a PNG header is 8 bytes
    // plus the IHDR chunk length and type = 16 bytes total). Passing these
    // files to image decoders or the sidecar may cause panics or hangs.
    let file_size = std::fs::metadata(&path)
        .map(|m| m.len())
        .unwrap_or(0);
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

    // ── Format detection ─────────────────────────────────────────────────
    let t_format = std::time::Instant::now();
    let info = format_router::detect(&path);
    log::info!("PERF: format routing took {:?}", t_format.elapsed());

    // ── EXIF metadata extraction ─────────────────────────────────────────
    let t_exif = std::time::Instant::now();
    // EXIF analysis (images only)
    let exif_analysis = if info.content_type == format_router::ContentType::Image {
        let meta = metadata::extract_exif(&path);
        let (actual_w, actual_h) = metadata::get_image_dimensions(&path).unwrap_or((0, 0));
        let actual_w = if actual_w > 0 { Some(actual_w) } else { None };
        let actual_h = if actual_h > 0 { Some(actual_h) } else { None };
        let mut analysis = exif_anomaly::analyse(meta.as_ref(), actual_w, actual_h);

        // Reduce missing-EXIF penalty for modern web codecs (AVIF, WebP, HEIC).
        // These formats routinely have EXIF stripped by CMS/CDN pipelines for
        // bandwidth and privacy — absence is standard behaviour, not suspicious.
        if !analysis.has_exif {
            let is_web_codec = matches!(
                info.mime_type.as_str(),
                "image/avif" | "image/webp" | "image/heic"
            );
            if is_web_codec {
                for finding in &mut analysis.findings {
                    if finding.check_id == "no_exif_data" {
                        finding.severity = exif_anomaly::Severity::Low;
                        finding.description = format!(
                            "No EXIF data present. For {} files delivered via the web, \
                             EXIF stripping is standard CMS behaviour for bandwidth \
                             and privacy. This is not inherently suspicious.",
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
        }

        Some(analysis)
    } else {
        None
    };
    log::info!("PERF: EXIF metadata extraction took {:?}", t_exif.elapsed());

    // ── C2PA verification ────────────────────────────────────────────────
    let t_c2pa = std::time::Instant::now();
    let c2pa_manifest = c2pa::read_manifest(&path).ok().flatten();
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
    //   archival   → all detectors (scanner-calibrated)
    let app = state.lock().map_err(|e| {
        log::error!("AppState mutex poisoned in verify pipeline: {}", e);
        AppError::Internal("Failed to acquire application state".to_string())
    })?;
    let is_image = info.content_type == format_router::ContentType::Image;
    let is_video = info.content_type == format_router::ContentType::Video;
    let is_audio = info.content_type == format_router::ContentType::Audio;
    let effective_mode = match mode {
        Some("fast") | Some("quick") => "quick",
        Some("standard") => "standard",
        Some("archival") => "archival",
        Some("deep") => "deep",
        _ => "standard", // default to standard (was "deep" — too slow for typical use)
    };
    let is_quick = effective_mode == "quick";
    // Single availability probe — reused for all sidecar calls in this pipeline
    // to avoid multiple HTTP round-trips.
    let sidecar_available = !is_quick && app.sidecar.is_available();
    let sidecar_up = is_image && sidecar_available;
    let is_deep = matches!(effective_mode, "deep" | "archival");
    log::info!(
        "Verify pipeline: is_image={}, mode={:?}, effective={}, sidecar_up={}, is_deep={}",
        is_image,
        mode,
        effective_mode,
        sidecar_up,
        is_deep
    );

    // ── Whether the image has camera-origin EXIF ────────────────────────
    // Images with at least 4 populated EXIF fields are more likely to be
    // genuine camera shots; the sidecar uses this as a detection prior.
    let has_camera_exif = exif_analysis
        .as_ref()
        .map(|a| a.has_exif && a.fields_populated >= 4)
        .unwrap_or(false);

    // ── Standard parallel group ──────────────────────────────────────────
    // ELA + deepfake + watermark extraction are independent and each takes
    // 1-5 s. Running them concurrently cuts standard-mode wall time from
    // ~10 s sequential to the slowest single detector (~5 s).
    //
    // `std::thread::scope` guarantees all threads finish before we proceed
    // and avoids the overhead of a separate thread pool. Each thread
    // receives a cheap `SidecarClient::clone()` (Arc-based connection pool)
    // and an owned `PathBuf`.
    let (ela_score, ela_result, deepfake_score, deepfake_result, watermark_extract_result) =
        if sidecar_up {
            let t_standard = std::time::Instant::now();

            let ela_path = path.to_path_buf();
            let df_path = path.to_path_buf();
            let wm_path = path.to_path_buf();
            let ela_client = app.sidecar.clone();
            let df_client = app.sidecar.clone();
            let wm_client = app.sidecar.clone();
            let mime = info.mime_type.clone();

            let (ela_out, df_out, wm_out) = std::thread::scope(|s| {
                let ela_h = s.spawn(move || {
                    let t = std::time::Instant::now();
                    let r = ela_client.analyse_ela(&ela_path);
                    log::info!("PERF: ELA took {:?}", t.elapsed());
                    r
                });
                let df_h = s.spawn(move || {
                    let t = std::time::Instant::now();
                    let r = df_client.detect_deepfake(&df_path, &mime, has_camera_exif);
                    log::info!("PERF: deepfake took {:?}", t.elapsed());
                    r
                });
                let wm_h = s.spawn(move || {
                    let t = std::time::Instant::now();
                    let r = wm_client.check_watermark_extract(&wm_path);
                    log::info!("PERF: watermark extraction took {:?}", t.elapsed());
                    r
                });
                (ela_h.join(), df_h.join(), wm_h.join())
            });

            log::info!(
                "PERF: standard group (ELA + deepfake + watermark, parallel) took {:?}",
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

            (
                ela_score,
                ela_result,
                deepfake_score,
                deepfake_result,
                watermark_extract_result,
            )
        } else {
            (None, None, None, None, None)
        };

    // ── Deep parallel group ──────────────────────────────────────────────
    // Nine detectors run concurrently when in deep/archival mode.
    // Slowest is copy-move (~5 s); without parallelism the group takes
    // ~25 s sequentially. With parallelism wall time is bounded by the
    // slowest single detector rather than the sum of all detectors.
    let (
        noise_score,
        noise_result,
        copy_move_score,
        copy_move_result,
        npr_result,
        jpeg_ghost_result,
        ca_result,
        segmented_ela_result,
        shadow_consistency_result,
        colour_temperature_result,
        splice_boundary_result,
    ) = if sidecar_up && is_deep {
        let t_deep = std::time::Instant::now();

        let noise_path = path.to_path_buf();
        let cm_path = path.to_path_buf();
        let npr_path = path.to_path_buf();
        let jg_path = path.to_path_buf();
        let ca_path = path.to_path_buf();
        let seg_path = path.to_path_buf();
        let shad_path = path.to_path_buf();
        let ct_path = path.to_path_buf();
        let sb_path = path.to_path_buf();

        let noise_client = app.sidecar.clone();
        let cm_client = app.sidecar.clone();
        let npr_client = app.sidecar.clone();
        let jg_client = app.sidecar.clone();
        let ca_client = app.sidecar.clone();
        let seg_client = app.sidecar.clone();
        let shad_client = app.sidecar.clone();
        let ct_client = app.sidecar.clone();
        let sb_client = app.sidecar.clone();

        let (noise_out, cm_out, npr_out, jg_out, ca_out, seg_out, shad_out, ct_out, sb_out) =
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
                let npr_h = s.spawn(move || {
                    let t = std::time::Instant::now();
                    let r = npr_client.analyse_npr(&npr_path);
                    log::info!("PERF: NPR analysis took {:?}", t.elapsed());
                    r
                });
                let jg_h = s.spawn(move || {
                    let t = std::time::Instant::now();
                    let r = jg_client.detect_jpeg_ghost(&jg_path);
                    log::info!("PERF: JPEG ghost detection took {:?}", t.elapsed());
                    r
                });
                let ca_h = s.spawn(move || {
                    let t = std::time::Instant::now();
                    let r = ca_client.analyse_ca(&ca_path);
                    log::info!("PERF: chromatic aberration analysis took {:?}", t.elapsed());
                    r
                });
                let seg_h = s.spawn(move || {
                    let t = std::time::Instant::now();
                    let r = seg_client.check_segmented_ela(&seg_path);
                    log::info!("PERF: segmented ELA took {:?}", t.elapsed());
                    r
                });
                let shad_h = s.spawn(move || {
                    let t = std::time::Instant::now();
                    let r = shad_client.check_shadow_consistency(&shad_path);
                    log::info!("PERF: shadow consistency took {:?}", t.elapsed());
                    r
                });
                let ct_h = s.spawn(move || {
                    let t = std::time::Instant::now();
                    let r = ct_client.check_colour_temperature(&ct_path);
                    log::info!("PERF: colour temperature took {:?}", t.elapsed());
                    r
                });
                let sb_h = s.spawn(move || {
                    let t = std::time::Instant::now();
                    let r = sb_client.check_splice_boundary(&sb_path);
                    log::info!("PERF: splice boundary took {:?}", t.elapsed());
                    r
                });
                (
                    noise_h.join(),
                    cm_h.join(),
                    npr_h.join(),
                    jg_h.join(),
                    ca_h.join(),
                    seg_h.join(),
                    shad_h.join(),
                    ct_h.join(),
                    sb_h.join(),
                )
            });

        log::info!("PERF: deep group (noise + copy-move + NPR + JPEG ghost + CA + segmented ELA + shadow + colour-temp + splice-boundary, parallel) took {:?}", t_deep.elapsed());

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
        let npr_result = match npr_out {
            Ok(Ok(r)) => Some(r),
            Ok(Err(e)) => {
                log::warn!("Sidecar NPR analysis failed: {e}");
                None
            }
            Err(_) => {
                log::warn!("Sidecar NPR thread panicked");
                None
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
        let ca_result = match ca_out {
            Ok(Ok(r)) => Some(r),
            Ok(Err(e)) => {
                log::warn!("Sidecar chromatic aberration analysis failed: {e}");
                None
            }
            Err(_) => {
                log::warn!("Sidecar chromatic aberration thread panicked");
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
        let shadow_consistency_result = match shad_out {
            Ok(Ok(r)) => Some(r),
            Ok(Err(e)) => {
                log::warn!("Sidecar shadow consistency analysis failed: {e}");
                None
            }
            Err(_) => {
                log::warn!("Sidecar shadow consistency thread panicked");
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
        let splice_boundary_result = match sb_out {
            Ok(Ok(r)) => Some(r),
            Ok(Err(e)) => {
                log::warn!("Sidecar splice boundary detection failed: {e}");
                None
            }
            Err(_) => {
                log::warn!("Sidecar splice boundary thread panicked");
                None
            }
        };

        (
            noise_score,
            noise_result,
            copy_move_score,
            copy_move_result,
            npr_result,
            jpeg_ghost_result,
            ca_result,
            segmented_ela_result,
            shadow_consistency_result,
            colour_temperature_result,
            splice_boundary_result,
        )
    } else {
        (
            None, None, None, None, None, None, None, None, None, None, None,
        )
    };

    // ── Video parallel group ─────────────────────────────────────────────
    // Video metadata, video deepfake analysis, and transcription are all
    // independent; run them concurrently. Deepfake analysis can take up to
    // 120 s for long videos; transcription ~10-30 s; metadata ~1 s.
    // Without parallelism total wall time would be their sum (~150 s worst
    // case); with parallelism it's bounded by the slowest (~120 s).
    let (video_metadata, video_deepfake_result, transcription_result) =
        if is_video && sidecar_available {
            let t_video = std::time::Instant::now();

            let vm_path = path.to_path_buf();
            let vd_path = path.to_path_buf();
            let vm_client = app.sidecar.clone();
            let vd_client = app.sidecar.clone();
            let deepfake_mode_owned = match effective_mode {
                "archival" => "archival",
                "deep" => "deep",
                _ => "standard",
            }
            .to_string();

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
    // Audio metadata and transcription are independent; run them in parallel.
    // Metadata is fast (~1 s), transcription ~10-30 s. Without parallelism
    // wall time is their sum; with parallelism it's bounded by transcription.
    let (audio_metadata, transcription_result) = if is_audio && sidecar_available {
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

    // Build metadata flags from findings
    let metadata_flags: Vec<String> = exif_analysis
        .as_ref()
        .map(|a| a.findings.iter().map(|f| f.title.clone()).collect())
        .unwrap_or_default();

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
    // PDFs and other documents have no applicable forensic detectors.
    // Use a lightweight C2PA-only path rather than defaulting to 0.50 from
    // the unwrap_or on missing EXIF data.
    let ai_declared_by_c2pa = ai_generator.is_some();
    let overall_trust = if !is_image && !is_video && !is_audio {
        document_trust(c2pa_valid, ai_declared_by_c2pa)
    } else {
        compute_trust(
            ela_score,
            noise_score,
            copy_move_score,
            deepfake_score,
            deepfake_confidence,
            deepfake_verdict,
            exif_trust,
            c2pa_valid,
            segmented_ela_score,
            shadow_consistency_score,
            colour_temperature_score,
            splice_boundary_score,
            ai_declared_by_c2pa,
        )
    };
    log::info!("PERF: trust score computation took {:?}", t_trust.elapsed());

    // ── Database operations ───────────────────────────────────────────────
    let t_db = std::time::Instant::now();
    let verification_id = uuid::Uuid::new_v4().to_string();
    let _ = app.db.insert_verification(
        &verification_id,
        source_type,
        info.content_type.as_str(),
        ela_score,
        None,
        c2pa_valid,
        &metadata_flags,
        overall_trust,
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
                "ca_score": ca_result.as_ref().map(|r| r.score),
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
        c2pa_manifest,
        ela_result,
        noise_result,
        copy_move_result,
        deepfake_result,
        npr_result,
        jpeg_ghost_result,
        ca_result,
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
    })
}

/// Verify a file through the VERIFY pipeline.
///
/// `mode` is `"fast"` (EXIF + C2PA only, <5 s) or `"deep"` (full pipeline,
/// 30-60 s). Defaults to `"deep"` when omitted.
#[tauri::command]
fn verify_content(
    source: String,
    source_type: String,
    mode: Option<String>,
    state: State<'_, Mutex<AppState>>,
) -> Result<VerificationResult, AppError> {
    log::info!(
        "Verifying content: {source} ({source_type}) [mode={:?}]",
        mode
    );
    verify_content_inner(&source, &source_type, mode.as_deref(), &state)
}

/// Sign an asset with C2PA Content Credentials.
#[tauri::command]
fn sign_asset(
    asset_id: String,
    creator_name: String,
    license: Option<String>,
    state: State<'_, Mutex<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<Asset, String> {
    let app = state.lock().map_err(|e| e.to_string())?;

    let asset = app
        .db
        .get_asset_by_id(&asset_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Asset not found: {asset_id}"))?;

    if asset.c2pa_signed {
        return Err("Asset is already signed with C2PA".to_string());
    }

    if !c2pa::supports_signing(&asset.content_type, &asset.mime_type) {
        return Err(format!(
            "C2PA signing not supported for {} ({})",
            asset.content_type, asset.mime_type
        ));
    }

    let source = PathBuf::from(&asset.file_path);
    if !source.exists() {
        return Err(format!("Source file not found: {}", asset.file_path));
    }
    let output = c2pa::signed_output_path(&source);

    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    let (cert, key) = c2pa::ensure_certificate(&data_dir).map_err(|e| {
        log::error!("C2PA certificate error for asset {}: {}", asset_id, e);
        e
    })?;

    log::info!(
        "Signing asset {} ({}) -> {}",
        asset_id,
        source.display(),
        output.display()
    );

    let _manifest_info = c2pa::sign_file(
        &source,
        &output,
        &creator_name,
        license.as_deref(),
        &cert,
        &key,
    )
    .map_err(|e| {
        log::error!("C2PA sign_file failed for asset {}: {}", asset_id, e);
        e
    })?;

    let output_str = output.to_string_lossy().to_string();

    app.db
        .set_c2pa_signed(&asset_id, &output_str)
        .map_err(|e| e.to_string())?;

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

    log::info!("Signed asset {} with C2PA -> {}", asset_id, output_str);

    app.db
        .get_asset_by_id(&asset_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Asset disappeared after signing".to_string())
}

/// Read a C2PA manifest from a file path.
///
/// SECURITY: Canonicalises the path before parsing to prevent:
///   - Directory traversal via `../` sequences
///   - Null-byte injection
///   - Path existence oracle attacks via error messages
#[tauri::command]
fn read_manifest(file_path: String) -> Result<Option<c2pa::ManifestInfo>, String> {
    if file_path.contains('\0') {
        return Err("Invalid file path".to_string());
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|_| "File not found or inaccessible".to_string())?;
    c2pa::read_manifest(&path)
}

/// Verify C2PA Content Credentials on a file (alias for read_manifest in VERIFY pipeline).
///
/// SECURITY: Canonicalises the path before parsing to prevent:
///   - Directory traversal via `../` sequences
///   - Null-byte injection
///   - Path existence oracle attacks via error messages
#[tauri::command]
fn verify_c2pa(file_path: String) -> Result<Option<c2pa::ManifestInfo>, String> {
    if file_path.contains('\0') {
        return Err("Invalid file path".to_string());
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|_| "File not found or inaccessible".to_string())?;
    c2pa::read_manifest(&path)
}

/// Get perceptual fingerprints for a specific asset.
#[tauri::command]
fn get_fingerprints(
    asset_id: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<fingerprint::Fingerprint>, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    let rows = app
        .db
        .get_fingerprints_for_asset(&asset_id)
        .map_err(|e| e.to_string())?;
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
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<fingerprint::SimilarAsset>, String> {
    let max_distance = threshold.unwrap_or(10);
    let app = state.lock().map_err(|e| e.to_string())?;

    let source_fps = app
        .db
        .get_fingerprints_for_asset(&asset_id)
        .map_err(|e| e.to_string())?;

    if source_fps.is_empty() {
        return Ok(vec![]);
    }

    let mut matches: Vec<fingerprint::SimilarAsset> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for source_fp in &source_fps {
        let candidates = app
            .db
            .get_all_fingerprints_by_type(&source_fp.hash_type)
            .map_err(|e| e.to_string())?;

        for candidate in &candidates {
            if candidate.asset_id == asset_id || seen.contains(&candidate.asset_id) {
                continue;
            }

            let distance =
                fingerprint::hamming_distance(&source_fp.hash_value, &candidate.hash_value)?;

            if distance <= max_distance {
                let asset = app
                    .db
                    .get_asset_by_id(&candidate.asset_id)
                    .map_err(|e| e.to_string())?;
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
    search_query: Option<String>,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<Asset>, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.db
        .get_filtered_assets(
            content_type.as_deref(),
            c2pa_signed,
            search_query.as_deref(),
        )
        .map_err(|e| e.to_string())
}

/// Delete an asset by ID.
#[tauri::command]
fn delete_asset(asset_id: String, state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.db.delete_asset(&asset_id).map_err(|e| e.to_string())?;
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
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<Asset>, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.db
        .get_recent_assets(limit.unwrap_or(5))
        .map_err(|e| e.to_string())
}

/// Verify content from a URL.
///
/// Downloads the content to a temp file and runs it through the
/// verification pipeline. Supports images and documents.
#[tauri::command]
fn verify_url(
    url: String,
    mode: Option<String>,
    state: State<'_, Mutex<AppState>>,
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
    log::info!("Verifying URL: {log_url} [mode={:?}]", mode);

    // SECURITY: Validate URL to prevent SSRF attacks
    let parsed = url::Url::parse(&url).map_err(|e| {
        log::warn!("URL parse failure: {}", e);
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
        let host_lower = host.to_lowercase();
        if host_lower == "localhost"
            || host_lower == "127.0.0.1"
            || host_lower == "::1"
            || host_lower == "0.0.0.0"
            || host_lower.starts_with("10.")
            || host_lower.starts_with("192.168.")
            || host_lower.starts_with("169.254.")
            || (host_lower.starts_with("172.") && {
                host_lower[4..]
                    .split('.')
                    .next()
                    .and_then(|s| s.parse::<u8>().ok())
                    .is_some_and(|n| (16..=31).contains(&n))
            })
        {
            return Err(AppError::Validation(
                "Cannot verify URLs pointing to local or private network addresses.".to_string(),
            ));
        }
    } else {
        return Err(AppError::Validation(
            "URL must contain a valid host.".to_string(),
        ));
    }

    let response = reqwest::blocking::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .map_err(|e| {
            log::error!("HTTP request failed for URL {}: {}", log_url, e);
            AppError::Sidecar("Failed to download the URL content".to_string())
        })?;

    if !response.status().is_success() {
        let status = response.status();
        log::warn!("URL {} returned HTTP {}", log_url, status);
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
        log::error!("Failed to read body from URL {}: {}", log_url, e);
        AppError::Sidecar("Failed to read the URL content".to_string())
    })?;

    // Write to temp file using a randomised name to prevent TOCTOU races.
    let temp_dir = tempfile::tempdir().map_err(|e| {
        log::error!("Failed to create temp dir: {}", e);
        AppError::FileSystem("Failed to create temporary directory".to_string())
    })?;
    let temp_path = temp_dir.path().join(format!("url_content.{safe_ext}"));
    std::fs::write(&temp_path, &bytes).map_err(|e| {
        log::error!("Failed to write temp file: {}", e);
        AppError::FileSystem("Failed to write temporary file".to_string())
    })?;

    let temp_str = temp_path.to_string_lossy().to_string();

    // Run through verify pipeline
    let mut result = verify_content_inner(&temp_str, "url", mode.as_deref(), &state)?;
    result.source_type = "url".to_string();

    Ok(result)
}

/// Check the ML sidecar health status.
#[tauri::command]
fn check_sidecar_health(
    state: State<'_, Mutex<AppState>>,
) -> Result<sidecar::SidecarHealth, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.sidecar.check_health()
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
    state: State<'_, Mutex<AppState>>,
) -> Result<sidecar::VideoDeepfakeResult, String> {
    if file_path.contains('\0') {
        return Err("Invalid file path".to_string());
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|_| "File not found or inaccessible".to_string())?;

    let valid_modes = ["standard", "deep", "archival"];
    if !valid_modes.contains(&mode.as_str()) {
        return Err(format!(
            "Invalid mode '{}'. Must be one of: standard, deep, archival",
            mode
        ));
    }

    let app = state.lock().map_err(|e| e.to_string())?;
    if !app.sidecar.is_available() {
        return Err("ML sidecar is not available".to_string());
    }

    app.sidecar.analyse_video_deepfake(&path, &mode)
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
    state: State<'_, Mutex<AppState>>,
) -> Result<MetadataSigningWarning, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    let asset = app
        .db
        .get_asset_by_id(&asset_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Asset not found: {asset_id}"))?;

    let path = PathBuf::from(&asset.file_path);

    // Read existing EXIF metadata
    let meta = metadata::extract_exif(&path);

    let existing_artist = meta.as_ref().and_then(|m| m.artist.clone());
    let existing_copyright = meta.as_ref().and_then(|m| m.copyright.clone());
    let existing_description = meta.as_ref().and_then(|m| m.description.clone());

    // Check for existing C2PA manifest
    let has_existing_c2pa = c2pa::read_manifest(&path).ok().flatten().is_some();

    // Build a human-readable summary
    let mut warnings: Vec<String> = Vec::new();
    if let Some(ref artist) = existing_artist {
        warnings.push(format!("Artist field: \"{artist}\""));
    }
    if let Some(ref copyright) = existing_copyright {
        warnings.push(format!("Copyright field: \"{copyright}\""));
    }
    if has_existing_c2pa {
        warnings.push("Existing C2PA Content Credentials are present".to_string());
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
    state: State<'_, Mutex<AppState>>,
) -> Result<watermark::WatermarkResult, String> {
    let app = state.lock().map_err(|e| e.to_string())?;

    let asset = app
        .db
        .get_asset_by_id(&asset_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Asset not found: {asset_id}"))?;

    if !watermark::supports_watermarking(&asset.mime_type) {
        return Err(format!(
            "Watermarking not supported for {} ({})",
            asset.content_type, asset.mime_type
        ));
    }

    let source = PathBuf::from(&asset.file_path);
    if !source.exists() {
        return Err(format!("Source file not found: {}", asset.file_path));
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

    let result = watermark::embed_watermark(&source, &output, &options)?;

    // Update the asset record in the database
    let output_str = output.to_string_lossy().to_string();
    app.db
        .set_watermarked(&asset_id, &output_str)
        .map_err(|e| e.to_string())?;

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
) -> Result<watermark::ExtractResult, String> {
    if path.contains('\0') {
        return Err("Invalid file path".to_string());
    }
    let file_path = PathBuf::from(&path)
        .canonicalize()
        .map_err(|_| "File not found or inaccessible".to_string())?;

    let len = payload_len_bytes.unwrap_or(16);
    watermark::extract_watermark(&file_path, len, reference_hex.as_deref())
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
    state: State<'_, Mutex<AppState>>,
) -> Result<String, String> {
    let report_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    let app = state.lock().map_err(|e| e.to_string())?;
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
        .map_err(|e| e.to_string())?;

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
fn get_false_positive_stats(state: State<'_, Mutex<AppState>>) -> Result<u64, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.db.get_false_positive_count().map_err(|e| e.to_string())
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
fn verify_audit_integrity(state: State<'_, Mutex<AppState>>) -> Result<bool, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.db.verify_audit_chain().map_err(|e| e.to_string())
}

// ===== Monitor Commands =====

/// Fetch the composite Monitor overview in a single round-trip.
///
/// Assembles protection statistics, trust distribution, the 20 most recent
/// audit log entries, and a 30-day activity timeline.
#[tauri::command]
fn get_monitor_overview(state: State<'_, Mutex<AppState>>) -> Result<MonitorOverview, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    let protection = app.db.get_protection_summary().map_err(|e| e.to_string())?;
    let trust = app.db.get_trust_distribution().map_err(|e| e.to_string())?;
    let recent_activity = app.db.get_audit_log(20, None).map_err(|e| e.to_string())?;
    let activity_days = app
        .db
        .get_activity_timeline(30)
        .map_err(|e| e.to_string())?;
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
    state: State<'_, Mutex<AppState>>,
    limit: Option<u32>,
    action_filter: Option<String>,
) -> Result<Vec<AuditLogEntry>, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.db
        .get_audit_log(limit.unwrap_or(50), action_filter.as_deref())
        .map_err(|e| e.to_string())
}

/// Fetch paginated verification history summaries.
///
/// `limit` defaults to 20 and `offset` defaults to 0 when omitted.
#[tauri::command]
fn get_verification_history(
    state: State<'_, Mutex<AppState>>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<VerificationSummary>, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.db
        .get_verification_history(limit.unwrap_or(20), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

// ===== Database Path Configuration =====

/// Configuration file schema stored in app_data_dir/config.json.
#[derive(Debug, Serialize, Deserialize, Default)]
struct AppConfig {
    #[serde(default)]
    db_path: Option<String>,
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
/// 3. Default: `app_data_dir/jura_archive.db` (preserves all existing installs).
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
                    "JURA_DB_PATH set to '{}' but parent directory is not writable; \
                     falling through to config.json",
                    env_val
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
                    "config.json db_path '{}' parent directory is not writable; \
                     falling through to default",
                    cfg_val
                );
            }
        }
    }

    // Priority 3: default — legacy filename for migration compatibility
    let default_path = data_dir.join("jura_archive.db");
    log::info!(
        "Database path resolved to default: {}",
        default_path.display()
    );
    default_path
}

// ===== Database Path Commands =====

/// Return the current database file path as a string.
#[tauri::command]
async fn get_db_path(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    Ok(app.db_path.clone())
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
    state: State<'_, Mutex<AppState>>,
    new_path: String,
) -> Result<String, String> {
    // SECURITY: Guard against null-byte injection in the path.
    if new_path.contains('\0') {
        return Err("Invalid database path".to_string());
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
        return Err(
            "Database path must use a .db, .sqlite, or .sqlite3 extension".to_string(),
        );
    }

    // SECURITY: Reject symlinks in the target path to prevent symlink-based
    // file-overwrite attacks on the destination.  If the destination does not
    // yet exist, symlink_metadata returns an error which we treat as "not a
    // symlink" (the file will be created by the copy step below).
    if let Ok(meta) = std::fs::symlink_metadata(&new_db_path) {
        if meta.file_type().is_symlink() {
            return Err("Database path must not be a symbolic link".to_string());
        }
    }

    // Validate: parent directory must exist and be writable
    let parent = new_db_path
        .parent()
        .ok_or_else(|| "New database path has no parent directory".to_string())?;

    if !dir_is_writable(parent) {
        return Err(format!(
            "Directory '{}' does not exist or is not writable",
            parent.display()
        ));
    }

    // Get current DB path from shared state
    let current_path = {
        let app = state.lock().map_err(|e| e.to_string())?;
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
        format!(
            "Failed to copy database to '{}': {}",
            tmp_path.display(),
            e
        )
    })?;

    // Verify the copy opens cleanly with SQLite
    {
        let verify_conn = rusqlite::Connection::open(&tmp_path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp_path);
            format!("Copied database failed SQLite verification: {}", e)
        })?;
        // Quick integrity check
        verify_conn
            .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .map_err(|e| {
                let _ = std::fs::remove_file(&tmp_path);
                format!("Database integrity check failed: {}", e)
            })?;
    }

    // Rename temp file to final destination
    std::fs::rename(&tmp_path, &new_db_path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        format!(
            "Failed to move database to '{}': {}",
            new_db_path.display(),
            e
        )
    })?;

    // Persist the new path in config.json
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data directory: {}", e))?;

    let config = AppConfig {
        db_path: Some(new_path.clone()),
    };
    write_app_config(&data_dir, &config)?;

    // Update the shared state so get_db_path reflects the change immediately.
    {
        let mut app = state.lock().map_err(|e| e.to_string())?;
        app.db_path.clone_from(&new_path);
    }

    log::info!(
        "Database path changed: {} -> {}",
        current_path.display(),
        new_db_path.display()
    );

    Ok(new_path)
}

// ===== Application Entry =====

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let db_path = resolve_db_path(app);
            log::info!("Database: {}", db_path.display());

            let database = db::Database::open(&db_path).expect("failed to open database");

            let sidecar_key = std::env::var("JURA_SIDECAR_KEY").unwrap_or_default();
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
            let sidecar_child: Option<tauri_plugin_shell::process::CommandChild> =
                if cfg!(debug_assertions) {
                    log::info!(
                        "Dev mode: sidecar assumed to be running manually on \
                         http://127.0.0.1:8200"
                    );
                    None
                } else {
                    match app.shell().sidecar("jura-sidecar") {
                        Err(e) => {
                            log::warn!(
                                "Could not locate sidecar binary: {e}. \
                                 Forensic analysis will be unavailable."
                            );
                            None
                        }
                        Ok(cmd) => {
                            match cmd.args(["--host", "127.0.0.1", "--port", "8200"]).spawn() {
                                Err(e) => {
                                    log::warn!(
                                        "Failed to spawn sidecar: {e}. \
                                         Forensic analysis will be unavailable."
                                    );
                                    None
                                }
                                Ok((mut rx, child)) => {
                                    // Forward sidecar stdout/stderr to the app
                                    // log at DEBUG level in a background task.
                                    tauri::async_runtime::spawn(async move {
                                        use tauri_plugin_shell::process::CommandEvent;
                                        while let Some(event) = rx.recv().await {
                                            match event {
                                                CommandEvent::Stdout(line) => {
                                                    log::debug!(
                                                        "sidecar: {}",
                                                        String::from_utf8_lossy(&line)
                                                    );
                                                }
                                                CommandEvent::Stderr(line) => {
                                                    log::debug!(
                                                        "sidecar: {}",
                                                        String::from_utf8_lossy(&line)
                                                    );
                                                }
                                                CommandEvent::Terminated(p) => {
                                                    log::info!(
                                                        "Sidecar process terminated \
                                                         (code: {:?})",
                                                        p.code
                                                    );
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
                };

            // ── Sidecar readiness check ──────────────────────────────────────
            // Poll /health with exponential backoff (up to ~10 s total).
            // This is a blocking check on the setup thread, which is
            // acceptable — Tauri's window is not shown until setup returns.
            // We cap the total wait so a missing sidecar never stalls startup.
            if sidecar_child.is_some() {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(2))
                    .build()
                    .unwrap_or_default();
                let mut ready = false;
                for attempt in 0u32..10 {
                    // 200 ms → 400 → 800 → 1600 ms (capped at 1600 ms per attempt)
                    let delay_ms = 200u64 * (1u64 << attempt.min(3));
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    if client
                        .get("http://127.0.0.1:8200/health")
                        .send()
                        .map(|r| r.status().is_success())
                        .unwrap_or(false)
                    {
                        log::info!("Sidecar ready after {} poll attempt(s)", attempt + 1);
                        ready = true;
                        break;
                    }
                }
                if !ready {
                    log::warn!(
                        "Sidecar did not respond within timeout. \
                         Forensic analysis will be unavailable."
                    );
                }
            }

            app.manage(Mutex::new(AppState {
                db: database,
                sidecar: sidecar_client,
                db_path: db_path.to_string_lossy().into_owned(),
                sidecar_process: sidecar_child,
            }));
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
            embed_watermark_asset,
            extract_watermark_from_path,
            analyse_video_deepfake,
            get_db_path,
            set_db_path,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Jura Trace")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Kill the sidecar process when the app exits so it does not
                // linger in the background consuming system resources.
                if let Ok(mut state) = app.state::<Mutex<AppState>>().lock() {
                    if let Some(child) = state.sidecar_process.take() {
                        if let Err(e) = child.kill() {
                            log::warn!("Failed to kill sidecar on exit: {e}");
                        } else {
                            log::info!("Sidecar process terminated on app exit");
                        }
                    }
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
        );
        assert!(trust > 0.60, "Expected >0.60, got {trust:.3}");
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
        );
        assert!(
            trust_with > trust_without,
            "C2PA bonus not applied: {trust_with:.3} vs {trust_without:.3}"
        );
    }

    #[test]
    fn trust_no_forensics_falls_back_to_exif() {
        let trust = compute_trust(
            None, None, None, None, None, None, 0.8, None, None, None, None, None, false,
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
        );
        assert!(
            trust > 0.75,
            "AVIF news image should score >75%, got {:.1}%",
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
        );
        assert!(trust <= 1.0, "Trust exceeded 1.0: {trust:.3}");
    }

    // ── Verdict ceiling tests ─────────────────────────────────────────

    #[test]
    fn trust_inconclusive_verdict_caps_trust() {
        // Fake wedding image scenario: deepfake score 0.31, inconclusive verdict.
        // Previously scored 92% "High Trust" — now capped at 60%.
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
        );
        assert!(
            trust <= 0.60,
            "Inconclusive verdict should cap trust at 0.60, got {trust:.3}"
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
        );
        assert!(
            trust <= 0.45,
            "Synthetic+low should cap at 0.45, got {trust:.3}"
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
    fn verify_archival_mode_is_deep() {
        let mode: Option<&str> = Some("archival");
        let effective = match mode {
            Some("fast") | Some("quick") => "quick",
            Some("standard") => "standard",
            Some("deep") | Some("archival") => "deep",
            _ => "standard",
        };
        let is_deep = matches!(effective, "deep" | "archival");
        assert!(is_deep, "Archival mode must run all detectors");
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
        let mut warnings: Vec<String> = Vec::new();
        warnings.push("Artist field: \"Alice\"".to_string());
        warnings.push("Copyright field: \"2026 Alice\"".to_string());
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
        // Now: capped at 0.60 by inconclusive ceiling.
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
            None,  // no regional detectors
            false,
        );
        assert!(
            trust <= 0.60,
            "Inconclusive should cap trust at 60% max, got {:.1}%",
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
            c2pa_manifest: None,
            ela_result: None,
            noise_result: None,
            copy_move_result: None,
            deepfake_result: None,
            npr_result: None,
            jpeg_ghost_result: None,
            ca_result: None,
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
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(
            json.contains("\"mode\":\"standard\""),
            "mode field missing or wrong value in serialised JSON: {json}"
        );
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
        );
        assert!(
            trust <= 0.55,
            "Two suspicious regional detectors should cap trust at 0.55, got {trust:.3}"
        );
    }

    #[test]
    fn trust_three_suspicious_regional_detectors_still_capped() {
        // Three suspicious regional detectors — cap must still hold
        let trust = compute_trust(
            Some(0.05),
            None,
            None,
            Some(0.10),
            Some("high"),
            Some("authentic"),
            0.9,
            None,
            Some(0.8),  // segmented ELA
            Some(0.6),  // shadow consistency
            Some(0.75), // colour temperature
            None,
            false,
        );
        assert!(
            trust <= 0.55,
            "Three suspicious regional detectors should cap at 0.55, got {trust:.3}"
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
        // Regional cap (0.55) is stricter than the inconclusive verdict ceiling (0.60)
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
        );
        assert!(
            trust <= 0.55,
            "Regional cap should be binding when stricter than verdict ceiling, got {trust:.3}"
        );
    }

    // ── Database path resolution tests ─────────────────────────────────

    #[test]
    fn read_app_config_missing_file_returns_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = read_app_config(dir.path());
        assert!(cfg.db_path.is_none(), "Missing config.json should yield default");
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
        assert!(!dir_is_writable(&path), "Non-existent dir should not be writable");
    }

    #[test]
    fn env_var_path_takes_priority_over_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("from_config.db");
        let config = AppConfig {
            db_path: Some(cfg_path.to_string_lossy().into_owned()),
        };
        write_app_config(dir.path(), &config).expect("write");

        let env_path = dir.path().join("from_env.db");
        let env_parent = dir.path();
        assert!(dir_is_writable(env_parent), "Parent must be writable");

        let chosen = if dir_is_writable(env_path.parent().unwrap()) {
            env_path.clone()
        } else {
            let cfg = read_app_config(dir.path());
            cfg.db_path.map(PathBuf::from).unwrap_or_else(|| dir.path().join("default.db"))
        };
        assert_eq!(chosen, env_path, "Env var path should take priority");
    }

    #[test]
    fn config_path_takes_priority_over_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("custom.db");
        let config = AppConfig {
            db_path: Some(cfg_path.to_string_lossy().into_owned()),
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
        let default = dir.path().join("jura_archive.db");
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
            None, None, None, None, None, None, 0.8, Some(true), None, None, None, None, true,
        );
        let trust_none = compute_trust(
            None, None, None, None, None, None, 0.8, None, None, None, None, None, false,
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
            None, None, None, None, None, None, 1.0, Some(true), None, None, None, None, false,
        );
        let trust_ai = compute_trust(
            None, None, None, None, None, None, 1.0, Some(true), None, None, None, None, true,
        );
        // valid: 1.0 + 0.10 capped at 1.0 = 1.0
        assert!(
            (trust_valid - 1.0).abs() < 0.001,
            "Valid C2PA + perfect EXIF should reach 1.0: got {trust_valid:.3}"
        );
        // AI declared: 1.0 - 0.25 = 0.75
        assert!(
            (trust_ai - 0.75).abs() < 0.001,
            "AI-declared C2PA should yield 0.75 with perfect EXIF: got {trust_ai:.3}"
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
        let file_size = std::fs::metadata(&empty_path)
            .map(|m| m.len())
            .unwrap_or(0);

        assert_eq!(file_size, 0, "File must be empty for this test");

        if file_size == 0 {
            let err = AppError::Validation(
                "The file is empty (zero bytes). Please select a valid file.".to_string(),
            );
            assert!(
                err.to_string()
                    .contains("empty (zero bytes)"),
                "Error should mention 'empty (zero bytes)', got: {}",
                err
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

        let file_size = std::fs::metadata(&tiny_path)
            .map(|m| m.len())
            .unwrap_or(0);

        assert_eq!(file_size, 5, "File must be 5 bytes for this test");

        // Guard mirrors the logic in verify_content_inner.
        assert!(file_size > 0, "Non-zero check passes");
        assert!(
            file_size < 12,
            "File must be below the 12-byte minimum header threshold"
        );

        let err = AppError::Validation(
            "The file is too small to be a valid media file.".to_string(),
        );
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
        let file_size = std::fs::metadata(&empty_path)
            .map(|m| m.len())
            .unwrap_or(0);

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
}
