// SPDX-License-Identifier: AGPL-3.0-or-later

//! Integration tests for the Jura Trace local REST API (port 8300).
//!
//! Each test spawns the Axum server on a randomly-assigned port, exercises it
//! via `reqwest`, then the tokio runtime is dropped naturally at the end of
//! the test function.
//!
//! The tests are compiled only when the `api` feature is enabled (it is in
//! `default`, so `cargo test` covers them automatically).

#![cfg(feature = "api")]

use jura_trace_lib::{api, db::Database, sidecar::SidecarClient, AppState, LicenceTier};
use reqwest::StatusCode;
use serde_json::Value;
use std::io::Write;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ── Test harness ─────────────────────────────────────────────────────────────

/// Build a throwaway `AppState` backed by a temp SQLite database.
///
/// Returns the state and the temp directory path. The caller should call
/// `std::mem::forget` on the `TempDir` if dropping inside an async context
/// to avoid the "Cannot drop a runtime in a context where blocking is not
/// allowed" panic.  The OS cleans up temp dirs on process exit.
fn build_test_state() -> (Arc<Mutex<AppState>>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("test.db");
    let db = Database::open(&db_path).expect("open test db");
    let sidecar = SidecarClient::new("http://127.0.0.1:8200", "");

    let state = AppState {
        db,
        sidecar,
        db_path: db_path.to_string_lossy().to_string(),
        licence_tier: LicenceTier::Community,
        sidecar_process: None,
        classifier_model_hash: None,
        univfd_probe_model_hash: None,
        ai_description_enabled: None,
        scheduler_handle: None,
        last_heatmap_session: None,
        last_sidecar_request_ts: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        power_saver_mode: false,
        respawn_in_progress: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        sidecar_port: 8200,
        sidecar_startup_status: std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0)),
        sidecar_startup_started_at: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
    };

    (Arc::new(Mutex::new(state)), dir)
}

/// Convenience wrapper for async tests: builds the `AppState` inside a
/// `spawn_blocking` call so that `SidecarClient::new` (which initialises a
/// blocking reqwest runtime) does not panic when called from an async context.
async fn build_test_state_async() -> Arc<Mutex<AppState>> {
    tokio::task::spawn_blocking(|| {
        let (state, dir) = build_test_state();
        // Prevent blocking I/O on drop inside async context.
        // The OS will reclaim the temp dir on process exit.
        std::mem::forget(dir);
        state
    })
    .await
    .expect("build_test_state spawn_blocking failed")
}

/// Bind a TCP listener on a random free port and return it.
fn bind_random_port() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("local addr").port();
    (listener, port)
}

/// Start the Axum server on the given `std::net::TcpListener`, returning the
/// base URL.  The server runs on a background tokio task for the lifetime of
/// the current test.
async fn start_test_server(state: Arc<Mutex<AppState>>, listener: TcpListener) -> String {
    // Convert std listener to tokio listener.
    listener.set_nonblocking(true).expect("set nonblocking");
    let tokio_listener = tokio::net::TcpListener::from_std(listener).expect("tokio listener");
    let port = tokio_listener.local_addr().expect("addr").port();

    api::routes::init_start_time();
    let router = api::build_router(state);

    tokio::spawn(async move {
        axum::serve(tokio_listener, router)
            .await
            .expect("server error");
    });

    // Wait until the server actually answers, rather than assuming it will
    // have started within a fixed delay.
    //
    // A TCP connect would not be signal enough: the listener is bound before
    // `axum::serve` is spawned, so the kernel accepts connections whether or
    // not the router is running yet. `/api/v1/health` needs no auth (see
    // `test_health_no_auth`), so a success from it is the first moment the
    // server is genuinely usable.
    //
    // This replaces a fixed 20 ms sleep. See `backlog/BL-TEST-002`: a
    // duration is an assumption about the runner, and CI runs on a shared
    // 2-vCPU VM where scheduling jitter makes that assumption occasionally
    // false. Every test in this file goes through here, so when that
    // assumption broke it would have surfaced as a connection error rather
    // than as a timing problem. Polling costs nothing in the normal case,
    // because it succeeds on the first attempt.
    let base_url = format!("http://127.0.0.1:{port}");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        match reqwest::get(format!("{base_url}/api/v1/health")).await {
            Ok(response) if response.status().is_success() => break,
            _ if std::time::Instant::now() >= deadline => {
                panic!("test server did not answer /api/v1/health within 10s")
            }
            _ => tokio::time::sleep(Duration::from_millis(5)).await,
        }
    }

    base_url
}

