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
                log_id             TEXT PRIMARY KEY,
                action             TEXT NOT NULL,
                target_type        TEXT NOT NULL,
                target_id          TEXT NOT NULL,
                details            TEXT,
                operator_id        TEXT NOT NULL DEFAULT 'local_user',
                algorithm_metadata TEXT,
                created_at         TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_assets_content_type ON assets(content_type);
            CREATE INDEX IF NOT EXISTS idx_fingerprints_asset   ON fingerprints(asset_id);
            CREATE INDEX IF NOT EXISTS idx_audit_target         ON audit_log(target_id);
            CREATE INDEX IF NOT EXISTS idx_audit_operator       ON audit_log(operator_id);
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
        operator_id: Option<&str>,
        algorithm_metadata: Option<&str>,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let log_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let op = operator_id.unwrap_or("local_user");

        conn.execute(
            "INSERT INTO audit_log (log_id, action, target_type, target_id, details, operator_id, algorithm_metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![log_id, action, target_type, target_id, details, op, algorithm_metadata, now],
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

#[cfg(test)]
mod tests {
    use super::*;

    fn open_temp_db() -> Database {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        // Keep dir alive by leaking — tests are short-lived
        std::mem::forget(dir);
        db
    }

    fn make_asset(id: &str, name: &str, created_at: &str) -> AssetRow {
        AssetRow {
            asset_id: id.to_string(),
            file_path: format!("/tmp/{name}"),
            file_name: name.to_string(),
            content_type: "image".to_string(),
            mime_type: "image/jpeg".to_string(),
            file_size: 1024,
            width: Some(800),
            height: Some(600),
            metadata_json: None,
            c2pa_signed: false,
            watermarked: false,
            created_at: created_at.to_string(),
        }
    }

    // ── Schema ─────────────────────────────────────────────────────

    #[test]
    fn schema_creates_tables() {
        let db = open_temp_db();
        let stats = db.get_stats().unwrap();
        assert_eq!(stats.total_assets, 0);
        assert_eq!(stats.total_fingerprints, 0);
        assert_eq!(stats.total_verifications, 0);
        assert_eq!(stats.c2pa_signed_count, 0);
    }

    #[test]
    fn schema_is_idempotent() {
        let db = open_temp_db();
        // init_schema was already called in open(); calling again should not error
        db.init_schema().unwrap();
    }

    // ── Assets ─────────────────────────────────────────────────────

    #[test]
    fn insert_and_retrieve_asset() {
        let db = open_temp_db();
        let row = make_asset("a1", "photo.jpg", "2026-01-01T00:00:00Z");
        db.insert_asset(&row).unwrap();

        let assets = db.get_all_assets().unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].asset_id, "a1");
        assert_eq!(assets[0].file_name, "photo.jpg");
        assert_eq!(assets[0].width, Some(800));
    }

    #[test]
    fn assets_ordered_most_recent_first() {
        let db = open_temp_db();
        db.insert_asset(&make_asset("older", "old.jpg", "2026-01-01T00:00:00Z"))
            .unwrap();
        db.insert_asset(&make_asset("newer", "new.jpg", "2026-06-01T00:00:00Z"))
            .unwrap();

        let assets = db.get_all_assets().unwrap();
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0].asset_id, "newer");
        assert_eq!(assets[1].asset_id, "older");
    }

    // ── Stats ──────────────────────────────────────────────────────

    #[test]
    fn stats_empty_db() {
        let db = open_temp_db();
        let stats = db.get_stats().unwrap();
        assert_eq!(stats.total_assets, 0);
        assert_eq!(stats.c2pa_signed_count, 0);
    }

    #[test]
    fn stats_counts_assets() {
        let db = open_temp_db();
        db.insert_asset(&make_asset("a1", "one.jpg", "2026-01-01T00:00:00Z"))
            .unwrap();
        db.insert_asset(&make_asset("a2", "two.jpg", "2026-01-02T00:00:00Z"))
            .unwrap();

        let stats = db.get_stats().unwrap();
        assert_eq!(stats.total_assets, 2);
    }

    // ── Audit log ──────────────────────────────────────────────────

    #[test]
    fn audit_log_default_operator() {
        let db = open_temp_db();
        db.log_action("import", "asset", "a1", Some("test details"), None, None)
            .unwrap();

        let conn = db.conn.lock().unwrap();
        let operator: String = conn
            .query_row(
                "SELECT operator_id FROM audit_log WHERE target_id = 'a1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(operator, "local_user");
    }

    #[test]
    fn audit_log_custom_operator() {
        let db = open_temp_db();
        db.log_action(
            "verify",
            "asset",
            "a2",
            None,
            Some("museum_admin"),
            None,
        )
        .unwrap();

        let conn = db.conn.lock().unwrap();
        let operator: String = conn
            .query_row(
                "SELECT operator_id FROM audit_log WHERE target_id = 'a2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(operator, "museum_admin");
    }

    #[test]
    fn audit_log_algorithm_metadata() {
        let db = open_temp_db();
        let meta = r#"{"algorithm":"sha256","version":"1.0"}"#;
        db.log_action("fingerprint", "asset", "a3", None, None, Some(meta))
            .unwrap();

        let conn = db.conn.lock().unwrap();
        let stored: Option<String> = conn
            .query_row(
                "SELECT algorithm_metadata FROM audit_log WHERE target_id = 'a3'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored.as_deref(), Some(meta));
    }
}
