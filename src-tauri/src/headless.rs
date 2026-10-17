// SPDX-License-Identifier: AGPL-3.0-or-later

//! The headless API server, `jura-trace-api` (v1.2.0 B2a stage 1).
//!
//! Runs the same REST API as the desktop app, with no window: it opens the
//! app's database, owns the analysis sidecar through
//! [`crate::sidecar_supervisor`], and serves on `127.0.0.1`. It also mints
//! API keys without a server, which in v1.1.0 no user could do at all.
//! Design: `docs/design/v1.2.0-headless-api-and-cli.md` sections 4 and 6.
//!
//! Exit codes: 0 after a clean shutdown, 1 for a usage error, 2 when the
//! server cannot start (port taken, database unopenable), 3 when
//! `--wait-ready` fails.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::sidecar_supervisor::{self, SidecarSupervisor};
use crate::{db, sidecar, AppState, SidecarStartupStatus};

const USAGE: &str = "\
jura-trace-api: the Jura Trace REST API without the desktop window.

USAGE:
    jura-trace-api [OPTIONS]                 serve on http://127.0.0.1:8300
    jura-trace-api keys add --name <LABEL>   create an API key, print it once, exit
    jura-trace-api keys list                 list API keys (never the keys themselves)

OPTIONS:
    --port <N>              port on 127.0.0.1 [default: 8300]
    --db <PATH>             database file [default: the desktop app's database]
    --ephemeral             a throwaway session: a new database and data folder in
                            the temporary directory, deleted when the server stops
                            cleanly, a key for the session printed once, and
                            Standard network mode. Not with --db
    --no-sidecar            run without the analysis sidecar; results are degraded
    --sidecar-binary <PATH> the sidecar executable [default: JURA_SIDECAR_BINARY,
                            then beside this executable]
    --models-dir <PATH>     model files for the sidecar [default: beside the sidecar]
    --wait-ready[=<SECS>]   wait for the sidecar before reporting ready; exit 3 if it
                            is not ready in time [default: 300]
    -h, --help              this text
    -V, --version           the version

The server listens on loopback only. Only one server, desktop app or
jura-trace-api, can use a port at a time; a taken port is an error.
Exit codes: 0 clean shutdown, 1 usage, 2 could not start, 3 --wait-ready failed.";

const EXIT_USAGE: i32 = 1;
const EXIT_START: i32 = 2;
const EXIT_NOT_READY: i32 = 3;
const DESKTOP_PORT: u16 = 8300;

#[derive(Debug, Default, PartialEq)]
struct ServeArgs {
    port: Option<u16>,
    db: Option<PathBuf>,
    no_sidecar: bool,
    sidecar_binary: Option<PathBuf>,
    models_dir: Option<PathBuf>,
    wait_ready: Option<u64>,
    ephemeral: bool,
}

#[derive(Debug, PartialEq)]
enum Cmd {
    Serve(ServeArgs),
    KeysAdd {
        name: String,
        rate_limit: i64,
        db: Option<PathBuf>,
    },
    KeysList {
        db: Option<PathBuf>,
    },
    Help,
    Version,
}

fn value(args: &mut std::slice::Iter<'_, String>, flag: &str) -> Result<String, String> {
    args.next()
        .cloned()
        .ok_or_else(|| format!("{flag} needs a value"))
}

fn parse(argv: &[String]) -> Result<Cmd, String> {
    let mut it = argv.iter();
    match argv.first().map(String::as_str) {
        Some("-h" | "--help" | "help") => return Ok(Cmd::Help),
        Some("-V" | "--version") => return Ok(Cmd::Version),
        Some("keys") => {
            it.next();
            let sub = it.next().map(String::as_str);
            let mut name = None;
            let mut rate_limit = 100;
            let mut db = None;
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--name" => name = Some(value(&mut it, a)?),
                    "--rate-limit" => {
                        rate_limit = value(&mut it, a)?
                            .parse::<i64>()
                            .ok()
                            .filter(|n| *n >= 1)
                            .ok_or("--rate-limit must be a whole number of at least 1")?
                    }
                    "--db" => db = Some(PathBuf::from(value(&mut it, a)?)),
                    other => return Err(format!("unknown option for keys: {other}")),
                }
            }
            return match sub {
                Some("add") => {
                    let name = name
                        .map(|n| n.trim().to_string())
                        .filter(|n| !n.is_empty())
                        .ok_or("keys add needs --name <LABEL>")?;
                    Ok(Cmd::KeysAdd {
                        name,
                        rate_limit,
                        db,
                    })
                }
                Some("list") => Ok(Cmd::KeysList { db }),
                _ => Err("keys needs a subcommand: add or list".into()),
            };
        }
        _ => {}
    }

    let mut s = ServeArgs::default();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--port" => {
                s.port = Some(
                    value(&mut it, a)?
                        .parse::<u16>()
                        .ok()
                        .filter(|p| *p != 0)
                        .ok_or("--port must be between 1 and 65535")?,
                )
            }
            "--db" => s.db = Some(PathBuf::from(value(&mut it, a)?)),
            "--no-sidecar" => s.no_sidecar = true,
            "--ephemeral" => s.ephemeral = true,
            "--sidecar-binary" => s.sidecar_binary = Some(PathBuf::from(value(&mut it, a)?)),
            "--models-dir" => s.models_dir = Some(PathBuf::from(value(&mut it, a)?)),
            "--wait-ready" => s.wait_ready = Some(300),
            w if w.starts_with("--wait-ready=") => {
                s.wait_ready = Some(
                    w["--wait-ready=".len()..]
                        .parse::<u64>()
                        .ok()
                        .filter(|n| *n >= 1)
                        .ok_or("--wait-ready=<SECS> must be a whole number of seconds")?,
                )
            }
            "-h" | "--help" => return Ok(Cmd::Help),
            other => return Err(format!("unknown option: {other}")),
        }
    }
    if s.ephemeral && s.db.is_some() {
        return Err("--ephemeral makes its own database; it cannot be combined with --db".into());
    }
    if s.no_sidecar && (s.wait_ready.is_some() || s.sidecar_binary.is_some()) {
        return Err("--no-sidecar cannot be combined with --wait-ready or --sidecar-binary".into());
    }
    Ok(Cmd::Serve(s))
}

