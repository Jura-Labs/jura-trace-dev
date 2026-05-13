// SPDX-License-Identifier: AGPL-3.0-or-later

//! JTV-98 BYOK Reverse Image Search — key store + provider whitelist.
//!
//! Stage 0 surface: provider validation + key length validation +
//! `keyring`-crate wrappers for the OS-native key store. NO HTTP / no
//! provider integration here — that lives in Stages 1 + 2.
//!
//! # Locked design (project_jtv98_ris_design.md)
//! - Service name `org.juralabs.trace`; account = provider name.
//! - Provider whitelist `["tineye", "google_vision"]`. Exact equality.
//! - NEVER log key values. Log only `"RIS key set for provider: {provider}"`.
//! - macOS Keychain / Windows Credential Manager / libsecret on Linux.
//!   Linux without a secret-service daemon hard-fails with
//!   [`AppError::Configuration`] — never a plaintext fallback.
//!
//! # Threat model notes (security-auditor 2026-05-03)
//! - Provider whitelist runs FIRST in every public call to prevent slot
//!   collision via path-traversal-style names or unicode homographs.
//! - Key length validation rejects empty / blank / too-short strings;
//!   the provider's own 401/403 surfaces semantic-format errors.
//! - `list_configured_providers` queries every whitelisted provider
//!   regardless of whether it's known to be present, so the response
//!   time does not differ between "configured" and "not configured".

use crate::error::AppError;

/// Service name used as the `service` field in OS-native key store entries.
/// Stable identifier — DO NOT change without a migration plan; existing
/// installations would lose their saved keys.
const SERVICE_NAME: &str = "org.juralabs.trace";

/// Whitelist of supported reverse image search providers. Exact equality
/// is enforced — no contains, no fuzzy match, no case-folding.
pub const VALID_PROVIDERS: &[&str] = &["tineye", "google_vision"];

/// Default per-provider daily call cap. Used by `db::record_ris_call` when
/// inserting a fresh row. User-adjustable up to [`MAX_DAILY_CAP`] via
/// Settings (Stage 1+ work). Public for the upcoming
/// `record_ris_call(provider, DEFAULT_DAILY_CAP)` call site in Stage 1.
#[allow(dead_code)] // consumed by Stage 1 verify-page wire-up
pub const DEFAULT_DAILY_CAP: u32 = 50;

/// Hard ceiling on the user-adjustable daily cap.
#[allow(dead_code)] // consumed by Stage 1 settings cap-adjust UI
pub const MAX_DAILY_CAP: u32 = 500;

/// Minimum acceptable key length. Both TinEye (40-char SHA-1) and Google
/// Vision (39-char AIza-prefixed) keys exceed this comfortably; the bound
/// catches paste-of-empty-string, paste-of-placeholder, and obvious typos.
const MIN_KEY_LEN: usize = 20;

/// Maximum acceptable key length. Generous bound that catches buffer-overflow
/// shaped inputs without rejecting legitimate provider key formats.
const MAX_KEY_LEN: usize = 512;

// ── Provider validation ──────────────────────────────────────────────

/// Validate the provider name against the whitelist (exact equality).
///
/// Must be the first call inside any public function that takes a provider
/// string. Returns [`AppError::Validation`] with a non-revealing message
/// (does NOT echo the offending name back, to avoid log-injection vectors).
pub fn validate_provider(provider: &str) -> Result<(), AppError> {
    if VALID_PROVIDERS.contains(&provider) {
        Ok(())
    } else {
        Err(AppError::Validation("Unknown provider".to_string()))
    }
}

/// Validate a key string before passing it to the OS keychain.
///
/// Rejects empty / whitespace-only / too-short / too-long / non-printable.
/// Does NOT enforce provider-specific format — semantic validation happens
/// when the user runs their first search and the provider returns 401/403.
fn validate_key(provider: &str, key: &str) -> Result<(), AppError> {
    if key.trim().is_empty() {
        return Err(AppError::Validation("API key is empty".to_string()));
    }
    let len = key.len();
    if len < MIN_KEY_LEN {
        return Err(AppError::Validation("API key is too short".to_string()));
    }
    if len > MAX_KEY_LEN {
        return Err(AppError::Validation("API key is too long".to_string()));
    }
    // Reject control characters and non-ASCII. The two providers in scope
    // both use ASCII-only key formats, and tightening here closes a class
    // of paste-from-rich-text issues.
    if key.chars().any(|c| !c.is_ascii() || c.is_control()) {
        return Err(AppError::Validation(
            "API key contains invalid characters".to_string(),
        ));
    }
    // Provider-specific format checks — fail-fast on common credential-
    // type confusion BEFORE the user ever issues a doomed search call.
    if provider == "google_vision" {
        let trimmed = key.trim();
        if trimmed.ends_with(".apps.googleusercontent.com") {
            return Err(AppError::Validation(
                "This looks like an OAuth Client ID, not an API key. \
                 Google Vision needs an API key (starts with `AIzaSy…`). \
                 Create one at console.cloud.google.com/apis/credentials → \
                 Create credentials → API key."
                    .to_string(),
            ));
        }
        // Google API keys all start with AIzaSy at present. Warn (not
        // reject — Google could issue keys with different prefixes in
        // future; we don't want to lock out legitimate keys).
        if !trimmed.starts_with("AIza") {
            log::warn!(
                "RIS google_vision key does not start with AIza — Google APIs \
                 typically use AIzaSy-prefixed keys; this may not work."
            );
        }
    }
    Ok(())
}

