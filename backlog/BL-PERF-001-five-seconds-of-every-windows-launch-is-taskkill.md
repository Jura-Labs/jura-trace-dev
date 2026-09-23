# BL-PERF-001: five seconds of every Windows launch is taskkill

**Status**: Open. Found 23 September 2026 while measuring where the Windows
startup wait goes after the B1 onedir port (PR #107).
**Severity**: Medium. Nothing is broken, but it is the largest single
remaining piece of the wait Windows users sit through on every launch, on
the platform with 64 per cent of downloads, and it is cheap to remove.
**Raised**: 23 September 2026, W39-25

## What happened

The B1 port probe measured the onedir app at 18 to 29 s from launch to
`sidecarAvailable` on `windows-latest`, against 28 to 39 s for the shipped
v1.1.0 `--onefile` build. That is less gain than expected, so the remaining
time was split in four probes on `probe/b1-windows-onedir`. None of them
builds or signs anything that ships.

**The sidecar is not the problem.** Run directly, outside the app, the
frozen onedir sidecar has its port open at about 3.0 s and `/health/ready`
answering at 3.2 s (run 35859940447, three runs, all within 0.1 s). Imports
take 1.8 s and the model warmup 0.8 s. Without the models directory it is
ready at 2.0 s. Frozen and unfrozen agree. Defender real-time scanning is
already off on GitHub-hosted runners, so those numbers carry no scanning
cost (see "What this does not cover").

**Five seconds of it is `taskkill`.** WMI process creation times for the
installed v1.1.0 app, launched twice (run 35861015549):

| | app started | `taskkill.exe` created | `taskkill.exe` alive | first `jura-sidecar.exe` created |
|---|---|---|---|---|
| Run 1, first launch | +0.00 s | +5.61 s | about 4.1 s | +9.80 s |
| Run 2, warm | +0.00 s | +0.43 s | about 5.1 s | +5.94 s |

The same `taskkill /F /IM jura-sidecar.exe`, run on its own on the same
runner with nothing to kill, takes 0.04 to 0.37 s (four runs). Run by the
app, it lives four to five seconds, and the sidecar is created the moment it
exits. Why it is slow only when the app runs it is not established. What is
established is that the app waits for it.

The mechanism is in `src-tauri/src/startup.rs`. `spawn_sidecar` (`:313`)
calls `kill_orphan_sidecars()` (`:326`) first, and on Windows that runs
`std::process::Command::new("taskkill")...output()` (`:232`), which blocks
until taskkill exits. `spawn_sidecar` is called synchronously from Tauri's
`setup` closure (`src-tauri/src/lib.rs:4279`), and the local REST API is only
spawned at the end of that closure (`lib.rs:4675`). So one blocked child
process delays the sidecar spawn **and** everything after it in setup. The
app's API on port 8300 first accepts a connection at 6.3 s on a warm launch
(run 35860504128) for that reason.

The whole warm-launch sequence, onefile v1.1.0, from the app's own log with
`RUST_LOG=info` plus the creation times above:

| From | To | What |
|---|---|---|
| 0 s | 0.4 s | setup up to `spawn_sidecar` |
| 0.4 s | 5.6 s | `taskkill`, blocking setup |
| 5.9 s | 13.6 s | onefile bootloader extracts, then starts its child (the second `jura-sidecar.exe`) |
| 13.6 s | 15 s | imports, warmup, bind, and the readiness poll's back-off (up to 1.6 s per attempt) |

With onedir the extract row becomes the 3 s measured above, and taskkill is
then the largest single term.

## Two smaller things found on the way

**`/api/v1/health` blocks on the sidecar while holding the state lock.**
`src-tauri/src/api/routes.rs:64-73` locks `AppState` and calls
`SidecarClient::is_available()` (`src-tauri/src/sidecar.rs:1001`), which makes
up to two HTTP attempts with a 10 s timeout each. While the sidecar is down
each call took 4.5 to 6 s in the probe, because a refused connection to
localhost is slow on Windows and it is tried twice. Every other user of the
lock waits behind it. It also means the health endpoint cannot answer a
3-second probe during startup, which is why the first B1 probe saw "app API
up" and "sidecar available" at the same instant and could not tell them
apart. `is_available()` has 22 call sites; whether others hold the lock the
same way was not checked.

**`/health/ready` is a flag that cannot be observed false.** `sidecar/main.py`
sets `application.state.ready = True` at the end of the lifespan warmup
(`:110`), but uvicorn runs the whole lifespan before it binds the socket
(`uvicorn/server.py`, `Server.startup`: `await self.lifespan.startup()` comes
first). The port is closed until the flag is already true. Harmless, but the
comment's model of startup is wrong, and `/health` (2.2 s per call even with
no models loaded, measured in run 35859940447) is what the app's API calls.

## Why it matters

B1 exists to cut the Windows wait. After B1 the wait is roughly taskkill 5 s,
sidecar 3 s, and poll granularity plus the health path for the rest.
Removing the taskkill block is worth about as much again as the extraction
B1 removes, for a few dozen lines, and it also brings the local API up five
seconds sooner.

## What would fix it

1. **Replace the `taskkill` child process with an in-process sweep on
   Windows.** `CreateToolhelp32Snapshot` to find processes named
   `jura-sidecar.exe`, `OpenProcess` plus `TerminateProcess` on each. That is
   milliseconds. `windows-sys` is already in `Cargo.lock` as a transitive
   dependency, so it adds no new supply chain; enable only the features
   needed. Keep the same log lines.
2. **Keep it before the spawn.** Do not "fix" this by moving the kill to a
   background thread: it kills by image name, so a sweep that runs after the
   new sidecar starts kills the new sidecar.
3. **Make `/api/v1/health` not block.** Report the readiness state the
   startup probe already maintains (`SidecarStartupStatus` in `lib.rs`)
   instead of making a live HTTP call under the lock, or at minimum take the
   client out of the lock before calling it.
4. **Measure again with the installer test's B1 step** (PR #107), which
   times launch to `sidecarAvailable` twice. Then tighten its budgets from
   those numbers.

## What not to do

- Do not remove the orphan sweep. It exists because an orphaned sidecar
  holds its files locked and breaks the next install (rc.21, April 2026). The
  B1 launcher's job object covers the launcher dying, but not the app being
  force-killed while the launcher lives.
- Do not read any of these timings as user-machine timings. See below.

## What this does not cover

GitHub's Windows runners ship with Defender real-time protection **off**
(`Get-MpComputerStatus` in run 35859940447). Every number here is therefore
a floor. On a user's machine Defender scans each newly loaded binary, which
is also why B1 signs the whole tree. The Defender cost of the onedir tree has
not been measured anywhere. The sanctioned way is `New-MpPerformanceRecording`
on a machine with real-time protection on.

## Update, 23 September 2026

Fixed on branch `w39-25-startup-fixes`, PR #109, not yet merged. Steps 1 to
3 above are done. The re-measure found a fourth cause. The sidecar's
`/health` asks Ollama at 127.0.0.1:11434 with a 2 s timeout on every call,
and on Windows that took 2.15 s whenever Ollama was not running. The app's
`SidecarClient::is_available()` used `/health` as its "is it up" probe, and
it runs once per verification. It now uses `/health/ready`.

Launch to `sidecarAvailable` on `windows-latest`, B1 port plus the fixes
(run 35867120171): 7.1 s on first launch and 3.7 to 3.8 s warm, against
25.7 to 28.5 s and 17.9 to 24.9 s with the port alone. The sidecar now spawns
0.5 s after launch, down from 5.6 s. Step 4, tightening the installer test's
budgets, waits until both #107 and #109 are merged.