/// Insert a bootstrap key directly into the database and return its raw key.
///
/// Runs inside `spawn_blocking` to avoid calling rusqlite from an async context.
async fn insert_bootstrap_key(state: Arc<Mutex<AppState>>) -> String {
    tokio::task::spawn_blocking(move || {
        let mut guard = state.lock().unwrap();
        guard
            .db
            .ensure_bootstrap_api_key()
            .expect("bootstrap")
            .expect("should create new key")
    })
    .await
    .expect("insert_bootstrap_key spawn_blocking failed")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Test 1: Health endpoint returns 200 with version info (no auth required).
#[tokio::test]
async fn test_health_no_auth() {
    let state = build_test_state_async().await;
    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let resp = reqwest::get(format!("{base_url}/api/v1/health"))
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string(), "version should be a string");
    // HealthResponse serialises with camelCase.
    assert!(
        body["uptimeSeconds"].is_number(),
        "uptimeSeconds should be present in health response"
    );
}

/// Test 2: Verify endpoint returns 401 without auth header.
#[tokio::test]
async fn test_verify_requires_auth() {
    let state = build_test_state_async().await;
    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new().text("mode", "quick").part(
        "file",
        reqwest::multipart::Part::bytes(minimal_png())
            .file_name("test.png")
            .mime_str("image/png")
            .unwrap(),
    );

    let resp = client
        .post(format!("{base_url}/api/v1/verify"))
        .multipart(form)
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["code"], "Unauthorized");
}

/// Test 3: Verify endpoint returns 401 with an invalid / random key.
#[tokio::test]
async fn test_verify_invalid_key() {
    let state = build_test_state_async().await;
    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(minimal_png())
            .file_name("test.png")
            .mime_str("image/png")
            .unwrap(),
    );

    let resp = client
        .post(format!("{base_url}/api/v1/verify"))
        .header(
            "Authorization",
            "Bearer jt_notarealkey0000000000000000000000",
        )
        .multipart(form)
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// Test 4: Auth key creation — POST /api/v1/auth/keys creates a key, then
/// authenticate with it successfully.
#[tokio::test]
async fn test_create_key_and_authenticate() {
    let state = build_test_state_async().await;

    // Seed a bootstrap key so we can call POST /auth/keys with auth.
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let bootstrap_key = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();

    // Create a new named key.
    let resp = client
        .post(format!("{base_url}/api/v1/auth/keys"))
        .header("Authorization", format!("Bearer {bootstrap_key}"))
        .json(&serde_json::json!({ "name": "test-key", "rate_limit": 200 }))
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::OK, "should create key");

    let body: Value = resp.json().await.expect("json");
    let new_key = body["data"]["key"].as_str().expect("key").to_string();
    assert!(new_key.starts_with("jt_"), "key should start with jt_");
    assert_eq!(body["data"]["rateLimit"], 200);

    // Use the new key to call an authenticated endpoint.
    let stats_resp = client
        .get(format!("{base_url}/api/v1/stats"))
        .header("Authorization", format!("Bearer {new_key}"))
        .send()
        .await
        .expect("request");

    assert_eq!(
        stats_resp.status(),
        StatusCode::OK,
        "new key should authenticate"
    );
}

/// Test 5: Key listing — GET /api/v1/auth/keys returns the created key.
#[tokio::test]
async fn test_list_keys() {
    let state = build_test_state_async().await;
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let auth = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base_url}/api/v1/auth/keys"))
        .header("Authorization", format!("Bearer {auth}"))
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("json");
    let keys = body["data"].as_array().expect("array");
    assert!(!keys.is_empty(), "should list at least the bootstrap key");

    // Verify that the bootstrap key appears.
    let bootstrap_entry = keys.iter().find(|k| k["name"] == "bootstrap");
    assert!(
        bootstrap_entry.is_some(),
        "bootstrap key should appear in list"
    );

    // Key hash must NOT be present in the response.
    for key in keys {
        assert!(
            key.get("keyHash").is_none() || key["keyHash"].is_null(),
            "keyHash must not be returned in key listing"
        );
    }
}

