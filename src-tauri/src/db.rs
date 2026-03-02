use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;
use std::sync::Mutex;

use crate::{AppStats, Asset};

/// Thread-safe database wrapper for SQLite operations.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) the database at the given path and initialise the schema.
    pub fn open(db_path: &Path) -> SqliteResult<Self> {
        let conn = Connection::open(db_path)?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Create tables if they do not already exist.
    fn init_schema(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS assets (
                asset_id     TEXT PRIMARY KEY,
                file_path    TEXT NOT NULL,
                file_name    TEXT NOT NULL,
                content_type TEXT NOT NULL,
                mime_type    TEXT NOT NULL,
                file_size    INTEGER NOT NULL,
                width        INTEGER,
                height       INTEGER,
                ai_description TEXT,
                ai_tags      TEXT,
                metadata_json TEXT,
                c2pa_signed  INTEGER NOT NULL DEFAULT 0,
                watermarked  INTEGER NOT NULL DEFAULT 0,
                collection_id TEXT,
                created_at   TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS fingerprints (
                fingerprint_id TEXT PRIMARY KEY,
                asset_id       TEXT NOT NULL REFERENCES assets(asset_id),
                hash_type      TEXT NOT NULL,
                hash_value     TEXT NOT NULL,
                created_at     TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS verifications (
                verification_id TEXT PRIMARY KEY,
                source_type     TEXT NOT NULL,
                content_type    TEXT NOT NULL,
                ela_score       REAL,
                deepfake_score  REAL,
                c2pa_valid      INTEGER,
                metadata_flags  TEXT,
                claim_verdict   TEXT,
                overall_trust   REAL NOT NULL DEFAULT 0.0,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audit_log (
                log_id      TEXT PRIMARY KEY,
                action      TEXT NOT NULL,
                target_type TEXT NOT NULL,
                target_id   TEXT NOT NULL,
                details     TEXT,
                created_at  TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_assets_content_type ON assets(content_type);
            CREATE INDEX IF NOT EXISTS idx_fingerprints_asset   ON fingerprints(asset_id);
            CREATE INDEX IF NOT EXISTS idx_audit_target         ON audit_log(target_id);
            ",
        )?;
        Ok(())
    }

    // ── Asset operations ──────────────────────────────────────────────

    /// Insert a new asset record.
    pub fn insert_asset(&self, asset: &AssetRow) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO assets (asset_id, file_path, file_name, content_type, mime_type,
                                 file_size, width, height, metadata_json, c2pa_signed,
                                 watermarked, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                asset.asset_id,
                asset.file_path,
                asset.file_name,
                asset.content_type,
                asset.mime_type,
                asset.file_size,
                asset.width,
                asset.height,
                asset.metadata_json,
                asset.c2pa_signed as i32,
                asset.watermarked as i32,
                asset.created_at,
            ],
        )?;
        Ok(())
    }

    /// Get all assets, most recent first.
    pub fn get_all_assets(&self) -> SqliteResult<Vec<Asset>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT asset_id, file_path, file_name, content_type, mime_type, file_size,
                    width, height, ai_description, ai_tags, metadata_json,
                    c2pa_signed, watermarked, created_at
             FROM assets ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let tags_json: Option<String> = row.get(9)?;
            let ai_tags: Option<Vec<String>> = tags_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok());

            Ok(Asset {
                asset_id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                content_type: row.get(3)?,
                mime_type: row.get(4)?,
                file_size: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                ai_description: row.get(8)?,
                ai_tags,
                metadata_json: row.get(10)?,
                c2pa_signed: row.get::<_, i32>(11)? != 0,
                watermarked: row.get::<_, i32>(12)? != 0,
                created_at: row.get(13)?,
            })
        })?;

        rows.collect()
    }

    /// Get dashboard statistics.
    pub fn get_stats(&self) -> SqliteResult<AppStats> {
        let conn = self.conn.lock().unwrap();

        let total_assets: u64 = conn.query_row("SELECT COUNT(*) FROM assets", [], |r| r.get(0))?;

        let c2pa_signed_count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM assets WHERE c2pa_signed = 1",
            [],
            |r| r.get(0),
        )?;

        let total_fingerprints: u64 =
            conn.query_row("SELECT COUNT(*) FROM fingerprints", [], |r| r.get(0))?;

        let total_verifications: u64 =
            conn.query_row("SELECT COUNT(*) FROM verifications", [], |r| r.get(0))?;

        Ok(AppStats {
            total_assets,
            total_fingerprints,
            total_verifications,
            c2pa_signed_count,
        })
    }

    // ── Audit log ─────────────────────────────────────────────────────

    /// Record an action in the immutable audit log.
    pub fn log_action(
        &self,
        action: &str,
        target_type: &str,
        target_id: &str,
        details: Option<&str>,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let log_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO audit_log (log_id, action, target_type, target_id, details, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![log_id, action, target_type, target_id, details, now],
        )?;
        Ok(())
    }
}

/// Row data for inserting a new asset (internal use).
pub struct AssetRow {
    pub asset_id: String,
    pub file_path: String,
    pub file_name: String,
    pub content_type: String,
    pub mime_type: String,
    pub file_size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub metadata_json: Option<String>,
    pub c2pa_signed: bool,
    pub watermarked: bool,
    pub created_at: String,
}
