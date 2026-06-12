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
    let bundle_id = "com.juralabs.jura-trace";
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
        // %APPDATA%\<bundle-id>\data
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(bundle_id).join("data"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

/// Maximum log file size before it is truncated (10 MiB).
/// When the file exceeds this size at startup the old content is discarded
/// so that the log file never grows unboundedly on long-running deployments.
const MAX_LOG_FILE_BYTES: u64 = 10 * 1024 * 1024;

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

            env_logger::Builder::from_default_env()
                .target(env_logger::Target::Pipe(Box::new(DualWriter {
                    file: std::sync::Mutex::new(file),
                })))
                .init();

            log_path
        }
        None => {
            // No log file — fall back to stdout only.
            env_logger::init();
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
/// JTV-184 Phase 2 — kill stale `jura-sidecar` processes from previous app
/// instances before spawning a fresh sidecar.
///
/// # Why this exists
///
/// A repeated pattern observed through the dev cycle (and confirmed on
/// 2026-05-16 during the v1.0 launch-prep smoke):
///
/// 1. User has Jura Trace running, sidecar bound on ephemeral port.
/// 2. User installs a new build (overwrites `/Applications/Jura Trace.app`)
///    without quitting the existing app first, OR Jura Trace force-quits /
///    crashes / is killed by `kill -9` from a debugging session.
/// 3. The old Tauri shell is gone but the PyInstaller-bootstrapped
///    `jura-sidecar` process tree (bootstrap parent + uvicorn child)
///    remains alive in the user's process table because `RunEvent::Exit`
///    never fired.
/// 4. The user launches the new app. Its sidecar spawns successfully on a
///    fresh ephemeral port (Option C protects against the port collision)
///    but the orphan from step 2 is still alive, eating ~300–500 MB RAM
///    and showing up in Activity Monitor as a confusing duplicate.
///
/// This function runs at startup BEFORE the spawn_sidecar call, sends
/// SIGKILL to any process whose name matches `jura-sidecar`, and waits
/// briefly for the kernel to reap them. The fresh spawn then has a clean
/// process tree.
///
/// # Cross-platform notes
///
/// - macOS / Linux: `pkill -KILL -f jura-sidecar` matches the full command
///   line, so both the bootstrap parent (`.../Contents/MacOS/jura-sidecar
///   --host 127.0.0.1 --port NNNNN`) and the uvicorn child (which inherits
///   the same arg vector via `execve`) are killed together. Our own
///   `jura-trace` parent is NOT matched, so this is safe to call from
///   `setup()`.
/// - Windows: `taskkill /F /IM jura-sidecar.exe` by image name. Same idea
///   — kills any leftover sidecar EXE regardless of which prior Jura Trace
///   spawned it.
///
/// Best-effort: if `pkill` / `taskkill` is absent (extremely unusual) or
/// returns non-zero, we log at DEBUG and proceed — orphans staying alive
/// is a memory / disk concern, not a correctness one. The fresh sidecar
/// will pick a different ephemeral port via Option C either way.
pub(crate) fn kill_orphan_sidecars() {
    #[cfg(unix)]
    {
        let output = std::process::Command::new("pkill")
            .args(["-KILL", "-f", "jura-sidecar"])
            .output();
        match output {
            Ok(o) if o.status.code() == Some(0) => {
                log::info!(
                    "Orphan-kill: SIGKILL sent to stale jura-sidecar process(es) \
                     from a previous Jura Trace instance"
                );
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            Ok(_) => {
                log::debug!("Orphan-kill: no stale jura-sidecar processes to terminate");
            }
            Err(e) => {
                log::debug!("Orphan-kill: pkill unavailable ({e}); skipping");
            }
        }
    }
    #[cfg(windows)]
    {
        let output = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "jura-sidecar.exe"])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                log::info!(
                    "Orphan-kill: taskkill terminated stale jura-sidecar.exe \
                     process(es) from a previous Jura Trace instance"
                );
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            Ok(_) => {
                log::debug!("Orphan-kill: no stale jura-sidecar.exe processes to terminate");
            }
            Err(e) => {
                log::debug!("Orphan-kill: taskkill unavailable ({e}); skipping");
            }
        }
    }
}

