//! FP Telemetry — Phase A client-side scaffold.
//!
//! This module is the client-side half of the false-positive telemetry
//! pipeline described in `docs/design/fp-telemetry-endpoint.md`.
//!
//! # What this module does (Phase A — no network calls)
//!
//! - Defines [`FpTelemetryReport`]: the wire-format struct that will be sent
//!   to the Jura Labs telemetry endpoint when Phase B is implemented.
//! - Provides [`build_report_from_fp_row`]: constructs a telemetry payload
//!   from a local [`FalsePositiveReport`] row, stripping all fields that must
//!   not leave the device (filenames, timestamps beyond day granularity, etc.).
//! - Provides [`serialize_report`]: emits the JSON wire format.
//! - Provides [`telemetry_enabled`]: reads the opt-in config flag. **Default:
//!   false.** No data leaves the device while this returns false.
//!
//! # What this module deliberately does NOT do (Phase A)
//!
//! The actual HTTP upload is a TODO stub. Phase B wires in the `reqwest`
//! client, exponential-backoff retry logic, and the upload queue. See the
//! design document for the full phased rollout plan.
//!
//! # Phase B implementation note
//!
//! When Phase B is ready, replace the body of [`upload_report`] with a
//! `reqwest` POST to `https://telemetry.juralabs.org/v1/fp-reports`.
//! The upload must:
//!
//! - Only fire when [`telemetry_enabled`] returns `true`.
//! - Include `Authorization: Bearer jt_inst_<install_uuid>` from config.
//! - Retry with exponential backoff (30s → 5min → 1hr → 24hr).
//! - Mark the local `false_positive_reports.uploaded_at` column on success.
//!
//! See `docs/design/fp-telemetry-endpoint.md` Part E for the full client
//! wiring plan.

use serde::{Deserialize, Serialize};

use crate::db::FalsePositiveReport;

// ── Wire-format schema version ────────────────────────────────────────────────

/// Current telemetry payload schema version.
///
/// Increment this integer when the payload structure changes in a breaking way
/// (e.g. new required fields, changed encoding, feature vector length changes).
/// The server refuses payloads with unsupported schema versions and returns a
/// descriptive 400 error instructing the user to upgrade.
pub const SCHEMA_VERSION: u8 = 1;

// ── FpTelemetryReport ─────────────────────────────────────────────────────────

/// Wire-format payload for a single false-positive report.
///
/// This struct is designed to carry only anonymised, non-identifying data.
/// See `docs/design/fp-telemetry-endpoint.md` Part B for the full field
/// justification and the list of local fields that are intentionally excluded.
///
/// Fields that must NEVER appear in this struct:
/// - File path or filename
/// - Full ISO-8601 timestamp (date only is transmitted)
/// - Raw image bytes or perceptual hash values
/// - GPS coordinates
/// - `reason_note` (free text; may contain user-typed PII)
/// - `verification_id` or `file_hash` (local identifiers)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FpTelemetryReport {
    /// UUID v4 — matches the local `false_positive_reports.id` column.
    /// Used for idempotent retries: the server deduplicates on this field.
    pub report_id: String,

    /// Schema version. Always [`SCHEMA_VERSION`] for reports built by this
    /// module. The server uses this to select the correct ingestion path.
    pub schema_version: u8,

    /// Reason code from the in-app modal.
    /// One of: `modern_codec`, `social_media`, `scanner`,
    /// `computational_photography`, `other`.
    pub reason_code: String,

    /// MIME type of the analysed file (e.g. `"image/jpeg"`).
    /// No filename or path is included.
    pub mime_type: Option<String>,

    /// Deepfake score (0.0–1.0) at the time the false-positive was flagged.
    pub deepfake_score: Option<f64>,

    /// Three-way verdict: `"authentic"` | `"inconclusive"` | `"synthetic"`.
    pub deepfake_verdict: Option<String>,

    /// Feature vector from the deepfake pipeline's 80–84 element feature
    /// extractor. `None` when the deepfake detector did not run (quick mode,
    /// video, PDF).
    ///
    /// Encoded as a JSON array of `f32` values. See Part B of the design
    /// document for the rationale for this encoding over base64 binary or
    /// Protobuf.
    pub feature_vector: Option<Vec<f32>>,

    /// Number of elements in `feature_vector`. Sent separately so the server
    /// can detect schema drift without parsing the array.
    pub feature_vector_length: Option<usize>,

    /// Calendar date the report was created, truncated to day granularity:
    /// `"YYYY-MM-DD"`. The full timestamp is never transmitted.
    pub reported_date: String,

    /// Jura Trace application version that produced this report.
    pub app_version: String,
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a [`FpTelemetryReport`] from a local [`FalsePositiveReport`] row.
///
/// This function is the anonymisation boundary. It copies only the fields that
/// are safe to transmit and performs the timestamp truncation (full RFC-3339
/// → date-only `YYYY-MM-DD`).
///
/// The `signal_scores_json` field from the local DB is *not* included in the
/// telemetry payload. The feature vector (when available) supersedes it.
/// Raw signal scores are an implementation detail and may contain fields whose
/// semantics change between versions.
///
/// `feature_vector` must be extracted separately from the sidecar result
/// before calling this function — it is not stored in the local DB row.
/// Pass `None` when the deepfake detector did not run.
pub fn build_report_from_fp_row(
    row: &FalsePositiveReport,
    feature_vector: Option<Vec<f32>>,
) -> FpTelemetryReport {
    let reported_date = truncate_to_date(&row.created_at);
    let fv_length = feature_vector.as_ref().map(|v| v.len());

    FpTelemetryReport {
        report_id: row.id.clone(),
        schema_version: SCHEMA_VERSION,
        reason_code: row.reason_code.clone(),
        mime_type: row.mime_type.clone(),
        deepfake_score: row.deepfake_score,
        deepfake_verdict: row.deepfake_verdict.clone(),
        feature_vector,
        feature_vector_length: fv_length,
        reported_date,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    }
    // NOTE: the following local fields are deliberately excluded:
    //   row.verification_id — local identifier, useless to server
    //   row.reason_note     — free text, may contain PII
    //   row.created_at      — replaced by date-only `reported_date`
}