/// Test 6: Key revocation — DELETE /api/v1/auth/keys/{id} revokes, then auth
/// fails with that key.
#[tokio::test]
async fn test_revoke_key() {
    let state = build_test_state_async().await;
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let bootstrap_key = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();

    // Create a secondary key to revoke.
    let create_resp = client
        .post(format!("{base_url}/api/v1/auth/keys"))
        .header("Authorization", format!("Bearer {bootstrap_key}"))
        .json(&serde_json::json!({ "name": "to-revoke" }))
        .send()
        .await
        .expect("request");
    assert_eq!(create_resp.status(), StatusCode::OK);

    let create_body: Value = create_resp.json().await.expect("json");
    let secondary_key = create_body["data"]["key"]
        .as_str()
        .expect("key")
        .to_string();
    let secondary_id = create_body["data"]["keyId"]
        .as_str()
        .expect("keyId")
        .to_string();

    // Verify secondary key works before revocation.
    let before_resp = client
        .get(format!("{base_url}/api/v1/stats"))
        .header("Authorization", format!("Bearer {secondary_key}"))
        .send()
        .await
        .expect("request");
    assert_eq!(
        before_resp.status(),
        StatusCode::OK,
        "key should work before revocation"
    );

    // Revoke the secondary key.
    let revoke_resp = client
        .delete(format!("{base_url}/api/v1/auth/keys/{secondary_id}"))
        .header("Authorization", format!("Bearer {bootstrap_key}"))
        .send()
        .await
        .expect("request");
    assert_eq!(revoke_resp.status(), StatusCode::OK);

    let revoke_body: Value = revoke_resp.json().await.expect("json");
    assert_eq!(revoke_body["data"]["revoked"], true);

    // Revoked key must now be rejected.
    let after_resp = client
        .get(format!("{base_url}/api/v1/stats"))
        .header("Authorization", format!("Bearer {secondary_key}"))
        .send()
        .await
        .expect("request");
    assert_eq!(
        after_resp.status(),
        StatusCode::UNAUTHORIZED,
        "revoked key should be rejected"
    );
}

/// Test 7: Verify file upload — POST /api/v1/verify with a small test image
/// returns a VerificationResult JSON with the top-level envelope fields.
#[tokio::test]
async fn test_verify_file_upload() {
    let state = build_test_state_async().await;
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let auth = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new().text("mode", "quick").part(
        "file",
        reqwest::multipart::Part::bytes(minimal_png())
            .file_name("test.png")
            .mime_str("image/png")
            .unwrap(),
    );

    let resp = client
        .post(format!("{base_url}/api/v1/verify"))
        .header("Authorization", format!("Bearer {auth}"))
        .multipart(form)
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::OK, "verify should return 200");

    let body: Value = resp.json().await.expect("json");
    // Top-level envelope fields.
    assert_eq!(body["apiVersion"], "1.0", "api_version should be 1.0");
    assert!(body["degraded"].is_boolean(), "degraded should be boolean");
    // Verification result fields (camelCase from serde rename_all).
    let data = &body["data"];
    // VerificationResult.overall_trust → "overallTrust"
    assert!(
        data["overallTrust"].is_number(),
        "overallTrust should be present in verification result"
    );
    assert!(data["mode"].is_string(), "mode should be present");
    assert!(
        data["contentType"].is_string(),
        "contentType should be present"
    );
}

/// Test 8: Stats endpoint — GET /api/v1/stats returns counts.
#[tokio::test]
async fn test_stats_endpoint() {
    let state = build_test_state_async().await;
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let auth = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let resp = reqwest::Client::new()
        .get(format!("{base_url}/api/v1/stats"))
        .header("Authorization", format!("Bearer {auth}"))
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("json");
    let data = &body["data"];
    for field in &[
        "totalAssets",
        "totalVerifications",
        "totalFingerprints",
        "c2paSignedCount",
    ] {
        assert!(
            data[field].is_number(),
            "{field} should be a number in stats"
        );
    }
    // Fresh database should have zero counts.
    assert_eq!(data["totalAssets"], 0);
    assert_eq!(data["totalVerifications"], 0);
}