/// JTV-184 Phase 3 — sweep stale `_MEIxxxxxx` PyInstaller extraction
/// directories from `$TMPDIR` before spawning a fresh sidecar.
///
/// # Why this exists
///
/// PyInstaller `--onefile` extracts the bundle payload to
/// `$TMPDIR/_MEIxxxxxx` on every cold launch. On clean process exit the
/// bootloader's `atexit` handler cleans up the directory. But on SIGKILL,
/// crash, or abrupt Tauri shell termination the cleanup never runs and
/// the directory persists indefinitely.
///
/// Live audit on the developer Mac on 2026-05-16 found 22 stale
/// `_MEI*` directories in `/var/folders/.../T/` totalling 3.5 GB — one
/// per recent failed-launch / force-quit cycle through the dev sprint.
/// On a 256 GB MacBook at 85% capacity this would tip the user into
/// "Your startup disk is almost full" territory inside a week of
/// occasional crashes. After Phase 0 each dir is ~250 MB instead of
/// ~1 GB, but the accumulation logic is the same.
///
/// # Safety
///
/// `remove_dir_all` on a directory still held open by an active process
/// fails with `EBUSY` on macOS / Linux (and `ERROR_SHARING_VIOLATION` on
/// Windows). Live sidecars created by THIS app — or any other still-
/// running PyInstaller `--onefile` app on the same machine — are
/// therefore preserved. Only true orphan directories are removed.
///
/// Runs AFTER `kill_orphan_sidecars()` so any orphan sidecar that was
/// holding a stale `_MEI*` open has just been SIGKILL'd; the kernel
/// reaps the file handles within the 300 ms grace period that
/// `kill_orphan_sidecars` already sleeps for, and the directory becomes
/// removable.
pub(crate) fn cleanup_stale_mei_dirs() {
    let tmp_dir = std::env::temp_dir();
    let Ok(entries) = std::fs::read_dir(&tmp_dir) else {
        return;
    };
    let mut cleaned: u32 = 0;
    let mut skipped: u32 = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("_MEI") {
            continue;
        }
        match std::fs::remove_dir_all(entry.path()) {
            Ok(_) => cleaned += 1,
            Err(_) => skipped += 1,
        }
    }
    if cleaned > 0 {
        log::info!(
            "_MEI cleanup: removed {cleaned} stale PyInstaller extract dir(s) \
             ({skipped} skipped — held open by active process)",
        );
    } else if skipped > 0 {
        log::debug!("_MEI cleanup: 0 removable, {skipped} held by active processes",);
    }
}

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

/// Poll the sidecar `/health/ready` endpoint until it responds or the timeout
/// elapses.
///
/// Uses exponential back-off: 200 ms → 400 → 800 → 1 600 ms (capped), up to
/// `max_attempts` total tries. Returns `true` when the sidecar is ready.
///
/// JTV-142 fix 3 (2026-05-02): polls `/health/ready`, not `/health`. The
/// `/health` endpoint runs `_ensure_model()` per request which can re-import
/// scikit-image / sklearn modules from `_MEIPASS` on a cold PyInstaller
/// bundle and block the response for hundreds of ms. `/health/ready` is a
/// constant-time bool read of a flag set during the FastAPI lifespan, so
/// every retry burns its full back-off interval rather than serialising on
/// the lazy CLIP probe. The full capability JSON at `/health` is fetched
/// separately by `SidecarClient::health()` once readiness is confirmed.
///
/// JTV-184 Phase 1 + Phase 3 (A2) note: as of 2026-05-16 this synchronous
/// blocking probe is no longer called from any v1.0 code path. The startup
/// readiness check uses an async indefinite-loop replacement inside the
/// background tokio task (see the Phase 1 block in `run()` setup); the
/// power-saver respawn no longer waits for readiness at all (fire-and-
/// forget — the next verify call inherits the still-spawning sidecar and
/// degrades gracefully via `SidecarClient::is_available`). The function
/// is retained for future single-shot callers (e.g. v1.0.1 `jura` CLI's
/// `--wait-ready` flag) and as defensive infrastructure should a future
/// path need synchronous readiness semantics. Marked `#[allow(dead_code)]`
/// so cargo does not warn about the absent call sites.
#[allow(dead_code)]
pub(crate) fn wait_for_sidecar_ready(max_attempts: u32, port: u16) -> bool {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    let url = format!("http://127.0.0.1:{port}/health/ready");
    for attempt in 0..max_attempts {
        let delay_ms = 200u64 * (1u64 << attempt.min(3));
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        if client
            .get(&url)
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            log::info!("Sidecar ready after {} poll attempt(s)", attempt + 1);
            return true;
        }
    }
    false
}

/// Pick a free loopback TCP port for the sidecar. Binds to `127.0.0.1:0` so
/// the OS allocates an ephemeral port, records it, then drops the listener so
/// the sidecar can bind.
///
/// **Accepted residual risk (security audit 2026-05-16 NEW-MED-5 / JTV-187):**
/// sub-millisecond race window between `drop(listener)` and the sidecar's
/// `bind()`. A local same-user process that wins the race could occupy the
/// freed port and receive one session's API key + image data. Accepted for
/// v1.0 on grounds of (a) very low exploitability — random port from the
/// ephemeral range, must win first-try, key regenerates per session — and
/// (b) local-only threat model where an in-process attacker already has
/// higher-leverage paths. v1.1 may revisit via fd-passing (eliminates the
/// race but needs sidecar-side `socket.fromfd()` + uvicorn `--fd` work).
///
/// Returns `None` if no port can be bound (extremely unlikely — would indicate
/// process-level resource exhaustion). Callers should fall back to a fixed
/// default in that case so the app can still attempt to spawn.
pub(crate) fn pick_ephemeral_port() -> Option<u16> {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    drop(listener);
    Some(port)
}
