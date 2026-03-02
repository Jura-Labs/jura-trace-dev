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
            CREATE INDEX IF NOT EXISTS idx_fingerprints_hash    ON fingerprints(hash_type, hash_value);
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

    // ── Fingerprint operations ────────────────────────────────────────

    /// Insert a fingerprint record.
    pub fn insert_fingerprint(
        &self,
        fingerprint_id: &str,
        asset_id: &str,
        hash_type: &str,
        hash_value: &str,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO fingerprints (fingerprint_id, asset_id, hash_type, hash_value, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![fingerprint_id, asset_id, hash_type, hash_value, now],
        )?;
        Ok(())
    }

    /// Get all fingerprints for a given asset.
    pub fn get_fingerprints_for_asset(&self, asset_id: &str) -> SqliteResult<Vec<FingerprintRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT fingerprint_id, asset_id, hash_type, hash_value, created_at
             FROM fingerprints WHERE asset_id = ?1",
        )?;
        let rows = stmt.query_map(params![asset_id], |row| {
            Ok(FingerprintRow {
                fingerprint_id: row.get(0)?,
                asset_id: row.get(1)?,
                hash_type: row.get(2)?,
                hash_value: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    /// Get all fingerprints of a given hash type (for similarity scanning).
    pub fn get_all_fingerprints_by_type(
        &self,
        hash_type: &str,
    ) -> SqliteResult<Vec<FingerprintRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT fingerprint_id, asset_id, hash_type, hash_value, created_at
             FROM fingerprints WHERE hash_type = ?1",
        )?;
        let rows = stmt.query_map(params![hash_type], |row| {
            Ok(FingerprintRow {
                fingerprint_id: row.get(0)?,
                asset_id: row.get(1)?,
                hash_type: row.get(2)?,
                hash_value: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    // ── Asset lookup ───────────────────────────────────────────────────

    /// Get a single asset by its ID.
    pub fn get_asset_by_id(&self, asset_id: &str) -> SqliteResult<Option<Asset>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT asset_id, file_path, file_name, content_type, mime_type, file_size,
                    width, height, ai_description, ai_tags, metadata_json,
                    c2pa_signed, watermarked, created_at
             FROM assets WHERE asset_id = ?1",
        )?;

        let result = stmt.query_row(params![asset_id], |row| {
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
        });

        match result {
            Ok(asset) => Ok(Some(asset)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Mark an asset as C2PA-signed and update its file path to the signed copy.
    pub fn set_c2pa_signed(&self, asset_id: &str, file_path: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE assets SET c2pa_signed = 1, file_path = ?1 WHERE asset_id = ?2",
            params![file_path, asset_id],
        )?;
        Ok(())
    }

    // ── Audit log ─────────────────────────────────────────────────────

    // ── Verification operations ─────────────────────────────────────────

    /// Insert a verification result record.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_verification(
        &self,
        verification_id: &str,
        source_type: &str,
        content_type: &str,
        ela_score: Option<f64>,
        deepfake_score: Option<f64>,
        c2pa_valid: Option<bool>,
        metadata_flags: &[String],
        overall_trust: f64,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let flags_json = serde_json::to_string(metadata_flags).unwrap_or_default();
        conn.execute(
            "INSERT INTO verifications (verification_id, source_type, content_type,
             ela_score, deepfake_score, c2pa_valid, metadata_flags, overall_trust, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                verification_id,
                source_type,
                content_type,
                ela_score,
                deepfake_score,
                c2pa_valid.map(|b| b as i32),
                flags_json,
                overall_trust,
                now
            ],
        )?;
        Ok(())
    }

    // ── Asset filtering ─────────────────────────────────────────────────

    /// Delete an asset and its associated fingerprints.
    pub fn delete_asset(&self, asset_id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM fingerprints WHERE asset_id = ?1",
            params![asset_id],
        )?;
        conn.execute(
            "DELETE FROM assets WHERE asset_id = ?1",
            params![asset_id],
        )?;
        Ok(())
    }

    /// Get assets matching optional filters, ordered by created_at DESC.
    pub fn get_filtered_assets(
        &self,
        content_type: Option<&str>,
        c2pa_signed: Option<bool>,
        search_query: Option<&str>,
    ) -> SqliteResult<Vec<Asset>> {
        let conn = self.conn.lock().unwrap();

        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ct) = content_type {
            conditions.push(format!("content_type = ?{}", param_values.len() + 1));
            param_values.push(Box::new(ct.to_string()));
        }
        if let Some(signed) = c2pa_signed {
            conditions.push(format!("c2pa_signed = ?{}", param_values.len() + 1));
            param_values.push(Box::new(signed as i32));
        }
        if let Some(query) = search_query {
            if !query.is_empty() {
                let n = param_values.len() + 1;
                conditions.push(format!("(file_name LIKE ?{n} OR mime_type LIKE ?{n})"));
                param_values.push(Box::new(format!("%{query}%")));
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT asset_id, file_path, file_name, content_type, mime_type, file_size,
                    width, height, ai_description, ai_tags, metadata_json,
                    c2pa_signed, watermarked, created_at
             FROM assets{where_clause} ORDER BY created_at DESC"
        );

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let tags_json: Option<String> = row.get(9)?;
            let ai_tags: Option<Vec<String>> =
                tags_json.as_deref().and_then(|s| serde_json::from_str(s).ok());

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

    /// Get the N most recently imported assets.
    pub fn get_recent_assets(&self, limit: u32) -> SqliteResult<Vec<Asset>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT asset_id, file_path, file_name, content_type, mime_type, file_size,
                    width, height, ai_description, ai_tags, metadata_json,
                    c2pa_signed, watermarked, created_at
             FROM assets ORDER BY created_at DESC LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            let tags_json: Option<String> = row.get(9)?;
            let ai_tags: Option<Vec<String>> =
                tags_json.as_deref().and_then(|s| serde_json::from_str(s).ok());

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

/// Row data for a fingerprint record.
pub struct FingerprintRow {
    pub fingerprint_id: String,
    pub asset_id: String,
    pub hash_type: String,
    pub hash_value: String,
    pub created_at: String,
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

    // ── Fingerprints ────────────────────────────────────────────────

    #[test]
    fn insert_and_retrieve_fingerprints() {
        let db = open_temp_db();
        db.insert_asset(&make_asset("a1", "photo.jpg", "2026-01-01T00:00:00Z"))
            .unwrap();

        db.insert_fingerprint("fp1", "a1", "ahash", "ff00ff00ff00ff00")
            .unwrap();
        db.insert_fingerprint("fp2", "a1", "dhash", "00ff00ff00ff00ff")
            .unwrap();
        db.insert_fingerprint("fp3", "a1", "phash", "1234567890abcdef")
            .unwrap();

        let fps = db.get_fingerprints_for_asset("a1").unwrap();
        assert_eq!(fps.len(), 3);
    }

    #[test]
    fn get_fingerprints_empty() {
        let db = open_temp_db();
        let fps = db.get_fingerprints_for_asset("nonexistent").unwrap();
        assert!(fps.is_empty());
    }

    #[test]
    fn fingerprints_counted_in_stats() {
        let db = open_temp_db();
        db.insert_asset(&make_asset("a1", "photo.jpg", "2026-01-01T00:00:00Z"))
            .unwrap();
        db.insert_fingerprint("fp1", "a1", "ahash", "ff00ff00ff00ff00")
            .unwrap();
        db.insert_fingerprint("fp2", "a1", "dhash", "00ff00ff00ff00ff")
            .unwrap();

        let stats = db.get_stats().unwrap();
        assert_eq!(stats.total_fingerprints, 2);
    }

    #[test]
    fn get_all_fingerprints_by_type() {
        let db = open_temp_db();
        db.insert_asset(&make_asset("a1", "one.jpg", "2026-01-01T00:00:00Z"))
            .unwrap();
        db.insert_asset(&make_asset("a2", "two.jpg", "2026-01-02T00:00:00Z"))
            .unwrap();

        db.insert_fingerprint("fp1", "a1", "ahash", "ff00ff00ff00ff00")
            .unwrap();
        db.insert_fingerprint("fp2", "a2", "ahash", "ff00ff00ff00ff01")
            .unwrap();
        db.insert_fingerprint("fp3", "a1", "dhash", "0000000000000000")
            .unwrap();

        let ahashes = db.get_all_fingerprints_by_type("ahash").unwrap();
        assert_eq!(ahashes.len(), 2);

        let dhashes = db.get_all_fingerprints_by_type("dhash").unwrap();
        assert_eq!(dhashes.len(), 1);
    }

    // ── Asset lookup ──────────────────────────────────────────────────

    #[test]
    fn get_asset_by_id_found() {
        let db = open_temp_db();
        db.insert_asset(&make_asset("a1", "photo.jpg", "2026-01-01T00:00:00Z"))
            .unwrap();

        let asset = db.get_asset_by_id("a1").unwrap();
        assert!(asset.is_some());
        assert_eq!(asset.unwrap().file_name, "photo.jpg");
    }

    #[test]
    fn get_asset_by_id_not_found() {
        let db = open_temp_db();
        let asset = db.get_asset_by_id("nonexistent").unwrap();
        assert!(asset.is_none());
    }

    #[test]
    fn set_c2pa_signed_updates_asset() {
        let db = open_temp_db();
        db.insert_asset(&make_asset("a1", "photo.jpg", "2026-01-01T00:00:00Z"))
            .unwrap();

        // Initially not signed
        let asset = db.get_asset_by_id("a1").unwrap().unwrap();
        assert!(!asset.c2pa_signed);
        assert_eq!(asset.file_path, "/tmp/photo.jpg");

        // Mark as signed with new path
        db.set_c2pa_signed("a1", "/tmp/photo_c2pa.jpg").unwrap();

        let asset = db.get_asset_by_id("a1").unwrap().unwrap();
        assert!(asset.c2pa_signed);
        assert_eq!(asset.file_path, "/tmp/photo_c2pa.jpg");
    }

    // ── Verification ────────────────────────────────────────────────

    #[test]
    fn insert_verification() {
        let db = open_temp_db();
        db.insert_verification(
            "v1",
            "file",
            "image",
            None,
            None,
            Some(true),
            &["No camera information".to_string()],
            0.8,
        )
        .unwrap();
    }

    #[test]
    fn verification_counted_in_stats() {
        let db = open_temp_db();
        db.insert_verification("v1", "file", "image", None, None, None, &[], 0.5)
            .unwrap();
        db.insert_verification("v2", "url", "image", None, None, None, &[], 0.3)
            .unwrap();

        let stats = db.get_stats().unwrap();
        assert_eq!(stats.total_verifications, 2);
    }

    // ── Delete asset ────────────────────────────────────────────────

    #[test]
    fn delete_asset_removes_asset() {
        let db = open_temp_db();
        db.insert_asset(&make_asset("a1", "photo.jpg", "2026-01-01T00:00:00Z"))
            .unwrap();
        assert!(db.get_asset_by_id("a1").unwrap().is_some());

        db.delete_asset("a1").unwrap();
        assert!(db.get_asset_by_id("a1").unwrap().is_none());
    }

    #[test]
    fn delete_asset_removes_fingerprints() {
        let db = open_temp_db();
        db.insert_asset(&make_asset("a1", "photo.jpg", "2026-01-01T00:00:00Z"))
            .unwrap();
        db.insert_fingerprint("fp1", "a1", "ahash", "ff00ff00ff00ff00")
            .unwrap();
        assert_eq!(db.get_fingerprints_for_asset("a1").unwrap().len(), 1);

        db.delete_asset("a1").unwrap();
        assert!(db.get_fingerprints_for_asset("a1").unwrap().is_empty());
    }

    // ── Filtered assets ─────────────────────────────────────────────

    #[test]
    fn filter_by_content_type() {
        let db = open_temp_db();
        let mut img = make_asset("a1", "photo.jpg", "2026-01-01T00:00:00Z");
        img.content_type = "image".to_string();
        db.insert_asset(&img).unwrap();

        let mut doc = make_asset("a2", "report.pdf", "2026-01-02T00:00:00Z");
        doc.content_type = "document".to_string();
        db.insert_asset(&doc).unwrap();

        let images = db
            .get_filtered_assets(Some("image"), None, None)
            .unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].asset_id, "a1");

        let docs = db
            .get_filtered_assets(Some("document"), None, None)
            .unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].asset_id, "a2");
    }

    #[test]
    fn filter_by_c2pa_signed() {
        let db = open_temp_db();
        db.insert_asset(&make_asset("a1", "photo.jpg", "2026-01-01T00:00:00Z"))
            .unwrap();
        db.insert_asset(&make_asset("a2", "other.jpg", "2026-01-02T00:00:00Z"))
            .unwrap();
        db.set_c2pa_signed("a2", "/tmp/other_c2pa.jpg").unwrap();

        let signed = db
            .get_filtered_assets(None, Some(true), None)
            .unwrap();
        assert_eq!(signed.len(), 1);
        assert_eq!(signed[0].asset_id, "a2");

        let unsigned = db
            .get_filtered_assets(None, Some(false), None)
            .unwrap();
        assert_eq!(unsigned.len(), 1);
        assert_eq!(unsigned[0].asset_id, "a1");
    }

    #[test]
    fn filter_by_search_query() {
        let db = open_temp_db();
        db.insert_asset(&make_asset("a1", "sunset_beach.jpg", "2026-01-01T00:00:00Z"))
            .unwrap();
        db.insert_asset(&make_asset("a2", "mountain_view.jpg", "2026-01-02T00:00:00Z"))
            .unwrap();

        let results = db
            .get_filtered_assets(None, None, Some("sunset"))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].asset_id, "a1");

        let all = db
            .get_filtered_assets(None, None, Some(""))
            .unwrap();
        assert_eq!(all.len(), 2);
    }

    // ── Audit log ───────────────────────────────────────────────────

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
