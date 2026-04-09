//! Automated URL watchlist scheduler for the Monitor tab.
//!
//! This module polls the `monitor_urls` table on a fixed cadence and runs the
//! full verify pipeline against each URL that is due for a check.  It is
//! spawned as a background task on app startup and cancelled cleanly on exit.
//!
//! ## Tier gating
//! The scheduler is **only active on Professional, Team, and Enterprise tiers**.
//! On the Community tier it logs a single notice and then idles indefinitely.
//! This enforcement is in-process and cannot be bypassed via the UI.
//!
//! ## Politeness
//! A [`INTER_URL_DELAY_SECS`]-second sleep is inserted between consecutive URL
//! checks within one wake cycle so the scheduler does not hammer downstream
//! servers when the watchlist is large.
//!
//! ## Cancellation
//! Call [`SchedulerHandle::cancel`] to stop the background task.  The handle
//! wraps an `Arc<AtomicBool>` flag; the scheduler loop checks it on every
//! wake and terminates without panicking.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::db::MonitorEvent;
use crate::{AppState, LicenceTier};

// ── Constants ────────────────────────────────────────────────────────────────

/// How often the scheduler wakes to check which URLs are due.
/// Individual URLs are only verified when their own `check_frequency` cadence
/// has elapsed — this constant is just the resolution of the polling loop.
const POLL_CADENCE_SECS: u64 = 60;

/// Politeness delay between consecutive URL checks within a single wake cycle.
/// Prevents the scheduler from hammering downstream servers when the watchlist
/// contains many entries.
const INTER_URL_DELAY_SECS: u64 = 1;

// ── Public API ───────────────────────────────────────────────────────────────

/// A handle that can be used to cancel the background scheduler task.
///
/// Dropping the handle does **not** cancel the task — call [`cancel`] explicitly.
///
/// [`cancel`]: SchedulerHandle::cancel
#[derive(Clone)]
pub struct SchedulerHandle {
    cancel_flag: Arc<AtomicBool>,
}

impl SchedulerHandle {
    fn new() -> Self {
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal the scheduler loop to stop on its next wake.
    ///
    /// This is a soft cancellation — the current in-progress check (if any)
    /// runs to completion before the loop exits.
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }
}

/// Spawn the monitor scheduler as a background Tauri async task.
///
/// Returns a [`SchedulerHandle`] that the caller (typically `lib.rs`'s
/// `RunEvent::Exit` handler) can use to stop the task.
///
/// The scheduler will not perform any checks on the [`LicenceTier::Community`]
/// tier — it logs a single notice and idles until cancelled.
pub fn spawn_scheduler(state: Arc<Mutex<AppState>>) -> SchedulerHandle {
    let handle = SchedulerHandle::new();
    let handle_clone = handle.clone();

    tauri::async_runtime::spawn(async move {
        run_scheduler(state, handle_clone).await;
    });

    handle
}

// ── Core loop ────────────────────────────────────────────────────────────────

/// Main scheduler loop.  Runs until [`SchedulerHandle::is_cancelled`] returns `true`.
async fn run_scheduler(state: Arc<Mutex<AppState>>, handle: SchedulerHandle) {
    // Check the licence tier once at startup so we emit at most one log line.
    let tier = {
        match state.lock() {
            Ok(guard) => guard.licence_tier,
            Err(e) => {
                log::error!("Monitor scheduler: AppState mutex poisoned on startup: {e}");
                return;
            }
        }
    };

    match tier {
        LicenceTier::Community => {
            log::info!(
                "Monitor scheduler: not active on the Community tier. \
                 Upgrade to Professional, Team, or Enterprise to enable \
                 automated URL watchlist checking."
            );
            // Idle until cancelled — keep the task alive so the handle remains valid.
            loop {
                if handle.is_cancelled() {
                    return;
                }
                tokio::time::sleep(Duration::from_secs(POLL_CADENCE_SECS)).await;
            }
        }
        LicenceTier::Professional | LicenceTier::Team | LicenceTier::Enterprise => {
            log::info!(
                "Monitor scheduler: started (cadence {}s, tier {:?}).",
                POLL_CADENCE_SECS,
                tier
            );
        }
    }

    loop {
        if handle.is_cancelled() {
            log::info!("Monitor scheduler: cancelled, shutting down.");
            return;
        }

        // Sleep first — on app start there is nothing urgent; real-time checks
        // are initiated by the user directly via the Monitor tab.
        tokio::time::sleep(Duration::from_secs(POLL_CADENCE_SECS)).await;

        if handle.is_cancelled() {
            log::info!("Monitor scheduler: cancelled, shutting down.");
            return;
        }

        // Re-read the tier on every wake cycle in case it was changed at runtime.
        let current_tier = {
            match state.lock() {
                Ok(guard) => guard.licence_tier,
                Err(e) => {
                    log::warn!("Monitor scheduler: state lock poisoned during poll: {e}");
                    continue;
                }
            }
        };

        if matches!(current_tier, LicenceTier::Community) {
            log::info!("Monitor scheduler: licence tier is now Community — skipping poll cycle.");
            continue;
        }

        run_poll_cycle(&state).await;
    }
}