/// Test 9: Rate limiting — send requests exceeding the per-key limit, verify
/// 429 responses with correct headers.
#[tokio::test]
async fn test_rate_limiting_429() {
    let state = build_test_state_async().await;
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let bootstrap_auth = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();

    // Create a key with a limit of 2 requests per minute.
    let create_resp = client
        .post(format!("{base_url}/api/v1/auth/keys"))
        .header("Authorization", format!("Bearer {bootstrap_auth}"))
        .json(&serde_json::json!({ "name": "limited", "rate_limit": 2 }))
        .send()
        .await
        .expect("request");
    assert_eq!(create_resp.status(), StatusCode::OK);

    let create_body: Value = create_resp.json().await.expect("json");
    let limited_key = create_body["data"]["key"]
        .as_str()
        .expect("key")
        .to_string();

    // Fire 3 requests — the third should be rate limited.
    let mut statuses = Vec::new();
    for _ in 0..3 {
        let resp = client
            .get(format!("{base_url}/api/v1/stats"))
            .header("Authorization", format!("Bearer {limited_key}"))
            .send()
            .await
            .expect("request");
        statuses.push(resp.status());
    }

    assert_eq!(statuses[0], StatusCode::OK, "first request should succeed");
    assert_eq!(statuses[1], StatusCode::OK, "second request should succeed");
    assert_eq!(
        statuses[2],
        StatusCode::TOO_MANY_REQUESTS,
        "third request should be rate-limited (429)"
    );

    // Fire another request and inspect headers and body.
    let rl_resp = client
        .get(format!("{base_url}/api/v1/stats"))
        .header("Authorization", format!("Bearer {limited_key}"))
        .send()
        .await
        .expect("request");

    assert_eq!(rl_resp.status(), StatusCode::TOO_MANY_REQUESTS);

    let headers = rl_resp.headers().clone();

    // All four rate-limit headers must be present.
    assert!(
        headers.contains_key("x-ratelimit-limit"),
        "X-RateLimit-Limit header should be present on 429"
    );
    assert!(
        headers.contains_key("x-ratelimit-remaining"),
        "X-RateLimit-Remaining header should be present on 429"
    );
    assert!(
        headers.contains_key("x-ratelimit-reset"),
        "X-RateLimit-Reset header should be present on 429"
    );
    assert!(
        headers.contains_key("retry-after"),
        "Retry-After header should be present on 429"
    );

    // Verify header values.
    assert_eq!(
        headers["x-ratelimit-limit"], "2",
        "limit should match key's rate_limit"
    );
    assert_eq!(headers["x-ratelimit-remaining"], "0", "no tokens remain");

    let reset_secs: u64 = headers["x-ratelimit-reset"]
        .to_str()
        .unwrap()
        .parse()
        .expect("reset seconds should be numeric");
    assert!(
        reset_secs > 0 && reset_secs <= 60,
        "reset should be 1-60 seconds"
    );

    // JSON body should contain the error code and rate limit info.
    let body: Value = rl_resp.json().await.expect("json");
    assert_eq!(body["code"], "RateLimitExceeded");
    let rl_info = &body["rateLimitInfo"];
    assert!(
        rl_info.is_object(),
        "rateLimitInfo should be present in 429 body"
    );
    assert_eq!(rl_info["limit"], 2);
    assert_eq!(rl_info["remaining"], 0);
}

/// Test 10: OpenAPI spec endpoint returns valid JSON at /openapi.json.
#[tokio::test]
async fn test_openapi_spec_available() {
    let state = build_test_state_async().await;
    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    // No auth required for the OpenAPI spec.
    let resp = reqwest::get(format!("{base_url}/openapi.json"))
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("json");
    assert!(
        body["openapi"].is_string(),
        "openapi version field should be present"
    );
    assert!(body["info"]["title"].is_string(), "title should be present");
    assert!(body["paths"].is_object(), "paths object should be present");

    // Verify key paths are documented.
    let paths = &body["paths"];
    assert!(
        paths.get("/api/v1/health").is_some(),
        "health path should be documented in spec"
    );
    assert!(
        paths.get("/api/v1/verify").is_some(),
        "verify path should be documented in spec"
    );
    assert!(
        paths.get("/api/v1/stats").is_some(),
        "stats path should be documented in spec"
    );
    assert!(
        paths.get("/api/v1/auth/keys").is_some(),
        "auth keys path should be documented in spec"
    );
    assert!(
        paths.get("/api/v1/auth/keys/{key_id}").is_some(),
        "revoke key path should be documented in spec"
    );

    // Verify bearer auth security scheme is present.
    let security_schemes = &body["components"]["securitySchemes"];
    assert!(
        security_schemes.get("bearerAuth").is_some(),
        "bearerAuth security scheme should be in spec"
    );
}

/// Test 11: Empty file upload returns 400 Bad Request.
#[tokio::test]
async fn test_verify_empty_file_rejected() {
    let state = build_test_state_async().await;
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let auth = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(vec![])
            .file_name("empty.png")
            .mime_str("image/png")
            .unwrap(),
    );

    let resp = client
        .post(format!("{base_url}/api/v1/verify"))
        .header("Authorization", format!("Bearer {auth}"))
        .multipart(form)
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["code"], "BadRequest");
}

/// Test 12: Fingerprint endpoint returns hashes for a valid image.
#[tokio::test]
async fn test_fingerprint_endpoint() {
    let state = build_test_state_async().await;
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let auth = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(minimal_png())
            .file_name("test.png")
            .mime_str("image/png")
            .unwrap(),
    );

    let resp = client
        .post(format!("{base_url}/api/v1/protect/fingerprint"))
        .header("Authorization", format!("Bearer {auth}"))
        .multipart(form)
        .send()
        .await
        .expect("request");

    let status = resp.status();
    let body: Value = resp.json().await.expect("json");

    if status != StatusCode::OK {
        panic!("Expected 200, got {status}: {body}");
    }

    let hashes = body["data"]["hashes"].as_array().expect("hashes array");
    assert!(!hashes.is_empty(), "should return at least one hash");

    // Each entry should have algorithm and hashHex fields.
    for entry in hashes {
        assert!(
            entry["algorithm"].is_string(),
            "algorithm should be a string"
        );
        assert!(entry["hashHex"].is_string(), "hashHex should be a string");
    }
}