// ── Key store wrappers ───────────────────────────────────────────────

/// Map a `keyring::Error` into [`AppError::Keyring`] — except for the
/// "not found" case, which the caller handles separately.
fn map_keyring(e: keyring::Error) -> AppError {
    AppError::Keyring(e.to_string())
}

/// Store the API key for a provider in the OS-native key store.
///
/// Logs ONLY the provider name. The key value is never written to logs,
/// stdout, stderr, the audit log, or the application database.
///
/// Overwrites any existing key for the provider — this is intentional and
/// supports key rotation without a separate "update" command.
pub fn store_ris_key(provider: &str, key: &str) -> Result<(), AppError> {
    validate_provider(provider)?;
    validate_key(provider, key)?;

    let entry = keyring::Entry::new(SERVICE_NAME, provider).map_err(map_keyring)?;
    entry.set_password(key).map_err(map_keyring)?;

    log::info!("RIS key set for provider: {provider}");
    Ok(())
}

/// Retrieve the API key for a provider from the OS-native key store.
///
/// Returns `Ok(None)` if no key has been stored for the provider — distinct
/// from a true error (keychain locked, secret-service unavailable).
///
/// NOT exposed via Tauri IPC. Stages 1+2 call this from inside the Rust
/// HTTP client just before issuing the provider request.
pub fn load_ris_key(provider: &str) -> Result<Option<String>, AppError> {
    validate_provider(provider)?;
    let entry = keyring::Entry::new(SERVICE_NAME, provider).map_err(map_keyring)?;
    match entry.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(map_keyring(e)),
    }
}

/// Remove the API key for a provider from the OS-native key store.
///
/// Idempotent: returns Ok even if no key was stored (NoEntry is treated as
/// already-deleted). Other errors propagate.
pub fn delete_ris_key(provider: &str) -> Result<(), AppError> {
    validate_provider(provider)?;
    let entry = keyring::Entry::new(SERVICE_NAME, provider).map_err(map_keyring)?;
    match entry.delete_credential() {
        Ok(()) => {
            log::info!("RIS key deleted for provider: {provider}");
            Ok(())
        }
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(map_keyring(e)),
    }
}