/// Entry point for the `jura-trace-api` binary. Returns the exit code.
pub fn main_with_args(argv: &[String]) -> i32 {
    let cmd = match parse(argv) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("jura-trace-api: {e}\n\nRun `jura-trace-api --help` for usage.");
            return EXIT_USAGE;
        }
    };
    match cmd {
        Cmd::Help => {
            println!("{USAGE}");
            0
        }
        Cmd::Version => {
            println!("jura-trace-api {}", env!("CARGO_PKG_VERSION"));
            0
        }
        cmd => {
            init_stderr_logging();
            match cmd {
                Cmd::KeysAdd {
                    name,
                    rate_limit,
                    db,
                } => keys_add(&name, rate_limit, db),
                Cmd::KeysList { db } => keys_list(db),
                Cmd::Serve(args) => serve(args),
                Cmd::Help | Cmd::Version => unreachable!(),
            }
        }
    }
}

/// Logs go to stderr so stdout carries only what a script reads: the key
/// from `keys add`, and the listening and ready lines from the server.
fn init_stderr_logging() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "warn,jura_trace_lib=info".into());
    let _ = env_logger::Builder::new()
        .parse_filters(&filter)
        .target(env_logger::Target::Stderr)
        .try_init();
}

/// `--db`, else the desktop app's own resolution (JURA_DB_PATH, config.json,
/// then the default file in the app data directory).
fn resolve_db(explicit: Option<PathBuf>) -> Result<(PathBuf, Option<PathBuf>), String> {
    if let Some(p) = explicit {
        return Ok((p, None));
    }
    let data_dir = crate::startup::dirs_next_data_dir()
        .ok_or("cannot find the app data directory (is HOME or APPDATA set?)")?;
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("cannot create {}: {e}", data_dir.display()))?;
    Ok((crate::config::resolve_db_path_in(&data_dir), Some(data_dir)))
}