/// Test 13: Rate-limit headers are present on successful requests too.
#[tokio::test]
async fn test_rate_limit_headers_on_success() {
    let state = build_test_state_async().await;
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let auth = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let resp = reqwest::Client::new()
        .get(format!("{base_url}/api/v1/stats"))
        .header("Authorization", format!("Bearer {auth}"))
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::OK);

    // Rate-limit headers should be attached to all authenticated responses.
    let headers = resp.headers();
    assert!(
        headers.contains_key("x-ratelimit-limit"),
        "X-RateLimit-Limit should be on successful responses"
    );
    assert!(
        headers.contains_key("x-ratelimit-remaining"),
        "X-RateLimit-Remaining should be on successful responses"
    );
    assert!(
        headers.contains_key("x-ratelimit-reset"),
        "X-RateLimit-Reset should be on successful responses"
    );
}

/// Test 14: Batch verify — POST /api/v1/verify/batch with multiple files.
#[tokio::test]
async fn test_batch_verify() {
    let state = build_test_state_async().await;
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let auth = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new()
        .text("mode", "quick")
        .part(
            "files",
            reqwest::multipart::Part::bytes(minimal_png())
                .file_name("a.png")
                .mime_str("image/png")
                .unwrap(),
        )
        .part(
            "files",
            reqwest::multipart::Part::bytes(minimal_png())
                .file_name("b.png")
                .mime_str("image/png")
                .unwrap(),
        );

    let resp = client
        .post(format!("{base_url}/api/v1/verify/batch"))
        .header("Authorization", format!("Bearer {auth}"))
        .multipart(form)
        .send()
        .await
        .expect("request");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "batch verify should return 200"
    );

    let body: Value = resp.json().await.expect("json");
    let data = &body["data"];
    assert_eq!(data["total"], 2, "should process 2 files");
    let items = data["items"].as_array().expect("items array");
    assert_eq!(items.len(), 2, "should return 2 results");
    for item in items {
        assert!(item["filename"].is_string(), "filename should be present");
    }
}

/// Test 15: Batch verify with no files returns 400.
#[tokio::test]
async fn test_batch_verify_no_files() {
    let state = build_test_state_async().await;
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let auth = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new().text("mode", "quick");

    let resp = client
        .post(format!("{base_url}/api/v1/verify/batch"))
        .header("Authorization", format!("Bearer {auth}"))
        .multipart(form)
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// Test 16: Verify endpoint streams the multipart upload to a tempfile rather
/// than buffering the whole body in memory.  A valid PNG posted to
/// `POST /api/v1/verify` must reach the verification pipeline and return
/// a 200 with a JSON body — confirming the streaming fix (Fix 6) does not
/// regress the happy path.
#[tokio::test]
async fn test_verify_file_streaming_returns_200() {
    let state = build_test_state_async().await;
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let auth = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new().text("mode", "quick").part(
        "file",
        reqwest::multipart::Part::bytes(minimal_png())
            .file_name("test.png")
            .mime_str("image/png")
            .unwrap(),
    );

    let resp = client
        .post(format!("{base_url}/api/v1/verify"))
        .header("Authorization", format!("Bearer {auth}"))
        .multipart(form)
        .send()
        .await
        .expect("request");

    // The sidecar is not available in CI, so we accept either 200 (full result)
    // or a degraded 200 — the key assertion is that the streaming rewrite does
    // not break the multipart parse or temp-file routing.
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "verify should return 200 for a valid PNG upload"
    );

    let body: Value = resp.json().await.expect("json body");
    // Top-level envelope fields (same shape as test_verify_file_upload).
    assert_eq!(body["apiVersion"], "1.0", "apiVersion must be 1.0");
    assert!(
        body["degraded"].is_boolean(),
        "degraded field must be present"
    );
    // Verify the routing reached the pipeline: overallTrust must be present.
    assert!(
        body["data"]["overallTrust"].is_number(),
        "overallTrust must be present — streaming fix must not break pipeline routing"
    );
}

