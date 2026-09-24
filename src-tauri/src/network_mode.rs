// SPDX-License-Identifier: AGPL-3.0-or-later

//! Product-wide network access control.
//!
//! `NetworkMode::Enhanced` (default) allows the optional outbound calls the
//! app makes on its own: the daily update check at launch, the Open-Meteo
//! weather lookup, and Watched Locations fetches. `NetworkMode::Standard`
//! turns those off. (OCSP/CRL and remote manifest fetching are not
//! implemented: c2pa is built without `fetch_remote_manifests`, and the
//! `enhanced` flag in `c2pa::read_manifest` is a label.)
//!
//! Standard mode is NOT "no outbound requests of any kind", and this header
//! said it was until 24 September 2026 (BL-CLAIM-004). Three calls happen
//! because a person asked for them, in either mode: checking for updates by
//! hand, verifying a URL, and the RFC 3161 timestamp request when signing.
//! The signing timestamp is governed by [`signing_timestamp`]: in Standard
//! mode it is sent only if the user said yes when asked, and signing is
//! refused until they have answered.
//!
//! Localhost calls (Ollama on port 11434, Python sidecar on port 8200, the
//! local REST API on port 8300) are **not** outbound network traffic and are
//! therefore always permitted regardless of mode.
//!
//! Every automatic outbound call must check [`is_enhanced`] first. Calls a
//! person explicitly asks for are listed above and must say so in the UI.
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
/// `Standard`: explicit user opt-in to turn off automatic outbound calls.
/// Calls a person explicitly asks for still happen; see the module header.
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
    /// The answer to "timestamp signatures while in Standard mode?", asked
    /// once at the first signing in Standard mode (BL-CLAIM-004, option 3).
    /// `None` means not asked yet. Absent in files written before
    /// 24 September 2026, which therefore parse as not asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    standard_mode_timestamp: Option<bool>,
}

/// Read the whole config, or `None` when the file is absent, unreadable or
/// unparseable. Callers decide what each of those means.
fn read_config(data_dir: &Path) -> Option<NetworkModeConfig> {
    let raw = std::fs::read_to_string(config_path(data_dir)).ok()?;
    serde_json::from_str::<NetworkModeConfig>(&raw).ok()
}

/// Write the whole config atomically (`.tmp` then rename).
fn write_config(data_dir: &Path, config: &NetworkModeConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialise network mode: {e}"))?;
    let path = config_path(data_dir);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, json.as_bytes())
        .map_err(|e| format!("Failed to write network mode config: {e}"))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| format!("Failed to commit network mode config: {e}"))?;
    Ok(())
}

fn config_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("network_mode.json")
}

// ===== Public API =====

/// Read the current `NetworkMode` from `<data_dir>/network_mode.json`.
///
/// Three cases, and they are deliberately not the same:
///
/// * **File absent** — first run, nobody has chosen. Returns `Enhanced`,
///   the product default set on 2026-04-25 (see the module header). This is
///   the only path that yields that default.
/// * **File unreadable** — returns `Standard`, and warns.
/// * **File unparseable** — returns `Standard`, and warns.
///
/// The last two used to return `Enhanced`, via `unwrap_or_default()`. That
/// was a silent failure in the permissive direction: if the file exists then
/// a user has chosen at some point, and when we cannot tell what they chose,
/// guessing "make network calls" can reverse an explicit air-gapped opt-out
/// without telling anybody.
///
/// The harm is asymmetric. A Standard user silently flipped to Enhanced
/// leaks a DNS query, a TLS SNI and traffic timing, which for a field
/// user under surveillance is unrecoverable once sent. An Enhanced user
/// silently dropped to Standard gets a weaker trust assessment, which is
/// visible in the verify UI and fixed by re-toggling a setting.
///
/// This also matches what the rest of the codebase already does: the C2PA
/// read paths in `lib.rs` use `.map(|d| is_enhanced(&d)).unwrap_or(false)`,
/// failing closed when the data directory cannot be resolved. Cases 2 and 3
/// were the inconsistency, not this change.
///
/// Never panics.
pub fn get_network_mode(data_dir: &Path) -> NetworkMode {
    let path = config_path(data_dir);
    if !path.exists() {
        return NetworkMode::Enhanced;
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            log::warn!(
                "network_mode.json exists but could not be read ({e}); \
                 falling back to Standard (fully offline) rather than \
                 guessing that outbound requests are wanted. \
                 Re-select your preference in Settings > Network Access."
            );
            return NetworkMode::Standard;
        }
    };
    match serde_json::from_str::<NetworkModeConfig>(&raw) {
        Ok(c) => c.mode,
        Err(e) => {
            log::warn!(
                "network_mode.json could not be parsed ({e}); \
                 falling back to Standard (fully offline) rather than \
                 guessing that outbound requests are wanted. \
                 Re-select your preference in Settings > Network Access."
            );
            NetworkMode::Standard
        }
    }
}

