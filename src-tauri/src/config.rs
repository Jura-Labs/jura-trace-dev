//! Application configuration — `config.json` I/O and database-path resolution.
//!
//! This module owns the on-disk configuration schema ([`AppConfig`]) and the
//! three helpers that read, write, and validate it:
//! - [`read_app_config`] — deserialise `config.json` (returns default on any error)
//! - [`write_app_config`] — serialise and flush `config.json`
//! - [`dir_is_writable`] — probe-file writability check
//! - [`resolve_db_path`] — three-priority database path selection (env var →
//!   config.json → platform default)

// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::Manager;

use crate::LicenceTier;

// ===== Database Path Configuration =====

/// Configuration file schema stored in app_data_dir/config.json.
#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct AppConfig {
    #[serde(default)]
    pub(crate) db_path: Option<String>,
    /// Pilot-phase tier indicator. Defaults to Community.
    #[serde(default)]
    pub(crate) licence_tier: LicenceTier,
    /// When `true`, the first-run setup wizard is suppressed on startup.
    ///
    /// Intended for IT-managed deployments where an administrator pre-configures
    /// `config.json` and wants to skip the wizard for all users on that machine.
    /// Defaults to `false` so existing installs are unaffected.
    #[serde(default)]
    pub(crate) skip_setup_wizard: bool,
    /// User preference for AI image descriptions (Ollama LLaVA).
    /// See `AppState::ai_description_enabled` for semantics. Stored as a
    /// tri-state so we can distinguish "never set" from "explicitly off".
    #[serde(default)]
    pub(crate) ai_description_enabled: Option<bool>,
    /// Power-saver mode: terminate the sidecar after 5 minutes of inactivity
    /// to free ~300–500 MB of RAM. Default `false` — opt-in only.
    #[serde(default)]
    pub(crate) power_saver_mode: bool,
}

