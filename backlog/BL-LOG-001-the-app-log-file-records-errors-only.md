# BL-LOG-001: the app's log file records errors only

**Status**: Open. Found 23 September 2026, W39-25, when a Windows probe
collected `jura-trace.log` from an installed build and it was 0 bytes.
**Severity**: Medium. Every `log::info!` and `log::warn!` in the Rust app is
discarded in production on every platform, so the log file a user can send
us after a failure shows only errors. That includes the whole sidecar
startup record ("Sidecar ready after N poll attempt(s)", the relayed uvicorn
lines, orphan-kill and `_MEI` cleanup) that earlier fixes added specifically
for diagnosis.
**Raised**: 23 September 2026

## What happened

`src-tauri/src/startup.rs:134` builds the logger with
`env_logger::Builder::from_default_env()`, and the fallback at `:144` is
`env_logger::init()`. Both take their filter from `RUST_LOG`, and with
`RUST_LOG` unset env_logger's default level is `error`. Nothing in
`src-tauri/src` sets a level: `grep -rn "RUST_LOG\|filter_level\|parse_filters"`
finds only the doc comment at `startup.rs:77`. An installed app launched
from the Start menu or the dock never has `RUST_LOG` set.

Evidence, both on `windows-latest` with the shipped v1.1.0 MSI:

- Launched normally (B1 port probe, run 35827994296): `jura-trace.log` 0 bytes.
- Launched with `RUST_LOG=info` in the environment (run 35860504128): 68
  lines covering two launches, including every sidecar startup line.

It went unnoticed because development runs from a terminal, where
`RUST_LOG` is usually set, and because the file exists, so it looks as if
logging works.

## What would fix it

Give the builder a default level that `RUST_LOG` can still override:
`env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))`,
in both branches. Then check the file cap at `startup.rs` (10 MiB,
truncate on open) still holds at `info` volume, and consider millisecond
timestamps (`format_timestamp_millis()`), since second resolution is what
made BL-PERF-001 need process creation times to find a 5-second gap.

## What not to do

Do not set `debug` as the default. The sidecar relay logs every line the
sidecar prints at `info`, and per-request lines at `debug` would make the
10 MiB cap truncate the startup record that matters.