/// True when a `network_mode.json` exists but cannot be read or parsed, so
/// the mode in force is the safe fallback rather than the user's choice.
///
/// Exposed so the UI can show a one-time notice telling the user their
/// preference could not be read and the app is running fully offline.
/// Without that, failing closed is silent in the other direction: an
/// Enhanced user would lose online verification and never learn why.
pub fn network_mode_is_degraded(data_dir: &Path) -> bool {
    let path = config_path(data_dir);
    if !path.exists() {
        return false;
    }
    match std::fs::read_to_string(&path) {
        Err(_) => true,
        Ok(raw) => serde_json::from_str::<NetworkModeConfig>(&raw).is_err(),
    }
}

/// Persist `mode` to `<data_dir>/network_mode.json`.
///
/// Written atomically (`.tmp` then rename) to avoid a partially-written file
/// being read on the next call.
///
/// Keeps the remembered signing-timestamp answer: switching mode must not
/// silently forget what the user said.
pub fn set_network_mode(data_dir: &Path, mode: NetworkMode) -> Result<(), String> {
    let standard_mode_timestamp = read_config(data_dir).and_then(|c| c.standard_mode_timestamp);
    write_config(
        data_dir,
        &NetworkModeConfig {
            mode,
            standard_mode_timestamp,
        },
    )
}

/// What signing should do about the RFC 3161 timestamp right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SigningTimestamp {
    /// Request a trusted timestamp (the only behaviour before 24 Sep 2026).
    Use,
    /// Sign without one. The seal then carries no proof of when.
    Skip,
    /// Standard mode and the user has not answered yet. Signing must not
    /// proceed, because either default would decide for them.
    Ask,
}

/// The remembered Standard-mode answer, `None` if not asked yet or if the
/// file cannot be read (treated as not asked: never guess "send").
pub fn get_standard_mode_timestamp(data_dir: &Path) -> Option<bool> {
    read_config(data_dir).and_then(|c| c.standard_mode_timestamp)
}

/// Remember (or, with `None`, forget) the Standard-mode answer, keeping the
/// mode as it currently resolves.
pub fn set_standard_mode_timestamp(data_dir: &Path, answer: Option<bool>) -> Result<(), String> {
    write_config(
        data_dir,
        &NetworkModeConfig {
            mode: get_network_mode(data_dir),
            standard_mode_timestamp: answer,
        },
    )
}

