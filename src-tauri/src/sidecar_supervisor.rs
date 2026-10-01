// SPDX-License-Identifier: AGPL-3.0-or-later

//! Sidecar process management with no Tauri types (v1.2.0 B2a stage 1).
//!
//! The headless `jura-trace-api` binary owns the Python sidecar through
//! [`SidecarSupervisor`]. The orphan sweep, the `_MEI` cleanup, the readiness
//! poll and the port picker moved here from `startup.rs`, which re-exports
//! them, so the desktop app and the headless binary share one implementation.
//! Design: `docs/design/v1.2.0-headless-api-and-cli.md` section 4.3.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::state::SidecarStartupStatus;

/// What the supervisor knows about its sidecar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidecarState {
    /// Spawned, `/health/ready` has not answered yet.
    Starting,
    /// `/health/ready` answered 200.
    Ready,
    /// No sidecar: not found, or not requested (`--no-sidecar`).
    Absent,
    /// The process exited, or could not be started.
    Failed,
}

#[derive(Debug)]
pub enum SupervisorError {
    Spawn(std::io::Error),
    /// The sidecar exited before it became ready.
    Exited(Option<i32>),
    /// Readiness did not arrive within the timeout.
    Timeout(Duration),
}

impl std::fmt::Display for SupervisorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(e) => write!(f, "could not start the analysis sidecar: {e}"),
            Self::Exited(Some(c)) => write!(f, "the analysis sidecar exited with code {c}"),
            Self::Exited(None) => write!(f, "the analysis sidecar was terminated by a signal"),
            Self::Timeout(t) => write!(
                f,
                "the analysis sidecar was not ready after {} s",
                t.as_secs()
            ),
        }
    }
}

impl std::error::Error for SupervisorError {}

/// Failed is encoded beside the three [`SidecarStartupStatus`] values so the
/// one atomic that `/api/v1/health` already reads carries it too.
const FAILED: u8 = 3;

/// A running sidecar child process, its port, and its readiness.
pub struct SidecarSupervisor {
    child: Arc<Mutex<Option<Child>>>,
    status: Arc<AtomicU8>,
}

impl SidecarSupervisor {
    /// Start `binary` listening on `127.0.0.1:port`, authenticated with
    /// `key`, and begin polling it for readiness in the background. Returns
    /// once the process has started, not once it is ready: the frozen bundle
    /// takes tens of seconds on a cold start.
    pub fn spawn(
        binary: &Path,
        models_dir: Option<&Path>,
        port: u16,
        key: &str,
    ) -> Result<Self, SupervisorError> {
        let mut cmd = Command::new(binary);
        cmd.args(["--host", "127.0.0.1", "--port", &port.to_string()])
            .env("JURA_SIDECAR_KEY", key)
            // Line-buffered output, so startup progress reaches the log as it
            // happens (JTV-142 fix 2).
            .env("PYTHONUNBUFFERED", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = models_dir {
            cmd.env("JURA_MODELS_DIR", dir);
        }
        // macOS: a launchd-started process does not see Homebrew's bin
        // directories, where the sidecar's ffmpeg probe looks (see
        // startup::spawn_sidecar).
        if cfg!(target_os = "macos") {
            let homebrew = "/opt/homebrew/bin:/usr/local/bin";
            let path = match std::env::var("PATH") {
                Ok(p) if !p.is_empty() => format!("{homebrew}:{p}"),
                _ => homebrew.to_string(),
            };
            cmd.env("PATH", path);
        }

        let mut child = cmd.spawn().map_err(SupervisorError::Spawn)?;
        log::info!(
            "Sidecar spawned (pid {}, port {port}) from {}",
            child.id(),
            binary.display()
        );
        for stream in [
            child
                .stdout
                .take()
                .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
            child
                .stderr
                .take()
                .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            std::thread::spawn(move || {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    log::info!("sidecar: {line}");
                }
            });
        }

        let status = Arc::new(AtomicU8::new(SidecarStartupStatus::Connecting.to_u8()));
        let child = Arc::new(Mutex::new(Some(child)));
        spawn_readiness_probe(port, Arc::clone(&status), Arc::clone(&child));
        Ok(Self { child, status })
    }

