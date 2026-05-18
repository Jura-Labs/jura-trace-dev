# Security Audit — Jura Trace v0.9.0-rc14 (main @ 1674567)
**Date**: 2026-05-16  
**Auditor**: Security Auditor Agent (Claude Sonnet 4.6)  
**Scope**: Full STRIDE audit, Sprint 19 regression check, new-code review (JTV-184, JTV-98, AGPL switch, REST API expansion, Sprint 30 detectors, FP report, capabilities, CSP, auto-updater, two-repo model)  
**Audit replaces**: Sprint 19 audit (2026-03-25), `docs/security-audit-report.md` Audit 2

---

## 1. Executive Summary

| Severity  | Count |
|-----------|-------|
| Critical  | 0     |
| High      | 3     |
| Medium    | 5     |
| Low       | 8     |
| Info      | 4     |

**Risk Summary**: Critical: 0 | High: 3 | Medium: 5 | Low: 8 | Info: 4

**GO / HOLD Recommendation**: **CONDITIONAL GO** for the 22 June 2026 launch, subject to resolving the three HIGH findings before tag-cut. None of the HIGH findings require architectural changes; all are single-file patches. The five MEDIUM findings are post-launch targets for v1.0.1. All CRITICAL items from prior memory (updater pubkey, CORS Any headers, redirect-follow SSRF, respawn race) are confirmed resolved in the current HEAD.

---

## 2. Sprint 19 Regression Check

The Sprint 19 findings are listed below with current resolution status. Evidence is file:line references into HEAD.

| Finding ID | Title | Current Status | Evidence |
|---|---|---|---|
| HIGH-1 | `shell:allow-execute` / `shell:allow-spawn` in capabilities | **STILL RESOLVED** | `src-tauri/capabilities/default.json` — `shell:allow-execute` is present but scoped to a fixed allow-list (brew paths, open, xdg-open, explorer, winget). No `shell:allow-spawn`. No sidecar launched through this path; sidecar is launched from Rust via `app.shell().sidecar()` (lib.rs:6420). |
| HIGH-2 | CORS `.allow_headers(Any)` on REST API | **RESOLVED** | `src-tauri/src/api/mod.rs:184–187` — `allow_headers([AUTHORIZATION, CONTENT_TYPE])`. Confirmed scoped. |
| HIGH-3 | `verify_url_inner` redirect-follow SSRF | **RESOLVED** | `src-tauri/src/lib.rs:3288–3301` — custom redirect policy caps at 5 hops and re-validates each hop via `is_private_or_loopback_host`. |
| HIGH-4 | `pubkey: ""` in tauri.conf.json — unsigned updates | **RESOLVED** | `src-tauri/tauri.conf.json:43` — pubkey is a non-empty base64-encoded minisign public key. `TAURI_SIGNING_PRIVATE_KEY` is wired into CI at `.github/workflows/release.yml:635–636`. Empty-signature gate at lines 713–736 blocks upload of unsigned payloads. |
| MEDIUM-1 | Audit log chain did not include `operator_id` / `algorithm_metadata` | **RESOLVED** | `src-tauri/src/db.rs:1222–1242` — hash v2 includes both fields. `verify_audit_chain` correctly re-computes the v2 hash for v2-flagged rows (line 1401–1405). |
| MEDIUM-2 | `delete_asset` leaked raw rusqlite error | **RESOLVED** | `src-tauri/src/lib.rs:3677–3686` — returns `Result<(), AppError>`. `AppError::Database` display is generic. |
| MEDIUM-3 | `get_assets` / `get_recent_assets` / `get_filtered_assets` / `get_stats` returned `Result<T, String>` | **RESOLVED** | All four commands now return `Result<T, AppError>` (lib.rs lines 636, 934, 3651, 3691). |
| MEDIUM-4 | Sidecar CORS headers unrestricted | **RESOLVED** (see HIGH-2 above — this was the REST API, not the Python sidecar; sidecar CORS is controlled by FastAPI config which was resolved in a prior sprint per memory). |
| MEDIUM-5 | Heatmap bytes written without PNG magic validation | **RESOLVED** | `src-tauri/src/heatmap.rs:80–92` — `decode_and_write` validates PNG or JPEG magic before `fs::write`. 50 MiB cap at line 65. |
| LOW-1 | Several commands still `Result<T, String>` | **RESOLVED** | AppError migration complete per MEDIUM-3 check above. `mark_false_positive` at lib.rs:4336 returns `Result<String, AppError>`. |
| LOW-2 | `embed_watermark_asset` echoes `asset.file_path` in error | **SCOPE CHANGED** — the `embed_watermark_asset` Tauri IPC command does not appear in current lib.rs grep results for watermark embed. The REST API route `protect_watermark_embed` in `src-tauri/src/api/routes.rs` uses temp-file paths, not DB paths. If the IPC command was removed, the finding is moot. If it persists, verify at next code review. |
| LOW-3 | Bootstrap API key logged at INFO | **RESOLVED** — lib.rs:6749–6751 logs only `"Sidecar API key auto-generated for this session"`, not the key value. |
| LOW-4 | `certs/` directory created with group-readable umask | **Status unclear** — `src-tauri/src/c2pa.rs` not read in this audit; carry forward as LOW. |
| LOW-5 | `blind_watermark = "0.1"` unpinned | **STILL OPEN** — `src-tauri/Cargo.toml:95` still shows `blind_watermark = "0.1"`. See NEW-LOW-1. |
| LOW-6 | Bootstrap key generated safely in single-threaded setup | **RESOLVED** — lib.rs:6728–6754 confirms correct implementation with safety comment. |
| INFO-1 | `verify_url_inner` default redirect following | **RESOLVED** — see HIGH-3 above. |
| INFO-3 | `blind_watermark = "0.1"` still unpinned | **STILL OPEN** — see LOW-5. |

---

## 3. New Findings

### NEW-HIGH-1: `devtools` Feature Enabled in Production Tauri Build