/// Test 17: A non-image file (PDF stub) verified in standard mode must NOT
/// return `degraded: true`, even though ELA and deepfake are both absent.
/// Prior to the Finding 5 fix, every non-image standard/deep verify was
/// incorrectly marked degraded because the heuristic was not content-aware.
#[tokio::test]
async fn test_non_image_standard_verify_not_degraded() {
    let state = build_test_state_async().await;
    let bootstrap_raw = insert_bootstrap_key(state.clone()).await;
    let auth = format!("jt_{bootstrap_raw}");

    let (listener, _) = bind_random_port();
    let base_url = start_test_server(state, listener).await;

    let client = reqwest::Client::new();
    // A minimal PDF stub — enough for the pipeline to detect the format
    // and route to the document path. The pipeline returns without ELA/deepfake
    // (document content), which was the false-degraded trigger before the fix.
    let pdf_stub = b"%PDF-1.4\n%%EOF\n";
    let form = reqwest::multipart::Form::new()
        .text("mode", "standard")
        .part(
            "file",
            reqwest::multipart::Part::bytes(pdf_stub.to_vec())
                .file_name("test.pdf")
                .mime_str("application/pdf")
                .unwrap(),
        );

    let resp = client
        .post(format!("{base_url}/api/v1/verify"))
        .header("Authorization", format!("Bearer {auth}"))
        .multipart(form)
        .send()
        .await
        .expect("request");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "non-image standard verify should return 200"
    );

    let body: Value = resp.json().await.expect("json body");
    assert_eq!(body["apiVersion"], "1.0");
    assert_eq!(
        body["degraded"], false,
        "non-image content must not be marked degraded (Finding 5): {body}"
    );
    // The pipeline should report document content type.
    assert_eq!(
        body["data"]["contentType"], "document",
        "PDF stub must be classified as document"
    );
}

// ── Verify-pipeline freeze-fix tests (VERIFY_GATE + snapshot-and-drop) ───────
//
// These three tests verify the fix for the verify-pipeline mutex hold that
// previously froze the Tauri event loop and blocked every other IPC command
// for the entire 5-45 s verify duration.
//
// Test strategy: call `verify_content_inner` directly (it is `pub`) with a
// real AppState backed by a temp SQLite database.  No stub sidecar is needed
// because when the sidecar is not reachable `sidecar.is_available()` returns
// false and all detector groups are skipped — graceful degradation means the
// pipeline still completes successfully (EXIF + C2PA + trust score) and
// writes its row, making these tests deterministic and fast (~100-400 ms each).

/// Write a minimal 16×16 PNG to a named temp file and return the path.
fn write_test_png() -> (tempfile::NamedTempFile, String) {
    let mut tmp = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .expect("temp png file");
    tmp.write_all(&minimal_png()).expect("write png");
    let path = tmp.path().to_string_lossy().to_string();
    (tmp, path)
}

/// Build a minimal `AppState` with a real Database — sync version for use
/// inside `std::thread::spawn` (no async context).
fn build_sync_state() -> (Arc<Mutex<AppState>>, tempfile::TempDir) {
    // Point at a port that nothing is listening on — sidecar will be
    // reported unavailable, all detector groups skip gracefully.
    build_sync_state_at("http://127.0.0.1:19999", 19999)
}

/// Like `build_sync_state` but with an explicit sidecar base URL/port, so a
/// test can point the pipeline at a local stub sidecar.
fn build_sync_state_at(
    sidecar_url: &str,
    sidecar_port: u16,
) -> (Arc<Mutex<AppState>>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("verify_test.db");
    let db = Database::open(&db_path).expect("open db");
    let sidecar = SidecarClient::new(sidecar_url, "");
    let state = AppState {
        db,
        sidecar,
        db_path: db_path.to_string_lossy().to_string(),
        licence_tier: LicenceTier::Community,
        sidecar_process: None,
        classifier_model_hash: Some("testhash_gbm".to_string()),
        univfd_probe_model_hash: Some("testhash_univfd".to_string()),
        ai_description_enabled: None,
        scheduler_handle: None,
        last_heatmap_session: None,
        last_sidecar_request_ts: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        power_saver_mode: false,
        respawn_in_progress: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        sidecar_port,
        sidecar_startup_status: Arc::new(std::sync::atomic::AtomicU8::new(0)),
        sidecar_startup_started_at: Arc::new(std::sync::atomic::AtomicU64::new(0)),
    };
    (Arc::new(Mutex::new(state)), dir)
}