/// List the providers that currently have a key stored in the OS-native
/// key store.
///
/// Iterates the FULL whitelist — not just providers known to be configured —
/// so the response time does not leak which providers are set
/// (timing side-channel mitigation per security-auditor STRIDE 2026-05-03).
///
/// Returned vector contains provider names ONLY. Key values are NEVER
/// returned and there is no IPC command that retrieves them.
pub fn list_configured_providers() -> Result<Vec<String>, AppError> {
    let mut configured = Vec::new();
    for provider in VALID_PROVIDERS {
        match load_ris_key(provider)? {
            Some(_) => configured.push((*provider).to_string()),
            None => continue,
        }
    }
    Ok(configured)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Provider whitelist ──────────────────────────────────────────

    #[test]
    fn validate_provider_accepts_whitelist() {
        assert!(validate_provider("tineye").is_ok());
        assert!(validate_provider("google_vision").is_ok());
    }

    #[test]
    fn validate_provider_rejects_unknown() {
        for bad in &[
            "",
            "TinEye",
            "tineye ",
            " tineye",
            "googlevision",
            "tineye/../evil",
            "../tineye",
            "tineye\0",
            "facebook",
        ] {
            let err = validate_provider(bad).unwrap_err();
            assert!(matches!(err, AppError::Validation(_)));
            // Non-revealing — does not echo the name back. Skip the
            // empty-string case because every string contains "".
            if !bad.is_empty() {
                assert!(
                    !err.to_string().contains(bad),
                    "error string should not echo provider name: {bad:?}",
                );
            }
        }
    }

    // ── Key validation ──────────────────────────────────────────────

    #[test]
    fn validate_key_rejects_empty() {
        assert!(validate_key("tineye", "").is_err());
        assert!(validate_key("tineye", "   ").is_err());
        assert!(validate_key("tineye", "\t\n").is_err());
    }

    #[test]
    fn validate_key_rejects_short() {
        assert!(validate_key("tineye", "abc").is_err());
        assert!(validate_key("tineye", &"x".repeat(MIN_KEY_LEN - 1)).is_err());
    }

    #[test]
    fn validate_key_accepts_minimum_length() {
        assert!(validate_key("tineye", &"x".repeat(MIN_KEY_LEN)).is_ok());
    }

    #[test]
    fn validate_key_rejects_too_long() {
        assert!(validate_key("tineye", &"x".repeat(MAX_KEY_LEN + 1)).is_err());
    }

    #[test]
    fn validate_key_rejects_non_ascii() {
        assert!(validate_key("tineye", "nötalongenough_keysuffix_padding").is_err());
    }

    #[test]
    fn validate_key_rejects_control_chars() {
        let key = format!("ABCDEF1234567890ABCDEF1234567890{}", '\x07');
        assert!(validate_key("tineye", &key).is_err());
    }

    #[test]
    fn validate_key_accepts_typical_format() {
        // Both provider key formats fit comfortably
        let tineye_like = "a".repeat(40); // 40-char hex
        assert!(validate_key("tineye", &tineye_like).is_ok());
        let google_like = format!("AIza{}", "B".repeat(35)); // 39-char AIza-prefixed
        assert!(validate_key("google_vision", &google_like).is_ok());
    }

    // ── Stage 1.5 — provider-specific format checks ─────────────────

    #[test]
    fn validate_key_rejects_oauth_client_id_for_google_vision() {
        // Common credential-type confusion — Google Cloud Console offers
        // both API keys and OAuth Client IDs side by side.
        let oauth_client_id =
            "958769449147-71a80094kgsv3gfpppbt0hejinj0f6mj.apps.googleusercontent.com";
        let err = validate_key("google_vision", oauth_client_id).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        let msg = err.to_string();
        assert!(msg.contains("OAuth Client ID"));
        assert!(msg.contains("AIzaSy"));
    }

    #[test]
    fn validate_key_oauth_client_id_only_rejected_for_google() {
        // The same string passed under TinEye should NOT trigger the
        // Google-specific rejection — TinEye doesn't have OAuth flows.
        // (It will likely fail at TinEye's end with a 401, but that's
        // semantic validation, not our format check.)
        let oauth_client_id =
            "958769449147-71a80094kgsv3gfpppbt0hejinj0f6mj.apps.googleusercontent.com";
        assert!(validate_key("tineye", oauth_client_id).is_ok());
    }

    // ── Constants ───────────────────────────────────────────────────

    #[test]
    fn whitelist_does_not_grow_silently() {
        // Tripwire — adding a provider must be a deliberate code change
        // accompanied by a CHECK constraint update in db.rs migration v8.
        assert_eq!(VALID_PROVIDERS.len(), 2);
        assert_eq!(VALID_PROVIDERS[0], "tineye");
        assert_eq!(VALID_PROVIDERS[1], "google_vision");
    }

    #[test]
    fn cap_constants_consistent_with_design_memo() {
        // project_jtv98_ris_design.md locked these. Changing them needs a
        // PM review, not just a code edit.
        assert_eq!(DEFAULT_DAILY_CAP, 50);
        assert_eq!(MAX_DAILY_CAP, 500);
    }

    // ── Keychain round-trip (CI-skip) ──────────────────────────────
    //
    // Real keychain tests live behind #[ignore] because GitHub Actions
    // runners don't have a usable secret service. Run locally with:
    //   cargo test ris::tests::keychain -- --ignored

    #[test]
    #[ignore = "requires-os-keychain"]
    fn keychain_round_trip() {
        let test_key = "x".repeat(40);
        let provider = "tineye";
        // Clean any leftover state from prior runs
        let _ = delete_ris_key(provider);

        store_ris_key(provider, &test_key).expect("store should succeed");
        let loaded = load_ris_key(provider)
            .expect("load should succeed")
            .expect("key should be present");
        assert_eq!(loaded, test_key);

        delete_ris_key(provider).expect("delete should succeed");
        let after = load_ris_key(provider).expect("load after delete should succeed");
        assert!(after.is_none());
    }

    #[test]
    #[ignore = "requires-os-keychain"]
    fn list_configured_providers_after_round_trip() {
        let _ = delete_ris_key("tineye");
        let _ = delete_ris_key("google_vision");

        store_ris_key("tineye", &"x".repeat(40)).unwrap();
        let listed = list_configured_providers().unwrap();
        assert_eq!(listed, vec!["tineye"]);

        delete_ris_key("tineye").unwrap();
        let after = list_configured_providers().unwrap();
        assert!(after.is_empty());
    }
}

// ════════════════════════════════════════════════════════════════════
// JTV-98 STAGE 1 — REVERSE IMAGE SEARCH HTTP PATH
// ════════════════════════════════════════════════════════════════════
//
// Stage 1 ships TinEye only. Single mode: full-bytes multipart upload.
// Hash-only mode is NOT supported by TinEye's public API — confirmed by
// the api-engineer research 2026-05-03 (no `phash=` / `xxhash=` /
// `image_hash=` parameter exists; ImagePrint is server-side only). The
// Stage 0 microcopy was retracted in the same commit chain.
//
// Provider response URLs are returned to the frontend as plain strings
// for text-only `<a>` rendering. NEVER prefetched, NEVER `<img src=>`,
// CSP `img-src` is NOT widened.
//
// Pre-pilot non-negotiables enforced here:
//   - API key in `x-api-key` HEADER only, never query string
//   - Per-call cooldown via `tokio::sync::Mutex<HashMap<...>>` in
//     AppState (NOT DB — courtesy rate-limit, not security boundary)
//   - SHA-256 of image bytes computed BEFORE HTTP call so the audit
//     entry fires the moment we cross the network boundary
//   - audit log target_id = SHA-256 (not file path — defends against
//     path-disclosure if the audit log leaks)
//   - quota pre-check via db.record_ris_call BEFORE issuing the HTTP
//     so we never burn quota on cap-rejected calls
//   - cap-reached / consent-rejected / cooldown-rejected path produces
//     NO audit entry (the network call did not happen)
//   - reqwest custom redirect policy re-applies the SSRF guard on
//     each hop (TinEye doesn't redirect today, but the policy is
//     defence-in-depth)

use serde::{Deserialize, Serialize};

/// Per-call cooldown enforced between successive RIS calls. The user-
/// visible message names this constant; align both if changed.
pub const COOLDOWN_SECONDS: u64 = 2;

/// HTTP request timeout. TinEye's p95 is < 8 s on full-bytes upload.
/// Google Vision is similar order of magnitude — same value reused.
const PROVIDER_TIMEOUT_SECONDS: u64 = 30;