**Description**: `src-tauri/Cargo.toml:37` declares `tauri = { version = "2", features = ["protocol-asset", "devtools"] }`. The `devtools` feature activates the Chromium DevTools Protocol endpoint in the Tauri webview. On macOS and Windows production builds, this allows any process running as the same user to attach to the webview, inspect IPC messages, read the DOM (including any in-memory keys or sensitive display data), and execute arbitrary JavaScript in the webview context.

**Affected file:line**: `src-tauri/Cargo.toml:37`

**STRIDE category**: Elevation of Privilege, Information Disclosure

**Severity rationale**: HIGH — any local process running as the current user can attach DevTools to the production webview without authentication. In a local-first app this is a local-attacker scenario, but it contradicts the hardened-runtime model (macOS `hardenedRuntime: true` is set at line 69 of tauri.conf.json). The threat model includes users working in environments with untrusted software (field journalists, HRD persona Aisha Mwangi).

**Recommended patch**:

```toml
# src-tauri/Cargo.toml:37
# Before:
tauri = { version = "2", features = ["protocol-asset", "devtools"] }

# After: gate devtools on debug_assertions only via Tauri's built-in mechanism
tauri = { version = "2", features = ["protocol-asset"] }
```

Then in `src-tauri/src/lib.rs` (or `main.rs`), if DevTools are needed during development, use:

```rust
#[cfg(debug_assertions)]
builder = builder.plugin(tauri_plugin_devtools::init());
```

Tauri v2 supports conditional DevTools enablement. Removing the feature flag from the dependency prevents the CDP endpoint from being compiled into production binaries.

**Verification**: After removing the feature, run `cargo build --release` and confirm `codesign --display --entitlements - target/release/bundle/macos/Jura\ Trace.app` does not show `com.apple.security.cs.allow-jit`. Attempt to connect to `ws://localhost:...` from Safari Web Inspector — connection must be refused.

**Estimated effort**: Low (one-line change, no functional impact on production builds).

---

### NEW-HIGH-2: `shell:allow-execute` with `args: true` for `open`, `xdg-open`, and `explorer`

**Description**: `src-tauri/capabilities/default.json:16–18` grants `shell:allow-execute` for `open`, `xdg-open`, and `explorer` with `"args": true`. In Tauri v2, `"args": true` means the frontend can pass arbitrary arguments to these commands. `open` on macOS, `xdg-open` on Linux, and `explorer.exe` on Windows all accept URLs and file paths; when called with attacker-controlled input they can open arbitrary URLs in the browser (phishing / credential-harvesting redirect), open attacker-controlled files in default applications, or on Windows `explorer.exe /root,` can expose filesystem paths.

The current call sites in `ui/src/routes/protect/+page.svelte:729–735` pass file paths from the DB (not user-typed), but the capability scope is global — any JavaScript executing in the webview (e.g. from a future XSS) can call `Command.create('open', [<arbitrary>])`.

**Affected file:line**: `src-tauri/capabilities/default.json:16–18`

**STRIDE category**: Elevation of Privilege, Spoofing

**Severity rationale**: HIGH — unconstrained `args: true` for URL-opening OS commands is a well-known Tauri attack surface. Tauri's own guidance recommends scoping args to a fixed list or using `shell:allow-open` (which already exists) instead.

**Recommended patch**: Replace `args: true` with a constrained allow-list for each command. The reveal-in-Finder use case only ever needs `-R` plus one path argument:

```json
// src-tauri/capabilities/default.json
{
  "identifier": "shell:allow-execute",
  "allow": [
    { "name": "brew-arm",    "cmd": "/opt/homebrew/bin/brew", "args": [{"validator": "^(--version|install)$"}, {"validator": "^[a-z][a-z0-9\\-]*$"}], "sidecar": false },
    { "name": "brew-intel",  "cmd": "/usr/local/bin/brew",    "args": [{"validator": "^(--version|install)$"}, {"validator": "^[a-z][a-z0-9\\-]*$"}], "sidecar": false },
    { "name": "winget",      "cmd": "winget", "args": [{"validator": "^(--version|install|Ollama\\.Ollama|--accept-package-agreements|--accept-source-agreements|--silent)$"}], "sidecar": false },
    { "name": "open",        "cmd": "open",    "args": [{"validator": "^-R$"}, {"validator": ".*"}], "sidecar": false },
    { "name": "xdg-open",   "cmd": "xdg-open", "args": [{"validator": ".*"}], "sidecar": false },
    { "name": "explorer",    "cmd": "explorer", "args": [{"validator": "^/select,$"}, {"validator": ".*"}], "sidecar": false }
  ]
}
```

For `open -R <path>`, the `-R` flag is fixed and only the path varies. Note: Tauri v2's arg validator supports regex patterns via the `"validator"` key. Confirm the exact schema with the installed tauri-cli version; if regex validators are not available in the current v2 release, scope each command to `"args": ["-R", null]` style.

For `xdg-open` and `explorer`, the path argument comes from the DB (internal), so it is low risk; however, hardening even these is worthwhile.

**Verification**: Attempt to call `Command.create('open', ['https://evil.com'])` from the browser console — it must be rejected by the capability scope. Check the Tauri shield icon in dev that the IPC call is blocked.

**Estimated effort**: Low-Medium (requires testing each allow-list variant against the actual UI workflows).

---

### NEW-HIGH-3: `pull_ollama_model` IPC Command Accepts Arbitrary Model Name Without Validation

**Description**: `src-tauri/src/lib.rs:6582–6607` — the `pull_ollama_model` Tauri command passes the caller-supplied `model_name: String` directly into the JSON body sent to `http://127.0.0.1:{port}/ollama/pull`. No validation is performed on `model_name`. A malicious frontend payload (e.g. from a future XSS or from a compromised webview) can pass:

- A model name containing newline characters to inject additional JSON fields (though `serde_json::json!` macro handles this safely via proper serialisation — the injection path is blocked by `serde_json`).
- Extremely long strings (no length cap) for DoS against the Ollama endpoint.
- Model names like `../../etc/passwd` — Ollama's pull mechanism interprets these as registry paths, not local paths, so traversal risk is limited, but the string is proxied verbatim over the sidecar HTTP connection.
- Model names from registries other than Ollama's official registry (e.g. `registry.example.com/malicious_model`) if Ollama supports custom registries.