fn open_db(explicit: Option<PathBuf>) -> Result<(db::Database, PathBuf, Option<PathBuf>), i32> {
    let (path, data_dir) = resolve_db(explicit).map_err(|e| {
        eprintln!("jura-trace-api: {e}");
        EXIT_START
    })?;
    let database = db::Database::open(&path).map_err(|e| {
        eprintln!(
            "jura-trace-api: cannot open the database {}: {e}",
            path.display()
        );
        EXIT_START
    })?;
    log::info!("Database: {}", path.display());
    Ok((database, path, data_dir))
}

/// Store a new key and return it with its id. Same shape as
/// POST /api/v1/auth/keys: 256 bits, SHA-256 at rest, shown once with the
/// jt_ prefix.
fn mint_key(
    database: &db::Database,
    name: &str,
    rate_limit: i64,
) -> Result<(String, String), String> {
    let (key, hash) = crate::api::auth::new_key();
    let key_id = uuid::Uuid::new_v4().to_string();
    database
        .create_api_key(&key_id, name, &hash, rate_limit)
        .map_err(|e| format!("could not store the key: {e}"))?;
    Ok((key, key_id))
}

fn keys_add(name: &str, rate_limit: i64, db: Option<PathBuf>) -> i32 {
    let (database, path, _) = match open_db(db) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let (key, key_id) = match mint_key(&database, name, rate_limit) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("jura-trace-api: {e}");
            return EXIT_START;
        }
    };
    println!("{key}");
    eprintln!(
        "Created key \"{name}\" ({key_id}) in {}. It is shown once; store it now.",
        path.display()
    );
    0
}

fn keys_list(db: Option<PathBuf>) -> i32 {
    let (database, _, _) = match open_db(db) {
        Ok(v) => v,
        Err(code) => return code,
    };
    match database.list_api_keys() {
        Ok(keys) => {
            println!("KEY ID\tNAME\tRATE LIMIT\tREVOKED\tCREATED");
            for k in keys {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    k.key_id, k.name, k.rate_limit, k.revoked, k.created_at
                );
            }
            0
        }
        Err(e) => {
            eprintln!("jura-trace-api: could not read keys: {e}");
            EXIT_START
        }
    }
}