/// BL-CLAIM-004 option 3. Enhanced mode timestamps. Standard mode follows
/// the remembered answer, and with no answer returns [`SigningTimestamp::Ask`].
pub fn signing_timestamp(data_dir: &Path) -> SigningTimestamp {
    match get_network_mode(data_dir) {
        NetworkMode::Enhanced => SigningTimestamp::Use,
        NetworkMode::Standard => match get_standard_mode_timestamp(data_dir) {
            Some(true) => SigningTimestamp::Use,
            Some(false) => SigningTimestamp::Skip,
            None => SigningTimestamp::Ask,
        },
    }
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
    fn signing_timestamp_follows_mode_and_answer() {
        let dir = tmp();
        // First run: Enhanced by default, timestamps.
        assert_eq!(signing_timestamp(dir.path()), SigningTimestamp::Use);
        set_network_mode(dir.path(), NetworkMode::Standard).unwrap();
        assert_eq!(signing_timestamp(dir.path()), SigningTimestamp::Ask);
        set_standard_mode_timestamp(dir.path(), Some(false)).unwrap();
        assert_eq!(signing_timestamp(dir.path()), SigningTimestamp::Skip);
        set_standard_mode_timestamp(dir.path(), Some(true)).unwrap();
        assert_eq!(signing_timestamp(dir.path()), SigningTimestamp::Use);
        set_standard_mode_timestamp(dir.path(), None).unwrap();
        assert_eq!(signing_timestamp(dir.path()), SigningTimestamp::Ask);
        // Setting the answer must not change the mode.
        assert_eq!(get_network_mode(dir.path()), NetworkMode::Standard);
    }

    #[test]
    fn switching_mode_keeps_the_remembered_answer() {
        let dir = tmp();
        set_network_mode(dir.path(), NetworkMode::Standard).unwrap();
        set_standard_mode_timestamp(dir.path(), Some(false)).unwrap();
        set_network_mode(dir.path(), NetworkMode::Enhanced).unwrap();
        assert_eq!(signing_timestamp(dir.path()), SigningTimestamp::Use);
        set_network_mode(dir.path(), NetworkMode::Standard).unwrap();
        assert_eq!(signing_timestamp(dir.path()), SigningTimestamp::Skip);
    }

    #[test]
    fn old_config_without_the_answer_parses_as_not_asked() {
        let dir = tmp();
        std::fs::write(
            dir.path().join("network_mode.json"),
            br#"{"mode":"standard"}"#,
        )
        .unwrap();
        assert!(!network_mode_is_degraded(dir.path()));
        assert_eq!(signing_timestamp(dir.path()), SigningTimestamp::Ask);
    }

    #[test]
    fn corrupted_file_asks_rather_than_sends() {
        let dir = tmp();
        std::fs::write(dir.path().join("network_mode.json"), b"not json").unwrap();
        assert_eq!(signing_timestamp(dir.path()), SigningTimestamp::Ask);
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
    fn corrupted_file_fails_closed_to_standard() {
        let dir = tmp();
        // A file that exists means a choice was made. If we cannot parse it
        // we do not know what that choice was, and guessing "make network
        // calls" can silently reverse an air-gapped user's opt-out.
        std::fs::write(dir.path().join("network_mode.json"), b"not json").unwrap();
        assert_eq!(get_network_mode(dir.path()), NetworkMode::Standard);
        assert!(!is_enhanced(dir.path()));
        assert!(network_mode_is_degraded(dir.path()));
    }

    #[test]
    fn valid_file_is_not_degraded() {
        let dir = tmp();
        set_network_mode(dir.path(), NetworkMode::Standard).unwrap();
        assert!(!network_mode_is_degraded(dir.path()));
        assert_eq!(get_network_mode(dir.path()), NetworkMode::Standard);

        set_network_mode(dir.path(), NetworkMode::Enhanced).unwrap();
        assert!(!network_mode_is_degraded(dir.path()));
        assert_eq!(get_network_mode(dir.path()), NetworkMode::Enhanced);
    }

    #[test]
    fn absent_file_is_the_product_default_and_not_degraded() {
        let dir = tmp();
        // The only path that yields Enhanced without an explicit choice.
        // This is the 2026-04-25 decision and the change above does not
        // touch it.
        assert_eq!(get_network_mode(dir.path()), NetworkMode::Enhanced);
        assert!(!network_mode_is_degraded(dir.path()));
    }

    #[test]
    fn valid_json_with_the_wrong_shape_fails_closed() {
        let dir = tmp();
        // Parses as JSON, but not as NetworkModeConfig. A partially written
        // or hand-edited file lands here rather than in the read-error path.
        std::fs::write(dir.path().join("network_mode.json"), br#"{"mode":"turbo"}"#).unwrap();
        assert_eq!(get_network_mode(dir.path()), NetworkMode::Standard);
        assert!(network_mode_is_degraded(dir.path()));
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_file_fails_closed() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tmp();
        let path = dir.path().join("network_mode.json");
        set_network_mode(dir.path(), NetworkMode::Enhanced).unwrap();
        // Make it unreadable so read_to_string errors rather than parse.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        let mode = get_network_mode(dir.path());
        let degraded = network_mode_is_degraded(dir.path());

        // Restore before asserting so a failure cannot leave an
        // undeletable temp directory behind.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        assert_eq!(mode, NetworkMode::Standard);
        assert!(degraded);
    }

    #[test]
    fn json_serialises_as_camel_case() {
        let json = serde_json::to_string(&NetworkMode::Enhanced).unwrap();
        assert_eq!(json, r#""enhanced""#);
        let json = serde_json::to_string(&NetworkMode::Standard).unwrap();
        assert_eq!(json, r#""standard""#);
    }
}