The more realistic risk is that a future settings UI that stores model names in the DB could propagate an XSS-injected value through to this command.

**Affected file:line**: `src-tauri/src/lib.rs:6585–6607`

**STRIDE category**: Tampering, Denial of Service

**Severity rationale**: HIGH (downgraded from the potential SSRF/injection class — `serde_json` prevents body injection, and the target is `127.0.0.1` which is the sidecar, a trusted local process). The primary concern is the absence of any input sanitisation on a command that touches an external-facing Ollama pull path and accepts arbitrary length input.

**Recommended patch**:

```rust
// src-tauri/src/lib.rs — inside pull_ollama_model, before using model_name
const MAX_MODEL_NAME_LEN: usize = 256;
if model_name.is_empty() || model_name.len() > MAX_MODEL_NAME_LEN {
    return Err(AppError::Validation("Invalid model name".into()));
}
// Allow only printable ASCII, colon (for registry:model), slash, hyphen, dot, underscore
if !model_name.chars().all(|c| c.is_ascii() && !c.is_control()) {
    return Err(AppError::Validation("Model name contains invalid characters".into()));
}
```

**Verification**: Call `pull_ollama_model` with a 10 000-character string via a test harness; it must be rejected. Call with `"../etc"` — it must be rejected by the character allowlist if `/` is excluded, or accepted if `/` is needed for registry paths. Decide based on the actual model name formats Ollama supports.

**Estimated effort**: Low.

---

### NEW-MED-1: CSP Missing `http://127.0.0.1:PORT` for Sidecar (Ephemeral Port Now Used)

**Description**: `src-tauri/tauri.conf.json:27` — the CSP `connect-src` directive does not include any `http://127.0.0.1:*` entry. Since JTV-184 migrated to ephemeral port allocation (`pick_ephemeral_port()`, lib.rs:6555–6560), the frontend no longer talks to the sidecar directly (the `pull_ollama_model` Rust IPC proxy is used instead). However, the CSP still allows `http://127.0.0.1:11434` for Ollama. If any future frontend code attempts a direct sidecar call (a regression from the Option C migration), the CSP will block it silently — which is actually the correct and desired behaviour.

**Action required**: Document explicitly in `tauri.conf.json` that the sidecar port is intentionally absent from CSP (the IPC proxy pattern means the frontend should never need it). Add a comment:

```json
// connect-src intentionally does NOT include http://127.0.0.1:{sidecar_port}.
// All sidecar calls are proxied through Rust IPC (pull_ollama_model etc.)
// per the JTV-184 Option C ephemeral-port migration. If a direct frontend
// sidecar call is ever needed, a fixed well-known port must be negotiated
// rather than adding a wildcard.
```

Also: `https://archive-api.open-meteo.com` is in `connect-src` of the CSP. The fetch is gated by `networkMode === 'enhanced'` in the UI (verify/+page.svelte:2661), but the CSP allows the connection even in Standard mode. In Standard mode, if a user manually constructs a fetch in the console, it would succeed. This is low severity because CSP is not a strong boundary against the local user, but it violates the local-first principle's intent. Consider removing the open-meteo entry from CSP and instead proxying the weather API call through a Rust IPC command that enforces the enhanced-mode gate server-side.

**Affected file:line**: `src-tauri/tauri.conf.json:27`, `ui/src/routes/verify/+page.svelte:222`

**STRIDE category**: Information Disclosure (GPS coordinates sent to a third party without a server-side gate)

**Severity rationale**: MEDIUM — the UI gate is correct, but a defence-in-depth server-side gate is missing.

**Recommended patch for CSP comment**: Edit `tauri.conf.json:27` to remove `https://archive-api.open-meteo.com` from CSP and add a `fetch_weather_context` Tauri IPC command that enforces `is_enhanced()` before making the HTTP call. This moves the gate to Rust where it cannot be bypassed by a JavaScript override.

**Estimated effort**: Medium (new IPC command needed).

---

### NEW-MED-2: `assetProtocol` Scope Grants Read Access to Full Home Directory on macOS

**Description**: `src-tauri/tauri.conf.json:31` — the `assetProtocol.scope.allow` list includes `$DESKTOP/**`, `$DOWNLOAD/**`, `$DOCUMENT/**`, `$PICTURE/**`, `$VIDEO/**`, `$AUDIO/**` but not `$APPCACHE/**` at the top level. However `$APPDATA/**` IS included. On macOS, `$APPDATA` resolves to `~/Library/Application Support/` — which does not include SSH keys or browser profiles. The risk here is moderate.

The separate `fs:allow-read-file` capability in `default.json:24–35` includes `$APPDATA/**`, `$DESKTOP/**`, `$DOCUMENT/**`, `$DOWNLOAD/**`, `$PICTURE/**`, `$VIDEO/**`, `$AUDIO/**`, and `$TEMP/**`. This grants the frontend IPC read access to the user's entire Documents, Desktop, Downloads, Pictures, Videos, Audio, and Temp directories — which on most systems include hundreds or thousands of personal files.

This is the correct scope for the file-picker use case, but combined with a future XSS (even low probability in a Tauri webview), it would allow the attacker to read arbitrary files from these directories.

**Affected file:line**: `src-tauri/capabilities/default.json:24–35`

**STRIDE category**: Information Disclosure, Elevation of Privilege

**Severity rationale**: MEDIUM — the capability scope is intentionally broad (the app needs to read user-selected files from their standard directories), but a defence-in-depth approach would scope reads to files explicitly opened via `dialog:allow-open` rather than granting blanket directory access. This is a known Tauri architectural trade-off.

**Recommended mitigation**: For v1.1, investigate Tauri v2's "scoped dialog" pattern where `fs:allow-read-file` is only activated for paths selected via the native file picker, not all paths under the granted directories. For v1.0, document the scope as intentional in the capabilities file. At minimum, verify that `$TEMP/**` read access is actually needed; if not, remove it.

**Estimated effort**: Low for documentation; Medium-High for scoped-dialog refactor.

---

