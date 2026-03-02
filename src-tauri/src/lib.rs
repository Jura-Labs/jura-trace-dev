use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

mod db;
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
fn import_files(paths: Vec<String>, state: State<'_, Mutex<AppState>>) -> Result<Vec<Asset>, String> {
    log::info!("Importing {} file(s)", paths.len());
    let app = state.lock().map_err(|e| e.to_string())?;

    let mut imported: Vec<Asset> = Vec::new();

    for path_str in &paths {
        let path = PathBuf::from(path_str);

        if !path.exists() {
            log::warn!("Skipping missing file: {}", path_str);
            continue;
        }

        // Skip directories — we process individual files
        if path.is_dir() {
            log::info!("Skipping directory: {}", path_str);
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
        let file_size = std::fs::metadata(&path)
            .map(|m| m.len())
            .unwrap_or(0);

        // 3. Extract metadata + dimensions (images)
        let (meta_json, width, height) =
            if info.content_type == format_router::ContentType::Image {
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
        );

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

/// Verify a file or URL through the VERIFY pipeline.
#[tauri::command]
fn verify_content(source: String, source_type: String) -> Result<VerificationResult, String> {
    // TODO: Route through verification pipeline (Phase 2)
    log::info!("Verifying content: {} ({})", source, source_type);
    Ok(VerificationResult {
        source_type,
        content_type: "unknown".to_string(),
        ela_score: None,
        deepfake_score: None,
        c2pa_valid: None,
        metadata_flags: vec![],
        claim_verdict: None,
        overall_trust: 0.0,
    })
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

            let database =
                db::Database::open(&db_path).expect("failed to open database");

            app.manage(Mutex::new(AppState { db: database }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_stats,
            import_files,
            get_assets,
            verify_content,
            get_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Jura Archive");
}