/// Test 18: State mutex is NOT held during the verify detector pipeline.
///
/// With the old code, `verify_content_inner` held the `AppState` mutex for
/// the entire pipeline (snapshot + all detector groups + DB write), so a
/// concurrent `try_lock()` would fail.  With the fix, the lock is dropped
/// after the snapshot and re-acquired once at the end for the DB write, so it
/// is free throughout the detector phase.
///
/// **This test used to be flaky, and the reason is worth keeping.** It pointed
/// at `build_sync_state()`, whose sidecar port has nothing listening on it, so
/// `sidecar_up` was false and every detector group was skipped. That collapses
/// the free window down to the gap between the snapshot drop and the final DB
/// write at `verify/pipeline.rs:1644` — the very window the test exists to
/// observe. A single `try_lock()` at a fixed 50 ms then samples an interval
/// that is only reliably wide on an idle machine, and under CI load it landed
/// on a held lock and failed a green branch.
///
/// Two changes make it deterministic. It now runs against a reachable stub
/// sidecar, as `verifies_are_serialised` does, so the detector phase really
/// happens and the free window is hundreds of milliseconds wide. And it
/// samples repeatedly until the worker finishes rather than at one instant:
/// the invariant is that the lock is free at *some* point while the pipeline
/// is still running, which the old long-held-lock code could never satisfy no
/// matter when it was sampled.
#[test]
fn lock_free_during_verify() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // Detector calls sleep for `delay` before answering 500, and detectors
    // degrade gracefully on errors, so the verify still completes while the
    // detector phase stays open long enough to observe.
    let (port, _windows) = spawn_stub_sidecar(Duration::from_millis(150));
    let (state, _dir) = build_sync_state_at(&format!("http://127.0.0.1:{port}"), port);
    let (tmp_png, png_path) = write_test_png();

    let done = Arc::new(AtomicBool::new(false));
    let done_worker = Arc::clone(&done);
    let state_clone = Arc::clone(&state);
    let path_clone = png_path.clone();

    // Spawn the verify on a worker thread.
    let handle = std::thread::spawn(move || {
        let result = jura_trace_lib::verify_content_inner(
            &path_clone,
            "file",
            Some("standard"),
            &state_clone,
        );
        done_worker.store(true, Ordering::SeqCst);
        result
    });

    // Give the worker thread time to enter the pipeline and take the lock for
    // the snapshot.  On the old code it would still be holding that lock for
    // the whole run, so every sample below would see it blocked.
    std::thread::sleep(Duration::from_millis(50));

    let mut observed_free = false;
    let mut samples = 0u32;
    while !done.load(Ordering::SeqCst) {
        samples += 1;
        let free = {
            let attempt = state.try_lock();
            attempt.is_ok()
            // guard, if any, drops here — never held across the sleep
        };
        if free {
            observed_free = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }

    // Wait for the verify to complete and confirm it succeeded.
    let result = handle.join().expect("verify thread panicked");

    assert!(
        samples > 0,
        "the pipeline finished before sampling began, so the invariant was \
         never exercised — the stub sidecar delay is too short"
    );
    assert!(
        observed_free,
        "AppState mutex must be free during the verify detector pipeline \
         (snapshot-and-drop fix regressed or not applied); {samples} samples \
         taken while the pipeline ran and the lock was held for every one"
    );
    assert!(
        result.is_ok(),
        "verify_content_inner must succeed when the sidecar answers 500 (graceful degradation): {result:?}"
    );

    // Keep the temp file alive until the verify is done.
    drop(tmp_png);
}

/// Spawn a minimal stub sidecar on an ephemeral port.
///
/// `GET /health` answers 200 immediately so `SidecarClient::is_available`
/// reports the sidecar as up.  Every other request (the detector calls)
/// sleeps for `delay`, then answers 500 — detectors degrade gracefully on
/// errors, so the verify still completes while the stub records the time
/// window in which detector traffic arrived.  Connections are handled on
/// their own threads so parallel detector calls within one verify are
/// served concurrently.  The accept-loop thread lives until process exit,
/// like the tokio servers spawned by the async tests above.
#[allow(clippy::type_complexity)]
fn spawn_stub_sidecar(
    delay: Duration,
) -> (
    u16,
    Arc<Mutex<Vec<(std::time::Instant, std::time::Instant)>>>,
) {
    use std::io::Read;

    fn read_request_head(stream: &mut std::net::TcpStream) -> String {
        // First read blocks for the request line + headers; the short
        // timeout afterwards drains whatever body bytes have arrived so the
        // client's write never stalls.
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        if let Ok(n) = stream.read(&mut chunk) {
            buf.extend_from_slice(&chunk[..n]);
        }
        stream
            .set_read_timeout(Some(Duration::from_millis(30)))
            .ok();
        while let Ok(n) = stream.read(&mut chunk) {
            if n == 0 || buf.len() > 262_144 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
        }
        String::from_utf8_lossy(&buf).into_owned()
    }

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub sidecar");
    let port = listener.local_addr().expect("stub addr").port();
    let windows = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&windows);

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let recorder = Arc::clone(&recorder);
            std::thread::spawn(move || {
                let head = read_request_head(&mut stream);
                if head.starts_with("GET") && head.contains("/health") {
                    let _ = stream.write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}",
                    );
                } else {
                    let start = std::time::Instant::now();
                    std::thread::sleep(delay);
                    let _ = stream.write_all(
                        b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    recorder
                        .lock()
                        .unwrap()
                        .push((start, std::time::Instant::now()));
                }
            });
        }
    });

    (port, windows)
}