/// Is a Jura Trace API answering on the desktop's port? Used to avoid
/// sweeping the desktop app's sidecar when we run beside it on another port.
fn desktop_api_answers() -> bool {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()
        .and_then(|c| {
            c.get(format!("http://127.0.0.1:{DESKTOP_PORT}/api/v1/health"))
                .send()
                .ok()
        })
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn serve(args: ServeArgs) -> i32 {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("jura-trace-api: cannot start the runtime: {e}");
            return EXIT_START;
        }
    };

    let port = args.port.unwrap_or(DESKTOP_PORT);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    // Bind first. A taken port means the desktop app or another server is
    // running, and finding out now, before anything is spawned or swept,
    // keeps us from touching its sidecar.
    let listener = match runtime.block_on(crate::api::bind(addr)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "jura-trace-api: cannot listen on {addr}: {e}. Is the Jura Trace app, or \
                 another jura-trace-api, already using port {port}? Quit it or use --port."
            );
            return EXIT_START;
        }
    };
    let local = listener
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| addr.to_string());

    // --ephemeral: everything this session writes goes in a temporary
    // folder that is removed when the server stops cleanly (the end of this
    // function). The database is new, so it has no key: one is minted and
    // printed below. The folder has no network_mode.json, which would mean
    // Enhanced mode; Standard is written instead, so the session makes no
    // network request it was not asked for (a signature's timestamp then has
    // to be asked for with timestamp=yes).
    let ephemeral = if args.ephemeral {
        match tempfile::Builder::new()
            .prefix("jura-trace-ephemeral-")
            .tempdir()
        {
            Ok(d) => Some(d),
            Err(e) => {
                eprintln!("jura-trace-api: cannot create a temporary folder: {e}");
                return EXIT_START;
            }
        }
    } else {
        None
    };
    let db_arg = match &ephemeral {
        Some(d) => {
            if let Err(e) =
                std::fs::write(d.path().join("network_mode.json"), r#"{"mode":"standard"}"#)
            {
                eprintln!(
                    "jura-trace-api: cannot write to {}: {e}",
                    d.path().display()
                );
                return EXIT_START;
            }
            Some(d.path().join("jura_trace.db"))
        }
        None => args.db.clone(),
    };
    let (database, db_path, data_dir) = match open_db(db_arg) {
        Ok(v) => v,
        Err(code) => return code,
    };
    if let Some(d) = &ephemeral {
        match mint_key(&database, "ephemeral", 600) {
            Ok((key, _)) => println!("jura-trace-api: ephemeral key {key}"),
            Err(e) => {
                eprintln!("jura-trace-api: {e}");
                return EXIT_START;
            }
        }
        eprintln!(
            "jura-trace-api: ephemeral session in {}. It is deleted when the server stops \
             with Ctrl+C or SIGTERM; a killed process leaves it behind.",
            d.path().display()
        );
    }

    let config = data_dir
        .as_deref()
        .map(crate::config::read_app_config)
        .unwrap_or_default();

    // ── Sidecar ──────────────────────────────────────────────────────────
    let sidecar_key = match std::env::var("JURA_SIDECAR_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => format!(
            "{}{}",
            uuid::Uuid::new_v4().as_simple(),
            uuid::Uuid::new_v4().as_simple()
        ),
    };
    let sidecar_port = sidecar_supervisor::pick_ephemeral_port().unwrap_or(8200);
    let mut models_dir = args.models_dir.clone();
    let supervisor = if args.no_sidecar {
        log::warn!("--no-sidecar: only the Rust detectors will run; results will be degraded");
        None
    } else {
        match sidecar_supervisor::resolve_sidecar_binary(args.sidecar_binary.as_deref()) {
            None => {
                if let Some(p) = &args.sidecar_binary {
                    eprintln!(
                        "jura-trace-api: --sidecar-binary {} does not exist",
                        p.display()
                    );
                    return EXIT_USAGE;
                }
                log::warn!(
                    "No analysis sidecar found (set --sidecar-binary or JURA_SIDECAR_BINARY). \
                     Serving without it: results will be degraded."
                );
                None
            }
            Some(binary) => {
                // The sweep kills every jura-sidecar by name. Beside a running
                // desktop app (we are on another port and its API answers on
                // 8300) that would kill the app's sidecar, so skip it.
                if port != DESKTOP_PORT && desktop_api_answers() {
                    log::warn!("The Jura Trace app is running; not sweeping for orphan sidecars");
                } else {
                    sidecar_supervisor::kill_orphan_sidecars();
                }
                sidecar_supervisor::cleanup_stale_mei_dirs();
                if models_dir.is_none() {
                    models_dir = sidecar_supervisor::models_dir_for(&binary);
                }
                match SidecarSupervisor::spawn(
                    &binary,
                    models_dir.as_deref(),
                    sidecar_port,
                    &sidecar_key,
                ) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        eprintln!("jura-trace-api: {e}");
                        return EXIT_START;
                    }
                }
            }
        }
    };

    let hash_of = |name: &str| {
        models_dir
            .as_ref()
            .and_then(|d| crate::verify::pipeline::compute_file_sha256(&d.join(name)))
    };
    let status = supervisor
        .as_ref()
        .map(SidecarSupervisor::status_handle)
        .unwrap_or_else(|| Arc::new(AtomicU8::new(SidecarStartupStatus::NotPresent.to_u8())));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let status_since = supervisor
        .as_ref()
        .map(SidecarSupervisor::since_handle)
        .unwrap_or_else(|| Arc::new(AtomicU64::new(now)));

    let state = Arc::new(Mutex::new(AppState {
        db: database,
        sidecar: sidecar::SidecarClient::new(
            &format!("http://127.0.0.1:{sidecar_port}"),
            &sidecar_key,
        ),
        db_path: db_path.to_string_lossy().into_owned(),
        licence_tier: config.licence_tier,
        sidecar_process: None,
        classifier_model_hash: hash_of("deepfake_classifier.joblib"),
        univfd_probe_model_hash: hash_of("univfd_probe.joblib"),
        ai_description_enabled: config.ai_description_enabled,
        scheduler_handle: None,
        last_heatmap_session: None,
        last_sidecar_request_ts: Arc::new(AtomicU64::new(now)),
        power_saver_mode: false,
        respawn_in_progress: Arc::new(AtomicBool::new(false)),
        sidecar_port,
        sidecar_startup_status: status,
        sidecar_startup_started_at: Arc::new(AtomicU64::new(now)),
        sidecar_status_since: status_since,
    }));

    let has_key = state
        .lock()
        .ok()
        .and_then(|g| g.db.list_api_keys().ok())
        .map(|keys| keys.iter().any(|k| !k.revoked))
        .unwrap_or(false);
    if !has_key {
        eprintln!(
            "jura-trace-api: there is no active API key, so every request except \
             /api/v1/health will get 401. Create one with: jura-trace-api keys add --name <label>"
        );
    }

    println!("jura-trace-api: listening on http://{local}");

    let code = runtime.block_on(async {
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(crate::api::serve(listener, state, async move {
            let _ = stop_rx.await;
        }));

        let mut code = 0;
        if let (Some(secs), Some(sup)) = (args.wait_ready, supervisor.as_ref()) {
            let wait = Duration::from_secs(secs);
            let ready = tokio::task::block_in_place(|| sup.wait_ready(wait));
            match ready {
                Ok(()) => println!("jura-trace-api: analysis engine ready"),
                Err(e) => {
                    eprintln!("jura-trace-api: --wait-ready: {e}");
                    code = EXIT_NOT_READY;
                }
            }
        }

        if code == 0 {
            shutdown_signal().await;
            log::info!("Shutting down");
        }
        let _ = stop_tx.send(());
        match server.await {
            Ok(Ok(())) => code,
            Ok(Err(e)) => {
                eprintln!("jura-trace-api: server error: {e}");
                EXIT_START
            }
            Err(e) => {
                eprintln!("jura-trace-api: server task failed: {e}");
                EXIT_START
            }
        }
    });

    if let Some(sup) = supervisor {
        sup.shutdown();
        // Windows: ending the launcher does not end the onedir bootloader it
        // started, which has the same image name. Sweep, unless the desktop
        // app is running, whose sidecar the sweep would also end.
        #[cfg(windows)]
        if !desktop_api_answers() {
            sidecar_supervisor::kill_orphan_sidecars();
        }
    }
    // The runtime holds the last references to the database; drop it first
    // so the file is closed (Windows will not delete an open file).
    drop(runtime);
    if let Some(d) = ephemeral {
        let path = d.path().to_path_buf();
        match d.close() {
            Ok(()) => log::info!("Ephemeral session removed: {}", path.display()),
            Err(e) => eprintln!(
                "jura-trace-api: could not remove the ephemeral session at {}: {e}",
                path.display()
            ),
        }
    }
    code
}