/// Execute one wake-up cycle: fetch enabled URLs, check which are due, verify
/// each in turn with a politeness delay between requests.
async fn run_poll_cycle(state: &Arc<Mutex<AppState>>) {
    let urls = {
        match state.lock() {
            Ok(guard) => match guard.db.list_monitor_urls(true) {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("Monitor scheduler: failed to list monitor URLs: {e}");
                    return;
                }
            },
            Err(e) => {
                log::warn!("Monitor scheduler: state lock error during poll: {e}");
                return;
            }
        }
    };

    if urls.is_empty() {
        return;
    }

    let now = Utc::now();
    let mut checked_count = 0u32;

    for monitor_url in &urls {
        // Fetch the most recent event for this URL to compute elapsed time.
        let last_event: Option<MonitorEvent> = {
            match state.lock() {
                Ok(guard) => match guard.db.get_latest_monitor_event(&monitor_url.url_id) {
                    Ok(ev) => ev,
                    Err(e) => {
                        log::warn!(
                            "Monitor scheduler: could not fetch latest event for {}: {e}",
                            monitor_url.url_id
                        );
                        None
                    }
                },
                Err(e) => {
                    log::warn!("Monitor scheduler: state lock error for event lookup: {e}");
                    None
                }
            }
        };

        // Parse last event timestamp; treat parse failure as "never checked".
        let last_at: Option<DateTime<Utc>> = last_event
            .as_ref()
            .and_then(|ev| DateTime::parse_from_rfc3339(&ev.checked_at).ok())
            .map(|dt| dt.with_timezone(&Utc));

        if !is_due(last_at, &monitor_url.check_frequency, now) {
            continue;
        }

        // Politeness delay between consecutive network requests.
        if checked_count > 0 {
            tokio::time::sleep(Duration::from_secs(INTER_URL_DELAY_SECS)).await;
        }

        log::info!(
            "Monitor scheduler: checking {} (frequency: {})",
            monitor_url.url,
            monitor_url.check_frequency
        );

        check_url(state, monitor_url, last_event.as_ref()).await;
        checked_count += 1;
    }

    if checked_count > 0 {
        log::info!("Monitor scheduler: poll cycle complete — {checked_count} URL(s) checked.");
    }
}