/// Test 19: Concurrent verify calls are serialised — never interleaved.
///
/// Each verify gets its OWN stub sidecar, so the time window in which a stub
/// receives detector traffic is exactly the window in which that verify's
/// gated section ran.  VERIFY_GATE serialises gated sections, so the two
/// windows must not overlap.  (Wall-clock ratio assertions are unreliable
/// here because the pre-gate stages — hashing, EXIF, C2PA — legitimately run
/// in parallel and dominate when the sidecar work is short.)
#[test]
fn verifies_are_serialised() {
    let delay = Duration::from_millis(150);
    let (port_a, windows_a) = spawn_stub_sidecar(delay);
    let (port_b, windows_b) = spawn_stub_sidecar(delay);

    let (state_a, _dir_a) = build_sync_state_at(&format!("http://127.0.0.1:{port_a}"), port_a);
    let (state_b, _dir_b) = build_sync_state_at(&format!("http://127.0.0.1:{port_b}"), port_b);

    let (tmp_a, path_a) = write_test_png();
    let (tmp_b, path_b) = write_test_png();

    // Spawn both verify calls at the same instant.
    let h_a = {
        let s = Arc::clone(&state_a);
        std::thread::spawn(move || {
            jura_trace_lib::verify_content_inner(&path_a, "file", Some("standard"), &s).is_ok()
        })
    };
    let h_b = {
        let s = Arc::clone(&state_b);
        std::thread::spawn(move || {
            jura_trace_lib::verify_content_inner(&path_b, "file", Some("standard"), &s).is_ok()
        })
    };

    assert!(
        h_a.join().expect("thread A panicked"),
        "verify A must succeed"
    );
    assert!(
        h_b.join().expect("thread B panicked"),
        "verify B must succeed"
    );

    let windows_a = windows_a.lock().unwrap();
    let windows_b = windows_b.lock().unwrap();
    assert!(
        !windows_a.is_empty(),
        "stub A must have received detector traffic — did the standard group run?"
    );
    assert!(
        !windows_b.is_empty(),
        "stub B must have received detector traffic — did the standard group run?"
    );

    // The gated section of one verify must finish before the other's begins.
    let a_start = windows_a.iter().map(|w| w.0).min().unwrap();
    let a_end = windows_a.iter().map(|w| w.1).max().unwrap();
    let b_start = windows_b.iter().map(|w| w.0).min().unwrap();
    let b_end = windows_b.iter().map(|w| w.1).max().unwrap();

    assert!(
        a_end <= b_start || b_end <= a_start,
        "Detector windows overlap — VERIFY_GATE is not serialising concurrent verifies \
         (A {a_start:?}..{a_end:?}, B {b_start:?}..{b_end:?})"
    );

    drop((tmp_a, tmp_b));
}

/// Test 20: After a successful verify, the verifications table has a row.
///
/// Verifies that the DB-write re-acquisition step actually commits the row
/// (regression guard for a hypothetical bug where the lock re-acquire fails
/// silently and the row is never written).
#[test]
fn verify_row_persisted() {
    let (state, _dir) = build_sync_state();
    let (tmp_png, png_path) = write_test_png();

    let result = jura_trace_lib::verify_content_inner(&png_path, "file", Some("quick"), &state);

    assert!(
        result.is_ok(),
        "verify_content_inner must succeed in quick mode: {result:?}"
    );

    // Read verification history through the Database public API.
    let history = state
        .lock()
        .expect("state lock")
        .db
        .get_verification_history(10, 0)
        .expect("get_verification_history");

    assert_eq!(
        history.len(),
        1,
        "exactly one verification row must be persisted after a successful verify \
         (re-acquire for DB write may have silently failed). Got {} rows.",
        history.len()
    );

    drop(tmp_png);
}

// ── Test image helper ─────────────────────────────────────────────────────────

/// A valid 16×16 pixel PNG in raw bytes.
///
/// Generated via Python's `struct`/`zlib` with correct CRC values.
/// Hard-coded to avoid any filesystem dependency at test time.
/// 16×16 is large enough for perceptual hash algorithms (need at least 8×8).
fn minimal_png() -> Vec<u8> {
    // 16×16 RGB PNG (solid red): signature + IHDR + IDAT + IEND.
    vec![
        137, 80, 78, 71, 13, 10, 26, 10, // PNG signature
        0, 0, 0, 13, // IHDR length = 13
        73, 72, 68, 82, // "IHDR"
        0, 0, 0, 16, 0, 0, 0, 16, // width=16, height=16
        8, 2, 0, 0, 0, // 8-bit RGB, no interlace
        144, 145, 104, 54, // IHDR CRC
        0, 0, 0, 23, // IDAT length = 23
        73, 68, 65, 84, // "IDAT"
        120, 156, 99, 248, 207, 192, 64, 18, 34, 77, 245, 168, 134, 81, 13, 67, 74, 3, 0, 144, 249,
        255, 1, 249, 225, 250, 120, // IDAT CRC
        0, 0, 0, 0, // IEND length = 0
        73, 69, 78, 68, // "IEND"
        174, 66, 96, 130, // IEND CRC
    ]
}
