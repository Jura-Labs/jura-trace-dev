//! Application entry-point helpers — logging, sidecar lifecycle, and port selection.
//!
//! This module owns the infrastructure that runs before and around the Tauri
//! builder, and the sidecar spawn/health helpers that are reused at runtime:
//!
//! - [`dirs_next_data_dir`] — early (pre-Tauri) data directory resolution
//! - [`init_logging`] — dual stdout + file logging initialisation
//! - [`kill_orphan_sidecars`] — SIGKILL any leftover `jura-sidecar` processes
//! - [`cleanup_stale_mei_dirs`] — remove stale PyInstaller `_MEI*` temp dirs
//! - [`spawn_sidecar`] — spawn the PyInstaller sidecar binary via Tauri shell
//! - [`wait_for_sidecar_ready`] — poll `/health/ready` with exponential back-off
//! - [`pick_ephemeral_port`] — bind `127.0.0.1:0` to obtain a free loopback port

// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::Write;
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

// Moved to sidecar_supervisor (v1.2.0 B2a stage 1) so the headless API shares
// them; re-exported so existing call sites are unchanged.
#[allow(unused_imports)]
pub(crate) use crate::sidecar_supervisor::{
    cleanup_stale_mei_dirs, kill_orphan_sidecars, pick_ephemeral_port, wait_for_sidecar_ready,
};

// ===== Application Entry =====

/// Best-effort early resolution of the application data directory.
///
/// Tauri's authoritative path resolver is only available after `.setup()` runs,
/// which is too late to capture early startup log messages.  This function
/// derives the same path using only standard library calls and the platform
/// environment so that [`init_logging`] can open the log file before the Tauri
/// builder is invoked.
///
/// Returns `None` if the home directory cannot be determined.
pub(crate) fn dirs_next_data_dir() -> Option<PathBuf> {
    // Must equal `identifier` in tauri.conf.json, which is what Tauri's own
    // app_data_dir() uses (test: `bundle_id_matches_tauri_conf`). Until
    // 1 October 2026 this read "com.juralabs.jura-trace", so the log file went
    // to a directory that held only a stale, empty jura_trace.db while the
    // real database lived under org.juralabs.trace.
    let bundle_id = BUNDLE_ID;
    #[cfg(target_os = "macos")]
    {
        // ~/Library/Application Support/<bundle-id>
        std::env::var_os("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library")
                .join("Application Support")
                .join(bundle_id)
        })
    }
    #[cfg(target_os = "linux")]
    {
        // $XDG_DATA_HOME/<bundle-id>  or  ~/.local/share/<bundle-id>
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            Some(PathBuf::from(xdg).join(bundle_id))
        } else {
            std::env::var_os("HOME").map(|h| {
                PathBuf::from(h)
                    .join(".local")
                    .join("share")
                    .join(bundle_id)
            })
        }
    }
    #[cfg(target_os = "windows")]
    {
        // %APPDATA%\<bundle-id>, which is Tauri v2's app_data_dir on Windows
        // (the roaming data dir joined with the identifier, no subfolder).
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(bundle_id))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

/// The application identifier, as in `tauri.conf.json`.
pub(crate) const BUNDLE_ID: &str = "org.juralabs.trace";

/// Maximum log file size before it is truncated (10 MiB).
/// When the file exceeds this size at startup the old content is discarded
/// so that the log file never grows unboundedly on long-running deployments.
const MAX_LOG_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// The filter used when `RUST_LOG` is unset, which is always the case for an
/// installed app launched from the Start menu or the Dock.
///
/// Until 23 September 2026 the builder took its filter from `RUST_LOG` alone,
/// and env_logger's default without it is `error`, so every `info` and `warn`
/// line was dropped in production and the log file a user could send us held
/// errors only (BL-LOG-001).
///
/// The whole crate logs at `info`, which means the file records local paths
/// of images being analysed (e.g. `fingerprint.rs`, `heatmap.rs`) and URLs
/// being verified, query strings stripped. Paul decided that on 23 September
/// 2026: the log stays on the user's machine and is only ever sent by them.
/// Other crates stay at `warn` so dependency chatter does not push the
/// startup record out of the 10 MiB cap. `RUST_LOG` still overrides.
const DEFAULT_LOG_FILTER: &str = "warn,jura_trace_lib=info";

