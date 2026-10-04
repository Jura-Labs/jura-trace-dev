// SPDX-License-Identifier: AGPL-3.0-or-later

//! `jura`: command-line client for the Jura Trace local REST API.
//!
//! A thin client. Every analysis runs in the server (the Jura Trace app, or
//! `jura-trace-api`); this binary uploads, waits, prints the response and
//! exits with a code from the published contract (`exit.rs`).
//!
//! Design: `docs/design/v1.2.0-headless-api-and-cli.md` sections 5, 7 and 8.

mod client;
mod config;
mod exit;
mod pretty;
mod verdict;

use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::Value;

use client::Client;
use config::KeySource;
use exit::{Exit, Failure};
use verdict::FailOn;

#[derive(Parser)]
#[command(
    name = "jura",
    version,
    about = "Command-line client for the Jura Trace local REST API",
    long_about = "Command-line client for the Jura Trace local REST API.\n\n\
        The analysis runs in Jura Trace (the app, or jura-trace-api), on this \
        machine by default. Results are evidence for a person to weigh, not a \
        finding that content is genuine or false.\n\n\
        Exit codes: 0 done (whatever the verdict), 1 usage, 2 unreachable, 3 auth, \
        4 local file, 5 unsupported content, 6 server error, 7 timeout, \
        8 incomplete, 20 --fail-on threshold met."
)]
struct Cli {
    #[command(flatten)]
    global: Global,
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
struct Global {
    /// API address. Also JURA_API_URL. Default http://127.0.0.1:8300
    #[arg(long, global = true, value_name = "URL")]
    api_url: Option<String>,
    /// API key. Also JURA_API_KEY, or the OS keyring (`jura auth set-key`).
    /// The least safe of the three: it lands in shell history.
    #[arg(long, global = true, value_name = "KEY")]
    api_key: Option<String>,
    /// Output format. `text` is for people and may change in any release;
    /// scripts use `json`, which is the API response body.
    #[arg(long, global = true, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// With --format json, one line per document instead of pretty-printed.
    #[arg(long, global = true)]
    compact: bool,
    /// Seconds to wait for each response; 0 waits indefinitely.
    #[arg(long, global = true, value_name = "SECS", default_value_t = 600)]
    timeout: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Subcommand)]
enum Command {
    /// Analyse files, or a URL, and print the result.
    ///
    /// Exits 0 whenever an analysis was produced, including a low-trust one.
    /// Use --fail-on to exit 20 on a band, and --require-complete to exit 8 on
    /// a degraded result.
    Verify(VerifyArgs),
    /// Print the versions of this client, the server and its API.
    Version,
    /// Store, inspect or test the API key.
    #[command(subcommand)]
    Auth(AuthCommand),
}

#[derive(Args)]
struct VerifyArgs {
    /// Files to upload, analysed one after another.
    #[arg(
        value_name = "PATH",
        required_unless_present = "url",
        conflicts_with = "url"
    )]
    paths: Vec<PathBuf>,
    /// Ask the server to download and analyse this URL instead.
    #[arg(long, value_name = "URL")]
    url: Option<String>,
    /// quick, standard or deep. The server's default is deep for files and
    /// standard for a URL.
    #[arg(long, value_enum)]
    mode: Option<Mode>,
    /// Exit 20 when the verdict band is this or worse. An inconclusive
    /// verdict exits 8.
    #[arg(long, value_enum, value_name = "BAND")]
    fail_on: Option<FailOn>,
    /// Exit 8 when some analysis services were unavailable.
    #[arg(long)]
    require_complete: bool,
    /// Wait until the analysis engine is ready before sending, for up to
    /// SECS (default 300).
    #[arg(long, value_name = "SECS", num_args = 0..=1, default_missing_value = "300")]
    wait_ready: Option<u64>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Mode {
    Quick,
    Standard,
    Deep,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Mode::Quick => "quick",
            Mode::Standard => "standard",
            Mode::Deep => "deep",
        }
    }
}

#[derive(Subcommand)]
enum AuthCommand {
    /// Store a key in the OS keyring. Pass `-` to read it from stdin, which
    /// keeps it out of shell history.
    SetKey {
        #[arg(value_name = "KEY")]
        key: String,
    },
    /// Show which key is in use and where it came from.
    ShowKey {
        /// Print the whole key, not a masked form.
        #[arg(long)]
        reveal: bool,
    },
    /// Check that a key is configured and that the server accepts it.
    Status,
    /// Remove the key from the OS keyring.
    ClearKey,
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // clap exits 2 for usage errors, which this contract gives to
            // "unreachable". Help and --version are not errors.
            let _ = e.print();
            let code = if e.use_stderr() {
                Exit::Usage
            } else {
                Exit::Success
            };
            std::process::exit(code.code());
        }
    };
    let code = match run(cli) {
        Ok(code) => code,
        Err(f) => {
            eprintln!("jura: {f}");
            f.exit
        }
    };
    std::process::exit(code.code());
}