    /// The atomic the readiness probe updates, in `SidecarStartupStatus`
    /// encoding (plus Failed). `AppState::sidecar_startup_status` takes a
    /// clone so `/api/v1/health` reflects it.
    pub fn status_handle(&self) -> Arc<AtomicU8> {
        Arc::clone(&self.status)
    }

    pub fn state(&self) -> SidecarState {
        decode(self.status.load(Ordering::Relaxed))
    }

    /// Block until the sidecar is ready, it exits, or `timeout` passes.
    pub fn wait_ready(&self, timeout: Duration) -> Result<(), SupervisorError> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.state() {
                SidecarState::Ready => return Ok(()),
                SidecarState::Failed | SidecarState::Absent => {
                    return Err(SupervisorError::Exited(self.exit_code()));
                }
                SidecarState::Starting => {}
            }
            if Instant::now() >= deadline {
                return Err(SupervisorError::Timeout(timeout));
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    fn exit_code(&self) -> Option<i32> {
        let mut guard = self.child.lock().ok()?;
        guard
            .as_mut()
            .and_then(|c| c.try_wait().ok().flatten())
            .and_then(|s| s.code())
    }

    /// Stop the sidecar and wait for it to exit. Safe to call more than once.
    ///
    /// On Unix this sends SIGTERM first and waits up to five seconds: the
    /// Linux `--onefile` bootloader runs Python as a child process and
    /// forwards SIGTERM to it, where SIGKILL would orphan it. SIGKILL follows
    /// only if the process is still there. On macOS the launcher stub has
    /// exec'd into the bundle, so there is one process either way. On
    /// Windows the launcher and the onedir bootloader are two processes;
    /// the headless binary follows this with the orphan sweep.
    pub fn shutdown(&self) {
        let child = self.child.lock().ok().and_then(|mut g| g.take());
        if let Some(mut child) = child {
            let pid = child.id();
            #[cfg(unix)]
            {
                // SAFETY: kill(2) on our own child's pid; the child has not
                // been reaped yet (we still hold it), so the pid is ours.
                unsafe { sigterm(pid as i32, 15) };
                let deadline = Instant::now() + Duration::from_secs(5);
                while Instant::now() < deadline {
                    if let Ok(Some(_)) = child.try_wait() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
            let _ = child.kill();
            let _ = child.wait();
            log::info!("Sidecar (pid {pid}) stopped");
        }
        self.status.store(FAILED, Ordering::Relaxed);
    }
}

impl Drop for SidecarSupervisor {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "kill"]
    fn sigterm(pid: i32, sig: i32) -> i32;
}

fn decode(v: u8) -> SidecarState {
    if v == FAILED {
        return SidecarState::Failed;
    }
    match SidecarStartupStatus::from_u8(v) {
        SidecarStartupStatus::Ready => SidecarState::Ready,
        SidecarStartupStatus::Connecting => SidecarState::Starting,
        SidecarStartupStatus::NotPresent => SidecarState::Absent,
    }
}

/// Poll `/health/ready` with the desktop's back-off (200, 400, 800, then
/// 1600 ms) until it answers, and mark Failed if the process exits first.
/// Runs on a plain thread so the supervisor works without a tokio runtime.
fn spawn_readiness_probe(port: u16, status: Arc<AtomicU8>, child: Arc<Mutex<Option<Child>>>) {
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap_or_default();
        let url = format!("http://127.0.0.1:{port}/health/ready");
        let started = Instant::now();
        let mut attempt: u32 = 0;
        loop {
            std::thread::sleep(Duration::from_millis(200u64 * (1u64 << attempt.min(3))));
            attempt += 1;
            let exited = match child.lock() {
                Ok(mut g) => match g.as_mut() {
                    Some(c) => c.try_wait().ok().flatten().map(|s| s.code()),
                    None => return, // shut down
                },
                Err(_) => return,
            };
            if let Some(code) = exited {
                log::error!("Sidecar exited before it was ready (code {code:?})");
                status.store(FAILED, Ordering::Relaxed);
                return;
            }
            if client
                .get(&url)
                .send()
                .map(|r| r.status().is_success())
                .unwrap_or(false)
            {
                log::info!(
                    "Sidecar ready after {attempt} poll attempt(s) ({} s)",
                    started.elapsed().as_secs()
                );
                status.store(SidecarStartupStatus::Ready.to_u8(), Ordering::Relaxed);
                return;
            }
        }
    });
}