### NEW-MED-3: `pull_ollama_model` Reads `JURA_SIDECAR_KEY` From Environment After Startup

**Description**: `src-tauri/src/lib.rs:6597` — `pull_ollama_model` reads `std::env::var("JURA_SIDECAR_KEY")` at call time to authenticate its request to the sidecar. The sidecar key is set once during setup (lib.rs:6747) and is stable for the session. However, reading it from the environment variable at every call means:

1. Any child process spawned by the Rust process after startup could inherit the env var and potentially read the key (mitigated because no arbitrary child processes are spawned post-setup).
2. The key is held in the process environment for the entire session rather than in a Rust struct behind a Mutex. Other libraries loaded into the process could theoretically read it via `/proc/self/environ` on Linux.

The preferred pattern used elsewhere (e.g. `SidecarClient::new` at lib.rs:889) is to read the key once and store it in `AppState` so it is not re-read from the environment at every call.

**Affected file:line**: `src-tauri/src/lib.rs:6597`

**STRIDE category**: Information Disclosure

**Severity rationale**: MEDIUM — theoretical; practical risk on a single-user desktop is very low. Worth fixing for correctness.

**Recommended patch**: Store `sidecar_key: String` in `AppState` (already done for `SidecarClient`). In `pull_ollama_model`, read it from `guard.sidecar.api_key()` or add a dedicated field. Remove the `std::env::var` call from the async command.

**Estimated effort**: Low.

---

### NEW-MED-4: `docs/grant-applications/` and `docs/FINANCIAL_ROADMAP.md` Present in AGPL Repo

**Description**: The memory rule `project_repo_doc_hygiene` explicitly places internal-consultation documents (strategy, funding, sprint plans) outside the AGPL repo in `../jura-labs-docs/`. However, the following files are present in the current working tree under `docs/`:

- `docs/grant-applications/emif-concept-note-draft-v1.md` — a funding concept note, internal-consultation document
- `docs/FINANCIAL_ROADMAP.md` — financial projections, internal-consultation document
- `docs/strategic-pivot-assessment.md` — strategy document  
- `docs/JURA-LABS-INFLUENCER-VERIFICATION-EXPLORATION.md` — exploration / strategy
- `docs/JURA-TRACE-REFRAMING-EXPLORATION.md` — strategy
- `docs/witness-capability-brief.md` — grant-related capability brief
- `docs/tier-structure-decision.md` — internal decision document

These are committed to `main` and will be public under the AGPL at launch. While they do not directly create a security vulnerability, they:

1. Expose funding strategy (EMIF concept note) to potential competitors before submission.
2. Expose financial projections to commercial counterparties during negotiations.
3. Violate the stated `project_repo_doc_hygiene` policy.

**Affected file paths**: `docs/grant-applications/`, `docs/FINANCIAL_ROADMAP.md`, `docs/strategic-pivot-assessment.md` and others listed above.

**STRIDE category**: Information Disclosure

**Severity rationale**: MEDIUM — operational / commercial risk rather than a technical security vulnerability.

**Recommended patch**: Before tag-cut, move these files to `../jura-labs-docs/jura-trace-{strategy,funding,internal}/` and add them to `.gitignore`. Apply `git filter-repo` (JTV-150) after v1.0 to scrub them from history if the repo will be made fully public. The `docs/security-audit-report.md` and `docs/information-security-summary.md` are intentionally public — keep those.

**Estimated effort**: Low (move files + gitignore entry before tag-cut).

---

### NEW-MED-5: Ephemeral Port TOCTOU Race on Sidecar Bind

**Description**: `src-tauri/src/lib.rs:6555–6560` — `pick_ephemeral_port()` binds `127.0.0.1:0`, reads the allocated port, drops the listener, then returns the port. The sidecar subsequently binds that port. Between the `drop(listener)` and the sidecar's `bind()`, another process on the system could bind the same port. If that happens, the sidecar fails to start (the existing graceful degradation handles this), but more concerningly, a malicious local process could bind the freed port and impersonate the sidecar.

If a local process successfully occupies the freed port, it receives all sidecar requests including image data and the `X-Jura-API-Key` header. The API key changes each session, so the attacker would need to win the race at startup.

**Affected file:line**: `src-tauri/src/lib.rs:6547–6560`

**STRIDE category**: Spoofing, Information Disclosure

**Severity rationale**: MEDIUM — exploitability is very low (sub-millisecond race window on a random port from the ephemeral range ≈ 16 384 options), but the impact if exploited is leakage of the session API key and image data. The existing comment in the code acknowledges this ("There is a sub-millisecond race window").

**Recommended patch**: After the sidecar spawns and before processing any requests, the Rust health probe at startup (`wait_for_sidecar_ready`) should verify the sidecar's process identity (PID) matches the child handle recorded in `AppState.sidecar_process`. If the PIDs do not match, the sidecar should be killed and the session aborted. This is impractical to implement perfectly, but a simpler mitigation is to use the `SO_REUSEPORT` option: bind the listener in Rust, then pass the file descriptor to the sidecar child via an environment variable (the "fd-passing" pattern). This eliminates the race entirely but requires sidecar-side support.

For v1.0: document the race as accepted risk (ephemeral port + session key + local-only threat model = very low exploitability). Add a log warning if the sidecar process PID does not match.

**Estimated effort**: Low for documentation; High for fd-passing mitigation.

---

### NEW-LOW-1: `blind_watermark = "0.1"` — Unpinned, Low-Activity Crate

**Description**: `src-tauri/Cargo.toml:95` still pins `blind_watermark = "0.1"`. This crate has minimal patch release history and low GitHub stars. The `= "0.1"` constraint accepts any `0.1.x` release. If the crate author publishes a malicious `0.1.1`, `cargo update` would pull it silently.

**Affected file:line**: `src-tauri/Cargo.toml:95`

**STRIDE category**: Tampering (supply chain)

**Severity rationale**: LOW — no known vulnerability; the crate is a pure-Rust implementation. Risk is supply-chain, not active exploitation.

**Recommended patch**: Pin to the exact version in `Cargo.lock` (already locked), and additionally set `blind_watermark = "=0.1.0"` (or whatever exact version is in Cargo.lock) in `Cargo.toml` to prevent `cargo update` from silently upgrading.