/// TinEye Commercial API base URL. Confirmed 2026-05-03 by api-engineer
/// research: stable URL since the 2022 HMAC → x-api-key migration.
const TINEYE_SEARCH_URL: &str = "https://api.tineye.com/rest/search/";

/// Google Cloud Vision API `images:annotate` endpoint. Confirmed by
/// api-engineer 2026-05-03. Auth uses the `X-goog-api-key` HEADER —
/// NEVER the `?key=…` query string (Google's own docs explicitly
/// recommend the header to keep the key out of access logs).
const GOOGLE_VISION_URL: &str = "https://vision.googleapis.com/v1/images:annotate";

/// Maximum image size accepted by TinEye uploads. TinEye does not
/// document a public ceiling; this defends against accidentally trying
/// to send a 100 MB scientific TIFF. Read by the IPC command's file
/// pre-flight via `RisProvider::max_image_bytes`.
pub const TINEYE_MAX_IMAGE_BYTES: u64 = 25 * 1024 * 1024; // 25 MB

/// Maximum image size accepted by Google Vision uploads. Google's
/// documented JSON request body limit is 10 MB; base64 overhead of
/// ~1.33× means the effective image cap is ~7.5 MB. Slightly under
/// for safety. Read by the IPC command's file pre-flight via
/// `RisProvider::max_image_bytes`.
pub const GOOGLE_VISION_MAX_IMAGE_BYTES: u64 = 7 * 1024 * 1024; // 7 MB

/// One reverse-image-search match returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RisMatch {
    /// Domain hosting the matched image (e.g. `news.bbc.co.uk`).
    pub domain: String,
    /// Backlink URL — the page where the matched image appears. Display
    /// as text-only `<a>`; do NOT render as `<img>`.
    pub backlink_url: String,
    /// Image URL on the matched page. Display as text only — NEVER prefetch.
    pub image_url: String,
    /// TinEye match score (0-100, higher = stronger match).
    pub score: f32,
    /// Crawl date of the backlink (ISO 8601), if reported.
    pub crawl_date: Option<String>,
}

/// Result returned by the `reverse_image_search` Tauri command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RisSearchResult {
    /// Provider that returned these matches (e.g. `"tineye"`).
    pub provider: String,
    /// All matches, in TinEye's response order (highest score first).
    pub matches: Vec<RisMatch>,
    /// Total match count reported by TinEye (may exceed `matches.len()`
    /// if the response was paginated; Stage 1 fetches the first page only).
    pub total: u32,
    /// TinEye's reported query latency in milliseconds.
    pub query_time_ms: Option<u32>,
    /// Today's quota usage AFTER this call (e.g. `13` of `50`).
    pub calls_today: u32,
    /// The configured daily cap for the provider.
    pub daily_cap: u32,
}

// ── Provider abstraction (for testability) ───────────────────────────

/// Trait abstraction over RIS providers. Production uses
/// [`TineyeProvider`] and [`GoogleVisionProvider`]; tests inject a
/// mock implementation. The `async_trait` crate would be cleaner but
/// adds a dep — stick to the bare async-fn-in-trait pattern (stable
/// since Rust 1.75). NB: async-fn-in-trait is NOT `dyn`-compatible,
/// so the IPC command dispatches by string match, not `dyn RisProvider`.
#[allow(async_fn_in_trait)]
pub trait RisProvider {
    /// Issue the search call and return matches. The caller has already
    /// validated consent + cooldown + quota + image size.
    async fn search(&self, image_bytes: &[u8], filename: &str)
        -> Result<RisSearchResult, AppError>;
}

/// Per-provider maximum image size (in bytes). TinEye accepts up to
/// 25 MB via multipart; Google Vision's 10 MB JSON body limit caps
/// the effective image at ~7 MB after base64 overhead. The IPC
/// command's file pre-flight uses this. Returns 0 for unknown
/// providers — the caller has already validated against the
/// whitelist so this is unreachable in practice.
pub fn max_image_bytes_for(provider: &str) -> u64 {
    match provider {
        "tineye" => TINEYE_MAX_IMAGE_BYTES,
        "google_vision" => GOOGLE_VISION_MAX_IMAGE_BYTES,
        _ => 0,
    }
}

/// Clamp a user-supplied daily cap into the [`DEFAULT_DAILY_CAP`,
/// `MAX_DAILY_CAP`] range. Used by the IPC command before passing
/// to `db.record_ris_call` so a malicious renderer can't request
/// a cap beyond the configured ceiling.
pub fn clamp_daily_cap(cap: u32) -> u32 {
    cap.clamp(DEFAULT_DAILY_CAP, MAX_DAILY_CAP)
}

/// Deduplicate matches by backlink URL — TinEye occasionally returns
/// the same backlink twice (different image URLs on the same page),
/// and a single Google Vision page may surface as both "full match"
/// and "partial match" if the page hosts multiple sizes. Keep the
/// first occurrence (which has the higher score per response order).
pub fn dedupe_matches(matches: Vec<RisMatch>) -> Vec<RisMatch> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(matches.len());
    for m in matches {
        if seen.insert(m.backlink_url.clone()) {
            out.push(m);
        }
    }
    out
}