/// Find the sidecar executable, in order: `explicit` (`--sidecar-binary`),
/// `JURA_SIDECAR_BINARY`, then beside the running executable, which is where
/// an installed app keeps it (on macOS the 34 KB launcher stub that execs the
/// bundle in `../Resources/sidecar-bundle/`). Returns `None` if none exists.
pub fn resolve_sidecar_binary(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return p.is_file().then(|| p.to_path_buf());
    }
    if let Some(p) = std::env::var_os("JURA_SIDECAR_BINARY").map(PathBuf::from) {
        if p.is_file() {
            return Some(p);
        }
        log::warn!("JURA_SIDECAR_BINARY={} does not exist", p.display());
    }
    let name = if cfg!(windows) {
        "jura-sidecar.exe"
    } else {
        "jura-sidecar"
    };
    let beside = std::env::current_exe().ok()?.parent()?.join(name);
    beside.is_file().then_some(beside)
}

/// The models directory that belongs with a sidecar binary: the bundle's
/// `Resources/models` on macOS, `models` beside it elsewhere.
pub fn models_dir_for(sidecar_binary: &Path) -> Option<PathBuf> {
    let dir = sidecar_binary.parent()?;
    [dir.join("../Resources/models"), dir.join("models")]
        .into_iter()
        .find(|p| p.is_dir())
}

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
/// - Windows: every process whose image name is `jura-sidecar.exe`, found
///   with a Toolhelp snapshot and ended with `TerminateProcess`, in process.
///   That covers both the B1 launcher and the onedir bootloader it runs,
///   which share the name. Until 23 September 2026 this ran
///   `taskkill /F /IM jura-sidecar.exe` with `.output()`, and on
///   `windows-latest` that child lived 4 to 5 s when the app started it
///   (0.04 s run by hand), blocking setup, the sidecar spawn and the local
///   API for the whole time (BL-PERF-001). The sweep must stay BEFORE the
///   spawn: it matches by name, so run later it would kill the new sidecar.
///
/// Best-effort: if `pkill` is absent (extremely unusual), the snapshot
/// fails, or nothing matches, we log at DEBUG and proceed — orphans staying alive
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
        let started = std::time::Instant::now();
        match terminate_processes_named("jura-sidecar.exe") {
            Ok(0) => {
                log::debug!(
                    "Orphan-kill: no stale jura-sidecar.exe processes to terminate ({} ms)",
                    started.elapsed().as_millis()
                );
            }
            Ok(n) => {
                log::info!(
                    "Orphan-kill: terminated {n} stale jura-sidecar.exe process(es) \
                     from a previous Jura Trace instance ({} ms)",
                    started.elapsed().as_millis()
                );
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            Err(e) => {
                log::debug!("Orphan-kill: process snapshot failed ({e}); skipping");
            }
        }
    }
}

/// End every process whose executable image name equals `image`
/// (case-insensitive), other than this one. Returns how many were ended.
/// See [`kill_orphan_sidecars`] for why this replaced `taskkill`.
#[cfg(windows)]
fn terminate_processes_named(image: &str) -> std::io::Result<u32> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    let own_pid = std::process::id();
    // SAFETY: plain Win32 calls. The snapshot handle is checked before use
    // and closed on every path out; each process handle is closed after use.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ended = 0;
        let mut more = Process32FirstW(snapshot, &mut entry) != 0;
        while more {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            if entry.th32ProcessID != own_pid && name.eq_ignore_ascii_case(image) {
                let process = OpenProcess(PROCESS_TERMINATE, 0, entry.th32ProcessID);
                if !process.is_null() {
                    if TerminateProcess(process, 1) != 0 {
                        ended += 1;
                    }
                    CloseHandle(process);
                }
            }
            more = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
        Ok(ended)
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