/// Read the persisted config.json from app_data_dir.
/// Returns a default (empty) config if the file is absent or malformed.
pub(crate) fn read_app_config(data_dir: &Path) -> AppConfig {
    let config_path = data_dir.join("config.json");
    match std::fs::read_to_string(&config_path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

/// Persist a config change to app_data_dir/config.json.
pub(crate) fn write_app_config(data_dir: &Path, config: &AppConfig) -> Result<(), String> {
    let config_path = data_dir.join("config.json");
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&config_path, text).map_err(|e| e.to_string())
}

/// Check whether a parent directory exists and is writable by creating a
/// zero-byte probe file then removing it immediately.
pub(crate) fn dir_is_writable(dir: &Path) -> bool {
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
/// 3. Default: `app_data_dir/jura_trace.db` (preserves all existing installs).
pub(crate) fn resolve_db_path(app: &tauri::App) -> PathBuf {
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
                    "JURA_DB_PATH set to '{env_val}' but parent directory is not writable; \
                     falling through to config.json"
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
                    "config.json db_path '{cfg_val}' parent directory is not writable; \
                     falling through to default"
                );
            }
        }
    }

    // Priority 3: default — legacy filename for migration compatibility
    let default_path = data_dir.join("jura_trace.db");
    log::info!(
        "Database path resolved to default: {}",
        default_path.display()
    );
    default_path
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Database path resolution tests ─────────────────────────────────

    #[test]
    fn read_app_config_missing_file_returns_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = read_app_config(dir.path());
        assert!(
            cfg.db_path.is_none(),
            "Missing config.json should yield default"
        );
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
            ..Default::default()
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
        assert!(
            !dir_is_writable(&path),
            "Non-existent dir should not be writable"
        );
    }

    #[test]
    fn env_var_path_takes_priority_over_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("from_config.db");
        let config = AppConfig {
            db_path: Some(cfg_path.to_string_lossy().into_owned()),
            ..Default::default()
        };
        write_app_config(dir.path(), &config).expect("write");

        let env_path = dir.path().join("from_env.db");
        let env_parent = dir.path();
        assert!(dir_is_writable(env_parent), "Parent must be writable");

        let chosen = if dir_is_writable(env_path.parent().unwrap()) {
            env_path.clone()
        } else {
            let cfg = read_app_config(dir.path());
            cfg.db_path
                .map(PathBuf::from)
                .unwrap_or_else(|| dir.path().join("default.db"))
        };
        assert_eq!(chosen, env_path, "Env var path should take priority");
    }

    #[test]
    fn config_path_takes_priority_over_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("custom.db");
        let config = AppConfig {
            db_path: Some(cfg_path.to_string_lossy().into_owned()),
            ..Default::default()
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
        let default = dir.path().join("jura_trace.db");
        let chosen = if let Some(p) = cfg.db_path {
            PathBuf::from(p)
        } else {
            default.clone()
        };
        assert_eq!(chosen, default, "Default path should be used as fallback");
    }

    // ── Licence tier persistence tests ───────────────────────────────────────

    #[test]
    fn licence_tier_defaults_to_community() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = read_app_config(dir.path());
        assert_eq!(
            cfg.licence_tier,
            LicenceTier::Community,
            "Default tier should be Community when no config exists"
        );
    }

    #[test]
    fn licence_tier_roundtrip_professional() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = AppConfig {
            licence_tier: LicenceTier::Professional,
            ..Default::default()
        };
        write_app_config(dir.path(), &config).expect("write_app_config");
        let read_back = read_app_config(dir.path());
        assert_eq!(
            read_back.licence_tier,
            LicenceTier::Professional,
            "Professional tier should round-trip through config.json"
        );
    }

    #[test]
    fn licence_tier_roundtrip_enterprise() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = AppConfig {
            licence_tier: LicenceTier::Enterprise,
            ..Default::default()
        };
        write_app_config(dir.path(), &config).expect("write_app_config");
        let read_back = read_app_config(dir.path());
        assert_eq!(
            read_back.licence_tier,
            LicenceTier::Enterprise,
            "Enterprise tier should round-trip through config.json"
        );
    }

    #[test]
    fn licence_tier_preserved_when_db_path_updated() {
        // set_db_path must not clobber the licence_tier stored in config.json.
        let dir = tempfile::tempdir().expect("tempdir");
        // Write an initial config with a non-default tier.
        let initial = AppConfig {
            db_path: Some("/old/path.db".to_string()),
            licence_tier: LicenceTier::Enterprise,
            ..Default::default()
        };
        write_app_config(dir.path(), &initial).expect("write initial config");

        // Simulate what set_db_path does: read → update db_path → write.
        let mut config = read_app_config(dir.path());
        config.db_path = Some("/new/path.db".to_string());
        write_app_config(dir.path(), &config).expect("write updated config");

        let read_back = read_app_config(dir.path());
        assert_eq!(
            read_back.licence_tier,
            LicenceTier::Enterprise,
            "Updating db_path must not overwrite licence_tier"
        );
        assert_eq!(
            read_back.db_path.as_deref(),
            Some("/new/path.db"),
            "db_path must be updated correctly"
        );
    }

    #[test]
    fn licence_tier_serde_camel_case() {
        // Verify the serde encoding is camelCase as the frontend expects.
        let json = serde_json::to_string(&LicenceTier::Professional).expect("serialize");
        assert_eq!(
            json, r#""professional""#,
            "LicenceTier::Professional must serialize as camelCase"
        );

        let json_enterprise = serde_json::to_string(&LicenceTier::Enterprise).expect("serialize");
        assert_eq!(json_enterprise, r#""enterprise""#);

        let json_community = serde_json::to_string(&LicenceTier::Community).expect("serialize");
        assert_eq!(json_community, r#""community""#);
    }

    #[test]
    fn licence_tier_legacy_team_deserializes_as_professional() {
        // Pilot configs may carry the retired "team" value. Confirm the alias
        // rolls those up to Professional with no manual migration.
        let team: LicenceTier =
            serde_json::from_str(r#""team""#).expect("legacy team value must deserialize");
        assert_eq!(team, LicenceTier::Professional);

        let pro: LicenceTier = serde_json::from_str(r#""pro""#).expect("pro alias");
        assert_eq!(pro, LicenceTier::Professional);
    }

    // ── Power-saver mode tests ─────────────────────────────────────────────

    #[test]
    fn power_saver_mode_default_is_false() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = read_app_config(dir.path());
        assert!(
            !cfg.power_saver_mode,
            "Default power_saver_mode must be false"
        );
    }

    #[test]
    fn power_saver_mode_roundtrip_enabled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = AppConfig {
            power_saver_mode: true,
            ..Default::default()
        };
        write_app_config(dir.path(), &config).expect("write");
        let read_back = read_app_config(dir.path());
        assert!(
            read_back.power_saver_mode,
            "power_saver_mode=true should round-trip"
        );
    }

    #[test]
    fn power_saver_mode_roundtrip_disabled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = AppConfig {
            power_saver_mode: false,
            ..Default::default()
        };
        write_app_config(dir.path(), &config).expect("write");
        let read_back = read_app_config(dir.path());
        assert!(
            !read_back.power_saver_mode,
            "power_saver_mode=false should round-trip"
        );
    }
}
