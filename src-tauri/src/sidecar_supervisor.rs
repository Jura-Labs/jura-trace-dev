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
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
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
    /// Unix seconds of the last change to `status`.
    since: Arc<AtomicU64>,
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Record a state change and when it happened. `/api/v1/ready` reports both.
fn set_status(status: &AtomicU8, since: &AtomicU64, value: u8) {
    if status.swap(value, Ordering::Relaxed) != value {
        since.store(unix_now(), Ordering::Relaxed);
    }
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
        // Windows: without CREATE_NO_WINDOW a console-subsystem child opens a
        // console window. Tauri's shell plugin set this for the desktop
        // sidecar; std::process does not.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
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
        let since = Arc::new(AtomicU64::new(unix_now()));
        let child = Arc::new(Mutex::new(Some(child)));
        spawn_readiness_probe(
            port,
            Arc::clone(&status),
            Arc::clone(&since),
            Arc::clone(&child),
        );
        Ok(Self {
            child,
            status,
            since,
        })
    }

    /// The atomic the readiness probe updates, in `SidecarStartupStatus`
    /// encoding (plus Failed). `AppState::sidecar_startup_status` takes a
    /// clone so `/api/v1/health` reflects it.
    pub fn status_handle(&self) -> Arc<AtomicU8> {
        Arc::clone(&self.status)
    }

    /// Unix seconds of the last state change, shared like `status_handle`.
    pub fn since_handle(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.since)
    }

    pub fn state(&self) -> SidecarState {
        decode(self.status.load(Ordering::Relaxed))
    }

    /// Unix seconds at which the sidecar entered its current state.
    pub fn since(&self) -> u64 {
        self.since.load(Ordering::Relaxed)
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
        set_status(&self.status, &self.since, FAILED);
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

/// Decode the status atomic: `SidecarStartupStatus` encoding, plus Failed.
pub fn decode(v: u8) -> SidecarState {
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
/// Once ready it keeps watching the process, once a second, so a sidecar
/// that dies later reports Failed and not a stale Ready (BL-SILENT-001).
/// Runs on a plain thread so the supervisor works without a tokio runtime.
fn spawn_readiness_probe(
    port: u16,
    status: Arc<AtomicU8>,
    since: Arc<AtomicU64>,
    child: Arc<Mutex<Option<Child>>>,
) {
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap_or_default();
        let url = format!("http://127.0.0.1:{port}/health/ready");
        let started = Instant::now();
        let mut attempt: u32 = 0;
        let mut ready = false;
        loop {
            let delay = if ready {
                1000
            } else {
                200u64 * (1u64 << attempt.min(3))
            };
            std::thread::sleep(Duration::from_millis(delay));
            let exited = match child.lock() {
                Ok(mut g) => match g.as_mut() {
                    Some(c) => c.try_wait().ok().flatten().map(|s| s.code()),
                    None => return, // shut down
                },
                Err(_) => return,
            };
            if let Some(code) = exited {
                if ready {
                    log::error!("Sidecar exited while running (code {code:?})");
                } else {
                    log::error!("Sidecar exited before it was ready (code {code:?})");
                }
                set_status(&status, &since, FAILED);
                return;
            }
            if ready {
                continue;
            }
            attempt += 1;
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
                set_status(&status, &since, SidecarStartupStatus::Ready.to_u8());
                ready = true;
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
    let exe = std::env::current_exe().ok()?;
    // Through a symlink (the CLI quickstart suggests one in /usr/local/bin on
    // macOS), current_exe() can be the link, not the app's binary. Resolve
    // it on Unix. Not on Windows, where canonicalize() returns a \\?\ path.
    #[cfg(unix)]
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    let beside = exe.parent()?.join(name);
    beside.is_file().then_some(beside)
}

/// Where the installers put the models, relative to the directory holding
/// the sidecar: macOS `Contents/MacOS/../Resources/models`; Windows
/// `models` beside it; Linux .deb and AppImage `usr/bin/../lib/Jura
/// Trace/models` (Tauri's resource directory, named after productName).
/// Until v1.2.0 the Linux layout was missing, so `jura-trace-api` on an
/// installed Linux system would have started the sidecar without its models.
const MODEL_DIRS: [&str; 3] = ["../Resources/models", "models", "../lib/Jura Trace/models"];

/// The models directory that belongs with a sidecar binary.
pub fn models_dir_for(sidecar_binary: &Path) -> Option<PathBuf> {
    let dir = sidecar_binary.parent()?;
    MODEL_DIRS
        .iter()
        .map(|rel| dir.join(rel))
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
/// This function runs at startup BEFORE the spawn_sidecar call, kills every
/// `jura-sidecar` process that no live Jura Trace process owns, and waits
/// briefly for the kernel to reap them. The fresh spawn then has a clean
/// process tree.
///
/// # Which sidecars are orphans
///
/// A sidecar is owned by the nearest ancestor process named `jura-trace`
/// (the desktop app) or `jura-trace-api` (the headless server). It is swept
/// when it has no such ancestor, because its owner died and it was
/// reparented, or when the owner is this process, which is about to start a
/// replacement (the desktop's respawn path) or is shutting down.
///
/// Until 5 October 2026 the sweep killed every process NAMED jura-sidecar.
/// Starting a second `jura-trace-api` then killed the first one's live
/// sidecar mid-run: a golden-verdict capture lost CLIP on 63 of 217 images,
/// through nothing louder than `degraded: true`. `jura-trace-api` guarded
/// only the desktop app's sidecar, by asking whether port 8300 answered.
/// Owned sidecars are now left alone whoever owns them.
///
/// # Cross-platform notes
///
/// - macOS / Linux: the process table comes from `ps -A -o pid= -o ppid=
///   -o comm=`, and orphans get `kill -KILL`. Every sidecar process is
///   named jura-sidecar: on macOS the launcher stub execs the bundle's
///   `jura-sidecar`, and on Linux the `--onefile` bootloader's Python child
///   is the same executable, whose parent is the bootloader, so the walk up
///   finds the owner through it. Until 2 October 2026 this used
///   `pkill -f`, which matched any process whose command line mentioned the
///   name and killed unrelated processes; matching is on the process name.
/// - Windows: the table comes from a Toolhelp snapshot and orphans are ended
///   with `TerminateProcess`, in process. That covers both the B1 launcher
///   and the onedir bootloader it runs, which share the name: once the
///   launcher has ended, the bootloader has no owner. Until 23 September 2026
///   this ran `taskkill /F /IM jura-sidecar.exe` with `.output()`, and on
///   `windows-latest` that child lived 4 to 5 s when the app started it
///   (0.04 s run by hand), blocking setup, the sidecar spawn and the local
///   API for the whole time (BL-PERF-001). The sweep must stay BEFORE the
///   spawn: run later, it would kill the new sidecar, which this process
///   owns.
///
/// Best-effort: if the process table cannot be read, or nothing matches, we
/// log at DEBUG and proceed. Orphans staying alive is a memory / disk
/// concern, not a correctness one. The fresh sidecar picks a different
/// ephemeral port via Option C either way.
pub(crate) fn kill_orphan_sidecars() {
    let started = std::time::Instant::now();
    let table = match process_table() {
        Ok(t) => t,
        Err(e) => {
            log::debug!("Orphan-kill: could not read the process table ({e}); skipping");
            return;
        }
    };
    let orphans = orphan_sidecars(&table, std::process::id());
    let owned = table
        .iter()
        .filter(|p| is_named(&p.name, SIDECAR_NAME))
        .count()
        - orphans.len();
    if orphans.is_empty() {
        log::debug!(
            "Orphan-kill: no orphan jura-sidecar processes ({owned} owned by a running Jura Trace left alone, {} ms)",
            started.elapsed().as_millis()
        );
        return;
    }
    let ended = terminate(&orphans);
    log::info!(
        "Orphan-kill: ended {ended} of {} orphan jura-sidecar process(es) {orphans:?}; \
         {owned} owned by a running Jura Trace left alone ({} ms)",
        orphans.len(),
        started.elapsed().as_millis()
    );
    std::thread::sleep(std::time::Duration::from_millis(300));
}

/// The sidecar's process name, without `.exe`.
const SIDECAR_NAME: &str = "jura-sidecar";

/// Process names that own a sidecar: the desktop app and the headless server.
const SIDECAR_OWNERS: [&str; 2] = ["jura-trace", "jura-trace-api"];

/// One row of the process table.
#[derive(Debug, Clone)]
struct Proc {
    pid: u32,
    ppid: u32,
    /// The executable's name or path, as the platform reports it.
    name: String,
}

/// Whether a process name (a bare name, a path, with or without `.exe`, any
/// case) is `want`.
fn is_named(name: &str, want: &str) -> bool {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let base = base
        .strip_suffix(".exe")
        .or_else(|| base.strip_suffix(".EXE"))
        .unwrap_or(base);
    base.eq_ignore_ascii_case(want)
}

/// The pids of the sidecars in `table` that are orphans: no ancestor is a
/// Jura Trace process, or the nearest one is `own_pid`. See
/// [`kill_orphan_sidecars`].
fn orphan_sidecars(table: &[Proc], own_pid: u32) -> Vec<u32> {
    let by_pid: std::collections::HashMap<u32, &Proc> = table.iter().map(|p| (p.pid, p)).collect();
    let owner = |p: &Proc| -> Option<u32> {
        let mut cur = p.ppid;
        // Bounded: a reused pid can make the parent links loop.
        for _ in 0..64 {
            let parent = by_pid.get(&cur)?;
            if SIDECAR_OWNERS.iter().any(|o| is_named(&parent.name, o)) {
                return Some(parent.pid);
            }
            if parent.ppid == parent.pid || parent.ppid == 0 {
                return None;
            }
            cur = parent.ppid;
        }
        None
    };
    table
        .iter()
        .filter(|p| p.pid != own_pid && is_named(&p.name, SIDECAR_NAME))
        .filter(|p| match owner(p) {
            None => true,
            Some(o) => o == own_pid,
        })
        .map(|p| p.pid)
        .collect()
}

/// `ps -A -o pid= -o ppid= -o comm=`. On macOS `comm` is the executable's
/// full path, which may contain spaces, so it is the rest of the line.
#[cfg(unix)]
fn process_table() -> std::io::Result<Vec<Proc>> {
    let out = std::process::Command::new("ps")
        .args(["-A", "-o", "pid=", "-o", "ppid=", "-o", "comm="])
        .output()?;
    if !out.status.success() {
        return Err(std::io::Error::other(format!("ps exited {}", out.status)));
    }
    Ok(parse_ps(&String::from_utf8_lossy(&out.stdout)))
}

#[cfg(unix)]
fn parse_ps(text: &str) -> Vec<Proc> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (pid, rest) = line.split_once(char::is_whitespace)?;
            let rest = rest.trim_start();
            let (ppid, name) = rest.split_once(char::is_whitespace)?;
            Some(Proc {
                pid: pid.parse().ok()?,
                ppid: ppid.parse().ok()?,
                name: name.trim().to_string(),
            })
        })
        .collect()
}

#[cfg(unix)]
fn terminate(pids: &[u32]) -> usize {
    pids.iter()
        .filter(|pid| {
            std::process::Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .output()
                .is_ok_and(|o| o.status.success())
        })
        .count()
}

/// Every process, from a Toolhelp snapshot. See [`kill_orphan_sidecars`] for
/// why this replaced `taskkill`.
#[cfg(windows)]
fn process_table() -> std::io::Result<Vec<Proc>> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    // SAFETY: plain Win32 calls. The snapshot handle is checked before use
    // and closed on the way out.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut table = Vec::new();
        let mut more = Process32FirstW(snapshot, &mut entry) != 0;
        while more {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            table.push(Proc {
                pid: entry.th32ProcessID,
                ppid: entry.th32ParentProcessID,
                name: String::from_utf16_lossy(&entry.szExeFile[..len]),
            });
            more = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
        Ok(table)
    }
}