struct Ctx {
    global: Global,
}

impl Ctx {
    fn api_url(&self) -> String {
        self.global
            .api_url
            .clone()
            .or_else(|| std::env::var("JURA_API_URL").ok().filter(|v| !v.is_empty()))
            .unwrap_or_else(|| config::DEFAULT_API_URL.to_string())
    }

    fn key(&self) -> Option<(String, KeySource)> {
        let env = std::env::var("JURA_API_KEY").ok();
        config::resolve_key(
            self.global.api_key.as_deref(),
            env.as_deref(),
            config::keyring_get,
        )
    }

    fn client(&self) -> Result<Client, Failure> {
        let timeout = (self.global.timeout > 0).then(|| Duration::from_secs(self.global.timeout));
        Client::new(&self.api_url(), self.key().map(|(k, _)| k), timeout)
    }

    /// Print a response body: verbatim JSON, or text for a person.
    fn print(&self, label: &str, resp: &client::Response) {
        let out = match self.global.format {
            Format::Json if self.global.compact => resp.raw.trim().to_string(),
            Format::Json => pretty::pretty(resp.raw.trim()),
            Format::Text => verdict::text(label, &resp.json).trim_end().to_string(),
        };
        let mut stdout = std::io::stdout().lock();
        let _ = writeln!(stdout, "{out}");
    }
}

fn run(cli: Cli) -> Result<Exit, Failure> {
    if cli.global.compact && cli.global.format != Format::Json {
        return Err(Failure::new(Exit::Usage, "--compact needs --format json"));
    }
    let ctx = Ctx { global: cli.global };
    match cli.command {
        Command::Verify(args) => verify(&ctx, args),
        Command::Version => version(&ctx),
        Command::Auth(cmd) => auth(&ctx, cmd),
    }
}

enum Input {
    Url(String),
    File(PathBuf),
}

fn verify(ctx: &Ctx, args: VerifyArgs) -> Result<Exit, Failure> {
    let client = ctx.client()?;
    if let Some(secs) = args.wait_ready {
        wait_ready(&client, Duration::from_secs(secs))?;
    }
    let mode = args.mode.map(Mode::as_str);

    // A URL, or the files in the order given.
    let inputs: Vec<Input> = match &args.url {
        Some(url) => vec![Input::Url(url.clone())],
        None => args.paths.iter().cloned().map(Input::File).collect(),
    };

    // Every input is attempted. The exit code is the first failure's, if any
    // (codes 1 to 8 mean no usable verification), otherwise 20 if any verdict
    // met --fail-on, otherwise 0. A server that cannot be reached, or will
    // not take the key, fails the same way for every input, so it stops the
    // run.
    let mut first_failure: Option<Exit> = None;
    let mut threshold_met = false;
    for input in inputs {
        let (label, sent) = match &input {
            Input::Url(url) => (url.clone(), client.verify_url(url, mode)),
            Input::File(path) => (path.display().to_string(), client.verify_file(path, mode)),
        };
        let outcome = sent.and_then(|resp| {
            ctx.print(&label, &resp);
            verdict::judge(&resp.json, args.fail_on, args.require_complete)
        });
        match outcome {
            Ok(Exit::Verdict) => threshold_met = true,
            Ok(_) => {}
            Err(f) => {
                eprintln!("jura: {label}: {f}");
                first_failure.get_or_insert(f.exit);
                if matches!(f.exit, Exit::Unreachable | Exit::Auth) {
                    break;
                }
            }
        }
    }
    Ok(match (first_failure, threshold_met) {
        (Some(code), _) => code,
        (None, true) => Exit::Verdict,
        (None, false) => Exit::Success,
    })
}