// ── Serialiser ────────────────────────────────────────────────────────────────

/// Serialise an [`FpTelemetryReport`] to the JSON wire format.
///
/// Returns a compact (no pretty-print) JSON string. The server accepts this
/// as `Content-Type: application/json`.
///
/// # Errors
///
/// Returns an error string if `serde_json` fails. In practice this should not
/// happen for a well-formed [`FpTelemetryReport`].
pub fn serialize_report(report: &FpTelemetryReport) -> Result<String, String> {
    serde_json::to_string(report).map_err(|e| format!("Failed to serialise telemetry report: {e}"))
}

// ── Opt-in gate ───────────────────────────────────────────────────────────────

/// Return `true` if the user has opted in to anonymous FP telemetry.
///
/// **Default: `false`.** No data leaves the device unless this returns `true`.
///
/// Phase A stub: reads the `JURA_TELEMETRY_ENABLED` environment variable.
/// Phase B implementation must read from `config.json` (the `telemetry_enabled`
/// key set by the Settings toggle or setup wizard checkbox).
///
/// The environment variable approach is intentional for Phase A: it allows
/// automated tests to verify the opt-in gate without touching `config.json`,
/// and it is impossible to accidentally enable telemetry in a production build
/// without an explicit environment variable (which no user would set).
pub fn telemetry_enabled() -> bool {
    // Phase A: always returns false unless the env var is set for testing.
    // Phase B: replace with config.json read:
    //   config.get("telemetry_enabled").and_then(|v| v.as_bool()).unwrap_or(false)
    std::env::var("JURA_TELEMETRY_ENABLED")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

// ── Upload stub ───────────────────────────────────────────────────────────────

/// Attempt to upload a single [`FpTelemetryReport`] to the Jura Labs endpoint.
///
/// # Phase A (current)
///
/// This function is a stub. It logs that it was called but makes no network
/// connection. It always returns `Ok(())`.
///
/// # Phase B implementation
///
/// Replace this body with:
/// 1. Check `telemetry_enabled()` — return early if false.
/// 2. Read install UUID from config.json.
/// 3. POST the serialised payload to `https://telemetry.juralabs.org/v1/fp-reports`
///    with `Authorization: Bearer jt_inst_<uuid>` and
///    `X-Jura-Schema-Version: 1`.
/// 4. On 201 or 200 → mark the local row `uploaded_at = now()`.
/// 5. On 429 → back off per the retry schedule in Part E.3 of the design doc.
/// 6. On 400 → log and abandon (not retryable; schema mismatch).
/// 7. On 5xx or network error → retry with exponential backoff.
///
/// See `docs/design/fp-telemetry-endpoint.md` Part E for the full plan.
///
/// # Errors
///
/// Returns an error string on serialisation failure. Network errors are
/// handled internally (retry logic); they do not propagate as errors from
/// this function.
pub fn upload_report(report: &FpTelemetryReport) -> Result<(), String> {
    if !telemetry_enabled() {
        log::debug!(
            "Telemetry disabled — FP report {} not uploaded",
            report.report_id
        );
        return Ok(());
    }

    // TODO(Phase B): replace this stub with the actual HTTP upload.
    // See the Phase B implementation note in the function doc-comment above.
    log::info!(
        "[TELEMETRY STUB] Would upload FP report {} (reason={}, schema_version={})",
        report.report_id,
        report.reason_code,
        report.schema_version,
    );

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Truncate an RFC-3339 timestamp to a `YYYY-MM-DD` date string.
///
/// If the input cannot be parsed as a date prefix, the full string is returned
/// as a safe fallback (the server will reject malformed dates, which is the
/// desired behaviour — better to fail loudly than silently transmit unexpected
/// data).
fn truncate_to_date(timestamp: &str) -> String {
    // RFC-3339 dates always begin with `YYYY-MM-DD` (10 chars).
    if timestamp.len() >= 10 && timestamp.chars().nth(4) == Some('-') {
        timestamp[..10].to_string()
    } else {
        // Unexpected format — return as-is so the server can reject it
        // with a descriptive error rather than silently mangling the date.
        timestamp.to_string()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fp_row(id: &str, created_at: &str) -> FalsePositiveReport {
        FalsePositiveReport {
            id: id.to_string(),
            verification_id: Some("ver-123".to_string()),
            reason_code: "modern_codec".to_string(),
            reason_note: Some("This is a HEIC file from an iPhone".to_string()),
            mime_type: Some("image/heic".to_string()),
            deepfake_score: Some(0.72),
            deepfake_verdict: Some("synthetic".to_string()),
            created_at: created_at.to_string(),
        }
    }

    #[test]
    fn build_report_copies_safe_fields() {
        let row = make_fp_row("rpt-abc-123", "2026-04-07T14:32:01Z");
        let report = build_report_from_fp_row(&row, None);

        assert_eq!(report.report_id, "rpt-abc-123");
        assert_eq!(report.reason_code, "modern_codec");
        assert_eq!(report.mime_type, Some("image/heic".to_string()));
        assert_eq!(report.deepfake_score, Some(0.72));
        assert_eq!(report.deepfake_verdict, Some("synthetic".to_string()));
        assert_eq!(report.schema_version, SCHEMA_VERSION);
        assert_eq!(report.app_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn build_report_strips_pii_fields() {
        // reason_note must not appear in the telemetry payload.
        // verification_id and the full timestamp must not appear.
        let row = make_fp_row("rpt-strip-test", "2026-04-07T14:32:01+01:00");
        let report = build_report_from_fp_row(&row, None);

        // Serialise and check raw JSON to make sure the fields are absent.
        let json = serialize_report(&report).unwrap();
        assert!(
            !json.contains("reason_note"),
            "reason_note must not appear in telemetry payload: {json}"
        );
        assert!(
            !json.contains("verification_id"),
            "verification_id must not appear in telemetry payload: {json}"
        );
        // Full timestamp must be stripped; date-only must appear.
        assert!(
            !json.contains("14:32:01"),
            "full timestamp must not appear in telemetry payload: {json}"
        );
        assert!(
            json.contains("2026-04-07"),
            "date-only string must appear in telemetry payload: {json}"
        );
    }

    #[test]
    fn serialize_report_produces_valid_json_with_expected_keys() {
        let row = make_fp_row("rpt-ser-test", "2026-04-07T09:00:00Z");
        let fv = Some(vec![0.1_f32, 0.2, 0.3]);
        let report = build_report_from_fp_row(&row, fv);

        let json = serialize_report(&report).expect("serialisation must succeed");

        // Must be parseable.
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("output must be valid JSON");

        // Required keys must be present.
        assert!(parsed["report_id"].is_string());
        assert!(parsed["schema_version"].is_number());
        assert!(parsed["reason_code"].is_string());
        assert!(parsed["reported_date"].is_string());
        assert!(parsed["app_version"].is_string());

        // Feature vector must be present and have the right length.
        assert!(parsed["feature_vector"].is_array());
        assert_eq!(parsed["feature_vector_length"], 3);
    }

    #[test]
    fn telemetry_enabled_is_off_by_default() {
        // Ensure JURA_TELEMETRY_ENABLED is not set in the test environment.
        // (CI and developer machines must not have this set.)
        // We cannot guarantee the env is clean, so we just verify the function
        // returns false when the env var is unset or empty.
        // If the env var IS set (e.g. someone is testing Phase B integration),
        // this test is a no-op — that is intentional.
        if std::env::var("JURA_TELEMETRY_ENABLED").is_err() {
            assert!(
                !telemetry_enabled(),
                "telemetry_enabled() must return false by default when env var is unset"
            );
        }
    }

    #[test]
    fn truncate_to_date_handles_various_formats() {
        assert_eq!(truncate_to_date("2026-04-07T14:32:01Z"), "2026-04-07");
        assert_eq!(truncate_to_date("2026-04-07T14:32:01+01:00"), "2026-04-07");
        assert_eq!(truncate_to_date("2026-04-07"), "2026-04-07");
        // Unexpected format falls through unchanged.
        assert_eq!(truncate_to_date("not-a-date"), "not-a-date");
    }
}