/// Ctrl+C everywhere; SIGTERM as well on Unix, which is what a service
/// manager or `docker stop` sends.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn defaults_serve_on_8300() {
        assert_eq!(parse(&[]).unwrap(), Cmd::Serve(ServeArgs::default()));
    }

    #[test]
    fn serve_options() {
        let Cmd::Serve(s) = parse(&args("--port 8400 --db /tmp/x.db --wait-ready=60")).unwrap()
        else {
            panic!()
        };
        assert_eq!(s.port, Some(8400));
        assert_eq!(s.db, Some(PathBuf::from("/tmp/x.db")));
        assert_eq!(s.wait_ready, Some(60));
        let Cmd::Serve(s) = parse(&args("--wait-ready")).unwrap() else {
            panic!()
        };
        assert_eq!(s.wait_ready, Some(300));
    }

    #[test]
    fn usage_errors() {
        for bad in [
            "--port",
            "--port 0",
            "--port 70000",
            "--bogus",
            "--wait-ready=0",
            "--no-sidecar --wait-ready",
            "keys",
            "keys add",
            "keys add --name",
            "keys remove",
            "keys add --name ci --rate-limit 0",
        ] {
            assert!(
                parse(&args(bad)).is_err(),
                "{bad:?} should be a usage error"
            );
        }
    }

    #[test]
    fn keys_commands() {
        assert_eq!(
            parse(&args("keys add --name ci --rate-limit 600")).unwrap(),
            Cmd::KeysAdd {
                name: "ci".into(),
                rate_limit: 600,
                db: None
            }
        );
        assert_eq!(
            parse(&args("keys list --db /tmp/x.db")).unwrap(),
            Cmd::KeysList {
                db: Some(PathBuf::from("/tmp/x.db"))
            }
        );
    }
}
