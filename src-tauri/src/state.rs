// SPDX-License-Identifier: AGPL-3.0-or-later
//! Application state types shared across Tauri commands.
//!
//! [`AppState`] is the single shared mutable state object managed by Tauri.
//! [`LicenceTier`] controls feature gating for Community / Professional /
//! Enterprise installations. [`SidecarStartupStatus`] and
//! [`SidecarStartupSnapshot`] carry the sidecar probe lifecycle so the
//! Settings UI can show a "Connecting…" indicator during the PyInstaller
//! cold-extract window on first launch.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8};
use std::sync::Arc;

use crate::{db, monitor_scheduler, sidecar};

/// Managed application state shared across Tauri commands.
/// JTV-184 Phase 1 — sidecar startup status surfaced to the frontend.
///
/// On a clean install of the v0.9.0 .app, the PyInstaller cold-extract of the
/// sidecar binary takes ~90 s (post-Phase-0 CLIP-strip; previously 4+ min on
/// the 731 MB bundle). Tauri's earlier give-up budget was ~140 s and the
/// Settings page only re-probed `/health` on manual Refresh — so a user who
/// opened Settings before ~90 s saw "Analysis Engine offline" and assumed the
/// app was broken.
///
/// This enum carries the probe lifecycle: it starts `Connecting` the moment
/// `spawn_sidecar` returns a child handle, transitions to `Ready` when
/// `/health/ready` first returns 200, and stays there for the rest of the
/// session. `NotPresent` covers dev builds (where `spawn_sidecar` returns
/// `None`) and the case where spawn itself failed. There is no `Failed` state
/// in v1.0 — the probe loop runs indefinitely so a late-arriving sidecar still
/// flips to `Ready`; if the process truly died, the existing per-request
/// `SidecarClient::is_available()` check (sidecar.rs) reports the gap.
///
/// Encoded as a `u8` so it can live in an `Arc<AtomicU8>` on AppState for
/// lock-free reads from the Tauri command and lock-free writes from the
/// background probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SidecarStartupStatus {
    /// Dev build (debug_assertions) or `spawn_sidecar` returned `None`. The
    /// frontend should render this as "Not running (development mode — start
    /// uvicorn manually)" rather than as a startup-in-progress state.
    NotPresent,
    /// Probe is in flight. The frontend should render an elapsed-time counter
    /// with explanatory copy ("Connecting (this can take ~1–2 minutes on the
    /// first launch after install while the analysis engine extracts").
    Connecting,
    /// `/health/ready` returned 200. The sidecar is reachable. Frontend
    /// renders the green Connected badge.
    Ready,
}

impl SidecarStartupStatus {
    pub fn to_u8(self) -> u8 {
        match self {
            Self::NotPresent => 0,
            Self::Connecting => 1,
            Self::Ready => 2,
        }
    }

    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Connecting,
            2 => Self::Ready,
            _ => Self::NotPresent,
        }
    }
}

/// Snapshot returned by the `get_sidecar_startup_status` Tauri command. The
/// elapsed counter lets the Settings page render "Connecting (32s elapsed)"
/// without the frontend having to track the start time itself (avoids a clock-
/// skew bug if the user's machine is in low-power-throttle).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarStartupSnapshot {
    pub status: SidecarStartupStatus,
    /// Seconds since the probe started. `0` until the probe begins (which
    /// is approximately `setup()` completion + the first `spawn_sidecar`
    /// call) and remains monotonically increasing thereafter.
    pub elapsed_secs: u64,
}