/// Build a reqwest client with the SSRF-safe redirect policy used by
/// every RIS provider. Mirrors `verify_url_inner` in lib.rs but uses
/// the async `reqwest::Client` because Tauri RIS commands are async.
///
/// No `.cookie_store(false)` — that method requires the `cookies`
/// reqwest feature which we don't enable. reqwest's default is
/// already "no cookie jar" which is what we want for one-shot uploads.
fn build_ris_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(PROVIDER_TIMEOUT_SECONDS))
        .connect_timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 3 {
                return attempt.error("too many redirects");
            }
            if let Some(host) = attempt.url().host_str() {
                if crate::is_private_or_loopback_host(host) {
                    return attempt.error("redirect to private address blocked");
                }
            }
            attempt.follow()
        }))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {e}")))
}

/// Production TinEye implementation. One per call — reqwest's `Client`
/// is internally `Arc`-shared so per-call construction is cheap.
pub struct TineyeProvider {
    api_key: String,
}

impl TineyeProvider {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl RisProvider for TineyeProvider {
    async fn search(
        &self,
        image_bytes: &[u8],
        filename: &str,
    ) -> Result<RisSearchResult, AppError> {
        let client = build_ris_client()?;

        // multipart upload — `image` is the documented field name.
        // We own the bytes; clone for multipart's lifetime.
        let part =
            reqwest::multipart::Part::bytes(image_bytes.to_vec()).file_name(filename.to_string());
        // TinEye expects the multipart field name `image_upload` (NOT
        // `image`). Confirmed against the live sandbox API on 2026-05-04
        // — sending as `image` returns a 400 "Please supply an image,
        // URL or fingerprint for searching." This field name matches
        // the official TinEye client libraries (pytineye / node / php)
        // and the searx engine source.
        let form = reqwest::multipart::Form::new().part("image_upload", part);

        // x-api-key header — NEVER query string (avoids leaking the key
        // into TinEye's access log + any intermediate proxy logs).
        let resp = client
            .post(TINEYE_SEARCH_URL)
            .header("x-api-key", &self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| AppError::Sidecar(format!("TinEye request failed: {e}")))?;

        let status = resp.status();

        // Map known TinEye error codes to user-actionable messages.
        // 402 Payment Required = bundle exhausted (TinEye's pricing model).
        // 401 = bad/missing key. 429 = burst limit at the server.
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            log::warn!("TinEye returned status {status}: {body}");
            let user_msg = match status.as_u16() {
                401 | 403 => {
                    "Your TinEye API key may be invalid or expired. \
                              Check it in Settings → Reverse Image Search."
                }
                402 => {
                    "Your TinEye search bundle is exhausted. Top up at \
                        services.tineye.com to continue searching."
                }
                429 => {
                    "TinEye is rate-limiting requests right now. \
                        Please try again in a few seconds."
                }
                _ => {
                    "The reverse image search could not be completed. \
                      Try again shortly."
                }
            };
            return Err(AppError::Validation(user_msg.to_string()));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Sidecar(format!("Failed to parse TinEye response: {e}")))?;

        // TinEye response shape (confirmed by api-engineer research):
        //   {
        //     "code": 200,
        //     "stats": { "total_results": N, "query_time": "..." },
        //     "results": { "matches": [ { ... } ] }
        //   }
        let total = body
            .pointer("/stats/total_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let query_time_ms = body
            .pointer("/stats/query_time")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f32>().ok())
            .map(|seconds| (seconds * 1000.0) as u32);