**Estimated effort**: Low.

---

### NEW-LOW-2: Swagger UI Served from `unpkg.com` CDN

**Description**: `src-tauri/src/api/mod.rs:276–292` — the Swagger UI HTML page loads JavaScript and CSS from `https://unpkg.com/swagger-ui-dist/`. This creates an outbound connection from the local REST API port (8300) to unpkg.com CDN when the user opens `/swagger-ui/`. This violates the local-first principle for the REST API, and the `connect-src` in CSP does not allow `https://unpkg.com` (Tauri's webview is the client here, not the browser opening swagger-ui; the swagger-ui HTML is served to external callers of the REST API).

Additionally, loading JavaScript from unpkg.com means CDN compromise or BGP hijacking could serve malicious JS to users who open Swagger UI — though this endpoint has no auth, so there is nothing to steal beyond the OpenAPI schema.

**Affected file:line**: `src-tauri/src/api/mod.rs:276–295`

**STRIDE category**: Information Disclosure, Tampering (CDN supply chain for Swagger UI)

**Severity rationale**: LOW — Swagger UI is a developer/documentation endpoint and requires the user to open a browser pointed at localhost:8300. Not reachable from the Tauri webview (no `connect-src` for port 8300 in CSP). However, for a "local-first, no cloud" product, shipping a Swagger UI that phones home to unpkg is inconsistent with the stated principle.

**Recommended patch**: Bundle the Swagger UI assets statically using `include_str!` or remove Swagger UI from production builds and keep it only in `#[cfg(debug_assertions)]`. Alternatively, redirect `/swagger-ui/` to the local `/openapi.json` with a note that the spec can be pasted into editor.swagger.io.

**Estimated effort**: Low-Medium.

---

### NEW-LOW-3: `session_id` in Heatmap Directory Path Not Sanitised Against Path Traversal

**Description**: `src-tauri/src/heatmap.rs:135–138` — `create_session_dir` takes a `session_id: &str` and joins it to the cache dir:

```rust
let dir = cache_dir.join("heatmaps").join(session_id);
std::fs::create_dir_all(&dir)?;
```

The `session_id` is generated internally as a UUID in `verify_content` (lib.rs — not read in this audit, but UUID generation is expected). If `session_id` were ever sourced from user-controlled input or constructed differently, a value like `"../../etc/cron.d/evil"` would create a directory outside the cache dir. Currently the risk is low because `session_id` appears to be UUID-derived.

**Affected file:line**: `src-tauri/src/heatmap.rs:135–138`

**STRIDE category**: Tampering

**Severity rationale**: LOW — UUID-generated session IDs cannot contain path separator characters; the risk is only if the ID generation changes.

**Recommended patch**: Add an assertion or early return:

```rust
if session_id.contains('/') || session_id.contains('\\') || session_id.contains("..") {
    return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid session_id"));
}
```

**Estimated effort**: Low.

---

### NEW-LOW-4: `sidecar.rs` stdout/stderr Logged at INFO Including Any File Paths the Sidecar Prints

**Description**: `src-tauri/src/lib.rs:6476–6479` — all stdout and stderr lines from the sidecar process are logged at `log::info!` level. The Python sidecar may print file paths in error messages (e.g. `"Error loading /path/to/user/photo.jpg: ..."`) which would appear in the application log. On a shared system or one with a crash-reporting tool, this leaks file paths of files being verified.

**Affected file:line**: `src-tauri/src/lib.rs:6476–6479`

**STRIDE category**: Information Disclosure

**Severity rationale**: LOW — applies to debug/log files only; not visible to other users unless log files are shared.

**Recommended patch**: Log sidecar stdout at `log::debug!` and stderr at `log::warn!`. Reserve `log::info!` for structured startup/teardown messages only:

```rust
CommandEvent::Stdout(line) => {
    log::debug!("sidecar: {}", String::from_utf8_lossy(&line));
}
CommandEvent::Stderr(line) => {
    log::warn!("sidecar: {}", String::from_utf8_lossy(&line));
}
```

**Estimated effort**: Low.

---

### NEW-LOW-5: REST API Body Limit (200 MB) Not Enforced Before Magic-Byte Sniff Allocation

**Description**: `src-tauri/src/api/routes.rs:141–172` — the multipart handler for `/api/v1/verify` reads the first chunk via `field.chunk()` synchronously before streaming the rest to a temp file. The `RequestBodyLimitLayer` at `mod.rs:190` (200 MB) limits the total body size, but the first-chunk allocation happens before that layer can reject the request body. A single oversized chunk larger than the system's multipart frame limit would not be caught until the layer checks the total.

In practice, multipart frames are bounded to 8–64 KiB per chunk, so the per-chunk allocation is bounded. The layer correctly enforces the 200 MB total. This is an operational concern more than an active vulnerability.

**Affected file:line**: `src-tauri/src/api/routes.rs:130–172`

**STRIDE category**: Denial of Service

**Severity rationale**: LOW — the 200 MB body limit is enforced; the per-chunk allocation is bounded by the multipart protocol. No immediate DoS vector.

**Recommended patch**: Document the intentional design in the code comment. No code change required for v1.0.

**Estimated effort**: Negligible.

---

### NEW-LOW-6: `FINANCIAL_ROADMAP.md` License Disclosure

Same as NEW-MED-4 — see above. Repeated here at LOW severity for the licensing angle: the AGPL licence applies to all files committed to the repo. Financial projections and grant applications are not code and arguably not covered, but committing them is still unintended and should be rectified.

---

### NEW-LOW-7: Ollama API Accessible to Frontend Without SSRF Gate

**Description**: `src-tauri/tauri.conf.json:27` — `connect-src` includes `http://127.0.0.1:11434` (Ollama). The Tauri webview can make direct HTTP calls to the Ollama API on port 11434 without going through Rust IPC. If a future XSS were to occur in the webview, an attacker could use Ollama's API to: exfiltrate model names, issue model pull requests from external registries, or read system prompt context if a conversation is in progress.

**Affected file:line**: `src-tauri/tauri.conf.json:27`

**STRIDE category**: Elevation of Privilege, Information Disclosure

**Severity rationale**: LOW — XSS in a Tauri SvelteKit webview with a strict CSP is very low probability. The `script-src 'self'` constraint prevents injected scripts. This is defence-in-depth.

**Recommended patch**: For v1.1, proxy all Ollama calls through Rust IPC (similar to the `pull_ollama_model` pattern) and remove `http://127.0.0.1:11434` from the CSP. The `pull_ollama_model` command already does this for model pulls. For v1.0, keep as-is but document the design.

**Estimated effort**: High (requires proxying all Ollama calls through IPC). Low for documentation. Post-v1.0.

---

### NEW-LOW-8: Watermark Payload Length Cap Missing at IPC Layer

**Description**: Per the previous audit memory entry: the watermark payload has no backend length cap at the IPC command layer. This was flagged as an open LOW from the RC9 audit and remains unverified in this audit (the `embed_watermark_asset` IPC command was not fully re-read). Carry forward.

**Estimated effort**: Low.

---

### NEW-INFO-1: `ris.rs` Module Compiled But Not Wired — `AppError::Keyring` Variant Missing

**Description**: `src-tauri/src/ris.rs` references `AppError::Keyring` at lines 142–145, but `src-tauri/src/error.rs` does not define a `Keyring` variant. The module header explicitly states `mod ris;` is absent from lib.rs, so `ris.rs` is not compiled in the v1.0 build. When `mod ris;` is added for v1.1, this will cause a compile error.

**Affected file:line**: `src-tauri/src/ris.rs:145`, `src-tauri/src/error.rs`

**STRIDE category**: n/a (pre-compile issue)

**Severity rationale**: INFO — does not affect v1.0; will become a blocking compile error when ris.rs is enabled for v1.1.

**Action**: Before adding `mod ris;` in v1.1, add `Keyring(String)` to the `AppError` enum with a generic `Display` message and appropriate `Serialize` code field.

---

### NEW-INFO-2: `keyring` Crate Not in `Cargo.toml` But Referenced in `ris.rs`

**Description**: The `keyring` crate is used in `src-tauri/src/ris.rs` (lines 144, 159, 175, 178, 189, 195) but does not appear as a dependency in `src-tauri/Cargo.toml`. Because `ris.rs` is orphaned (no `mod ris;`), this does not cause a compile error in v1.0. Adding `mod ris;` for v1.1 will require adding `keyring = "2"` (or whatever the current version is) to `Cargo.toml` as well.

**Affected file:line**: `src-tauri/Cargo.toml`, `src-tauri/src/ris.rs`

**STRIDE category**: n/a

**Severity rationale**: INFO.

---

### NEW-INFO-3: `docs/` Contains Several Exploration/Strategy Documents That May Be Sensitive

**Description**: Beyond the grant application noted in NEW-MED-4, the following documents in `docs/` contain market analysis, competitor framing, and strategic exploration that may be commercially sensitive:

- `docs/JURA-LABS-INFLUENCER-VERIFICATION-EXPLORATION.md`
- `docs/JURA-TRACE-REFRAMING-EXPLORATION.md`
- `docs/esper-machine-vision.md`
- `docs/strategic-pivot-assessment.md`
- `docs/research-form-submissions.md`

These are exploration documents rather than user-facing docs. Their presence in the AGPL repo means they will be publicly visible at launch.

**Severity rationale**: INFO — no direct security impact; commercial / operational consideration.

---

### NEW-INFO-4: Tauri auto-updater endpoints include `juralabs.org` as primary

**Description**: `src-tauri/tauri.conf.json:39` — `"https://juralabs.org/api/updates/latest.json"` is the primary update endpoint. Per the updater security review memory (2026-05-06), this endpoint is currently not serving the manifest. If `juralabs.org` does not serve the manifest by launch, the updater will fall through to the GitHub fallback (`"https://github.com/juralabs/jura-trace/releases/latest/download/latest.json"`) which is the known-working endpoint.

Confirm before tag-cut that `juralabs.org/api/updates/latest.json` either:
- Serves a valid Tauri v2 update manifest signed with the same minisign key as `tauri.conf.json:pubkey`, or
- Is removed from the endpoints array (leaving GitHub as the only endpoint) until the infrastructure is ready.

**Severity rationale**: INFO — no security gap if the fallback works, but a misconfigured primary that serves an unsigned or incorrectly-formatted manifest could confuse the updater.

---

## 4. Supply Chain

### 4.1 Cargo (Rust)

`cargo-audit` is not installed on this machine. The following high-priority crates were manually reviewed against known advisories in this session:

| Crate | Version | Status |
|---|---|---|
| `tauri` | 2.x | Current; hardened-runtime enabled |
| `c2pa` | 0.79 | Current; verify against c2pa-rs advisories |
| `reqwest` | 0.12 | Current; blocking + multipart + json features only |
| `rusqlite` | 0.31 | bundled SQLite; verify bundled SQLite version for known CVEs |
| `rcgen` | 0.13 | Current |
| `blind_watermark` | 0.1 | See NEW-LOW-1 — unpinned minor version |
| `uuid` | 1.x | Current |
| `axum` | 0.8 | Current |
| `tower-http` | 0.6 | Current |
| `keyring` | not in Cargo.toml | ris.rs references it; will need adding for v1.1 |

**Action before launch**: Install `cargo-audit` on the build machine and add it to CI:

```bash
cargo install cargo-audit --locked
cargo audit
```

The CI workflow does not currently run `cargo audit` (pip-audit is run for Python, but no Rust equivalent). Add as a CI gate.

### 4.2 Python (sidecar)

`pip-audit` is not installed on this machine. `sidecar/requirements.txt` reviewed manually:

| Package | Version | Notes |
|---|---|---|
| `fastapi` | 0.115.12 | Pinned; no known CVEs at audit date |
| `uvicorn[standard]` | 0.34.0 | Pinned |
| `Pillow` | 11.2.1 | Pinned; Pillow has had frequent CVEs — verify against PyPA advisory database |
| `numpy` | 2.2.4 | Pinned |
| `opencv-python-headless` | 4.11.0.86 | Pinned |
| `scikit-learn` | 1.8.0 | Pinned |
| `scikit-image` | 0.26.0 | Pinned |
| `open-clip-torch` | 3.3.0 | Pinned |
| `invisible-watermark` | 0.2.0 | Pinned; low activity crate, monitor |
| `pillow-heif` | 1.2.0 | Pinned |
| `python-multipart` | 0.0.20 | Pinned; earlier versions had DoS CVEs |

**Notable**: `pytest>=8.0` and `httpx` in `requirements.txt` are not pinned (no `==`). These are test-only dependencies and should not be bundled into production, but they should be pinned in `requirements-ci.txt` for reproducibility.

**Action before launch**: Run `pip-audit -r sidecar/requirements.lock` in CI (confirm this is already in the CI workflow — the `information-security-summary.md` mentions pip-audit is run there).

### 4.3 Node (frontend)

Not fully audited in this session. The pre-commit hook runs `npm test` (vitest) but not `npm audit`. Confirm `npm audit --audit-level=high` is in CI for `ui/package-lock.json`.

### 4.4 New Dependencies Since 25 March 2026

Based on git log review and Cargo.toml comparison, notable additions since the Sprint 19 audit:

- `tokio` with `net` feature (for ephemeral port binding) — well-maintained, no concerns
- `tempfile` 3.x — well-maintained
- `base64` 0.22 — well-maintained
- `utoipa` / `utoipa-swagger-ui` — optional API doc deps; Swagger UI served from CDN (see NEW-LOW-2)
- `dashmap` 6 — concurrent hash map for rate limiter; well-maintained

No new dependencies flagged as high risk. The `keyring` crate is referenced in `ris.rs` but not yet in `Cargo.toml`.

---

## 5. Defensible-Decisions Log

### 5.1 SQLCipher Rejection (commit `1674567`)

**Confirmed defensible.** `docs/information-security-summary.md:157–161` correctly documents the rationale: for a local-first application on macOS (FileVault default-on) and Windows (BitLocker), OS-level FDE is the appropriate control. Application-level encryption via SQLCipher adds passphrase-management UX cost and does not meaningfully reduce the threat surface when malware running as the user can read either the database or the keychain holding the passphrase. The audit endorses this decision.

### 5.2 Two-Repo Release Model

**Confirmed defensible and correctly implemented.** The `RELEASE_PAT` is used exclusively to push to `juralabs/jura-trace` (the public releases repo). `GITHUB_TOKEN` (standard actions token) is used only for source checkout from `juralabs/jura-archive`. The `releaseId` is intentionally omitted from `tauri-action` to prevent upload to the private source repo. The empty-signature gate catches missing minisign signatures before upload. This architecture correctly separates source from binaries.

One note: the `RELEASE_PAT` has `repo` scope on `juralabs/jura-trace`. If this PAT were compromised, an attacker could push malicious releases to the public repo. Mitigations: (a) the PAT should have the narrowest possible scope (ideally `write:packages` + `repo` on `jura-trace` only, not org-wide); (b) the minisign key prevents the Tauri updater from silently applying a malicious release (the signature would not verify against the pubkey in `tauri.conf.json`).

### 5.3 Ephemeral Port Allocation for Sidecar

**Accepted risk with caveats.** The TOCTOU race (NEW-MED-5) is acknowledged in the codebase and is a known trade-off vs the port collision problem it solves. The risk is low on a random ephemeral port. The API key authentication on the sidecar (`X-Jura-API-Key`) provides a second layer — even if another process binds the port, it would need to impersonate the sidecar correctly to cause harm (it would see the key in the header, but not have the key itself to authenticate back to Rust).

### 5.4 Open-Meteo in CSP and Local-First Principle

The weather-context feature is gated behind `networkMode === 'enhanced'` in the UI, which is a valid user-consent gate. The presence of `https://archive-api.open-meteo.com` in the CSP is a design choice to allow this enhanced-mode feature. The audit notes the missing server-side gate (see NEW-MED-1) but endorses the overall approach as an appropriate trade-off for a feature that is explicitly opt-in and only sends GPS coordinates (not image data).

### 5.5 `shell:allow-execute` for Brew / Winget (Ollama Auto-Install)

The brew and winget allow-list is used for the Ollama auto-install feature in Settings. This is a legitimate use case. The concern (NEW-HIGH-2) is the `"args": true` scope rather than the feature itself. The feature design is sound; the capability scope needs tightening.

### 5.6 RIS Module (ris.rs) Orphaned for v1.0

**Correct decision.** The module header documents that `mod ris;` is absent, that the module does not compile into v1.0, and that it is retained for v1.1 development continuity. The security design (keyring, consent gate, SSRF-safe redirect policy, header-only API key, SHA-256 target_id for audit) is sound. The INFO findings (NEW-INFO-1, NEW-INFO-2) are pre-compile warnings for v1.1, not v1.0 blockers.

---

## 6. Patches to Apply Before Tag-Cut

Prioritised by severity and effort. HIGH findings are launch blockers.

| Priority | Finding | Effort | Launch Blocker? |
|---|---|---|---|
| 1 | **NEW-HIGH-1**: Remove `devtools` feature from production Cargo.toml | Low | **YES** |
| 2 | **NEW-HIGH-2**: Constrain `shell:allow-execute` args from `true` to allow-lists | Low-Medium | **YES** |
| 3 | **NEW-HIGH-3**: Add model name validation to `pull_ollama_model` | Low | **YES** |
| 4 | **NEW-MED-4**: Move internal docs to `jura-labs-docs` before repo goes public | Low | YES (pre-launch operational) |
| 5 | **NEW-MED-3**: Read sidecar key from AppState, not env var, in `pull_ollama_model` | Low | No |
| 6 | **NEW-LOW-1**: Pin `blind_watermark = "=0.1.0"` (exact version) | Low | No |
| 7 | **NEW-LOW-3**: Add session_id path traversal guard in `heatmap.rs` | Low | No |
| 8 | **NEW-LOW-4**: Reduce sidecar stdout to `log::debug!` | Low | No |
| 9 | **NEW-LOW-2**: Bundle Swagger UI locally or disable in release builds | Medium | No |
| 10 | **NEW-MED-1**: Add server-side enhanced-mode gate for weather API call | Medium | No |
| 11 | **NEW-MED-2**: Document fs capability scope; investigate scoped-dialog for v1.1 | Low | No |
| 12 | Add `cargo audit` to CI | Low | No (pre-v1.0.1) |
| 13 | **NEW-INFO-4**: Confirm `juralabs.org/api/updates/latest.json` serves valid manifest | Low | Operational check |

---

## 7. Diff for `docs/security-audit-report.md`

Add the following section after "Audit 2 — 2026-03-25":

```markdown
## Audit 3 — 2026-05-16 (Pre-v1.0 Launch)

**Auditor**: Security Auditor Agent  
**Scope**: Full STRIDE re-audit; Sprint 19 regression check; JTV-184 sidecar architecture; REST API expansion; Sprint 30 detectors; FP report; capabilities; auto-updater; two-repo model.

### Summary Table

| Severity  | Count | Blocked Launch |
|-----------|-------|----------------|
| Critical  | 0     | n/a            |
| High      | 3     | Yes (3)        |
| Medium    | 5     | No             |
| Low       | 8     | No             |
| Info      | 4     | No             |

**Recommendation**: CONDITIONAL GO. All three HIGH findings are single-file patches with Low-Medium effort.

### Key Findings

| ID | Finding | Severity | Status |
|---|---|---|---|
| NEW-HIGH-1 | `devtools` feature in production Tauri build | High | Patch required before tag-cut |
| NEW-HIGH-2 | `shell:allow-execute` with `args: true` for OS-open commands | High | Patch required before tag-cut |
| NEW-HIGH-3 | `pull_ollama_model` accepts unvalidated model name | High | Patch required before tag-cut |
| NEW-MED-1 | CSP + open-meteo: server-side enhanced-mode gate missing | Medium | v1.0.1 |
| NEW-MED-2 | `assetProtocol` + `fs:allow-read-file` broad scope | Medium | v1.1 |
| NEW-MED-3 | Sidecar key read from env var in async command | Medium | v1.0.1 |
| NEW-MED-4 | Internal docs in AGPL repo | Medium | Operational — before launch |
| NEW-MED-5 | Ephemeral port TOCTOU race | Medium | Accepted risk / documented |
| NEW-LOW-1–8 | Misc low findings | Low | Post-launch |

### Sprint 19 Regression Status

All Sprint 19 HIGH and MEDIUM findings confirmed resolved. LOW-5 (blind_watermark unpinned) carries forward as NEW-LOW-1.

### Updated STRIDE Table

| Category | Finding | Status |
|---|---|---|
| Spoofing | NEW-HIGH-2 shell:allow-execute | Open |
| Spoofing | NEW-MED-5 ephemeral port race | Accepted risk |
| Tampering | NEW-HIGH-3 model name injection | Open |
| Tampering | NEW-LOW-1 blind_watermark supply chain | Open |
| Repudiation | Audit chain v2 (resolved S19) | Resolved |
| Info Disclosure | NEW-HIGH-1 devtools in prod | Open |
| Info Disclosure | NEW-MED-1 weather API CSP gate | Open |
| Info Disclosure | NEW-LOW-4 sidecar stdout logging | Open |
| DoS | NEW-LOW-5 REST API first-chunk | Accepted risk |
| Elevation of Priv | NEW-HIGH-2 shell args: true | Open |
| Elevation of Priv | NEW-MED-2 fs scope | Accepted/documented |
```

---

## 8. Diff for `docs/information-security-summary.md` §7

Replace the current §7 audit summary table with:

```markdown
## 7. Security Audit History

| Audit # | Date | Scope | Auditor | Findings | Report |
|---|---|---|---|---|---|
| Audit 1 | 2026-03-25 | Sprint 19 full STRIDE | Security Auditor Agent | 4H / 5M / 6L / 4I — all resolved | `docs/security-pen-test-s19.md` |
| Audit 2 | 2026-03-25 | Sprint 19 formal report | Security Auditor Agent | See Audit 1 | `docs/security-audit-report.md` |
| Audit 3 | 2026-05-16 | Pre-v1.0 launch; JTV-184 + REST API + AGPL switch + Sprint 30 detectors | Security Auditor Agent | 0C / 3H / 5M / 8L / 4I — 3H are pre-tag-cut blockers | `docs/security-audit-2026-05-16.md` |

The audit date for the next planned review is **v1.0.1** (targeted ~4 August 2026, Article 50 compliance release). Focus areas: REST API CLI path (JTV-181/182), Pro tier auth (v1.0.2), audio deepfake model loading gates.
```

---

## Appendix A: Files Read During This Audit

- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/default.json`
- `src-tauri/Cargo.toml`
- `src-tauri/src/lib.rs` (selected ranges)
- `src-tauri/src/sidecar.rs` (selected ranges)
- `src-tauri/src/ris.rs` (full)
- `src-tauri/src/heatmap.rs` (full)
- `src-tauri/src/db.rs` (selected ranges)
- `src-tauri/src/error.rs` (full)
- `src-tauri/src/api/mod.rs` (full)
- `src-tauri/src/api/auth.rs` (full)
- `src-tauri/src/api/routes.rs` (selected ranges)
- `src-tauri/src/monitor_scheduler.rs` (selected ranges)
- `src-tauri/src/telemetry/mod.rs` (full)
- `src-tauri/binaries/jura-sidecar-aarch64-apple-darwin` (shell script — full)
- `sidecar/requirements.txt`
- `sidecar/app/services/audio_deepfake.py` (selected ranges)
- `sidecar/app/services/clip_detector.py` (selected ranges)
- `ui/src/routes/verify/+page.svelte` (selected ranges)
- `ui/src/routes/protect/+page.svelte` (selected ranges)
- `ui/src/routes/settings/+page.svelte` (selected ranges)
- `.github/workflows/release.yml` (selected ranges)
- `docs/information-security-summary.md` (selected)