fn default_logger() -> env_logger::Builder {
    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(DEFAULT_LOG_FILTER),
    );
    // Second resolution hid a 5-second gap in the startup sequence
    // (BL-PERF-001), so timestamps carry milliseconds.
    builder.format_timestamp_millis();
    builder
}

/// Initialise the logging subsystem.
///
/// Writes to:
/// - stdout (always), so `RUST_LOG` / terminal still works in development
/// - `app_data_dir/jura-trace.log` (production builds), so IT managers can
///   inspect logs without attaching a terminal
///
/// The log file is truncated when it exceeds [`MAX_LOG_FILE_BYTES`] so that
/// long-running managed deployments do not accumulate unbounded disk usage.
/// The path is resolved from the Tauri app data directory; if that cannot be
/// determined before the Tauri app is built (we call this from `run()` before
/// `.setup()`), we fall back to stdout-only.
///
/// Returns the path that was opened, or `None` when file logging was skipped.
pub(crate) fn init_logging(app_data_dir: Option<&std::path::Path>) -> Option<PathBuf> {
    // Attempt to open a log file when a data directory is available.
    let log_path = app_data_dir.map(|dir| dir.join("jura-trace.log"));

    let file_target: Option<std::fs::File> = log_path.as_ref().and_then(|p| {
        // Ensure the parent directory exists.
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Rotate (truncate) if the file already exceeds the size cap.
        if let Ok(meta) = std::fs::metadata(p) {
            if meta.len() > MAX_LOG_FILE_BYTES {
                // Truncate by re-opening with create(true) + truncate(true).
                let _ = std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(p);
            }
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .ok()
    });

    match file_target {
        Some(file) => {
            // Fan-out writer: send every log line to both stdout and the file.
            struct DualWriter {
                file: std::sync::Mutex<std::fs::File>,
            }
            impl Write for DualWriter {
                fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                    // Best-effort write to file; ignore failures so a full disk
                    // never causes the app to crash.
                    let _ = self.file.lock().map(|mut f| f.write_all(buf));
                    // Always write to stdout.
                    std::io::stdout().write(buf)
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    let _ = self.file.lock().map(|mut f| f.flush());
                    std::io::stdout().flush()
                }
            }

            default_logger()
                .target(env_logger::Target::Pipe(Box::new(DualWriter {
                    file: std::sync::Mutex::new(file),
                })))
                .init();

            log_path
        }
        None => {
            // No log file — fall back to stdout only.
            default_logger().init();
            None
        }
    }
}

