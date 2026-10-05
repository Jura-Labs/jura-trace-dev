// SPDX-License-Identifier: AGPL-3.0-or-later

//! Assertion A5 of the design (section 11): every exit code, from the real
//! binary, as a subprocess. The server is a stub that answers with the API's
//! own envelopes, so each case controls exactly what the client sees and the
//! tests need no frozen sidecar. Every case checks the command's own exit
//! status before it looks at anything the command printed.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

const KEY: &str = "jt_0123456789abcdef0123456789abcdef";

/// One request as the stub saw it.
#[derive(Clone, Debug)]
struct Seen {
    method: String,
    path: String,
    auth: Option<String>,
    body: String,
}

type Handler = dyn Fn(&Seen) -> (u16, String, Duration) + Send + Sync;

/// A one-route-table HTTP server on a free port. Returns its base URL and
/// the requests it received.
fn stub(
    handler: impl Fn(&Seen) -> (u16, String, Duration) + Send + Sync + 'static,
) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    let handler: Arc<Handler> = Arc::new(handler);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let log = Arc::clone(&log);
            let handler = Arc::clone(&handler);
            std::thread::spawn(move || serve(stream, &*handler, &log));
        }
    });
    (base, seen)
}

fn serve(mut stream: TcpStream, handler: &Handler, log: &Mutex<Vec<Seen>>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let (mut len, mut chunked, mut auth) = (0usize, false, None);
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h).unwrap_or(0) == 0 || h == "\r\n" {
            break;
        }
        let lower = h.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            len = v.trim().parse().unwrap_or(0);
        }
        if lower.starts_with("transfer-encoding:") && lower.contains("chunked") {
            chunked = true;
        }
        if lower.starts_with("authorization:") {
            auth = Some(h["authorization:".len()..].trim().to_string());
        }
    }
    let mut body = Vec::new();
    if chunked {
        loop {
            let mut size = String::new();
            reader.read_line(&mut size).unwrap();
            let n = usize::from_str_radix(size.trim(), 16).unwrap_or(0);
            let mut chunk = vec![0; n + 2];
            reader.read_exact(&mut chunk).unwrap();
            if n == 0 {
                break;
            }
            body.extend_from_slice(&chunk[..n]);
        }
    } else {
        body.resize(len, 0);
        reader.read_exact(&mut body).unwrap();
    }
    let seen = Seen {
        method,
        path,
        auth,
        body: String::from_utf8_lossy(&body).into_owned(),
    };
    log.lock().unwrap().push(seen.clone());
    let (status, payload, delay) = handler(&seen);
    std::thread::sleep(delay);
    // The sign route answers with the file and a suggested name, as the
    // real server does.
    let disposition = if seen.path == "/api/v1/protect/sign" && status == 200 {
        "Content-Disposition: attachment; filename=\"photo_signed.jpg\"\r\n"
    } else {
        ""
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n{disposition}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );
}

fn ok(body: Value) -> (u16, String, Duration) {
    (200, body.to_string(), Duration::ZERO)
}

fn err(status: u16, code: &str) -> (u16, String, Duration) {
    (
        status,
        json!({ "code": code, "message": format!("stub {code}") }).to_string(),
        Duration::ZERO,
    )
}

fn result(band: &str, degraded: bool) -> Value {
    json!({
        "data": {
            "overallTrust": 0.31, "mode": "deep", "contentType": "image",
            "verdict": { "band": band, "score": 0.31, "ceilingApplied": null,
                         "bandBoundaries": { "trusted": 0.7, "uncertain": 0.4 } },
            "detectorsRun": ["exif_anomaly", "c2pa", "ela", "deepfake"],
            "inputSha256": "abc",
            "provenance": { "engineVersion": "1.2.0", "sidecarVersion": "1.2.0",
                            "verificationMode": "deep", "timestampUtc": "2026-10-03T10:00:00Z" }
        },
        "apiVersion": "1.0",
        "degraded": degraded
    })
}

/// A server whose verify route always answers `band`.
fn verify_server(band: &'static str, degraded: bool) -> (String, Arc<Mutex<Vec<Seen>>>) {
    stub(move |s| match s.path.as_str() {
        "/api/v1/verify" | "/api/v1/verify/url" => ok(result(band, degraded)),
        _ => err(404, "NotFound"),
    })
}