        let matches: Vec<RisMatch> = body
            .pointer("/results/matches")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| {
                        let domain = m.get("domain")?.as_str()?.to_string();
                        let image_url = m.get("image_url")?.as_str()?.to_string();
                        let score = m.get("score")?.as_f64()? as f32;
                        let backlinks = m.get("backlinks")?.as_array()?;
                        let first = backlinks.first()?;
                        let backlink_url = first.get("url")?.as_str()?.to_string();
                        let crawl_date = first
                            .get("crawl_date")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        Some(RisMatch {
                            domain,
                            backlink_url,
                            image_url,
                            score,
                            crawl_date,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(RisSearchResult {
            provider: "tineye".to_string(),
            matches,
            total,
            query_time_ms,
            // Caller (`reverse_image_search_inner`) overwrites these after
            // calling `db.record_ris_call`. We can't fill them here because
            // the provider trait has no DB access — keep the layering clean.
            calls_today: 0,
            daily_cap: 0,
        })
    }
}

// ── Google Vision Web Detection provider ────────────────────────────
//
// Confirmed by api-engineer 2026-05-03:
//   - Endpoint: POST https://vision.googleapis.com/v1/images:annotate
//   - Auth: `X-goog-api-key` HEADER (NOT `?key=…` query string —
//     Google's own docs explicitly recommend the header to keep keys
//     out of access logs and TLS-MITM-proxy logs).
//   - Body: JSON with image.content as base64; features:
//     [{type: "WEB_DETECTION", maxResults: N}].
//   - Response: responses[].webDetection.pagesWithMatchingImages[]
//     is the primary field for our RisMatch shape.
//   - Errors: HTTP 400/401/403/429/5xx with body
//     {"error":{"code":N,"message":"…","status":"…"}} where status
//     distinguishes RESOURCE_EXHAUSTED (quota) from PERMISSION_DENIED
//     (key invalid / Vision API not enabled / billing off).
//   - Body cap: 10 MB JSON; effective image cap ~7.5 MB after base64.
//
// SECURITY: never use `image.source.imageUri` — that has Google fetch
// the URL we provide, bypassing our SSRF protections. CI grep guard
// added in S2.3 to prevent any future PR re-introducing this path.

/// Production Google Vision implementation.
pub struct GoogleVisionProvider {
    api_key: String,
}

impl GoogleVisionProvider {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl RisProvider for GoogleVisionProvider {
    async fn search(
        &self,
        image_bytes: &[u8],
        _filename: &str,
    ) -> Result<RisSearchResult, AppError> {
        use base64::{engine::general_purpose::STANDARD as B64, Engine};
        let client = build_ris_client()?;
        let body = serde_json::json!({
            "requests": [{
                "image": { "content": B64.encode(image_bytes) },
                "features": [{ "type": "WEB_DETECTION", "maxResults": 50 }]
            }]
        });

        let resp = client
            .post(GOOGLE_VISION_URL)
            // X-goog-api-key header — NEVER `?key=…` query string.
            .header("X-goog-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Sidecar(format!("Google Vision request failed: {e}")))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        // Distinguish quota-exhausted from permission-denied — both
        // surface as HTTP 403 in Google APIs but differ in the body's
        // `status` field per AIP-193. INVALID_ARGUMENT (400) is the
        // most common gotcha — usually a credential-type mismatch
        // (OAuth Client ID pasted where an API key is expected).
        if !status.is_success() {
            log::warn!("Google Vision returned status {status}: {text}");
            let parsed: Option<serde_json::Value> = serde_json::from_str(&text).ok();
            let goog_status = parsed
                .as_ref()
                .and_then(|v| v.pointer("/error/status"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let goog_message = parsed
                .as_ref()
                .and_then(|v| v.pointer("/error/message"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let user_msg: String = match status.as_u16() {
                400 if goog_message.to_lowercase().contains("api key not valid") => {
                    "Google rejected your API key as invalid. \
                     Common cause: the credential you pasted is an OAuth Client ID \
                     (ends with `.apps.googleusercontent.com`) instead of an API key \
                     (starts with `AIzaSy…`). Create an API key at \
                     console.cloud.google.com/apis/credentials and re-save in Settings."
                        .to_string()
                }
                400 => format!(
                    "Google Vision rejected the request as invalid \
                     ({status_str}). Detail: {detail}",
                    status_str = goog_status,
                    detail = if goog_message.is_empty() {
                        "(no message)"
                    } else {
                        goog_message
                    },
                ),
                401 => "Your Google Vision API key may be invalid or revoked. \
                       Check it in Settings → Reverse Image Search. Note an API key \
                       must start with `AIzaSy…` — if yours ends in \
                       `.apps.googleusercontent.com` it is an OAuth Client ID and will not work."
                    .to_string(),
                403 if goog_status == "RESOURCE_EXHAUSTED" => {
                    "Your Google Vision quota is exhausted for this billing period.".to_string()
                }
                403 => "Google Vision rejected the request — your API key may not be authorised \
                     for the Vision API, billing may not be enabled on your Google Cloud \
                     project, or the key may be IP/referrer-restricted in a way that excludes \
                     this device."
                    .to_string(),
                429 => {
                    "Google Vision is rate-limiting requests right now. Please try again shortly."
                        .to_string()
                }
                _ => format!(
                    "Google Vision returned HTTP {} — could not complete the search. {}",
                    status.as_u16(),
                    if goog_message.is_empty() {
                        "Try again shortly.".to_string()
                    } else {
                        format!("Detail: {goog_message}")
                    },
                ),
            };
            return Err(AppError::Validation(user_msg));
        }

        let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            AppError::Sidecar(format!("Failed to parse Google Vision response: {e}"))
        })?;

        // pagesWithMatchingImages[] is the primary RisMatch source.
        // For each page entry: backlinkUrl = page.url; imageUrl = first
        // fullMatchingImages[].url, falling back to first
        // partialMatchingImages[].url; score = 100 if full match present,
        // else 70 if partial, else 50.
        let pages = body
            .pointer("/responses/0/webDetection/pagesWithMatchingImages")
            .and_then(|v| v.as_array());
        let matches: Vec<RisMatch> = match pages {
            Some(arr) => arr
                .iter()
                .filter_map(|page| {
                    let backlink_url = page.get("url")?.as_str()?.to_string();
                    let domain = url::Url::parse(&backlink_url)
                        .ok()
                        .and_then(|u| u.host_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| backlink_url.clone());
                    let full = page
                        .get("fullMatchingImages")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|m| m.get("url"))
                        .and_then(|v| v.as_str());
                    let partial = page
                        .get("partialMatchingImages")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|m| m.get("url"))
                        .and_then(|v| v.as_str());
                    let (image_url, score) = if let Some(u) = full {
                        (u.to_string(), 100.0_f32)
                    } else if let Some(u) = partial {
                        (u.to_string(), 70.0_f32)
                    } else {
                        // Page is referenced via visuallySimilar match —
                        // no image URL on the page entry, fall back to
                        // the page URL itself.
                        (backlink_url.clone(), 50.0_f32)
                    };
                    Some(RisMatch {
                        domain,
                        backlink_url,
                        image_url,
                        score,
                        // Google doesn't report a per-result crawl date.
                        crawl_date: None,
                    })
                })
                .collect(),
            None => Vec::new(),
        };

        Ok(RisSearchResult {
            provider: "google_vision".to_string(),
            // Google reports total via the size of the array, not a
            // separate count field.
            total: matches.len() as u32,
            matches,
            // Google doesn't report a per-call query-time field in
            // the standard response.
            query_time_ms: None,
            // Caller fills these from db.record_ris_call.
            calls_today: 0,
            daily_cap: 0,
        })
    }
}

// ── Pre-flight checks (consent, cooldown, file size) ────────────────

/// Enforce the locked design's consent gate. Full-bytes mode is the
/// only mode supported, and `consent_given` MUST be true.
pub fn check_consent_gate(consent_given: bool) -> Result<(), AppError> {
    if !consent_given {
        return Err(AppError::Validation(
            "Sending image data to a third party requires your explicit consent. \
             Please confirm in the dialogue before proceeding."
                .to_string(),
        ));
    }
    Ok(())
}

/// Returns Ok(()) if at least `COOLDOWN_SECONDS` has elapsed since the
/// last successful call for this provider. Otherwise returns an error
/// the frontend can render with a remaining-seconds countdown.
///
/// Updates the cooldown timestamp to "now" on success — caller must
/// roll back via `clear_cooldown` if the subsequent quota / HTTP call
/// fails (so the user isn't double-penalised for a failed network call).
pub async fn check_and_set_cooldown(
    cooldowns: &tokio::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>,
    provider: &str,
) -> Result<(), AppError> {
    let mut map = cooldowns.lock().await;
    let now = std::time::Instant::now();
    if let Some(last) = map.get(provider) {
        let elapsed = now.duration_since(*last);
        if elapsed.as_secs() < COOLDOWN_SECONDS {
            let remaining = COOLDOWN_SECONDS - elapsed.as_secs();
            return Err(AppError::Validation(format!(
                "Please wait {remaining} seconds before searching again."
            )));
        }
    }
    map.insert(provider.to_string(), now);
    Ok(())
}

/// Roll back the cooldown timestamp set by `check_and_set_cooldown`.
/// Used when the call after the cooldown gate fails (HTTP error, quota
/// exceeded post-check) so the user isn't penalised for a non-event.
pub async fn clear_cooldown(
    cooldowns: &tokio::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>,
    provider: &str,
) {
    let mut map = cooldowns.lock().await;
    map.remove(provider);
}

#[cfg(test)]
mod search_tests {
    use super::*;

    #[test]
    fn consent_gate_rejects_false() {
        let err = check_consent_gate(false).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains("consent"));
    }

    #[test]
    fn consent_gate_accepts_true() {
        assert!(check_consent_gate(true).is_ok());
    }

    #[tokio::test]
    async fn cooldown_blocks_second_call_within_window() {
        let map = tokio::sync::Mutex::new(std::collections::HashMap::new());
        check_and_set_cooldown(&map, "tineye").await.unwrap();
        // Immediate second call — must fail.
        let err = check_and_set_cooldown(&map, "tineye").await.unwrap_err();
        assert!(err.to_string().contains("seconds before searching again"));
    }

    #[tokio::test]
    async fn cooldown_independent_per_provider() {
        let map = tokio::sync::Mutex::new(std::collections::HashMap::new());
        check_and_set_cooldown(&map, "tineye").await.unwrap();
        // Different provider — must succeed.
        check_and_set_cooldown(&map, "google_vision")
            .await
            .expect("different provider should not be cooled down");
    }

    #[tokio::test]
    async fn clear_cooldown_unblocks_immediate_retry() {
        let map = tokio::sync::Mutex::new(std::collections::HashMap::new());
        check_and_set_cooldown(&map, "tineye").await.unwrap();
        clear_cooldown(&map, "tineye").await;
        // Immediate retry now succeeds.
        check_and_set_cooldown(&map, "tineye").await.unwrap();
    }

    // Tripwire — if someone bumps the cap above 100 MB it's likely a bug.
    // Evaluated at compile time so the constant can't drift unnoticed.
    const _: () = {
        assert!(TINEYE_MAX_IMAGE_BYTES > 1_000_000);
        assert!(TINEYE_MAX_IMAGE_BYTES < 100 * 1024 * 1024);
    };

    #[test]
    fn ris_match_serialises_to_camel_case() {
        let m = RisMatch {
            domain: "example.com".into(),
            backlink_url: "https://example.com/page".into(),
            image_url: "https://example.com/img.jpg".into(),
            score: 87.5,
            crawl_date: Some("2026-01-01".into()),
        };
        let json = serde_json::to_value(&m).unwrap();
        assert!(json.get("backlinkUrl").is_some());
        assert!(json.get("imageUrl").is_some());
        assert!(json.get("crawlDate").is_some());
    }

    // ── Stage 2 tests — GoogleVisionProvider + per-provider cap ─────

    #[test]
    fn max_image_bytes_for_known_providers() {
        assert_eq!(max_image_bytes_for("tineye"), TINEYE_MAX_IMAGE_BYTES);
        assert_eq!(
            max_image_bytes_for("google_vision"),
            GOOGLE_VISION_MAX_IMAGE_BYTES
        );
    }

    #[test]
    fn max_image_bytes_for_unknown_returns_zero() {
        // Defence-in-depth — caller MUST validate against the whitelist
        // first; if they don't, the cap of 0 will trip on any non-empty
        // image and reject the call rather than allow an unbounded upload.
        assert_eq!(max_image_bytes_for("midjourney"), 0);
        assert_eq!(max_image_bytes_for(""), 0);
    }

    // Google's 10 MB JSON-body cap means base64 image bytes must be smaller
    // than TinEye's binary multipart cap. Compile-time check, same reasoning.
    const _: () = assert!(GOOGLE_VISION_MAX_IMAGE_BYTES < TINEYE_MAX_IMAGE_BYTES);

    #[test]
    fn google_vision_provider_constructs() {
        let p = GoogleVisionProvider::new("AIza-fake-key-for-construct-only".to_string());
        // Just exercise the constructor + that the type is named correctly.
        // Real HTTP path is exercised by the (deferred) integration test.
        let _ = p; // suppress unused-variable warning
    }

    // ── Stage 3 tests — dedup + cap clamping ─────────────────────────

    #[test]
    fn dedupe_matches_keeps_first_of_duplicates() {
        let m = |url: &str, score: f32| RisMatch {
            domain: "example.com".into(),
            backlink_url: url.into(),
            image_url: "https://x".into(),
            score,
            crawl_date: None,
        };
        let input = vec![
            m("https://a.example/page", 95.0),
            m("https://b.example/page", 80.0),
            m("https://a.example/page", 70.0), // duplicate of [0]
            m("https://c.example/page", 60.0),
            m("https://b.example/page", 50.0), // duplicate of [1]
        ];
        let out = dedupe_matches(input);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].backlink_url, "https://a.example/page");
        assert_eq!(out[0].score, 95.0); // first occurrence kept
        assert_eq!(out[1].backlink_url, "https://b.example/page");
        assert_eq!(out[1].score, 80.0); // first occurrence kept
        assert_eq!(out[2].backlink_url, "https://c.example/page");
    }

    #[test]
    fn dedupe_matches_empty_returns_empty() {
        assert!(dedupe_matches(Vec::new()).is_empty());
    }

    #[test]
    fn dedupe_matches_no_duplicates_returns_input_intact() {
        let m = |url: &str| RisMatch {
            domain: "example.com".into(),
            backlink_url: url.into(),
            image_url: "https://x".into(),
            score: 80.0,
            crawl_date: None,
        };
        let input = vec![
            m("https://a/page"),
            m("https://b/page"),
            m("https://c/page"),
        ];
        let out = dedupe_matches(input);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn clamp_daily_cap_within_range_passes_through() {
        assert_eq!(clamp_daily_cap(100), 100);
        assert_eq!(clamp_daily_cap(DEFAULT_DAILY_CAP), DEFAULT_DAILY_CAP);
        assert_eq!(clamp_daily_cap(MAX_DAILY_CAP), MAX_DAILY_CAP);
    }

    #[test]
    fn clamp_daily_cap_below_default_clamps_up() {
        // A renderer trying to lower the cap below the default is
        // probably trying to lock another user out — clamp UP to default
        // so the floor is enforced even on hostile input.
        assert_eq!(clamp_daily_cap(0), DEFAULT_DAILY_CAP);
        assert_eq!(clamp_daily_cap(1), DEFAULT_DAILY_CAP);
        assert_eq!(clamp_daily_cap(DEFAULT_DAILY_CAP - 1), DEFAULT_DAILY_CAP);
    }

    #[test]
    fn clamp_daily_cap_above_max_clamps_down() {
        assert_eq!(clamp_daily_cap(MAX_DAILY_CAP + 1), MAX_DAILY_CAP);
        assert_eq!(clamp_daily_cap(u32::MAX), MAX_DAILY_CAP);
    }

    #[test]
    fn google_vision_constants_match_documented_limits() {
        // Tripwire — if anyone bumps these without re-checking the
        // api-engineer research (2026-05-03), the test fails so the
        // change is visible at code-review time.
        // Google docs: 10 MB JSON body; we cap effective image at 7 MB.
        assert_eq!(GOOGLE_VISION_MAX_IMAGE_BYTES, 7 * 1024 * 1024);
        assert_eq!(
            GOOGLE_VISION_URL,
            "https://vision.googleapis.com/v1/images:annotate"
        );
    }

    /// MockProvider for testing — kept minimal because the dispatch
    /// glue lives in `lib.rs` and would need a larger refactor (inject
    /// a `dyn RisProvider` into the Tauri command) to unit-test
    /// end-to-end. The component pieces (consent gate, cooldown,
    /// `record_ris_call`) are all individually tested above and in
    /// `db::tests`.
    pub(crate) struct MockProvider {
        pub(crate) result: Result<RisSearchResult, AppError>,
    }

    impl RisProvider for MockProvider {
        async fn search(
            &self,
            _image_bytes: &[u8],
            _filename: &str,
        ) -> Result<RisSearchResult, AppError> {
            self.result.clone()
        }
    }

    #[tokio::test]
    async fn mock_provider_returns_configured_result() {
        let mock = MockProvider {
            result: Ok(RisSearchResult {
                provider: "tineye".to_string(),
                matches: vec![],
                total: 0,
                query_time_ms: None,
                calls_today: 1,
                daily_cap: 50,
            }),
        };
        let r = mock.search(&[], "test.jpg").await.unwrap();
        assert_eq!(r.provider, "tineye");
        assert_eq!(r.calls_today, 1);
    }

    #[tokio::test]
    async fn mock_provider_propagates_err() {
        let mock = MockProvider {
            result: Err(AppError::Sidecar("simulated".to_string())),
        };
        let err = mock.search(&[], "test.jpg").await.unwrap_err();
        assert!(matches!(err, AppError::Sidecar(_)));
    }
}