#[cfg(windows)]
fn terminate(pids: &[u32]) -> usize {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    let mut ended = 0;
    for &pid in pids {
        // SAFETY: the handle is checked before use and closed after.
        unsafe {
            let process = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if !process.is_null() {
                if TerminateProcess(process, 1) != 0 {
                    ended += 1;
                }
                CloseHandle(process);
            }
        }
    }
    ended
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

#[cfg(test)]
mod tests {

    fn proc(pid: u32, ppid: u32, name: &str) -> Proc {
        Proc {
            pid,
            ppid,
            name: name.to_string(),
        }
    }

    #[test]
    fn orphan_sweep_spares_sidecars_another_jura_trace_owns() {
        // Two servers, each with a sidecar; the second one (pid 20) sweeps.
        // Seen 5 October 2026: the name-only sweep killed pid 11.
        let table = [
            proc(1, 0, "/sbin/launchd"),
            proc(
                10,
                5,
                "/Volumes/MAC SSD/dev/cargo-target/debug/jura-trace-api",
            ),
            proc(
                11,
                10,
                "/Applications/Jura Trace.app/Contents/MacOS/jura-sidecar",
            ),
            proc(
                20,
                5,
                "/Volumes/MAC SSD/dev/cargo-target/debug/jura-trace-api",
            ),
            proc(
                30,
                1,
                "/Applications/Jura Trace.app/Contents/MacOS/jura-trace",
            ),
            proc(
                31,
                30,
                "/Applications/Jura Trace.app/Contents/MacOS/jura-sidecar",
            ),
        ];
        assert!(orphan_sidecars(&table, 20).is_empty());
    }

    #[test]
    fn orphan_sweep_takes_reparented_sidecars_and_its_own() {
        let table = [
            proc(1, 0, "launchd"),
            // An owner that died: reparented to pid 1.
            proc(40, 1, "jura-sidecar"),
            // A Linux --onefile pair whose owner died: the child's parent is
            // the bootloader, so both are orphans.
            proc(50, 1, "jura-sidecar"),
            proc(51, 50, "jura-sidecar"),
            // This process's own sidecar (desktop respawn; headless shutdown).
            proc(60, 1, "jura-trace"),
            proc(61, 60, "jura-sidecar"),
            // A Windows bootloader whose launcher has ended: parent pid gone.
            proc(70, 999, "jura-sidecar.exe"),
            // Not a sidecar.
            proc(80, 1, "tail"),
        ];
        let mut got = orphan_sidecars(&table, 60);
        got.sort();
        assert_eq!(got, vec![40, 50, 51, 61, 70]);
    }

    #[test]
    fn orphan_sweep_finds_the_owner_through_a_launcher_and_a_bootloader() {
        // Windows: app -> B1 launcher -> onedir bootloader, all live.
        let table = [
            proc(4, 0, "System"),
            proc(100, 4, "Jura-Trace-Api.EXE"),
            proc(101, 100, "jura-sidecar.exe"),
            proc(102, 101, "jura-sidecar.exe"),
        ];
        assert!(orphan_sidecars(&table, 999).is_empty());
        // A pid-reuse loop terminates.
        let looped = [proc(7, 8, "jura-sidecar"), proc(8, 7, "sh")];
        assert_eq!(orphan_sidecars(&looped, 1), vec![7]);
    }

    #[test]
    fn process_names_match_exactly() {
        assert!(is_named("jura-sidecar", "jura-sidecar"));
        assert!(is_named(
            "C:\\Program Files\\Jura Trace\\jura-sidecar.exe",
            "jura-sidecar"
        ));
        assert!(!is_named("jura-sidecar.log", "jura-sidecar"));
        assert!(!is_named("jura-trace-api", "jura-trace"));
        assert!(!is_named("tail", "jura-sidecar"));
    }

    #[cfg(unix)]
    #[test]
    fn ps_output_parses_paths_with_spaces() {
        let table = parse_ps(
            "    1     0 /sbin/launchd\n24157 24143 /Volumes/MAC SSD/dev/cargo-target/debug/jura-trace-api\n garbage\n",
        );
        assert_eq!(table.len(), 2);
        assert_eq!(table[1].pid, 24157);
        assert_eq!(table[1].ppid, 24143);
        assert!(is_named(&table[1].name, "jura-trace-api"));
    }

    #[cfg(unix)]
    #[test]
    fn the_real_process_table_contains_this_process() {
        let table = process_table().unwrap();
        assert!(table.iter().any(|p| p.pid == std::process::id()));
    }
    use super::*;

    /// Each installer's layout, built in a temporary directory.
    #[test]
    fn models_are_found_in_every_installed_layout() {
        for (bin_dir, models) in [
            (
                "Jura Trace.app/Contents/MacOS",
                "Jura Trace.app/Contents/Resources/models",
            ),
            ("Jura Trace", "Jura Trace/models"),
            ("usr/bin", "usr/lib/Jura Trace/models"),
        ] {
            let root = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(root.path().join(bin_dir)).unwrap();
            std::fs::create_dir_all(root.path().join(models)).unwrap();
            let sidecar = root.path().join(bin_dir).join("jura-sidecar");
            std::fs::write(&sidecar, b"").unwrap();
            let found = models_dir_for(&sidecar).unwrap_or_else(|| panic!("{bin_dir}: no models"));
            assert_eq!(
                std::fs::canonicalize(found).unwrap(),
                std::fs::canonicalize(root.path().join(models)).unwrap(),
                "{bin_dir}"
            );
        }
    }

    #[test]
    fn no_models_is_none() {
        let root = tempfile::tempdir().unwrap();
        let sidecar = root.path().join("jura-sidecar");
        std::fs::write(&sidecar, b"").unwrap();
        assert_eq!(models_dir_for(&sidecar), None);
    }
}
