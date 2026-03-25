use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Mutex;

use crate::{
    ActivityDay, AppStats, Asset, AuditLogEntry, ProtectionSummary, TrustDistribution,
    VerificationSummary,
};

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

            CREATE TABLE IF NOT EXISTS false_positive_reports (
                id                  TEXT PRIMARY KEY,
                verification_id     TEXT,
                file_hash           TEXT,
                reason_code         TEXT NOT NULL,
                reason_note         TEXT,
                mime_type           TEXT,
                deepfake_score      REAL,
                deepfake_verdict    TEXT,
                signal_scores_json  TEXT,
                created_at          TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_assets_content_type    ON assets(content_type);
            CREATE INDEX IF NOT EXISTS idx_fingerprints_asset      ON fingerprints(asset_id);
            CREATE INDEX IF NOT EXISTS idx_fingerprints_hash       ON fingerprints(hash_type, hash_value);
            CREATE INDEX IF NOT EXISTS idx_audit_target            ON audit_log(target_id);
            CREATE INDEX IF NOT EXISTS idx_audit_operator          ON audit_log(operator_id);
            CREATE INDEX IF NOT EXISTS idx_audit_created           ON audit_log(created_at);
            CREATE INDEX IF NOT EXISTS idx_verifications_created   ON verifications(created_at);
            CREATE INDEX IF NOT EXISTS idx_fp_reports_reason       ON false_positive_reports(reason_code);
            CREATE INDEX IF NOT EXISTS idx_fp_reports_created      ON false_positive_reports(created_at);

            CREATE TABLE IF NOT EXISTS monitor_urls (
                url_id              TEXT PRIMARY KEY,
                asset_id            TEXT
                                        REFERENCES assets(asset_id)
                                        ON DELETE SET NULL,
                url                 TEXT NOT NULL,
                label               TEXT,
                check_frequency     TEXT NOT NULL DEFAULT 'daily',
                last_checked_at     TEXT,
                last_status         TEXT,
                last_content_hash   TEXT,
                last_c2pa_valid     INTEGER,
                last_watermark_match INTEGER,
                enabled             INTEGER NOT NULL DEFAULT 1,
                created_at          TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at          TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS monitor_events (
                event_id            TEXT PRIMARY KEY,
                url_id              TEXT NOT NULL
                                        REFERENCES monitor_urls(url_id)
                                        ON DELETE CASCADE,
                event_type          TEXT NOT NULL,
                checked_at          TEXT NOT NULL DEFAULT (datetime('now')),
                content_hash        TEXT,
                c2pa_valid          INTEGER,
                watermark_uuid      TEXT,
                watermark_confidence REAL,
                http_status         INTEGER,
                response_time_ms    INTEGER,
                case_status         TEXT NOT NULL DEFAULT 'new',
                case_notes          TEXT,
                case_updated_at     TEXT,
                detail_json         TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_monitor_urls_asset
                ON monitor_urls(asset_id);
            CREATE INDEX IF NOT EXISTS idx_monitor_urls_enabled_checked
                ON monitor_urls(enabled, last_checked_at);
            CREATE INDEX IF NOT EXISTS idx_monitor_events_url_checked
                ON monitor_events(url_id, checked_at);
            CREATE INDEX IF NOT EXISTS idx_monitor_events_type
                ON monitor_events(event_type);
            CREATE INDEX IF NOT EXISTS idx_monitor_events_case_active
                ON monitor_events(case_status, checked_at)
                WHERE case_status NOT IN ('dismissed', 'check_ok');
            ",
        )?;

        // Schema migration: add hash chain columns to audit_log if absent.
        // ALTER TABLE ADD COLUMN is a no-op-safe operation — SQLite ignores it
        // gracefully when the column already exists (via the duplicate column
        // error being suppressed). We use IGNORE to handle fresh databases
        // (where the columns appear in the CREATE TABLE above) equally.
        let _ = conn.execute_batch(
            "ALTER TABLE audit_log ADD COLUMN prev_hash TEXT;
             ALTER TABLE audit_log ADD COLUMN entry_hash TEXT;",
        );

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

    /// Mark an asset as watermarked and update its stored file path to the
    /// watermarked output file.
    pub fn set_watermarked(&self, asset_id: &str, file_path: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE assets SET watermarked = 1, file_path = ?1 WHERE asset_id = ?2",
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
        conn.execute("DELETE FROM assets WHERE asset_id = ?1", params![asset_id])?;
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
                // SECURITY: Escape SQLite LIKE metacharacters before interpolating
                // into the pattern.  Without escaping, a user who searches for `%`
                // would match every row, and `_` would act as a single-character
                // wildcard — this is LIKE injection (not SQL injection, but still
                // a correctness and DoS concern).
                let escaped = query
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_");
                let n = param_values.len() + 1;
                conditions.push(format!(
                    "(file_name LIKE ?{n} ESCAPE '\\' OR mime_type LIKE ?{n} ESCAPE '\\')"
                ));
                param_values.push(Box::new(format!("%{escaped}%")));
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

    // ── Audit log ─────────────────────────────────────────────────────

    // ── False-positive reports ─────────────────────────────────────────

    /// Insert a false-positive report submitted by the user.
    ///
    /// `signal_scores_json` should be a JSON object mapping signal names to
    /// their raw scores, serialised by the caller from the `VerificationResult`.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_false_positive(
        &self,
        id: &str,
        verification_id: Option<&str>,
        file_hash: Option<&str>,
        reason_code: &str,
        reason_note: Option<&str>,
        mime_type: Option<&str>,
        deepfake_score: Option<f64>,
        deepfake_verdict: Option<&str>,
        signal_scores_json: Option<&str>,
        created_at: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO false_positive_reports
             (id, verification_id, file_hash, reason_code, reason_note,
              mime_type, deepfake_score, deepfake_verdict, signal_scores_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                verification_id,
                file_hash,
                reason_code,
                reason_note,
                mime_type,
                deepfake_score,
                deepfake_verdict,
                signal_scores_json,
                created_at,
            ],
        )?;
        Ok(())
    }

    /// Return the total number of false-positive reports stored.
    pub fn get_false_positive_count(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let count: u64 =
            conn.query_row("SELECT COUNT(*) FROM false_positive_reports", [], |r| {
                r.get(0)
            })?;
        Ok(count)
    }

    /// Return all false-positive reports, most recent first.
    pub fn get_false_positive_reports(
        &self,
    ) -> Result<Vec<FalsePositiveReport>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, verification_id, reason_code, reason_note, mime_type,
                    deepfake_score, deepfake_verdict, created_at
             FROM false_positive_reports
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FalsePositiveReport {
                id: row.get(0)?,
                verification_id: row.get(1)?,
                reason_code: row.get(2)?,
                reason_note: row.get(3)?,
                mime_type: row.get(4)?,
                deepfake_score: row.get(5)?,
                deepfake_verdict: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        let reports = rows.collect::<SqliteResult<Vec<_>>>()?;
        Ok(reports)
    }

    // ── Monitor queries ────────────────────────────────────────────────

    /// Retrieve audit log entries ordered by most recent first.
    ///
    /// `limit` caps the number of rows returned. `action_filter` constrains
    /// results to entries whose `action` column exactly matches the provided
    /// value (e.g. `"import"`, `"verify"`, `"sign"`).
    pub fn get_audit_log(
        &self,
        limit: u32,
        action_filter: Option<&str>,
    ) -> SqliteResult<Vec<AuditLogEntry>> {
        let conn = self.conn.lock().unwrap();

        // Use a single parameterised query path.  When no filter is needed we
        // supply a wildcard that matches every action value via LIKE '%%',
        // avoiding a split if/else that would cause stmt lifetime issues.
        let sql = "SELECT log_id, action, target_type, target_id, details, created_at
                   FROM audit_log
                   WHERE (?1 IS NULL OR action = ?1)
                   ORDER BY created_at DESC
                   LIMIT ?2";

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params![action_filter, limit], |row| {
            Ok(AuditLogEntry {
                log_id: row.get(0)?,
                action: row.get(1)?,
                target_type: row.get(2)?,
                target_id: row.get(3)?,
                details: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        rows.collect()
    }

    /// Retrieve lightweight summaries of past verification runs.
    ///
    /// Results are ordered most recent first. `offset` supports pagination.
    pub fn get_verification_history(
        &self,
        limit: u32,
        offset: u32,
    ) -> SqliteResult<Vec<VerificationSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT verification_id, source_type, content_type,
                    ela_score, deepfake_score, c2pa_valid,
                    overall_trust, created_at
             FROM verifications
             ORDER BY created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let rows = stmt.query_map(params![limit, offset], |row| {
            let c2pa_raw: Option<i32> = row.get(5)?;
            Ok(VerificationSummary {
                verification_id: row.get(0)?,
                source_type: row.get(1)?,
                content_type: row.get(2)?,
                ela_score: row.get(3)?,
                deepfake_score: row.get(4)?,
                c2pa_valid: c2pa_raw.map(|v| v != 0),
                overall_trust: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;

        rows.collect()
    }

    /// Compute the trust-score distribution across all stored verifications.
    ///
    /// Thresholds: high >= 0.7, medium 0.4–0.7 (exclusive), low < 0.4.
    pub fn get_trust_distribution(&self) -> SqliteResult<TrustDistribution> {
        let conn = self.conn.lock().unwrap();

        let total: u64 = conn.query_row("SELECT COUNT(*) FROM verifications", [], |r| r.get(0))?;

        let high_count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM verifications WHERE overall_trust >= 0.7",
            [],
            |r| r.get(0),
        )?;

        let medium_count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM verifications WHERE overall_trust >= 0.4 AND overall_trust < 0.7",
            [],
            |r| r.get(0),
        )?;

        let low_count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM verifications WHERE overall_trust < 0.4",
            [],
            |r| r.get(0),
        )?;

        let (average_trust, latest_at): (f64, Option<String>) = conn.query_row(
            "SELECT COALESCE(AVG(overall_trust), 0.0), MAX(created_at) FROM verifications",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        Ok(TrustDistribution {
            total,
            high_count,
            medium_count,
            low_count,
            average_trust,
            latest_at,
        })
    }

    /// Aggregate protection statistics across all stored assets.
    ///
    /// Counts total assets, C2PA-signed assets, watermarked assets, and the
    /// number of distinct assets that have at least one fingerprint record.
    /// Also builds a per-`content_type` count map.
    pub fn get_protection_summary(&self) -> SqliteResult<ProtectionSummary> {
        let conn = self.conn.lock().unwrap();

        let total_assets: u64 = conn.query_row("SELECT COUNT(*) FROM assets", [], |r| r.get(0))?;

        let c2pa_signed: u64 = conn.query_row(
            "SELECT COUNT(*) FROM assets WHERE c2pa_signed = 1",
            [],
            |r| r.get(0),
        )?;

        let watermarked: u64 = conn.query_row(
            "SELECT COUNT(*) FROM assets WHERE watermarked = 1",
            [],
            |r| r.get(0),
        )?;

        // Distinct assets that have at least one fingerprint record
        let fingerprinted: u64 = conn.query_row(
            "SELECT COUNT(DISTINCT asset_id) FROM fingerprints",
            [],
            |r| r.get(0),
        )?;

        // Per-content-type counts
        let mut by_content_type = std::collections::HashMap::new();
        {
            let mut stmt =
                conn.prepare("SELECT content_type, COUNT(*) FROM assets GROUP BY content_type")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })?;
            for row in rows {
                let (ct, count) = row?;
                by_content_type.insert(ct, count);
            }
        }

        let (earliest_at, latest_at): (Option<String>, Option<String>) = conn.query_row(
            "SELECT MIN(created_at), MAX(created_at) FROM assets",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        Ok(ProtectionSummary {
            total_assets,
            c2pa_signed,
            watermarked,
            fingerprinted,
            by_content_type,
            earliest_at,
            latest_at,
        })
    }

    /// Return per-day activity counts for the last `days` calendar days.
    ///
    /// Each entry aggregates how many audit log entries of each action type
    /// occurred on that UTC date. Days with no activity are omitted.
    pub fn get_activity_timeline(&self, days: u32) -> SqliteResult<Vec<ActivityDay>> {
        let conn = self.conn.lock().unwrap();

        // Collect all relevant rows: (date_str, action)
        // SQLite's substr gives YYYY-MM-DD from an ISO-8601 timestamp.
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let mut stmt = conn.prepare(
            "SELECT substr(created_at, 1, 10) AS day, action
             FROM audit_log
             WHERE created_at >= ?1
             ORDER BY day ASC",
        )?;

        // Accumulate into a BTreeMap keyed by date string for stable ordering.
        let mut map: std::collections::BTreeMap<String, ActivityDay> =
            std::collections::BTreeMap::new();

        let rows = stmt.query_map(params![cutoff_str], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (day, action) = row?;
            let entry = map.entry(day.clone()).or_insert_with(|| ActivityDay {
                date: day,
                imports: 0,
                verifications: 0,
                signings: 0,
                deletions: 0,
            });
            match action.as_str() {
                "import" => entry.imports += 1,
                "verify" => entry.verifications += 1,
                "sign" => entry.signings += 1,
                "delete" => entry.deletions += 1,
                _ => {} // false_positive, fingerprint, etc. — not surfaced in UI
            }
        }

        Ok(map.into_values().collect())
    }

    /// Record an action in the immutable audit log.
    ///
    /// Each entry includes a SHA-256 chain hash that binds it to the previous
    /// entry. Deleting or reordering rows will break the chain, which can be
    /// detected by `verify_audit_chain`.
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
        // Use millisecond precision to reduce the probability of two entries
        // sharing the same `created_at` value (which would make the chain
        // ordering non-deterministic when tiebreaking by UUID).
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let op = operator_id.unwrap_or("local_user");

        // Retrieve the entry_hash of the most recent log entry to form the
        // chain. For the very first entry the genesis sentinel is used.
        let prev_hash: String = conn
            .query_row(
                "SELECT COALESCE(entry_hash, '') FROM audit_log ORDER BY created_at DESC, log_id DESC LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "genesis".to_string());

        // SHA-256(prev_hash || action || target_type || target_id || details || created_at)
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(b"|");
        hasher.update(action.as_bytes());
        hasher.update(b"|");
        hasher.update(target_type.as_bytes());
        hasher.update(b"|");
        hasher.update(target_id.as_bytes());
        hasher.update(b"|");
        hasher.update(details.unwrap_or("").as_bytes());
        hasher.update(b"|");
        hasher.update(now.as_bytes());
        let entry_hash = format!("{:x}", hasher.finalize());

        conn.execute(
            "INSERT INTO audit_log (log_id, action, target_type, target_id, details, operator_id, algorithm_metadata, created_at, prev_hash, entry_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![log_id, action, target_type, target_id, details, op, algorithm_metadata, now, prev_hash, entry_hash],
        )?;
        Ok(())
    }

    /// Verify the integrity of the audit log hash chain.
    ///
    /// Walks every audit log entry in insertion order and recomputes each
    /// entry's SHA-256 hash. Returns `Ok(true)` if the chain is intact,
    /// `Ok(false)` if any entry has been modified, deleted, or reordered, or
    /// if any entry is missing its hash (legacy rows pre-migration).
    pub fn verify_audit_chain(&self) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT action, target_type, target_id, details, created_at, prev_hash, entry_hash
             FROM audit_log
             ORDER BY created_at ASC, log_id ASC",
        )?;

        type AuditRow = (
            String,
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
        );
        let rows: Vec<AuditRow> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut expected_prev = "genesis".to_string();

        for (action, target_type, target_id, details, created_at, prev_hash, entry_hash) in rows {
            // Entries without hash columns are pre-migration rows; treat as
            // unverifiable and skip rather than failing the whole chain.
            let (Some(stored_prev), Some(stored_hash)) = (prev_hash, entry_hash) else {
                continue;
            };

            if stored_prev != expected_prev {
                return Ok(false);
            }

            let mut hasher = Sha256::new();
            hasher.update(stored_prev.as_bytes());
            hasher.update(b"|");
            hasher.update(action.as_bytes());
            hasher.update(b"|");
            hasher.update(target_type.as_bytes());
            hasher.update(b"|");
            hasher.update(target_id.as_bytes());
            hasher.update(b"|");
            hasher.update(details.as_deref().unwrap_or("").as_bytes());
            hasher.update(b"|");
            hasher.update(created_at.as_bytes());
            let computed = format!("{:x}", hasher.finalize());

            if computed != stored_hash {
                return Ok(false);
            }

            expected_prev = stored_hash;
        }

        Ok(true)
    }

    // ── Monitor URL operations ─────────────────────────────────────────

    /// Register a URL for periodic monitoring.
    ///
    /// Generates a new UUID for `url_id`, inserts the row, and returns the
    /// fully-populated `MonitorUrl` struct.
    pub fn add_monitor_url(
        &self,
        url: &str,
        label: Option<&str>,
        asset_id: Option<&str>,
        frequency: &str,
    ) -> SqliteResult<MonitorUrl> {
        let conn = self.conn.lock().unwrap();
        let url_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO monitor_urls
             (url_id, asset_id, url, label, check_frequency, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)",
            params![url_id, asset_id, url, label, frequency, now],
        )?;
        Ok(MonitorUrl {
            url_id,
            asset_id: asset_id.map(str::to_string),
            url: url.to_string(),
            label: label.map(str::to_string),
            check_frequency: frequency.to_string(),
            last_checked_at: None,
            last_status: None,
            last_content_hash: None,
            last_c2pa_valid: None,
            last_watermark_match: None,
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Delete a monitored URL and all its events (CASCADE).
    pub fn remove_monitor_url(&self, url_id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM monitor_urls WHERE url_id = ?1",
            params![url_id],
        )?;
        Ok(())
    }

    /// Return all monitored URLs, optionally restricted to enabled entries only.
    ///
    /// Results are ordered by `created_at` descending so the most recently
    /// added URLs appear first.
    pub fn list_monitor_urls(&self, enabled_only: bool) -> SqliteResult<Vec<MonitorUrl>> {
        let conn = self.conn.lock().unwrap();
        let sql = "SELECT url_id, asset_id, url, label, check_frequency,
                          last_checked_at, last_status, last_content_hash,
                          last_c2pa_valid, last_watermark_match, enabled,
                          created_at, updated_at
                   FROM monitor_urls
                   WHERE (?1 = 0 OR enabled = 1)
                   ORDER BY created_at DESC";
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params![enabled_only as i32], |row| {
            Ok(MonitorUrl {
                url_id: row.get(0)?,
                asset_id: row.get(1)?,
                url: row.get(2)?,
                label: row.get(3)?,
                check_frequency: row.get(4)?,
                last_checked_at: row.get(5)?,
                last_status: row.get(6)?,
                last_content_hash: row.get(7)?,
                last_c2pa_valid: row.get::<_, Option<i32>>(8)?.map(|v| v != 0),
                last_watermark_match: row.get::<_, Option<i32>>(9)?.map(|v| v != 0),
                enabled: row.get::<_, i32>(10)? != 0,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?;
        rows.collect()
    }

    /// Return the most recent events for a given URL, newest first.
    pub fn get_monitor_events(&self, url_id: &str, limit: u32) -> SqliteResult<Vec<MonitorEvent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT event_id, url_id, event_type, checked_at,
                    content_hash, c2pa_valid, watermark_uuid, watermark_confidence,
                    http_status, response_time_ms,
                    case_status, case_notes, case_updated_at
             FROM monitor_events
             WHERE url_id = ?1
             ORDER BY checked_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![url_id, limit], |row| {
            Ok(MonitorEvent {
                event_id: row.get(0)?,
                url_id: row.get(1)?,
                event_type: row.get(2)?,
                checked_at: row.get(3)?,
                content_hash: row.get(4)?,
                c2pa_valid: row.get::<_, Option<i32>>(5)?.map(|v| v != 0),
                watermark_uuid: row.get(6)?,
                watermark_confidence: row.get(7)?,
                http_status: row.get(8)?,
                response_time_ms: row.get(9)?,
                case_status: row.get(10)?,
                case_notes: row.get(11)?,
                case_updated_at: row.get(12)?,
            })
        })?;
        rows.collect()
    }

    /// Update the case management status and optional notes on a monitor event.
    ///
    /// Also stamps `case_updated_at` with the current UTC time.
    pub fn update_case_status(
        &self,
        event_id: &str,
        status: &str,
        notes: Option<&str>,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE monitor_events
             SET case_status = ?1, case_notes = ?2, case_updated_at = datetime('now')
             WHERE event_id = ?3",
            params![status, notes, event_id],
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

/// A user-submitted false-positive report for a verification result.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FalsePositiveReport {
    /// UUID for this report.
    pub id: String,
    /// The verification result this report relates to, if known.
    pub verification_id: Option<String>,
    /// Structured reason code (e.g. `"deepfake_score_too_high"`, `"ela_codec_artefact"`).
    pub reason_code: String,
    /// Optional free-text explanation from the user.
    pub reason_note: Option<String>,
    /// MIME type of the file that was verified.
    pub mime_type: Option<String>,
    /// Deepfake score at the time of the false-positive report.
    pub deepfake_score: Option<f64>,
    /// Deepfake verdict at the time of the false-positive report.
    pub deepfake_verdict: Option<String>,
    /// ISO-8601 timestamp when the report was submitted.
    pub created_at: String,
}

/// A URL registered for periodic monitoring.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorUrl {
    /// UUID (v4) primary key.
    pub url_id: String,
    /// Optional link to an asset in the local catalogue.
    pub asset_id: Option<String>,
    /// Absolute URL being monitored.
    pub url: String,
    /// User-supplied human label.
    pub label: Option<String>,
    /// Scheduling cadence: `"hourly"` | `"daily"` | `"weekly"`.
    pub check_frequency: String,
    /// ISO 8601 timestamp of the most recent check.
    pub last_checked_at: Option<String>,
    /// Outcome of the most recent check: `"ok"` | `"changed"` | `"missing"` | `"error"`.
    pub last_status: Option<String>,
    /// SHA-256 hex of content at the last check.
    pub last_content_hash: Option<String>,
    /// Whether the C2PA manifest was valid at the last check.
    pub last_c2pa_valid: Option<bool>,
    /// Whether the watermark matched at the last check.
    pub last_watermark_match: Option<bool>,
    /// `true` if monitoring is active; `false` if paused.
    pub enabled: bool,
    /// ISO 8601 timestamp when this URL was first added.
    pub created_at: String,
    /// ISO 8601 timestamp of the most recent metadata update.
    pub updated_at: String,
}

/// One recorded check result for a monitored URL.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorEvent {
    /// UUID (v4) primary key.
    pub event_id: String,
    /// The URL this event belongs to.
    pub url_id: String,
    /// Categorised outcome (e.g. `"check_ok"`, `"content_changed"`, `"c2pa_stripped"`).
    pub event_type: String,
    /// ISO 8601 timestamp when this check was performed.
    pub checked_at: String,
    /// SHA-256 hex of content at this check.
    pub content_hash: Option<String>,
    /// Whether the C2PA manifest was valid at this check.
    pub c2pa_valid: Option<bool>,
    /// Extracted watermark UUID, if any.
    pub watermark_uuid: Option<String>,
    /// Watermark extraction confidence 0.0–1.0.
    pub watermark_confidence: Option<f64>,
    /// HTTP status code returned by the server.
    pub http_status: Option<i32>,
    /// Round-trip response time in milliseconds.
    pub response_time_ms: Option<i32>,
    /// Case management state: `"new"` | `"investigating"` | `"resolved"` | `"escalated"` | `"dismissed"`.
    pub case_status: String,
    /// Free-text notes from the user about this event.
    pub case_notes: Option<String>,
    /// ISO 8601 timestamp of the most recent `case_status` change.
    pub case_updated_at: Option<String>,
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
        db.log_action("verify", "asset", "a2", None, Some("museum_admin"), None)
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

        let images = db.get_filtered_assets(Some("image"), None, None).unwrap();
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

        let signed = db.get_filtered_assets(None, Some(true), None).unwrap();
        assert_eq!(signed.len(), 1);
        assert_eq!(signed[0].asset_id, "a2");

        let unsigned = db.get_filtered_assets(None, Some(false), None).unwrap();
        assert_eq!(unsigned.len(), 1);
        assert_eq!(unsigned[0].asset_id, "a1");
    }

    #[test]
    fn filter_by_search_query() {
        let db = open_temp_db();
        db.insert_asset(&make_asset(
            "a1",
            "sunset_beach.jpg",
            "2026-01-01T00:00:00Z",
        ))
        .unwrap();
        db.insert_asset(&make_asset(
            "a2",
            "mountain_view.jpg",
            "2026-01-02T00:00:00Z",
        ))
        .unwrap();

        let results = db.get_filtered_assets(None, None, Some("sunset")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].asset_id, "a1");

        let all = db.get_filtered_assets(None, None, Some("")).unwrap();
        assert_eq!(all.len(), 2);
    }

    // ── False-positive reports ───────────────────────────────────────

    #[test]
    fn test_insert_and_count_false_positives() {
        let db = open_temp_db();
        assert_eq!(db.get_false_positive_count().unwrap(), 0);

        db.insert_false_positive(
            "fp-report-1",
            Some("ver-abc"),
            Some("sha256:deadbeef"),
            "deepfake_score_too_high",
            Some("Image is a photograph of a painting, not a deepfake."),
            Some("image/jpeg"),
            Some(0.82),
            Some("synthetic"),
            Some(r#"{"ela":0.12,"noise":0.07}"#),
            "2026-03-18T12:00:00Z",
        )
        .unwrap();

        db.insert_false_positive(
            "fp-report-2",
            None,
            None,
            "ela_codec_artefact",
            None,
            Some("image/webp"),
            None,
            None,
            None,
            "2026-03-18T13:00:00Z",
        )
        .unwrap();

        assert_eq!(db.get_false_positive_count().unwrap(), 2);
    }

    #[test]
    fn test_get_false_positive_reports() {
        let db = open_temp_db();

        db.insert_false_positive(
            "r1",
            Some("v1"),
            None,
            "ela_codec_artefact",
            Some("WebP compression artefact."),
            Some("image/webp"),
            Some(0.55),
            Some("inconclusive"),
            None,
            "2026-03-18T10:00:00Z",
        )
        .unwrap();

        db.insert_false_positive(
            "r2",
            None,
            None,
            "noise_false_positive",
            None,
            Some("image/avif"),
            None,
            None,
            None,
            "2026-03-18T11:00:00Z",
        )
        .unwrap();

        let reports = db.get_false_positive_reports().unwrap();
        assert_eq!(reports.len(), 2);
        // Most recent first
        assert_eq!(reports[0].id, "r2");
        assert_eq!(reports[1].id, "r1");

        // Verify round-trip of optional fields
        let r1 = reports.iter().find(|r| r.id == "r1").unwrap();
        assert_eq!(r1.verification_id.as_deref(), Some("v1"));
        assert_eq!(r1.reason_code, "ela_codec_artefact");
        assert_eq!(
            r1.reason_note.as_deref(),
            Some("WebP compression artefact.")
        );
        assert_eq!(r1.mime_type.as_deref(), Some("image/webp"));
        assert!((r1.deepfake_score.unwrap() - 0.55).abs() < 1e-9);
        assert_eq!(r1.deepfake_verdict.as_deref(), Some("inconclusive"));

        let r2 = reports.iter().find(|r| r.id == "r2").unwrap();
        assert!(r2.verification_id.is_none());
        assert!(r2.deepfake_score.is_none());
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

    // ── Monitor queries ─────────────────────────────────────────────

    #[test]
    fn get_audit_log_returns_entries_most_recent_first() {
        let db = open_temp_db();

        db.log_action("import", "asset", "a1", Some("first"), None, None)
            .unwrap();
        db.log_action("verify", "file", "a1", Some("second"), None, None)
            .unwrap();
        db.log_action("sign", "asset", "a1", Some("third"), None, None)
            .unwrap();

        let entries = db.get_audit_log(10, None).unwrap();
        assert_eq!(entries.len(), 3);
        // Most recent sign action should appear first
        assert_eq!(entries[0].action, "sign");
        assert_eq!(entries[2].action, "import");
    }

    #[test]
    fn get_audit_log_respects_limit() {
        let db = open_temp_db();
        for i in 0..10 {
            db.log_action("import", "asset", &format!("a{i}"), None, None, None)
                .unwrap();
        }

        let entries = db.get_audit_log(3, None).unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn get_audit_log_action_filter() {
        let db = open_temp_db();

        db.log_action("import", "asset", "a1", None, None, None)
            .unwrap();
        db.log_action("verify", "file", "a1", None, None, None)
            .unwrap();
        db.log_action("import", "asset", "a2", None, None, None)
            .unwrap();

        let imports = db.get_audit_log(50, Some("import")).unwrap();
        assert_eq!(imports.len(), 2);
        assert!(imports.iter().all(|e| e.action == "import"));

        let verifications = db.get_audit_log(50, Some("verify")).unwrap();
        assert_eq!(verifications.len(), 1);
    }

    #[test]
    fn get_audit_log_entry_fields() {
        let db = open_temp_db();
        db.log_action(
            "import",
            "asset",
            "asset-uuid-1",
            Some("{\"mime\":\"image/jpeg\"}"),
            None,
            None,
        )
        .unwrap();

        let entries = db.get_audit_log(1, None).unwrap();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.action, "import");
        assert_eq!(e.target_type, "asset");
        assert_eq!(e.target_id, "asset-uuid-1");
        assert!(e.details.is_some());
        assert!(!e.log_id.is_empty());
        assert!(!e.created_at.is_empty());
    }

    #[test]
    fn audit_chain_intact_for_single_entry() {
        let db = open_temp_db();
        db.log_action("import", "asset", "a1", None, None, None)
            .unwrap();
        assert!(db.verify_audit_chain().unwrap());
    }

    #[test]
    fn audit_chain_intact_for_multiple_entries() {
        let db = open_temp_db();
        db.log_action("import", "asset", "a1", None, None, None)
            .unwrap();
        db.log_action("verify", "file", "a1", None, None, None)
            .unwrap();
        db.log_action("sign", "asset", "a1", None, None, None)
            .unwrap();
        assert!(db.verify_audit_chain().unwrap());
    }

    #[test]
    fn audit_chain_detects_tampered_entry() {
        let db = open_temp_db();
        db.log_action("import", "asset", "a1", None, None, None)
            .unwrap();
        db.log_action("verify", "file", "a2", None, None, None)
            .unwrap();

        // Directly corrupt the entry_hash of the first row to simulate tampering.
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE audit_log SET entry_hash = 'deadbeef' WHERE target_id = 'a1'",
                [],
            )
            .unwrap();
        }

        assert!(!db.verify_audit_chain().unwrap());
    }

    #[test]
    fn audit_chain_entries_include_hash_columns() {
        let db = open_temp_db();
        db.log_action("import", "asset", "x1", Some("details"), None, None)
            .unwrap();

        let conn = db.conn.lock().unwrap();
        let (prev_hash, entry_hash): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT prev_hash, entry_hash FROM audit_log WHERE target_id = 'x1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();

        // First entry: prev_hash should be the genesis sentinel.
        assert_eq!(prev_hash.as_deref(), Some("genesis"));
        // entry_hash must be a 64-character hex string (SHA-256).
        assert!(entry_hash.is_some());
        assert_eq!(entry_hash.unwrap().len(), 64);
    }

    #[test]
    fn get_verification_history_returns_summaries() {
        let db = open_temp_db();

        db.insert_verification(
            "v1",
            "file",
            "image",
            Some(0.12),
            Some(0.08),
            Some(true),
            &[],
            0.85,
        )
        .unwrap();
        db.insert_verification(
            "v2",
            "url",
            "image",
            Some(0.65),
            Some(0.72),
            None,
            &[],
            0.35,
        )
        .unwrap();

        let history = db.get_verification_history(10, 0).unwrap();
        assert_eq!(history.len(), 2);

        // Most recent first — v2 was inserted later so it should be first
        let v2 = &history[0];
        assert_eq!(v2.verification_id, "v2");
        assert_eq!(v2.source_type, "url");
        assert!((v2.overall_trust - 0.35).abs() < 1e-9);
        assert!(v2.c2pa_valid.is_none());

        let v1 = &history[1];
        assert_eq!(v1.verification_id, "v1");
        assert_eq!(v1.c2pa_valid, Some(true));
        assert!((v1.ela_score.unwrap() - 0.12).abs() < 1e-9);
    }

    #[test]
    fn get_verification_history_pagination() {
        let db = open_temp_db();

        for i in 0..5u32 {
            db.insert_verification(
                &format!("v{i}"),
                "file",
                "image",
                None,
                None,
                None,
                &[],
                0.5,
            )
            .unwrap();
        }

        let page1 = db.get_verification_history(2, 0).unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = db.get_verification_history(2, 2).unwrap();
        assert_eq!(page2.len(), 2);

        // Pages should not overlap
        let ids1: Vec<_> = page1.iter().map(|v| &v.verification_id).collect();
        let ids2: Vec<_> = page2.iter().map(|v| &v.verification_id).collect();
        assert!(ids1.iter().all(|id| !ids2.contains(id)));
    }

    #[test]
    fn get_trust_distribution_empty_db() {
        let db = open_temp_db();
        let dist = db.get_trust_distribution().unwrap();
        assert_eq!(dist.total, 0);
        assert_eq!(dist.high_count, 0);
        assert_eq!(dist.medium_count, 0);
        assert_eq!(dist.low_count, 0);
        assert!((dist.average_trust - 0.0).abs() < 1e-9);
        assert!(dist.latest_at.is_none());
    }

    #[test]
    fn get_trust_distribution_bucketing() {
        let db = open_temp_db();

        // High: >= 0.7
        db.insert_verification("v1", "file", "image", None, None, None, &[], 0.9)
            .unwrap();
        db.insert_verification("v2", "file", "image", None, None, None, &[], 0.7)
            .unwrap();
        // Medium: >= 0.4 and < 0.7
        db.insert_verification("v3", "file", "image", None, None, None, &[], 0.5)
            .unwrap();
        // Low: < 0.4
        db.insert_verification("v4", "file", "image", None, None, None, &[], 0.2)
            .unwrap();

        let dist = db.get_trust_distribution().unwrap();
        assert_eq!(dist.total, 4);
        assert_eq!(dist.high_count, 2);
        assert_eq!(dist.medium_count, 1);
        assert_eq!(dist.low_count, 1);
        assert!(dist.average_trust > 0.0);
        assert!(dist.latest_at.is_some());
    }

    #[test]
    fn get_protection_summary_empty_db() {
        let db = open_temp_db();
        let summary = db.get_protection_summary().unwrap();
        assert_eq!(summary.total_assets, 0);
        assert_eq!(summary.c2pa_signed, 0);
        assert_eq!(summary.watermarked, 0);
        assert_eq!(summary.fingerprinted, 0);
        assert!(summary.by_content_type.is_empty());
        assert!(summary.earliest_at.is_none());
        assert!(summary.latest_at.is_none());
    }

    #[test]
    fn get_protection_summary_counts() {
        let db = open_temp_db();

        let a1 = make_asset("a1", "photo.jpg", "2026-01-01T00:00:00Z");
        db.insert_asset(&a1).unwrap();
        db.set_c2pa_signed("a1", "/tmp/photo_c2pa.jpg").unwrap();

        let mut a2 = make_asset("a2", "other.jpg", "2026-01-02T00:00:00Z");
        a2.content_type = "document".to_string();
        db.insert_asset(&a2).unwrap();

        db.insert_fingerprint("fp1", "a1", "ahash", "ff00ff00ff00ff00")
            .unwrap();

        let summary = db.get_protection_summary().unwrap();
        assert_eq!(summary.total_assets, 2);
        assert_eq!(summary.c2pa_signed, 1);
        assert_eq!(summary.fingerprinted, 1);
        assert_eq!(*summary.by_content_type.get("image").unwrap(), 1);
        assert_eq!(*summary.by_content_type.get("document").unwrap(), 1);
        assert!(summary.earliest_at.is_some());
        assert!(summary.latest_at.is_some());
    }

    #[test]
    fn get_activity_timeline_empty_db() {
        let db = open_temp_db();
        let days = db.get_activity_timeline(30).unwrap();
        assert!(days.is_empty());
    }

    #[test]
    fn get_activity_timeline_counts_actions() {
        let db = open_temp_db();

        // All within the last 30 days
        let now = chrono::Utc::now().to_rfc3339();
        let conn = db.conn.lock().unwrap();
        // Insert directly to control timestamps precisely
        conn.execute(
            "INSERT INTO audit_log (log_id, action, target_type, target_id, details, operator_id, created_at)
             VALUES ('l1', 'import', 'asset', 'a1', NULL, 'local_user', ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO audit_log (log_id, action, target_type, target_id, details, operator_id, created_at)
             VALUES ('l2', 'import', 'asset', 'a2', NULL, 'local_user', ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO audit_log (log_id, action, target_type, target_id, details, operator_id, created_at)
             VALUES ('l3', 'verify', 'file', 'a1', NULL, 'local_user', ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO audit_log (log_id, action, target_type, target_id, details, operator_id, created_at)
             VALUES ('l4', 'sign', 'asset', 'a1', NULL, 'local_user', ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO audit_log (log_id, action, target_type, target_id, details, operator_id, created_at)
             VALUES ('l5', 'delete', 'asset', 'a3', NULL, 'local_user', ?1)",
            params![now],
        )
        .unwrap();
        drop(conn);

        let days = db.get_activity_timeline(30).unwrap();
        assert_eq!(days.len(), 1);

        let today = &days[0];
        assert_eq!(today.imports, 2);
        assert_eq!(today.verifications, 1);
        assert_eq!(today.signings, 1);
        assert_eq!(today.deletions, 1);
    }

    #[test]
    fn get_activity_timeline_excludes_old_entries() {
        let db = open_temp_db();

        // One recent, one old (beyond the window)
        let recent = chrono::Utc::now().to_rfc3339();
        let old = "2020-01-01T00:00:00Z";
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO audit_log (log_id, action, target_type, target_id, details, operator_id, created_at)
             VALUES ('r1', 'import', 'asset', 'a1', NULL, 'local_user', ?1)",
            params![recent],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO audit_log (log_id, action, target_type, target_id, details, operator_id, created_at)
             VALUES ('o1', 'import', 'asset', 'a2', NULL, 'local_user', ?1)",
            params![old],
        )
        .unwrap();
        drop(conn);

        let days = db.get_activity_timeline(30).unwrap();
        // Only the recent entry should appear; the 2020 one is outside the window
        let total_imports: u64 = days.iter().map(|d| d.imports).sum();
        assert_eq!(total_imports, 1);
    }

    // ── Monitor URLs ─────────────────────────────────────────────────

    #[test]
    fn test_add_and_list_monitor_urls() {
        let db = open_temp_db();

        db.add_monitor_url(
            "https://example.com/image1.jpg",
            Some("Test Image 1"),
            None,
            "daily",
        )
        .unwrap();
        db.add_monitor_url("https://example.com/image2.jpg", None, None, "weekly")
            .unwrap();

        let urls = db.list_monitor_urls(false).unwrap();
        assert_eq!(urls.len(), 2);

        // Both should be enabled by default
        assert!(urls.iter().all(|u| u.enabled));

        // check_frequency should be preserved
        let u1 = urls
            .iter()
            .find(|u| u.label.as_deref() == Some("Test Image 1"))
            .unwrap();
        assert_eq!(u1.check_frequency, "daily");
        assert_eq!(u1.url, "https://example.com/image1.jpg");

        let u2 = urls
            .iter()
            .find(|u| u.url == "https://example.com/image2.jpg")
            .unwrap();
        assert_eq!(u2.check_frequency, "weekly");
        assert!(u2.label.is_none());
    }

    #[test]
    fn test_remove_monitor_url_cascades_events() {
        let db = open_temp_db();

        let monitor = db
            .add_monitor_url("https://example.com/photo.jpg", None, None, "daily")
            .unwrap();
        let url_id = monitor.url_id.clone();

        // Insert an event directly via raw SQL to simulate a check having been run
        {
            let conn = db.conn.lock().unwrap();
            let event_id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO monitor_events
                 (event_id, url_id, event_type, checked_at, case_status)
                 VALUES (?1, ?2, 'check_ok', datetime('now'), 'new')",
                params![event_id, url_id],
            )
            .unwrap();
        }

        // Confirm the event exists
        let events = db.get_monitor_events(&url_id, 10).unwrap();
        assert_eq!(events.len(), 1);

        // Remove the URL — should CASCADE-delete the event
        db.remove_monitor_url(&url_id).unwrap();

        // URL is gone
        let urls = db.list_monitor_urls(false).unwrap();
        assert!(urls.is_empty());

        // Events are gone too
        let events_after = db.get_monitor_events(&url_id, 10).unwrap();
        assert!(events_after.is_empty());
    }

    #[test]
    fn test_update_case_status() {
        let db = open_temp_db();

        let monitor = db
            .add_monitor_url("https://example.com/doc.pdf", None, None, "daily")
            .unwrap();
        let url_id = monitor.url_id.clone();

        // Insert an event directly
        let event_id = uuid::Uuid::new_v4().to_string();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO monitor_events
                 (event_id, url_id, event_type, checked_at, case_status)
                 VALUES (?1, ?2, 'content_changed', datetime('now'), 'new')",
                params![event_id, url_id],
            )
            .unwrap();
        }

        // Verify initial state
        let events = db.get_monitor_events(&url_id, 10).unwrap();
        assert_eq!(events[0].case_status, "new");
        assert!(events[0].case_notes.is_none());

        // Update the status
        db.update_case_status(
            &event_id,
            "investigating",
            Some("Checking with rights holder"),
        )
        .unwrap();

        let events = db.get_monitor_events(&url_id, 10).unwrap();
        assert_eq!(events[0].case_status, "investigating");
        assert_eq!(
            events[0].case_notes.as_deref(),
            Some("Checking with rights holder")
        );
        assert!(events[0].case_updated_at.is_some());
    }

    #[test]
    fn test_list_monitor_urls_enabled_filter() {
        let db = open_temp_db();

        let enabled = db
            .add_monitor_url(
                "https://example.com/active.jpg",
                Some("Active"),
                None,
                "daily",
            )
            .unwrap();

        // Add a second URL then disable it via raw SQL
        let disabled = db
            .add_monitor_url(
                "https://example.com/paused.jpg",
                Some("Paused"),
                None,
                "weekly",
            )
            .unwrap();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE monitor_urls SET enabled = 0 WHERE url_id = ?1",
                params![disabled.url_id],
            )
            .unwrap();
        }

        // Without filter: both returned
        let all = db.list_monitor_urls(false).unwrap();
        assert_eq!(all.len(), 2);

        // With filter: only the enabled one
        let active = db.list_monitor_urls(true).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].url_id, enabled.url_id);
        assert!(active[0].enabled);
    }
}