fn photo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("photo.jpg");
    std::fs::write(&path, b"\xff\xd8\xff\xe0 not really a jpeg").unwrap();
    (dir, path)
}

/// Run `jura` with a clean environment: no inherited key or address, and
/// the OS keyring switched off so a developer's stored key cannot leak in.
fn jura(base: &str, key: Option<&str>, args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_jura"));
    cmd.env_remove("JURA_API_KEY")
        .env_remove("JURA_API_URL")
        .env("JURA_NO_KEYRING", "1")
        .env("JURA_API_URL", base)
        .args(args);
    if let Some(k) = key {
        cmd.env("JURA_API_KEY", k);
    }
    cmd.output().expect("run jura")
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("exited with a code")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn p(path: &Path) -> &str {
    path.to_str().unwrap()
}

// ── 0 and 20: an analysis was produced ──────────────────────────────────────

#[test]
fn exit_0_for_a_low_trust_verdict_without_fail_on() {
    let (base, seen) = verify_server("untrusted", false);
    let (_d, file) = photo();
    let out = jura(&base, Some(KEY), &["verify", p(&file), "--format", "json"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let printed: Value = serde_json::from_str(&stdout(&out)).expect("stdout is the JSON body");
    assert_eq!(printed, result("untrusted", false), "the body, unchanged");
    let req = &seen.lock().unwrap()[0];
    assert_eq!(
        (req.method.as_str(), req.path.as_str()),
        ("POST", "/api/v1/verify")
    );
    assert_eq!(req.auth.as_deref(), Some(format!("Bearer {KEY}").as_str()));
    assert!(
        req.body.contains("name=\"file\""),
        "multipart field is `file`"
    );
}

#[test]
fn exit_20_when_the_band_meets_fail_on() {
    let (_d, file) = photo();
    for (band, fail_on, want) in [
        ("untrusted", "untrusted", 20),
        ("uncertain", "untrusted", 0),
        ("uncertain", "uncertain", 20),
        ("trusted", "uncertain", 0),
    ] {
        let (base, _) = verify_server(band, false);
        let out = jura(
            &base,
            Some(KEY),
            &["verify", p(&file), "--fail-on", fail_on],
        );
        assert_eq!(
            code(&out),
            want,
            "{band} --fail-on {fail_on}: {}",
            stderr(&out)
        );
        assert!(
            stdout(&out).contains(&format!("verdict      {band}")),
            "{}",
            stdout(&out)
        );
    }
}

// ── 8: no usable verdict ────────────────────────────────────────────────────

#[test]
fn exit_8_for_an_inconclusive_band_with_fail_on() {
    let (base, _) = verify_server("inconclusive", false);
    let (_d, file) = photo();
    let out = jura(
        &base,
        Some(KEY),
        &["verify", p(&file), "--fail-on", "untrusted"],
    );
    assert_eq!(code(&out), 8, "{}", stderr(&out));
    assert!(stderr(&out).contains("inconclusive"), "{}", stderr(&out));
    // Without --fail-on it is a completed analysis like any other.
    let out = jura(&base, Some(KEY), &["verify", p(&file)]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
}

#[test]
fn exit_8_for_a_degraded_result_with_require_complete() {
    let (base, _) = verify_server("uncertain", true);
    let (_d, file) = photo();
    let out = jura(
        &base,
        Some(KEY),
        &["verify", p(&file), "--require-complete"],
    );
    assert_eq!(code(&out), 8, "{}", stderr(&out));
    let out = jura(&base, Some(KEY), &["verify", p(&file)]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("degraded"), "{}", stdout(&out));
}

// ── 1: usage ────────────────────────────────────────────────────────────────

#[test]
fn exit_1_for_usage_errors_not_clap_s_2() {
    let (base, seen) = verify_server("trusted", false);
    let (_d, file) = photo();
    for args in [
        vec!["verify"],
        vec!["verify", p(&file), "--no-such-flag"],
        vec!["verify", p(&file), "--mode", "Deep"],
        vec!["verify", p(&file), "--compact"],
        vec!["frobnicate"],
    ] {
        let out = jura(&base, Some(KEY), &args);
        assert_eq!(code(&out), 1, "{args:?}: {}", stderr(&out));
    }
    assert!(
        seen.lock().unwrap().is_empty(),
        "no request for a usage error"
    );
    // Help and version are not errors.
    assert_eq!(code(&jura(&base, None, &["--help"])), 0);
    assert_eq!(code(&jura(&base, None, &["--version"])), 0);
}

#[test]
fn exit_1_when_the_server_refuses_the_request_as_malformed() {
    let (base, _) = stub(|_| err(400, "InvalidParameter"));
    let (_d, file) = photo();
    let out = jura(&base, Some(KEY), &["verify", p(&file)]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("InvalidParameter"),
        "{}",
        stderr(&out)
    );
}

// ── 2: unreachable ──────────────────────────────────────────────────────────

#[test]
fn exit_2_when_nothing_is_listening() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let (_d, file) = photo();
    let out = jura(
        &format!("http://127.0.0.1:{port}"),
        Some(KEY),
        &["verify", p(&file)],
    );
    assert_eq!(code(&out), 2, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("could not connect"),
        "{}",
        stderr(&out)
    );
    // A file large enough that the upload is under way when the connection
    // is refused: reqwest calls that a body error, not a connect error.
    // Found 4 October 2026 with a 163 KB JPEG, which exited 6.
    let dir = tempfile::tempdir().unwrap();
    let big = dir.path().join("big.jpg");
    std::fs::write(&big, vec![0xffu8; 2 * 1024 * 1024]).unwrap();
    let out = jura(
        &format!("http://127.0.0.1:{port}"),
        Some(KEY),
        &["verify", p(&big)],
    );
    assert_eq!(code(&out), 2, "{}", stderr(&out));
}

// ── 3: auth ─────────────────────────────────────────────────────────────────

#[test]
fn exit_3_with_no_key_before_any_request() {
    let (base, seen) = verify_server("trusted", false);
    let (_d, file) = photo();
    let out = jura(&base, None, &["verify", p(&file)]);
    assert_eq!(code(&out), 3, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("jura-trace-api keys add"),
        "{}",
        stderr(&out)
    );
    assert!(seen.lock().unwrap().is_empty());
}

/// A server that refuses the key before reading the upload, and closes the
/// connection, as the real server's middleware can. The client never reads
/// that 401 while it is still sending; it must still exit 3, not 6.
#[test]
fn exit_3_when_the_key_is_refused_mid_upload() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut head = [0u8; 1024];
            let _ = stream.read(&mut head);
            let body = r#"{"code":"Unauthorized","message":"Valid API key required."}"#;
            let _ = write!(
                stream,
                "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            // Close without draining the upload, so the client's write fails.
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    });
    let dir = tempfile::tempdir().unwrap();
    let big = dir.path().join("big.jpg");
    std::fs::write(&big, vec![0xffu8; 8 * 1024 * 1024]).unwrap();
    let out = jura(&base, Some(KEY), &["verify", p(&big)]);
    assert_eq!(code(&out), 3, "{}", stderr(&out));
}

#[test]
fn exit_3_when_the_server_rejects_the_key() {
    let (base, _) = stub(|_| err(401, "Unauthorized"));
    let (_d, file) = photo();
    let out = jura(&base, Some(KEY), &["verify", p(&file)]);
    assert_eq!(code(&out), 3, "{}", stderr(&out));
    let out = jura(&base, Some(KEY), &["auth", "status"]);
    assert_eq!(code(&out), 3, "{}", stderr(&out));
}

// ── 4: local file ───────────────────────────────────────────────────────────

#[test]
fn exit_4_for_missing_or_empty_files_without_uploading() {
    let (base, seen) = verify_server("trusted", false);
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.jpg");
    std::fs::write(&empty, b"").unwrap();
    let missing = dir.path().join("missing.jpg");
    for f in [&missing, &empty, &dir.path().to_path_buf()] {
        let out = jura(&base, Some(KEY), &["verify", p(f)]);
        assert_eq!(code(&out), 4, "{}: {}", f.display(), stderr(&out));
    }
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn exit_4_when_the_server_says_the_file_is_empty() {
    let (base, _) = stub(|_| err(400, "EmptyFile"));
    let (_d, file) = photo();
    assert_eq!(code(&jura(&base, Some(KEY), &["verify", p(&file)])), 4);
}

// ── 5: unsupported content ──────────────────────────────────────────────────

#[test]
fn exit_5_for_unsupported_content() {
    let (base, _) = stub(|_| err(422, "UnsupportedFormat"));
    let (_d, file) = photo();
    let out = jura(&base, Some(KEY), &["verify", p(&file)]);
    assert_eq!(code(&out), 5, "{}", stderr(&out));
}

// ── 6: server ───────────────────────────────────────────────────────────────

#[test]
fn exit_6_for_server_errors_and_unusable_answers() {
    let (_d, file) = photo();
    let (base, _) = stub(|_| err(500, "Internal"));
    assert_eq!(code(&jura(&base, Some(KEY), &["verify", p(&file)])), 6);
    let (base, _) = stub(|_| (502, "<html>bad gateway</html>".into(), Duration::ZERO));
    assert_eq!(code(&jura(&base, Some(KEY), &["verify", p(&file)])), 6);
    let (base, _) = stub(|_| (200, "not json".into(), Duration::ZERO));
    assert_eq!(code(&jura(&base, Some(KEY), &["verify", p(&file)])), 6);
}

// ── 7: timeout ──────────────────────────────────────────────────────────────

#[test]
fn exit_7_when_the_response_is_slower_than_timeout() {
    let (base, _) = stub(|_| {
        (
            200,
            result("trusted", false).to_string(),
            Duration::from_secs(4),
        )
    });
    let (_d, file) = photo();
    let out = jura(&base, Some(KEY), &["verify", p(&file), "--timeout", "1"]);
    assert_eq!(code(&out), 7, "{}", stderr(&out));
}

#[test]
fn wait_ready_waits_then_proceeds_or_gives_up() {
    let (_d, file) = photo();
    // Starting for the first two polls, then ready.
    let polls = Arc::new(Mutex::new(0));
    let counter = Arc::clone(&polls);
    let (base, seen) = stub(move |s| match s.path.as_str() {
        "/api/v1/ready" => {
            let mut n = counter.lock().unwrap();
            *n += 1;
            let state = if *n < 3 { "starting" } else { "ready" };
            ok(
                json!({ "data": { "server": "ready", "sidecar": state }, "apiVersion": "1.0", "degraded": state != "ready" }),
            )
        }
        "/api/v1/verify" => ok(result("trusted", false)),
        _ => err(404, "NotFound"),
    });
    let out = jura(
        &base,
        Some(KEY),
        &["verify", p(&file), "--wait-ready", "10"],
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let paths: Vec<String> = seen
        .lock()
        .unwrap()
        .iter()
        .map(|s| s.path.clone())
        .collect();
    assert_eq!(
        paths.last().map(String::as_str),
        Some("/api/v1/verify"),
        "{paths:?}"
    );
    assert_eq!(
        paths.iter().filter(|p| *p == "/api/v1/ready").count(),
        3,
        "{paths:?}"
    );
    // No key is sent to /ready.
    assert!(seen
        .lock()
        .unwrap()
        .iter()
        .filter(|s| s.path == "/api/v1/ready")
        .all(|s| s.auth.is_none()));

    // Never ready: 7, and the file is never sent.
    let (base, seen) = stub(|_| {
        ok(json!({ "data": { "sidecar": "starting" }, "apiVersion": "1.0", "degraded": true }))
    });
    let out = jura(&base, Some(KEY), &["verify", p(&file), "--wait-ready", "2"]);
    assert_eq!(code(&out), 7, "{}", stderr(&out));
    assert!(seen
        .lock()
        .unwrap()
        .iter()
        .all(|s| s.path == "/api/v1/ready"));

    // Failed: 7 at once, without waiting out the limit.
    let (base, _) = stub(|_| {
        ok(
            json!({ "data": { "sidecar": "failed", "detail": "stopped" }, "apiVersion": "1.0", "degraded": true }),
        )
    });
    let start = std::time::Instant::now();
    let out = jura(
        &base,
        Some(KEY),
        &["verify", p(&file), "--wait-ready", "60"],
    );
    assert_eq!(code(&out), 7, "{}", stderr(&out));
    assert!(start.elapsed() < Duration::from_secs(10));

    // Nothing listening for the whole wait: 2.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let out = jura(
        &format!("http://127.0.0.1:{port}"),
        Some(KEY),
        &["verify", p(&file), "--wait-ready", "1"],
    );
    assert_eq!(code(&out), 2, "{}", stderr(&out));
}

// ── Several inputs ──────────────────────────────────────────────────────────

#[test]
fn a_failure_outranks_a_met_threshold_and_every_input_is_tried() {
    let (base, seen) = verify_server("untrusted", false);
    let (_d, file) = photo();
    let missing = file.with_file_name("missing.jpg");
    let out = jura(
        &base,
        Some(KEY),
        &[
            "verify",
            p(&file),
            p(&missing),
            p(&file),
            "--fail-on",
            "untrusted",
            "--format",
            "json",
            "--compact",
        ],
    );
    assert_eq!(code(&out), 4, "{}", stderr(&out));
    assert_eq!(
        seen.lock().unwrap().len(),
        2,
        "both readable files were sent"
    );
    let printed = stdout(&out);
    let lines: Vec<&str> = printed.lines().collect();
    assert_eq!(lines.len(), 2, "one compact document per result");
    for l in lines {
        serde_json::from_str::<Value>(l).expect("each line is a JSON document");
    }
}

#[test]
fn ndjson_is_one_line_per_input_with_its_exit_code() {
    let (base, _) = verify_server("untrusted", false);
    let (_d, file) = photo();
    let missing = file.with_file_name("missing.jpg");
    let out = jura(
        &base,
        Some(KEY),
        &[
            "verify",
            p(&file),
            p(&missing),
            p(&file),
            "--fail-on",
            "untrusted",
            "--format",
            "ndjson",
        ],
    );
    // A failure outranks a met threshold, as in every format.
    assert_eq!(code(&out), 4, "{}", stderr(&out));
    let printed = stdout(&out);
    let lines: Vec<Value> = printed
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("not JSON ({e}): {l}")))
        .collect();
    assert_eq!(
        lines.len(),
        3,
        "one line per input, the failure included:\n{printed}"
    );
    assert_eq!(lines[0]["input"], p(&file));
    assert_eq!(lines[0]["exit"], 20);
    assert_eq!(
        lines[0]["response"],
        result("untrusted", false),
        "the body, unchanged"
    );
    assert_eq!(lines[1]["input"], p(&missing));
    assert_eq!(lines[1]["exit"], 4);
    assert!(lines[1]["error"]["message"]
        .as_str()
        .unwrap()
        .contains("no such file"));
    assert!(lines[1].get("response").is_none());
    assert_eq!(lines[2]["exit"], 20);
}