pub struct AppState {
    pub db: db::Database,
    pub sidecar: sidecar::SidecarClient,
    /// The current database file path (may differ from app_data_dir default
    /// if the user has configured a custom location).
    pub db_path: String,
    /// Active licence tier for this installation (pilot phase: manually settable).
    pub licence_tier: LicenceTier,
    /// Handle to the spawned PyInstaller sidecar process.
    /// Present only in production builds where the binary was found and launched
    /// successfully. `None` in development (manual uvicorn) or if spawn failed.
    pub sidecar_process: Option<tauri_plugin_shell::process::CommandChild>,
    /// SHA-256 hex digest of the GBM classifier model file, computed once at
    /// startup. `None` if the model file is not present.
    pub classifier_model_hash: Option<String>,
    /// SHA-256 hex digest of the UnivFD CLIP probe (`models/univfd_probe.joblib`),
    /// computed once at startup. `None` if the optional CLIP probe file is
    /// not present. Surfaced on every VerificationResult via MethodologyRecord
    /// (JTV-181) so the v1.0.1 `jura` CLI and external reproducibility tooling
    /// can pin the exact CLIP ensemble used to produce a given result.
    pub univfd_probe_model_hash: Option<String>,
    /// User preference for AI image descriptions via Ollama LLaVA.
    ///
    /// - `Some(true)`  — explicitly enabled by the user
    /// - `Some(false)` — explicitly disabled by the user
    /// - `None`        — not yet decided; treated as disabled at verify time
    ///   to protect perf until the user opts in via Settings. The setup wizard
    ///   and Settings page are expected to resolve this to an explicit value.
    ///
    /// This feature adds 5–30 s per image verify when active and depends on
    /// Ollama + LLaVA being installed. Gated here so users who care about
    /// verify speed can turn it off, while users who want rich descriptions
    /// can keep it on.
    pub ai_description_enabled: Option<bool>,
    /// Handle for the background URL watchlist scheduler (Monitor tab, paid tiers).
    ///
    /// `None` before the scheduler has been started.  Used in the
    /// `RunEvent::Exit` handler to cleanly stop the task.
    pub scheduler_handle: Option<monitor_scheduler::SchedulerHandle>,
    /// UUID of the most recent verify session whose heatmap files are still
    /// on disk. Cleared when a new session starts (the previous session dir
    /// is deleted before writing new files).
    pub last_heatmap_session: Option<String>,
    /// Unix timestamp (seconds since epoch) of the last successful HTTP request
    /// dispatched to the Python sidecar. Updated atomically by the verify pipeline
    /// on every sidecar call. Used by the power-saver idle-killer to determine
    /// whether the sidecar has been idle long enough to terminate.
    ///
    /// Wrapped in `Arc` so it can be cheaply shared with background tasks
    /// without holding the `AppState` mutex.
    pub last_sidecar_request_ts: Arc<AtomicU64>,
    /// Whether power-saver mode is active. When `true`, the sidecar process is
    /// terminated after `SIDECAR_IDLE_SECONDS_BEFORE_KILL` seconds of inactivity
    /// and respawned on the next verification request. Default `false`.
    pub power_saver_mode: bool,
    /// Serialises the power-saver respawn sequence so that two concurrent
    /// verify calls cannot each pass the `sidecar_process.is_none()` check
    /// and independently spawn duplicate processes. Set with
    /// `compare_exchange(false, true)` before spawning; cleared once the
    /// new child handle is stored.
    pub respawn_in_progress: Arc<AtomicBool>,
    /// Loopback TCP port on which the Python sidecar is listening for this
    /// session (Option C port-collision fix, 2026-05-12). Picked once at
    /// startup via `pick_ephemeral_port()` so a stale sidecar from a previous
    /// launch / CI runner / unrelated process holding port 8200 cannot
    /// prevent the new app from starting. All HTTP clients (the readiness
    /// poller, `SidecarClient`, the Ollama-pull IPC proxy) construct their
    /// URLs from this port. Stable across the lifetime of the process —
    /// power-saver respawns reuse the same port.
    pub sidecar_port: u16,
    /// JTV-184 Phase 1 — sidecar startup status surfaced to the Settings UI so
    /// users on a clean install see "Connecting…" instead of "Offline" during
    /// the PyInstaller cold-extract window.
    ///
    /// Encoded as a `u8` for lock-free atomic access via
    /// [`SidecarStartupStatus::from_u8`] / [`SidecarStartupStatus::to_u8`].
    /// Updated by the background readiness probe spawned in `run()`; read by
    /// the `get_sidecar_startup_status` Tauri command and surfaced via the
    /// `sidecar-status-changed` Tauri event on every transition.
    pub sidecar_startup_status: Arc<AtomicU8>,
    /// Unix epoch (seconds) when the sidecar startup probe began. Used by the
    /// frontend to render an elapsed-seconds counter while the probe is in
    /// the `Connecting` state ("Connecting (32s elapsed)"). `0` until the
    /// probe starts; never reset (subsequent power-saver respawns reuse the
    /// same start-time so the elapsed counter measures total session uptime,
    /// not respawn freshness).
    pub sidecar_startup_started_at: Arc<AtomicU64>,
}

/// The licence tier active for this installation.
///
/// Internal codenames (Flint / Stratum / Bedrock) are used in code;
/// user-facing display maps these to plain English names
/// (Community / Professional / Enterprise).
///
/// The `Team` variant was retired on 2026-05-04 (3-tier simplification).
/// `#[serde(alias = "team")]` on `Professional` rolls any pilot config that
/// still carries `"team"` up to Professional with no manual migration needed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LicenceTier {
    /// Free tier — AGPL-3.0-or-later. Full verify pipeline + Sovereign-mode signing.
    #[default]
    Community,
    /// Individual commercial licence — £199/year.
    #[serde(alias = "team", alias = "pro")]
    Professional,
    /// Enterprise licence — from £6,000/year, unlimited seats.
    Enterprise,
}
