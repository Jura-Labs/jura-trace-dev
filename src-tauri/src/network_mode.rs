//! Product-wide network access control.
//!
//! `NetworkMode::Enhanced` (default) enables online verification features —
//! C2PA OCSP/CRL revocation checks, remote manifest fetching, FP telemetry,
//! URL watchlist HTTP fetches.  `NetworkMode::Standard` is an explicit user
//! opt-in for fully local / air-gapped operation.
//!
//! Localhost calls (Ollama on port 11434, Python sidecar on port 8200, the
//! local REST API on port 8300) are **not** outbound network traffic and are
//! therefore always permitted regardless of mode.
//!
//! All network-touching code paths call [`is_enhanced`] before making any
//! external request.  That function is the single choke point — every future
//! gated feature must use it.
//!
//! # Default change — 2026-04-25
//!
//! Default flipped from `Standard` to `Enhanced` after the C2PA Validator
//! evaluation found that real-world content credentials (Pixel manifests,
//! Adobe-issued certs, remote manifest URLs) require network access for
//! correct trust assessment.  The local-first USP is preserved as an
//! explicit user choice (Settings → Network Access → Standard) rather than
//! the default; surveys of pilot interviewees indicated near-zero demand
//! for offline-only operation outside specialist archival contexts.

use serde::{Deserialize, Serialize};
use std::path::Path;

// ===== Type =====

/// Controls whether the application may make outbound HTTP calls.
///
/// `Enhanced` (default): online verification features enabled (OCSP/CRL
/// revocation, remote manifest fetch, FP telemetry, URL watchlist HTTP
/// checks).
///
/// `Standard`: explicit user opt-in for fully local / air-gapped operation.
/// No outbound requests of any kind are made to external hosts.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum NetworkMode {
    Standard,
    #[default]
    Enhanced,
}

// ===== On-disk config =====

/// Shape of `<data_dir>/network_mode.json`.
#[derive(Debug, Default, Serialize, Deserialize)]
struct NetworkModeConfig {
    mode: NetworkMode,
}

fn config_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("network_mode.json")
}

// ===== Public API =====

/// Read the current `NetworkMode` from `<data_dir>/network_mode.json`.
///
/// Returns `NetworkMode::Enhanced` when the file is absent (first-run default)
/// or when it cannot be parsed.  Never panics.
pub fn get_network_mode(data_dir: &Path) -> NetworkMode {
    let path = config_path(data_dir);
    if !path.exists() {
        return NetworkMode::Enhanced;
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return NetworkMode::Enhanced,
    };
    serde_json::from_str::<NetworkModeConfig>(&raw)
        .map(|c| c.mode)
        .unwrap_or_default()
}

/// Persist `mode` to `<data_dir>/network_mode.json`.
///
/// Written atomically (`.tmp` then rename) to avoid a partially-written file
/// being read on the next call.
pub fn set_network_mode(data_dir: &Path, mode: NetworkMode) -> Result<(), String> {
    let config = NetworkModeConfig { mode };
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialise network mode: {e}"))?;
    let path = config_path(data_dir);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, json.as_bytes())
        .map_err(|e| format!("Failed to write network mode config: {e}"))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| format!("Failed to commit network mode config: {e}"))?;
    Ok(())
}

/// Returns `true` when the application may make outbound network requests.
///
/// This is the single choke point for all external HTTP calls.  Call this
/// before any request to an external host (OCSP, CRL, remote manifest, FP
/// telemetry, URL watchlist fetch).  Localhost calls are always exempt.
///
/// ```rust,ignore
/// if !network_mode::is_enhanced(&data_dir) {
///     return Err("Network access is disabled. Switch to Enhanced mode in Settings.".into());
/// }
/// ```
pub fn is_enhanced(data_dir: &Path) -> bool {
    get_network_mode(data_dir) == NetworkMode::Enhanced
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("create tempdir")
    }

    #[test]
    fn default_is_enhanced_when_file_absent() {
        let dir = tmp();
        assert_eq!(get_network_mode(dir.path()), NetworkMode::Enhanced);
    }

    #[test]
    fn is_enhanced_true_by_default() {
        let dir = tmp();
        assert!(is_enhanced(dir.path()));
    }

    #[test]
    fn roundtrip_standard() {
        let dir = tmp();
        set_network_mode(dir.path(), NetworkMode::Standard).expect("set standard");
        assert_eq!(get_network_mode(dir.path()), NetworkMode::Standard);
        assert!(!is_enhanced(dir.path()));
    }

    #[test]
    fn roundtrip_enhanced() {
        let dir = tmp();
        set_network_mode(dir.path(), NetworkMode::Enhanced).expect("set enhanced");
        assert_eq!(get_network_mode(dir.path()), NetworkMode::Enhanced);
        assert!(is_enhanced(dir.path()));
    }

    #[test]
    fn toggle_to_standard_then_back_to_enhanced() {
        let dir = tmp();
        set_network_mode(dir.path(), NetworkMode::Standard).expect("set standard");
        assert_eq!(get_network_mode(dir.path()), NetworkMode::Standard);
        set_network_mode(dir.path(), NetworkMode::Enhanced).expect("toggle back");
        assert_eq!(get_network_mode(dir.path()), NetworkMode::Enhanced);
    }

    #[test]
    fn corrupted_file_defaults_to_enhanced() {
        let dir = tmp();
        // Write garbage so parse fails — falls back to the typed Default
        // (Enhanced) via unwrap_or_default().
        std::fs::write(dir.path().join("network_mode.json"), b"not json").unwrap();
        assert_eq!(get_network_mode(dir.path()), NetworkMode::Enhanced);
        assert!(is_enhanced(dir.path()));
    }

    #[test]
    fn json_serialises_as_camel_case() {
        let json = serde_json::to_string(&NetworkMode::Enhanced).unwrap();
        assert_eq!(json, r#""enhanced""#);
        let json = serde_json::to_string(&NetworkMode::Standard).unwrap();
        assert_eq!(json, r#""standard""#);
    }
}
