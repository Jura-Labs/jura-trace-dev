// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `jura-trace-api` binary, run as a subprocess (v1.2.0 B2a stage 1).
//! Design: `docs/design/v1.2.0-headless-api-and-cli.md` section 11 (A1, A2).
//! All run with `--no-sidecar` and a temporary database, so they need no
//! frozen sidecar and never touch the user's own data.

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_jura-trace-api");

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn temp_db() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("api.db").to_string_lossy().into_owned();
    (dir, db)
}

/// Start the server and return it with the first line it printed on stdout.
fn start(port: u16, db: &str) -> (Child, String) {
    let mut child = Command::new(BIN)
        .args(["--port", &port.to_string(), "--no-sidecar", "--db", db])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    (child, line.trim().to_string())
}

fn get(port: u16, path: &str, key: Option<&str>) -> (u16, String) {
    let mut req = reqwest::blocking::Client::new().get(format!("http://127.0.0.1:{port}{path}"));
    if let Some(k) = key {
        req = req.bearer_auth(k);
    }
    let resp = req.send().unwrap();
    (resp.status().as_u16(), resp.text().unwrap())
}

fn stop(mut child: Child) -> Option<i32> {
    #[cfg(unix)]
    {
        // SIGTERM, which is what a service manager or `docker stop` sends.
        unsafe { libc_kill(child.id() as i32, 15) };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while std::time::Instant::now() < deadline {
            if let Some(status) = child.try_wait().unwrap() {
                return status.code();
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let _ = child.kill();
        None
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
        let _ = child.wait();
        None
    }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "kill"]
    fn libc_kill(pid: i32, sig: i32) -> i32;
}

/// A2: a busy port is fatal, and the error names the port. Until v1.2.0 the
/// API logged "listening" and carried on without a server.
#[test]
fn a_taken_port_is_fatal_and_named() {
    let held = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = held.local_addr().unwrap().port();
    let (_dir, db) = temp_db();
    let out = Command::new(BIN)
        .args(["--port", &port.to_string(), "--no-sidecar", "--db", &db])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "a taken port must exit 2");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&port.to_string()),
        "stderr must name the port: {stderr}"
    );
}

/// A1, as the address the server's own socket reports, and the whole path a
/// user takes: mint a key without a server, start the server, use the key,
/// stop it cleanly.
#[test]
fn serves_on_loopback_with_a_key_minted_offline() {
    let (_dir, db) = temp_db();

    let out = Command::new(BIN)
        .args(["keys", "add", "--name", "test", "--db", &db])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "keys add prints the key and nothing else: {stdout:?}"
    );
    let key = lines[0];
    assert!(
        key.starts_with("jt_") && key.len() == 67,
        "unexpected key shape: {key}"
    );

    let listed = Command::new(BIN)
        .args(["keys", "list", "--db", &db])
        .output()
        .unwrap();
    let listed = String::from_utf8(listed.stdout).unwrap();
    assert!(listed.contains("\ttest\t100\tfalse\t"), "{listed}");
    assert!(
        !listed.contains(&key[3..]),
        "keys list must never print a key"
    );

    let port = free_port();
    let (child, first) = start(port, &db);
    assert_eq!(
        first,
        format!("jura-trace-api: listening on http://127.0.0.1:{port}")
    );

    let (status, body) = get(port, "/api/v1/health", None);
    assert_eq!(status, 200);
    assert!(body.contains("\"sidecarAvailable\":false"), "{body}");

    assert_eq!(get(port, "/api/v1/stats", None).0, 401);
    let (status, body) = get(port, "/api/v1/stats", Some(key));
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"totalAssets\""), "{body}");

    if cfg!(unix) {
        assert_eq!(stop(child), Some(0), "SIGTERM must give a clean exit 0");
    } else {
        stop(child);
    }
}

#[test]
fn usage_errors_exit_1() {
    for args in [&["--bogus"][..], &["--port", "0"], &["keys", "add"]] {
        let out = Command::new(BIN).args(args).output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{args:?}");
    }
}
