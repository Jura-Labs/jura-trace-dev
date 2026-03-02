use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

mod c2pa;
mod db;
mod exif_anomaly;
mod fingerprint;
mod format_router;
mod metadata;

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
    pub ela_score: Option<f64>,
    pub deepfake_score: Option<f64>,
    pub c2pa_valid: Option<bool>,
    pub metadata_flags: Vec<String>,
    pub claim_verdict: Option<String>,
    pub overall_trust: f64,
    pub exif_analysis: Option<exif_anomaly::ExifAnalysis>,
    pub c2pa_manifest: Option<c2pa::ManifestInfo>,
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

/// Managed application state shared across Tauri commands.
pub struct AppState {
    pub db: db::Database,
}

// ===== Tauri Commands =====

/// Get application statistics for the dashboard.
#[tauri::command]
fn get_stats(state: State<'_, Mutex<AppState>>) -> Result<AppStats, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.db.get_stats().map_err(|e| e.to_string())
}

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

        // 2. File size
        let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        // 3. Extract metadata + dimensions (images)
        let (meta_json, width, height) = if info.content_type == format_router::ContentType::Image {
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

/// Verify a file through the VERIFY pipeline.
///
/// Runs EXIF anomaly detection and C2PA manifest reading, computing
/// an overall trust score. ELA and deepfake detection require the
/// Python sidecar (Phase 2).
#[tauri::command]
fn verify_content(
    source: String,
    source_type: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<VerificationResult, String> {
    log::info!("Verifying content: {source} ({source_type})");

    let path = std::path::PathBuf::from(&source);
    if !path.exists() {
        return Err(format!("File not found: {source}"));
    }

    // Format detection
    let info = format_router::detect(&path);

    // EXIF analysis (images only)
    let exif_analysis = if info.content_type == format_router::ContentType::Image {
        let meta = metadata::extract_exif(&path);
        let (actual_w, actual_h) = metadata::get_image_dimensions(&path).unwrap_or((0, 0));
        let actual_w = if actual_w > 0 { Some(actual_w) } else { None };
        let actual_h = if actual_h > 0 { Some(actual_h) } else { None };
        Some(exif_anomaly::analyse(meta.as_ref(), actual_w, actual_h))
    } else {
        None
    };

    // C2PA check
    let c2pa_manifest = c2pa::read_manifest(&path).ok().flatten();
    let c2pa_valid = c2pa_manifest.as_ref().map(|m| m.is_valid);

    // Build metadata flags from findings
    let metadata_flags: Vec<String> = exif_analysis
        .as_ref()
        .map(|a| a.findings.iter().map(|f| f.title.clone()).collect())
        .unwrap_or_default();

    // Compute overall trust: combine EXIF trust with C2PA bonus
    let exif_trust = exif_analysis.as_ref().map(|a| a.trust_score).unwrap_or(0.5);
    let c2pa_bonus = if c2pa_valid == Some(true) { 0.1 } else { 0.0 };
    let overall_trust = (exif_trust + c2pa_bonus).min(1.0);

    // Store verification in database
    let app = state.lock().map_err(|e| e.to_string())?;
    let verification_id = uuid::Uuid::new_v4().to_string();
    let _ = app.db.insert_verification(
        &verification_id,
        &source_type,
        info.content_type.as_str(),
        None,
        None,
        c2pa_valid,
        &metadata_flags,
        overall_trust,
    );

    let _ = app.db.log_action(
        "verify",
        "file",
        &source,
        Some(
            &serde_json::json!({
                "exif_trust": exif_trust,
                "c2pa_valid": c2pa_valid,
                "findings_count": metadata_flags.len(),
            })
            .to_string(),
        ),
        None,
        None,
    );

    log::info!(
        "Verification complete: trust={overall_trust:.2}, findings={}",
        metadata_flags.len()
    );

    Ok(VerificationResult {
        source_type,
        content_type: info.content_type.as_str().to_string(),
        ela_score: None,
        deepfake_score: None,
        c2pa_valid,
        metadata_flags,
        claim_verdict: None,
        overall_trust,
        exif_analysis,
        c2pa_manifest,
    })
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
    let (cert, key) = c2pa::ensure_certificate(&data_dir)?;

    let _manifest_info = c2pa::sign_file(
        &source,
        &output,
        &creator_name,
        license.as_deref(),
        &cert,
        &key,
    )?;

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
#[tauri::command]
fn read_manifest(file_path: String) -> Result<Option<c2pa::ManifestInfo>, String> {
    c2pa::read_manifest(std::path::Path::new(&file_path))
}

/// Verify C2PA Content Credentials on a file (alias for read_manifest in VERIFY pipeline).
#[tauri::command]
fn verify_c2pa(file_path: String) -> Result<Option<c2pa::ManifestInfo>, String> {
    c2pa::read_manifest(std::path::Path::new(&file_path))
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
    let _ = app.db.log_action("delete", "asset", &asset_id, None, None, None);
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

/// Get application version.
#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ===== Application Entry =====

/// Resolve the database path inside the Tauri app data directory.
fn resolve_db_path(app: &tauri::App) -> PathBuf {
    let data_dir = app
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");

    std::fs::create_dir_all(&data_dir).expect("failed to create app data directory");

    data_dir.join("jura_archive.db")
}

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

            app.manage(Mutex::new(AppState { db: database }));
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
            get_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Jura Archive");
}