/// Run the full verify pipeline against a single monitored URL and persist the
/// result as a `monitor_events` row.
///
/// Uses `standard` mode for all automated checks — a reasonable balance between
/// thoroughness and performance.  Deep and archival modes require explicit user
/// invocation from the Monitor tab.
async fn check_url(
    state: &Arc<Mutex<AppState>>,
    monitor_url: &crate::db::MonitorUrl,
    previous_event: Option<&MonitorEvent>,
) {
    // verify_url_inner is synchronous (uses reqwest::blocking), so we must call
    // it on a blocking thread to avoid starving the async runtime.
    let url = monitor_url.url.clone();
    let url_id = monitor_url.url_id.clone();
    let state_arc = state.clone();
    let prev_hash = previous_event.and_then(|ev| ev.content_hash.clone());

    let result = tokio::task::spawn_blocking(move || {
        crate::verify_url_inner(&url, Some("standard"), &state_arc)
    })
    .await;

    match result {
        Err(join_err) => {
            // The blocking task itself panicked — log and record an error event.
            log::error!("Monitor scheduler: blocking task panicked for {url_id}: {join_err}");
            persist_error_event(state, &url_id, &format!("Internal error: {join_err}")).await;
        }
        Ok(Err(app_err)) => {
            // verify_url_inner returned an application-level error (network
            // failure, SSRF blocked, invalid content type, etc.).
            log::warn!("Monitor scheduler: verify failed for {url_id}: {app_err}");
            persist_error_event(state, &url_id, &app_err.to_string()).await;
        }
        Ok(Ok(result)) => {
            // Successful verify — compute event type from delta.
            let new_hash = result.input_sha256.as_deref();
            let c2pa_valid = result.c2pa_valid;
            let watermark_confidence = result
                .watermark_extract_result
                .as_ref()
                .map(|w| w.confidence);
            let watermark_uuid = result
                .watermark_extract_result
                .as_ref()
                .and_then(|w| w.extracted_payload.clone());
            let watermark_match = result
                .watermark_extract_result
                .as_ref()
                .map(|w| w.has_watermark);

            let event_type = determine_event_type(
                new_hash,
                prev_hash.as_deref(),
                c2pa_valid,
                previous_event.and_then(|e| e.c2pa_valid),
            );

            // Persist event and update the summary row.
            let url_id2 = url_id.clone();
            let event_type2 = event_type.clone();
            let new_hash_owned = new_hash.map(str::to_string);
            let wm_uuid = watermark_uuid.clone();

            let persist_ok = {
                match state.lock() {
                    Err(e) => {
                        log::warn!(
                            "Monitor scheduler: state lock error during persist for {url_id}: {e}"
                        );
                        false
                    }
                    Ok(guard) => {
                        let ev_result = guard.db.insert_monitor_event(
                            &url_id2,
                            &event_type2,
                            new_hash_owned.as_deref(),
                            c2pa_valid,
                            wm_uuid.as_deref(),
                            watermark_confidence,
                            None, // http_status — not exposed by verify_url_inner
                            None, // response_time_ms — not timed at this layer
                            None, // detail_json
                        );
                        if let Err(ref e) = ev_result {
                            log::warn!(
                                "Monitor scheduler: failed to insert event for {url_id2}: {e}"
                            );
                        }

                        let upd_result = guard.db.update_monitor_url_last_checked(
                            &url_id2,
                            &event_type2,
                            new_hash_owned.as_deref(),
                            c2pa_valid,
                            watermark_match,
                        );
                        if let Err(ref e) = upd_result {
                            log::warn!(
                                "Monitor scheduler: failed to update last_checked for {url_id2}: {e}"
                            );
                        }

                        ev_result.is_ok() && upd_result.is_ok()
                    }
                }
            };

            if persist_ok {
                log::info!(
                    "Monitor scheduler: {url_id} — event_type={event_type}, \
                     trust={:.2}, c2pa={:?}",
                    result.overall_trust,
                    c2pa_valid,
                );
            }
        }
    }
}

/// Persist a scheduler error event without crashing the loop.
async fn persist_error_event(state: &Arc<Mutex<AppState>>, url_id: &str, message: &str) {
    // Truncate the message to avoid bloating the database with enormous traces.
    let detail = serde_json::json!({ "error": &message[..message.len().min(500)] }).to_string();

    if let Ok(guard) = state.lock() {
        let _ = guard.db.insert_monitor_event(
            url_id,
            "error",
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&detail),
        );
        let _ = guard
            .db
            .update_monitor_url_last_checked(url_id, "error", None, None, None);
    }
}

/// Choose the `event_type` string based on what changed between this check and
/// the previous one.
///
/// Priority order:
/// 1. `content_changed` — the SHA-256 hash of the content changed
/// 2. `c2pa_stripped`   — C2PA was valid before, now absent/invalid
/// 3. `c2pa_changed`    — C2PA validity status changed in any other direction
/// 4. `check_ok`        — nothing changed
fn determine_event_type(
    new_hash: Option<&str>,
    prev_hash: Option<&str>,
    new_c2pa: Option<bool>,
    prev_c2pa: Option<bool>,
) -> String {
    if let (Some(n), Some(p)) = (new_hash, prev_hash) {
        if n != p {
            return "content_changed".to_string();
        }
    } else if new_hash.is_some() != prev_hash.is_some() {
        // One is None (first check or missing hash) — treat as changed only
        // when there was a previous hash to compare against.
        if prev_hash.is_some() {
            return "content_changed".to_string();
        }
    }

    match (prev_c2pa, new_c2pa) {
        (Some(true), Some(false)) | (Some(true), None) => {
            return "c2pa_stripped".to_string();
        }
        (prev, cur) if prev != cur => {
            return "c2pa_changed".to_string();
        }
        _ => {}
    }

    "check_ok".to_string()
}