/// Spawn the PyInstaller sidecar binary and return the child handle.
///
/// Used both at startup and by the power-saver respawn path when the sidecar
/// has been idle-killed and a new verification request arrives.
///
/// In debug builds (`cargo tauri dev`) the binary is absent; the function logs
/// a notice and returns `None` so the developer's manual `uvicorn` process is used.
///
/// The caller is responsible for polling `/health` after a successful spawn to
/// wait for the sidecar to become ready before dispatching requests.
///
/// `port` is the loopback TCP port the sidecar should bind. From v1.0 (Option C
/// port-collision fix, 2026-05-12) this is picked dynamically by the Rust
/// startup via `pick_ephemeral_port()` rather than being hard-coded to 8200,
/// so a stale sidecar from a previous launch / a CI runner / an unrelated
/// process holding 8200 cannot prevent the new app from starting.
pub(crate) fn spawn_sidecar(
    app: &tauri::AppHandle,
    port: u16,
) -> Option<tauri_plugin_shell::process::CommandChild> {
    if cfg!(debug_assertions) {
        return None;
    }

    // JTV-184 Phase 2: clean up orphans before spawning fresh. See
    // [`kill_orphan_sidecars`] for the full rationale and cross-platform
    // notes. Runs every spawn (not just startup) so the power-saver
    // respawn path also benefits — a wedged sidecar from a prior respawn
    // attempt is reaped before the next attempt.
    kill_orphan_sidecars();

    // JTV-184 Phase 3: sweep stale `_MEIxxxxxx` PyInstaller extract dirs
    // from $TMPDIR. Runs AFTER `kill_orphan_sidecars` so any orphan that
    // was holding a stale _MEI open has just been SIGKILL'd — the dirs
    // are then removable. See [`cleanup_stale_mei_dirs`].
    cleanup_stale_mei_dirs();

    // Set JURA_MODELS_DIR so the sidecar can find model files.
    if let Ok(resource_dir) = app.path().resource_dir() {
        let models_dir = resource_dir.join("models");
        if models_dir.is_dir() {
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var("JURA_MODELS_DIR", &models_dir);
            }
        }
    }

    match app.shell().sidecar("jura-sidecar") {
        Err(e) => {
            log::warn!(
                "Could not locate sidecar binary for (re)spawn: {e}. \
                 Forensic analysis will be unavailable."
            );
            None
        }
        Ok(cmd) => {
            // macOS-only: launchd-launched apps inherit a limited PATH that
            // does NOT include Homebrew directories (/opt/homebrew/bin on
            // Apple Silicon, /usr/local/bin on Intel).  The sidecar's
            // ffmpeg/ffprobe health probe uses `shutil.which()` which
            // only searches PATH, so without this prepend the sidecar
            // reports "FFmpeg not installed" even when Homebrew has it.
            // Linux and Windows package managers put ffmpeg in PATH by
            // default — only macOS needs the augmentation.
            let augmented_path = {
                let homebrew = "/opt/homebrew/bin:/usr/local/bin";
                match std::env::var("PATH") {
                    Ok(p) if !p.is_empty() => format!("{homebrew}:{p}"),
                    _ => homebrew.to_string(),
                }
            };
            let cmd = cmd.env("PATH", augmented_path);
            // JTV-142 fix 2 (2026-05-02): without PYTHONUNBUFFERED, Python's
            // stdout is fully buffered when piped to Tauri's CommandEvent
            // stream. uvicorn's "Application startup complete" + bind log
            // can be held in a 64 KB buffer for the entire startup window,
            // which makes the "process alive but Settings shows Offline"
            // symptom hard to diagnose. Forcing line-buffered flush makes
            // startup progress visible in the Rust log reader in real time.
            let cmd = cmd.env("PYTHONUNBUFFERED", "1");
            let port_str = port.to_string();
            match cmd
                .args(["--host", "127.0.0.1", "--port", &port_str])
                .spawn()
            {
                Err(e) => {
                    log::warn!(
                        "Failed to (re)spawn sidecar: {e}. \
                     Forensic analysis will be unavailable."
                    );
                    None
                }
                Ok((mut rx, child)) => {
                    log::info!("Sidecar spawned (pid {}, port {port})", child.pid());
                    tauri::async_runtime::spawn(async move {
                        use tauri_plugin_shell::process::CommandEvent;
                        while let Some(event) = rx.recv().await {
                            match event {
                                CommandEvent::Stdout(line) => {
                                    // JTV-142 fix 2: surface sidecar startup at
                                    // info so port-bind / model-warmup progress
                                    // is visible without raising the global log
                                    // level. Volume is tolerable because the
                                    // sidecar prints sparingly post-startup.
                                    log::info!("sidecar: {}", String::from_utf8_lossy(&line));
                                }
                                CommandEvent::Stderr(line) => {
                                    log::info!("sidecar: {}", String::from_utf8_lossy(&line));
                                }
                                CommandEvent::Terminated(p) => {
                                    log::info!("Sidecar process terminated (code: {:?})", p.code);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    });
                    Some(child)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The early data-dir resolver must point where Tauri does, or the log
    /// and the headless API look in a different directory from the app's
    /// database.
    #[test]
    fn bundle_id_matches_tauri_conf() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["identifier"].as_str(), Some(BUNDLE_ID));
    }

    #[test]
    fn data_dir_ends_in_the_bundle_id() {
        if let Some(dir) = dirs_next_data_dir() {
            assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some(BUNDLE_ID));
        }
    }
}