/// Poll `GET /api/v1/ready` (no key needed) until the engine is ready.
/// A server that never answers is 2; one that answers but whose engine is
/// not ready in time is 7; an engine that has failed or is absent will not
/// become ready by waiting, so that is 7 at once.
fn wait_ready(client: &Client, limit: Duration) -> Result<(), Failure> {
    let deadline = Instant::now() + limit;
    let mut answered = false;
    loop {
        match client.get_open("/api/v1/ready") {
            Ok(resp) => {
                answered = true;
                match resp.json.pointer("/data/sidecar").and_then(Value::as_str) {
                    Some("ready") => return Ok(()),
                    Some(state @ ("failed" | "absent")) => {
                        let detail = resp
                            .json
                            .pointer("/data/detail")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        return Err(Failure::new(
                            Exit::Timeout,
                            format!("the analysis engine is {state} and will not become ready by waiting. {detail}"),
                        ));
                    }
                    _ => {}
                }
            }
            // A server from before v1.2.0 has no /ready; fall back to
            // /health, which is slower but says whether the sidecar answers.
            Err(f) if f.message.contains("HTTP 404") => {
                answered = true;
                if let Ok(h) = client.get_open("/api/v1/health") {
                    if h.json.get("sidecarAvailable").and_then(Value::as_bool) == Some(true) {
                        return Ok(());
                    }
                }
            }
            Err(f) if f.exit == Exit::Unreachable || f.exit == Exit::Timeout => {}
            Err(f) => return Err(f),
        }
        if Instant::now() >= deadline {
            return Err(if answered {
                Failure::new(
                    Exit::Timeout,
                    format!(
                        "--wait-ready: the analysis engine was not ready after {} s",
                        limit.as_secs()
                    ),
                )
            } else {
                Failure::new(
                    Exit::Unreachable,
                    format!(
                        "--wait-ready: nothing answered at {} in {} s",
                        client.base(),
                        limit.as_secs()
                    ),
                )
            });
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn version(ctx: &Ctx) -> Result<Exit, Failure> {
    let cli_version = env!("CARGO_PKG_VERSION");
    let client = ctx.client()?;
    let health = client.get_open("/api/v1/health");
    let ready = client.get_open("/api/v1/ready").ok();
    let engine = health.as_ref().ok().and_then(|h| {
        h.json
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let api = ready.as_ref().and_then(|r| {
        r.json
            .get("apiVersion")
            .and_then(Value::as_str)
            .map(str::to_string)
    });

    match ctx.global.format {
        Format::Json => {
            let doc = serde_json::json!({
                "cli": cli_version,
                "server": client.base(),
                "engineVersion": engine,
                "apiVersion": api,
            });
            let out = if ctx.global.compact {
                doc.to_string()
            } else {
                serde_json::to_string_pretty(&doc).unwrap_or_default()
            };
            println!("{out}");
        }
        Format::Text => {
            println!("jura      {cli_version}");
            println!("server    {}", client.base());
            if let Some(e) = &engine {
                println!("engine    {e}");
            }
            if let Some(a) = &api {
                println!("api       {a}");
            }
        }
    }
    health.map(|_| Exit::Success)
}

fn auth(ctx: &Ctx, cmd: AuthCommand) -> Result<Exit, Failure> {
    match cmd {
        AuthCommand::SetKey { key } => {
            let key = if key == "-" {
                if std::io::stdin().is_terminal() {
                    eprintln!("Paste the key, then press Enter:");
                }
                let mut s = String::new();
                std::io::stdin()
                    .read_to_string(&mut s)
                    .map_err(|e| Failure::new(Exit::Usage, format!("could not read stdin: {e}")))?;
                s.trim().to_string()
            } else {
                key.trim().to_string()
            };
            config::check_key_shape(&key)?;
            config::keyring_set(&key)?;
            println!("Stored {} in the OS keyring.", config::mask(&key));
            println!("Run `jura auth status` to check the server accepts it.");
            Ok(Exit::Success)
        }
        AuthCommand::ShowKey { reveal } => {
            let (key, source) = ctx.key().ok_or_else(no_key)?;
            let shown = if reveal {
                key.clone()
            } else {
                config::mask(&key)
            };
            println!("{shown}  (from {})", source.describe());
            Ok(Exit::Success)
        }
        AuthCommand::Status => {
            let (key, source) = ctx.key().ok_or_else(no_key)?;
            config::check_key_shape(&key)?;
            let client = ctx.client()?;
            client.get("/api/v1/stats")?;
            println!(
                "{} from {} is accepted by {}",
                config::mask(&key),
                source.describe(),
                client.base()
            );
            Ok(Exit::Success)
        }
        AuthCommand::ClearKey => {
            if config::keyring_clear()? {
                println!("Removed the key from the OS keyring.");
            } else {
                println!("There was no key in the OS keyring.");
            }
            Ok(Exit::Success)
        }
    }
}

fn no_key() -> Failure {
    Failure::new(
        Exit::Auth,
        "no API key. Set JURA_API_KEY, or store one with `jura auth set-key`. \
         To create one: `jura-trace-api keys add --name <label>`",
    )
}