#[test]
fn verify_url_posts_json_with_the_mode() {
    let (base, seen) = verify_server("trusted", false);
    let out = jura(
        &base,
        Some(KEY),
        &[
            "verify",
            "--url",
            "https://example.org/a.jpg",
            "--mode",
            "quick",
        ],
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let req = &seen.lock().unwrap()[0];
    assert_eq!(req.path, "/api/v1/verify/url");
    let body: Value = serde_json::from_str(&req.body).unwrap();
    assert_eq!(
        body,
        json!({ "url": "https://example.org/a.jpg", "mode": "quick" })
    );
}

// ── sign ────────────────────────────────────────────────────────────────────

fn sign_server() -> (String, Arc<Mutex<Vec<Seen>>>) {
    stub(|s| match s.path.as_str() {
        "/api/v1/protect/sign" => (200, "SIGNED-BYTES".into(), Duration::ZERO),
        _ => err(404, "NotFound"),
    })
}

#[test]
fn sign_writes_the_copy_beside_the_original() {
    let (base, seen) = sign_server();
    let (dir, file) = photo();
    let out = jura(
        &base,
        Some(KEY),
        &[
            "sign",
            p(&file),
            "--creator",
            "Ada Lovelace",
            "--license",
            "CC-BY-4.0",
        ],
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let signed = dir.path().join("photo_signed.jpg");
    assert_eq!(std::fs::read(&signed).unwrap(), b"SIGNED-BYTES");
    assert!(
        stdout(&out).contains("photo_signed.jpg"),
        "{}",
        stdout(&out)
    );
    let req = seen.lock().unwrap()[0].clone();
    assert!(req.body.contains("name=\"creator_name\"") && req.body.contains("Ada Lovelace"));
    assert!(req.body.contains("CC-BY-4.0"));
    assert!(
        !req.body.contains("name=\"timestamp\""),
        "no timestamp field unless asked"
    );
    // The original is untouched.
    assert_ne!(std::fs::read(&file).unwrap(), b"SIGNED-BYTES");

    // An existing copy is not replaced without --force, and nothing is sent.
    let before = seen.lock().unwrap().len();
    let out = jura(
        &base,
        Some(KEY),
        &["sign", p(&file), "--creator", "Ada", "-o", p(&signed)],
    );
    assert_eq!(code(&out), 4, "{}", stderr(&out));
    assert_eq!(seen.lock().unwrap().len(), before);
    let out = jura(
        &base,
        Some(KEY),
        &[
            "sign",
            p(&file),
            "--creator",
            "Ada",
            "-o",
            p(&signed),
            "--force",
            "--no-timestamp",
        ],
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let last = seen.lock().unwrap().last().unwrap().clone();
    assert!(
        last.body.contains("name=\"timestamp\"") && last.body.contains("\r\n\r\nno\r\n"),
        "{}",
        last.body.len()
    );
}

#[test]
fn sign_exit_codes() {
    let (_d, file) = photo();
    // The timestamp question, unanswered: 1, with the server's advice.
    let (base, _) = stub(|_| err(409, "TimestampChoiceRequired"));
    let out = jura(&base, Some(KEY), &["sign", p(&file), "--creator", "Ada"]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(stderr(&out).contains("TimestampChoiceRequired"));
    // A format the server cannot sign: 5.
    let (base, _) = stub(|_| err(422, "C2pa"));
    assert_eq!(
        code(&jura(
            &base,
            Some(KEY),
            &["sign", p(&file), "--creator", "Ada"]
        )),
        5
    );
    // No --creator, or both timestamp flags: usage, before any request.
    let (base, seen) = sign_server();
    assert_eq!(code(&jura(&base, Some(KEY), &["sign", p(&file)])), 1);
    assert_eq!(
        code(&jura(
            &base,
            Some(KEY),
            &[
                "sign",
                p(&file),
                "--creator",
                "A",
                "--timestamp",
                "--no-timestamp"
            ]
        )),
        1
    );
    assert!(seen.lock().unwrap().is_empty());
}

// ── version and auth ────────────────────────────────────────────────────────

#[test]
fn version_reports_client_engine_and_api() {
    let (base, _) = stub(|s| match s.path.as_str() {
        "/api/v1/health" => ok(
            json!({ "status": "ok", "version": "1.2.0", "sidecarAvailable": true, "uptimeSeconds": 1 }),
        ),
        "/api/v1/ready" => {
            ok(json!({ "data": { "sidecar": "ready" }, "apiVersion": "1.0", "degraded": false }))
        }
        _ => err(404, "NotFound"),
    });
    let out = jura(&base, None, &["version", "--format", "json"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let v: Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(v["engineVersion"], "1.2.0");
    assert_eq!(v["apiVersion"], "1.0");
    assert_eq!(v["cli"], env!("CARGO_PKG_VERSION"));

    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let out = jura(&format!("http://127.0.0.1:{port}"), None, &["version"]);
    assert_eq!(code(&out), 2);
    assert!(
        stdout(&out).contains(env!("CARGO_PKG_VERSION")),
        "still prints its own version"
    );
}

#[test]
fn auth_show_key_masks_and_names_the_source() {
    let (base, _) = stub(|_| ok(json!({ "data": {}, "apiVersion": "1.0", "degraded": false })));
    let out = jura(&base, Some(KEY), &["auth", "show-key"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("jt_0123…cdef"), "{}", stdout(&out));
    assert!(!stdout(&out).contains(KEY));
    assert!(stdout(&out).contains("JURA_API_KEY"));
    let out = jura(&base, Some(KEY), &["auth", "show-key", "--reveal"]);
    assert!(stdout(&out).contains(KEY));
    // A flag outranks the environment.
    let out = jura(
        &base,
        Some(KEY),
        &["--api-key", "jt_fromtheflag0000", "auth", "show-key"],
    );
    assert!(stdout(&out).contains("--api-key"), "{}", stdout(&out));

    let out = jura(&base, Some(KEY), &["auth", "status"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert_eq!(code(&jura(&base, None, &["auth", "show-key"])), 3);
    assert_eq!(
        code(&jura(&base, Some("not-a-key"), &["auth", "status"])),
        3
    );
}