// ── Pure helpers (testable without I/O) ──────────────────────────────────────

/// Return `true` if a URL is due for a check given the timestamp of its most
/// recent event and its configured `check_frequency`.
///
/// If `last_event_at` is `None` the URL has never been checked and is always
/// considered due.
pub fn is_due(last_event_at: Option<DateTime<Utc>>, frequency: &str, now: DateTime<Utc>) -> bool {
    match last_event_at {
        None => true, // Never checked — always due.
        Some(last) => {
            let elapsed = now.signed_duration_since(last);
            match frequency {
                "hourly" => elapsed >= chrono::Duration::hours(1),
                "daily" => elapsed >= chrono::Duration::days(1),
                "weekly" => elapsed >= chrono::Duration::weeks(1),
                other => {
                    log::warn!(
                        "Monitor scheduler: unknown check_frequency {:?} — treating as daily",
                        other
                    );
                    elapsed >= chrono::Duration::days(1)
                }
            }
        }
    }
}

/// Return the UTC timestamp at which the next check is due.
///
/// Returns `now` when `last_event_at` is `None` (i.e. due immediately).
///
/// This function is part of the public API surface for future UI use (e.g.
/// displaying "next check at …" in the Monitor tab) even though it is not yet
/// called from non-test code.
#[allow(dead_code)]
pub fn next_due_after(last_event_at: Option<DateTime<Utc>>, frequency: &str) -> DateTime<Utc> {
    match last_event_at {
        None => Utc::now(),
        Some(last) => match frequency {
            "hourly" => last + chrono::Duration::hours(1),
            "daily" => last + chrono::Duration::days(1),
            "weekly" => last + chrono::Duration::weeks(1),
            _ => last + chrono::Duration::days(1),
        },
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(year: i32, month: u32, day: u32, hour: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, min, 0)
            .unwrap()
    }

    // ── is_due ────────────────────────────────────────────────────────────────

    #[test]
    fn is_due_never_checked_always_true() {
        let now = utc(2026, 4, 7, 12, 0);
        assert!(is_due(None, "daily", now));
        assert!(is_due(None, "hourly", now));
        assert!(is_due(None, "weekly", now));
    }

    #[test]
    fn is_due_hourly_not_elapsed() {
        let last = utc(2026, 4, 7, 12, 0);
        let now = utc(2026, 4, 7, 12, 30); // only 30 min later
        assert!(!is_due(Some(last), "hourly", now));
    }

    #[test]
    fn is_due_hourly_just_elapsed() {
        let last = utc(2026, 4, 7, 12, 0);
        let now = utc(2026, 4, 7, 13, 0); // exactly 1 hour later
        assert!(is_due(Some(last), "hourly", now));
    }

    #[test]
    fn is_due_daily_not_elapsed() {
        let last = utc(2026, 4, 7, 12, 0);
        let now = utc(2026, 4, 7, 23, 59); // under 24 h
        assert!(!is_due(Some(last), "daily", now));
    }

    #[test]
    fn is_due_daily_just_elapsed() {
        let last = utc(2026, 4, 7, 12, 0);
        let now = utc(2026, 4, 8, 12, 0); // exactly 24 h later
        assert!(is_due(Some(last), "daily", now));
    }

    #[test]
    fn is_due_weekly_not_elapsed() {
        let last = utc(2026, 4, 1, 0, 0);
        let now = utc(2026, 4, 6, 23, 59); // under 7 days
        assert!(!is_due(Some(last), "weekly", now));
    }

    #[test]
    fn is_due_weekly_just_elapsed() {
        let last = utc(2026, 4, 1, 0, 0);
        let now = utc(2026, 4, 8, 0, 0); // exactly 7 days later
        assert!(is_due(Some(last), "weekly", now));
    }

    #[test]
    fn is_due_unknown_frequency_falls_back_to_daily() {
        let last = utc(2026, 4, 7, 0, 0);
        let now_under = utc(2026, 4, 7, 23, 59);
        let now_over = utc(2026, 4, 8, 0, 0);
        assert!(!is_due(Some(last), "monthly", now_under));
        assert!(is_due(Some(last), "monthly", now_over));
    }

    // ── next_due_after ────────────────────────────────────────────────────────

    #[test]
    fn next_due_after_never_checked_is_now() {
        // next_due_after(None, _) should be approximately Utc::now().
        // We just check it is in the very recent past or present — not a future
        // time — since we cannot freeze time in a pure unit test.
        let result = next_due_after(None, "daily");
        let diff = Utc::now().signed_duration_since(result);
        // Should be within 1 second either way.
        assert!(diff.num_seconds().abs() < 2);
    }

    #[test]
    fn next_due_after_hourly() {
        let last = utc(2026, 4, 7, 10, 0);
        let expected = utc(2026, 4, 7, 11, 0);
        assert_eq!(next_due_after(Some(last), "hourly"), expected);
    }

    #[test]
    fn next_due_after_daily() {
        let last = utc(2026, 4, 7, 10, 0);
        let expected = utc(2026, 4, 8, 10, 0);
        assert_eq!(next_due_after(Some(last), "daily"), expected);
    }

    #[test]
    fn next_due_after_weekly() {
        let last = utc(2026, 4, 1, 8, 0);
        let expected = utc(2026, 4, 8, 8, 0);
        assert_eq!(next_due_after(Some(last), "weekly"), expected);
    }

    // ── determine_event_type ──────────────────────────────────────────────────

    #[test]
    fn event_type_first_check_no_prev_hash() {
        // First check — no previous hash, C2PA unchanged (both None).
        assert_eq!(
            determine_event_type(Some("abc123"), None, None, None),
            "check_ok"
        );
    }

    #[test]
    fn event_type_content_changed() {
        assert_eq!(
            determine_event_type(Some("newhash"), Some("oldhash"), Some(true), Some(true)),
            "content_changed"
        );
    }

    #[test]
    fn event_type_c2pa_stripped() {
        // C2PA was valid, now invalid.
        assert_eq!(
            determine_event_type(None, None, Some(false), Some(true)),
            "c2pa_stripped"
        );
        // C2PA was valid, now absent.
        assert_eq!(
            determine_event_type(None, None, None, Some(true)),
            "c2pa_stripped"
        );
    }

    #[test]
    fn event_type_c2pa_changed_other_direction() {
        // C2PA was invalid, now valid.
        assert_eq!(
            determine_event_type(None, None, Some(true), Some(false)),
            "c2pa_changed"
        );
    }

    #[test]
    fn event_type_check_ok_no_changes() {
        assert_eq!(
            determine_event_type(Some("same"), Some("same"), Some(true), Some(true)),
            "check_ok"
        );
    }

    // ── Scheduler decision logic (pure, no I/O) ───────────────────────────────

    /// Simulate the decision the scheduler makes when choosing which URLs to
    /// check in a single poll cycle, without any network or database I/O.
    #[test]
    fn scheduler_selects_only_due_urls() {
        let now = utc(2026, 4, 7, 12, 0);

        struct FakeUrl {
            id: &'static str,
            frequency: &'static str,
            last_checked: Option<DateTime<Utc>>,
        }

        let urls = vec![
            FakeUrl {
                id: "url-1",
                frequency: "hourly",
                last_checked: Some(utc(2026, 4, 7, 11, 30)), // 30 min ago — not due
            },
            FakeUrl {
                id: "url-2",
                frequency: "hourly",
                last_checked: Some(utc(2026, 4, 7, 10, 59)), // 61 min ago — due
            },
            FakeUrl {
                id: "url-3",
                frequency: "daily",
                last_checked: None, // never checked — always due
            },
            FakeUrl {
                id: "url-4",
                frequency: "weekly",
                last_checked: Some(utc(2026, 4, 1, 12, 0)), // exactly 6 days ago — not due
            },
            FakeUrl {
                id: "url-5",
                frequency: "weekly",
                last_checked: Some(utc(2026, 3, 31, 12, 0)), // 7 days ago — due
            },
        ];

        let due: Vec<&str> = urls
            .iter()
            .filter(|u| is_due(u.last_checked, u.frequency, now))
            .map(|u| u.id)
            .collect();

        assert_eq!(due, vec!["url-2", "url-3", "url-5"]);
    }
}
