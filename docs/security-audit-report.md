# Security Audit Report — Jura Trace

**Scope**: Full application security review
**Date**: 2026-03-21
**Auditor**: Security Auditor Agent
**Version audited**: 0.4.0 (Phase 2 complete)
**Risk Summary**: Critical: 3 | High: 6 | Medium: 7 | Low: 5

---

## Executive Summary

Jura Trace is a local-first Tauri v2 desktop application that handles sensitive digital assets for cultural institutions. The overall security posture is reasonable for a pre-release application — the Rust backend uses parameterised queries throughout, no `@html` injection is present in the frontend, and the sidecar correctly binds to `127.0.0.1`. However, three critical issues require immediate remediation before any public release: an unrestricted URL scheme in the `verify_url` command enables SSRF attacks against internal services; the Tauri filesystem capability grants whole-filesystem read and write access with no directory scoping; and the CSP `connect-src` allows connections to any `localhost:*` port, undermining the local-first isolation boundary.

---

## Findings

---

### CRITICAL Findings

---

#### [CRITICAL-1] Unrestricted URL scheme in `verify_url` — SSRF vulnerability

**File**: `src-tauri/src/lib.rs`, line 1197
**STRIDE category**: Elevation of Privilege, Information Disclosure

**Description**
The `verify_url` Tauri command accepts a URL string from the frontend and immediately issues an HTTP request with no validation of the scheme, host, or port. An attacker who can influence the URL value — including the legitimate local user submitting a crafted input — can direct the Rust HTTP client to issue requests to any address reachable from the machine.

**Attack scenario**
A user (or a malicious file that manipulates the UI) submits a URL such as `http://127.0.0.1:11434/api/generate` to the verify pipeline. The Rust backend will faithfully issue a POST-equivalent GET to the Ollama API or any other locally-bound service (router admin interface, local Kubernetes API, metadata services in cloud environments). The response body is propagated back through the `VerificationResult` error path, potentially disclosing internal data. In a cloud/CI environment this is a full SSRF.

```rust
// src-tauri/src/lib.rs, line 1197 — no validation before request:
let response = reqwest::blocking::Client::new()
    .get(&url)           // <-- url is unsanitised frontend input
    .timeout(std::time::Duration::from_secs(30))
    .send()
    ...
```

**Recommended fix**
Parse the URL and reject anything that is not `http://` or `https://` pointing at a public (non-loopback, non-RFC-1918) address before issuing the request.

```rust
fn validate_verify_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url)
        .map_err(|_| "Invalid URL format".to_string())?;

    match parsed.scheme() {
        "http" | "https" => {}
        s => return Err(format!("Unsupported URL scheme: {s}")),
    }

    let host = parsed.host_str()
        .ok_or("URL has no host")?;

    // Block loopback and private ranges
    let blocked_hosts = ["localhost", "127.0.0.1", "::1", "0.0.0.0"];
    if blocked_hosts.contains(&host) {
        return Err("Verification of localhost URLs is not permitted".to_string());
    }

    // Block RFC-1918 private ranges (basic check — use ip_network crate for rigour)
    if host.starts_with("192.168.") || host.starts_with("10.")
        || host.starts_with("172.16.") || host.starts_with("169.254.")
    {
        return Err("Verification of private-network URLs is not permitted".to_string());
    }

    Ok(())
}
```

Add `url = "2"` to `Cargo.toml` and call `validate_verify_url(&url)?` at the start of `verify_url`.

**Verification**: Attempt to submit `http://127.0.0.1:8200/health` as a URL to verify; the command must return an error, not a forensic result.

---

#### [CRITICAL-2] Tauri filesystem capability grants unrestricted read and write access

**File**: `src-tauri/capabilities/default.json`, lines 11–12
**STRIDE category**: Elevation of Privilege, Information Disclosure

**Description**
The capability file grants `fs:allow-read` and `fs:allow-write` without any scope constraints. In Tauri v2, unscoped filesystem permissions grant the webview read and write access to the **entire filesystem** accessible to the process user. The frontend can therefore call Tauri's `fs` plugin to read arbitrary files (SSH keys, browser credentials, environment files) or overwrite any file the OS user owns.

```json
// src-tauri/capabilities/default.json — no scope defined:
"fs:allow-read",
"fs:allow-write"
```

**Attack scenario**
A persistent XSS bug in the frontend (or a future dependency introducing one) would immediately escalate to full filesystem access. Even without XSS, any Svelte component that passes attacker-controlled paths to the `fs` plugin API would expose the whole filesystem.

**Recommended fix**
Replace the broad permissions with scoped variants that confine access to the application data directory and the user's home directory for file-open operations.

```json
{
  "permissions": [
    "core:default",
    "shell:allow-open",
    "dialog:allow-open",
    "dialog:allow-save",
    {
      "identifier": "fs:allow-read",
      "allow": [
        { "path": "$APPDATA/**" },
        { "path": "$HOME/**" }
      ]
    },
    {
      "identifier": "fs:allow-write",
      "allow": [
        { "path": "$APPDATA/**" }
      ]
    }
  ]
}
```

Note: the `$HOME/**` read scope is broad but appropriate for a tool that analyses user files. Writing should be confined to app data only; the sign and watermark commands derive output paths from the source path and write to the same directory, so `$DOCUMENT/**` may also be required.

**Verification**: Attempt to read `/etc/passwd` (or `C:\Windows\System32\config\SAM`) via the Tauri `fs` plugin from the webview; the call must be blocked.

---

#### [CRITICAL-3] CSP `connect-src` allows connections to any localhost port

**File**: `src-tauri/tauri.conf.json`, line 25
**STRIDE category**: Elevation of Privilege, Information Disclosure

**Description**
The Content Security Policy uses `connect-src 'self' http://localhost:*`. The wildcard port allows webview JavaScript to issue `fetch()` or `XMLHttpRequest` calls to any port on localhost, bypassing the local-first principle. This includes the Ollama API (11434), any local database admin tools, development servers, or other sensitive local services.

```json
// src-tauri/tauri.conf.json, line 25:
"csp": "default-src 'self'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; connect-src 'self' http://localhost:*"
```

**Attack scenario**
A script injected via a dependency or XSS can exfiltrate data to `http://localhost:11434/api/chat` or probe any locally bound service. Additionally, `data:` in `img-src` allows arbitrary `data:` URI images to be rendered, which has historically been exploited for data exfiltration via CSS timing attacks.

**Recommended fix**
Pin `connect-src` to exact ports and remove `data:` from `img-src`:

```json
"csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' blob: asset: https://asset.localhost; connect-src ipc: http://ipc.localhost http://127.0.0.1:8200 http://127.0.0.1:11434; object-src 'none'; base-uri 'self';"
```

Rationale for each directive:
- `data:` removed from `img-src` — the app renders heatmap images via `blob:` objects created from base64, so `data:` is not required in production.
- `connect-src` pinned to IPC, sidecar port 8200, and Ollama port 11434 only.
- `object-src 'none'` blocks Flash/plugin execution.
- `base-uri 'self'` prevents `<base>` tag hijacking.

**Verification**: Open browser devtools in the webview, attempt `fetch('http://localhost:3000')`, confirm it is blocked.

---

### HIGH Findings

---

#### [HIGH-1] No file size validation before memory-loading files in the Rust import pipeline

**File**: `src-tauri/src/lib.rs`, lines 220–244
**STRIDE category**: Denial of Service

**Description**
`import_files` reads file metadata to obtain the size (line 220), but this value is only stored in the database — it is never used to gate whether the file is loaded into memory for EXIF extraction or fingerprinting. Image libraries (`kamadak-exif`, `image`) will attempt to decode arbitrarily large files. A crafted 4 GB TIFF with valid magic bytes would cause the process to allocate several gigabytes.

There is also no maximum dimension check before calling `metadata::get_image_dimensions`, which decodes the image header. A decompression-bomb PNG (e.g. 1×1 pixel compressed to 45 MB that expands to 50,000×50,000) would exhaust memory.

**Recommended fix**

```rust
const MAX_FILE_SIZE_BYTES: u64 = 200 * 1024 * 1024; // 200 MB

let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
if file_size > MAX_FILE_SIZE_BYTES {
    log::warn!("Skipping oversized file ({}): {} bytes", path_str, file_size);
    continue;
}
```

For decompression bombs, use the `image` crate's dimension-only reader before full decode:

```rust
// Check dimensions before full decode
use image::io::Reader as ImageReader;
let reader = ImageReader::open(&path).map_err(|e| e.to_string())?;
let (w, h) = reader.into_dimensions()
    .map_err(|e| format!("Failed to read image dimensions: {e}"))?;
if w > 20_000 || h > 20_000 {
    return Err(format!("Image dimensions {}x{} exceed maximum allowed size", w, h));
}
```

**Verification**: Attempt to import a crafted 1×1 PNG that decompresses to > 20,000 pixels; the file should be rejected with an error.

---

#### [HIGH-2] `verify_content` accepts arbitrary file paths with no canonicalisation

**File**: `src-tauri/src/lib.rs`, lines 534–538
**STRIDE category**: Tampering, Information Disclosure

**Description**
The `verify_content` and `verify_url` (via temp file path) commands accept a `source` string parameter and construct a `PathBuf` directly from it without canonicalisation or traversal checks.

```rust
fn verify_content_inner(source: &str, ...) -> Result<VerificationResult, String> {
    let path = std::path::PathBuf::from(source);  // no validation
    if !path.exists() {
        return Err(format!("File not found: {source}"));  // leaks path in error
    }
```

While Tauri's IPC mechanism provides some sandboxing, a path such as `../../etc/passwd` or `/proc/self/mem` submitted via the `verify_content` command would be processed and the error message would confirm or deny path existence. Combined with the broad `fs:allow-read` capability, this could allow filesystem probing.

**Recommended fix**
Canonicalise paths and validate they fall within expected directories:

```rust
fn validate_file_path(source: &str) -> Result<std::path::PathBuf, String> {
    let path = std::path::PathBuf::from(source);

    // Canonicalise resolves symlinks and ../sequences
    let canonical = path.canonicalize()
        .map_err(|_| "File not found or inaccessible".to_string())?;  // don't echo path

    // Ensure no null bytes in the original string
    if source.contains('\0') {
        return Err("Invalid path: contains null byte".to_string());
    }

    Ok(canonical)
}
```

Note the error message deliberately omits the path to avoid information disclosure.

**Verification**: Submit `../../etc/passwd` as the source path; confirm the error message does not confirm whether the file exists.

---

#### [HIGH-3] Temporary files for URL verification are not securely created (fixed suffix leaks MIME guess)

**File**: `src-tauri/src/lib.rs`, lines 1234–1237
**STRIDE category**: Information Disclosure, Tampering

**Description**
The `verify_url` function writes downloaded content to a temp file with a predictable name pattern `url_content.{ext}`. The `ext` value is derived from the URL or `Content-Type` header — both attacker-controlled — and is interpolated directly into the filename with no length or character validation. An attacker-controlled extension of `../../../home/user/.bashrc` would create a file outside the temp directory (path traversal via extension).

Additionally, the `tempfile::tempdir()` directory is dropped (deleted) at the end of scope, but only if `verify_content_inner` returns before the `TempDir` is consumed. If a panic occurs during the verify pipeline, `TempDir`'s `Drop` implementation still cleans up, but the file path has already been passed to `verify_content_inner` as a string — meaning the cleanup cannot be guaranteed if the inner function holds a reference past the directory's lifetime.

**Recommended fix**

```rust
// Sanitise the extension to alphanumeric only, max 6 chars
let safe_ext: String = ext
    .chars()
    .filter(|c| c.is_alphanumeric())
    .take(6)
    .collect();
let safe_ext = if safe_ext.is_empty() { "bin".to_string() } else { safe_ext };

// Use NamedTempFile instead of manual path construction
let mut temp_file = tempfile::Builder::new()
    .prefix("jura_url_")
    .suffix(&format!(".{safe_ext}"))
    .tempfile()
    .map_err(|e| format!("Failed to create temp file: {e}"))?;

std::io::Write::write_all(&mut temp_file, &bytes)
    .map_err(|e| format!("Failed to write temp file: {e}"))?;

let temp_path = temp_file.path().to_path_buf();
```

**Verification**: Submit a URL where the server returns `Content-Type: image/png/../../../evil`; confirm the temp file is created with a sanitised extension.

---

#### [HIGH-4] `get_filtered_assets` constructs a SQL query with dynamically assembled WHERE clause

**File**: `src-tauri/src/db.rs`, lines 393–422
**STRIDE category**: Tampering

**Description**
The `get_filtered_assets` function builds a SQL query by assembling a `WHERE` clause string at runtime and concatenating it into the full query string. Although the *parameter values* are correctly bound via rusqlite's parameterised query system, the *condition strings themselves* (e.g., `"content_type = ?1"`) are constructed by string interpolation of `param_values.len() + 1`. The search query value uses a `LIKE '%{query}%'` pattern where `query` is the user-supplied search string.

While the `%` wildcards are wrapped in the parameter binding (line 408: `format!("%{query}%")`), a user supplying `%` or `_` characters in the search box will cause unintended wildcard expansion in the LIKE clause (LIKE injection, not SQL injection, but still a correctness and DoS concern).

**Recommended fix**
Escape LIKE special characters before interpolating into the pattern:

```rust
if let Some(query) = search_query {
    if !query.is_empty() {
        // Escape LIKE metacharacters to prevent wildcard injection
        let escaped = query.replace('\\', "\\\\")
                           .replace('%', "\\%")
                           .replace('_', "\\_");
        let n = param_values.len() + 1;
        conditions.push(format!("(file_name LIKE ?{n} ESCAPE '\\' OR mime_type LIKE ?{n} ESCAPE '\\')"));
        param_values.push(Box::new(format!("%{escaped}%")));
    }
}
```

**Verification**: Search for `%` in the asset library; confirm it does not return all assets.

---

#### [HIGH-5] Python sidecar video/audio endpoints bypass the `_read_and_validate` size check

**File**: `sidecar/app/api/forensics.py`, lines 454–473
**STRIDE category**: Denial of Service

**Description**
The `/video/metadata`, `/audio/metadata`, and `/video/frames` endpoints call `await file.read()` directly without passing through the `_read_and_validate` helper that enforces the `settings.max_image_size` (20 MB) limit. Video files can easily be hundreds of megabytes or gigabytes. A user submitting a 2 GB video to the sidecar would cause the sidecar process to exhaust memory loading the full file before writing it to a temp file.

```python
# forensics.py, line 454 — no size check:
async def video_metadata(file: UploadFile = File(...)) -> VideoMetadataResponse:
    contents = await file.read()  # unbounded read
```

**Recommended fix**

```python
MAX_VIDEO_SIZE = 500 * 1024 * 1024  # 500 MB

async def _read_media(file: UploadFile, max_size: int = MAX_VIDEO_SIZE) -> bytes:
    contents = await file.read()
    if len(contents) == 0:
        raise HTTPException(status_code=400, detail="Empty file uploaded")
    if len(contents) > max_size:
        raise HTTPException(
            status_code=413,
            detail=f"File too large ({len(contents)} bytes). Max: {max_size}",
        )
    return contents
```

Apply `_read_media` to the three video/audio endpoints.

**Verification**: Upload a file larger than 500 MB to `/forensics/video/metadata`; confirm HTTP 413 is returned.

---

#### [HIGH-6] C2PA private key stored unprotected in the app data directory

**File**: `src-tauri/src/c2pa.rs`, lines 52–63
**STRIDE category**: Spoofing, Repudiation

**Description**
The C2PA signing key is written as a PEM file to `<AppData>/certs/jura_key.pem` with no encryption or passphrase protection. Any process running as the same OS user can read this key. In a shared workstation environment, this allows another user's process (via privilege escalation) or malware to sign arbitrary files under the institution's identity.

The key is self-signed and therefore trust is inherently limited, but it still undermines the repudiation property of the audit trail — a stolen key allows signing of forged provenance chains that the app would validate as authentic.

**Recommended fix**
In the short term, set restrictive file permissions on the key file after creation:

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&key_path)
        .map_err(|e| format!("Failed to stat key file: {e}"))?
        .permissions();
    perms.set_mode(0o600); // owner read/write only
    std::fs::set_permissions(&key_path, perms)
        .map_err(|e| format!("Failed to set key permissions: {e}"))?;
}
```

In the medium term, consider using the OS keychain (macOS Keychain, Windows DPAPI, Linux Secret Service) via the `keyring` crate to store the key material.

**Verification**: On Unix, confirm `ls -la <AppData>/certs/jura_key.pem` shows `0600` permissions after app first launch.

---

### MEDIUM Findings

---

#### [MEDIUM-1] Sidecar has no authentication — any local process can submit files for analysis

**File**: `sidecar/main.py`, `sidecar/app/api/forensics.py`
**STRIDE category**: Elevation of Privilege, Spoofing

**Description**
The FastAPI sidecar binds to `127.0.0.1:8200` (correct) but exposes all forensic endpoints with no authentication whatsoever. Any process running on the same machine (including other user-space applications and browser scripts if the CSP is misconfigured) can submit arbitrary files for analysis. While the sidecar cannot write to the Rust backend's SQLite database, it does perform significant computation, and its results feed back into trust scores — a malicious local process could pre-warm the sidecar's analysis cache or probe its forensic algorithms.

**Recommended fix**
Add a shared secret header that the Rust backend generates on startup and passes to the sidecar client:

```python
# In sidecar config
sidecar_api_key: str = ""  # Empty = unauthenticated (backward compat)

# Middleware
from fastapi import Request, HTTPException

@app.middleware("http")
async def verify_api_key(request: Request, call_next):
    if settings.sidecar_api_key:
        key = request.headers.get("X-Jura-API-Key", "")
        if key != settings.sidecar_api_key:
            return Response(status_code=401, content="Unauthorised")
    return await call_next(request)
```

**Verification**: Start the sidecar with `JURA_SIDECAR_API_KEY=secret`; confirm that requests without the header return 401.

---

#### [MEDIUM-2] Audit log has no cryptographic chaining or integrity protection

**File**: `src-tauri/src/db.rs`, lines 797–818
**STRIDE category**: Repudiation, Tampering

**Description**
Audit log entries are inserted as independent rows with no hash chaining (each entry does not include a hash of the previous entry). An attacker with physical access to the SQLite database file (which is unencrypted) can delete, modify, or reorder audit entries without leaving any cryptographic evidence. The `operator_id` field is hardcoded to `"local_user"` and provides no actual identity assurance.

There is no integrity check on the database file itself — it is writable by any process running as the app user.

**Recommended fix**

Add a `prev_hash` column and compute a SHA-256 chain hash on each insert:

```rust
pub fn log_action(...) -> SqliteResult<()> {
    let conn = self.conn.lock().unwrap();

    // Retrieve hash of the most recent entry
    let prev_hash: Option<String> = conn.query_row(
        "SELECT entry_hash FROM audit_log ORDER BY created_at DESC LIMIT 1",
        [],
        |r| r.get(0),
    ).ok().flatten();

    let prev_hash_str = prev_hash.as_deref().unwrap_or("genesis");

    // Compute entry hash: SHA-256(prev_hash || action || target_id || created_at)
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(prev_hash_str.as_bytes());
    hasher.update(action.as_bytes());
    hasher.update(target_id.as_bytes());
    hasher.update(now.as_bytes());
    let entry_hash = hex::encode(hasher.finalize());

    // INSERT including prev_hash and entry_hash columns
    ...
}
```

**Verification**: Manually delete a row from `audit_log` and run an integrity check; the chain hash verification should detect the gap.

---

#### [MEDIUM-3] `data:` URI in CSP `img-src` permits arbitrary data URI images

**File**: `src-tauri/tauri.conf.json`, line 25
**STRIDE category**: Information Disclosure

**Description**
The CSP allows `img-src 'self' data: blob:`. The `data:` scheme in `img-src` allows inline images of any type and is not required for the application's functionality — heatmap images are passed as base64 strings and could equally be rendered via `blob:` URLs. Permitting `data:` in `img-src` is a known CSP weakening technique used to bypass `default-src` restrictions in some contexts.

This is addressed as part of the CRITICAL-3 fix above (removing `data:` from `img-src`).

**Recommended fix**
Convert base64 heatmap rendering to use `URL.createObjectURL` with `Blob` objects, then serve images via `blob:` URIs. The frontend already uses `createObjectURL` in the export helpers. The `img-src` directive then becomes `img-src 'self' blob: asset: https://asset.localhost`.

**Verification**: Check that all heatmap `<img>` tags use `blob:` URLs after the change; confirm `data:` URLs are blocked by CSP.

---

#### [MEDIUM-4] `blind_watermark` crate is at version 0.1 with limited ecosystem visibility

**File**: `src-tauri/Cargo.toml`, line 58
**STRIDE category**: Supply Chain

**Description**
The `blind_watermark = "0.1"` crate is used for DWT-DCT-SVD frequency-domain watermarking. At version 0.1 this is a pre-stability release. The crate's maintenance status and vulnerability disclosure process are unknown. Watermarking is applied to files that are then distributed externally, so a bug in the watermark embedding could corrupt assets or embed malformed data.

Additionally, `invisible-watermark>=0.2.0` in the Python sidecar (`requirements.txt`) is pinned with `>=` rather than `==`, meaning upgrades could introduce breaking changes silently.

**Recommended fix**
- Pin `invisible-watermark` to an exact version: `invisible-watermark==0.2.0`
- Review the `blind_watermark` crate source before the v1.0 release; consider forking if it is unmaintained
- Run `cargo audit` in CI to detect CVEs in the full dependency tree

**Verification**: Run `cargo audit` and confirm zero known advisories; confirm `pip-audit` passes for sidecar requirements.

---

#### [MEDIUM-5] The `verify_url` audit log entry records the full URL as the `target_id`

**File**: `src-tauri/src/lib.rs`, line 879 (inside `verify_content_inner`)
**STRIDE category**: Information Disclosure

**Description**
When verifying a URL, `verify_content_inner` is called with `source = temp_file_path`. The audit log records the temp file path as `target_id` via `log_action("verify", "file", source, ...)`. For URL verifications, `source` is the temp file path — not the original URL — which is acceptable. However, before this point, the raw URL (which may contain credentials in the query string, e.g. `?token=abc123`) is logged at `INFO` level via `log::info!("Verifying URL: {url}")` at line 1195.

If `env_logger` is configured to write to a file in the future, credentials in URLs would be persisted.

**Recommended fix**
Redact the URL before logging:

```rust
// Redact query string from logged URL
let log_url = url::Url::parse(&url)
    .map(|mut u| { u.set_query(None); u.set_fragment(None); u.to_string() })
    .unwrap_or_else(|_| "<invalid URL>".to_string());
log::info!("Verifying URL: {log_url} [mode={:?}]", mode);
```

**Verification**: Submit a URL with a query string token; confirm the log output does not contain the token value.

---

#### [MEDIUM-6] Temp files in Python sidecar use fixed suffixes that reveal file type

**File**: `sidecar/app/services/video_metadata.py`, line 27; `audio_metadata.py`, line 27; `video_frames.py`, line 31
**STRIDE category**: Information Disclosure

**Description**
Temporary files are created with `NamedTemporaryFile(suffix=".mp4", delete=False)` using fixed suffixes regardless of the actual file type submitted. The `.mp4` suffix is misleading for audio or video files of other formats. More importantly, `delete=False` without a corresponding cleanup on error in the `finally` block relies on `os.unlink(tmp_path)` — if `tmp_path` is not assigned (e.g., `NamedTemporaryFile` itself raises), the reference is undefined and the unlink call would raise `NameError`.

The `finally: os.unlink(tmp_path)` pattern is safe when the `with` block succeeds, but not when the temp file creation itself fails. Use `delete=True` with a context manager or guard the unlink.

**Recommended fix**

```python
import tempfile, os

tmp = tempfile.NamedTemporaryFile(suffix=".mp4", delete=False)
tmp_path = tmp.name
try:
    tmp.write(video_bytes)
    tmp.close()
    # ... process tmp_path ...
finally:
    try:
        os.unlink(tmp_path)
    except FileNotFoundError:
        pass
```

Or preferably use `tempfile.TemporaryDirectory` as a context manager.

**Verification**: Confirm temp files are not left on disk after normal operation and after exception paths.

---

#### [MEDIUM-7] `requirements.txt` uses unpinned version ranges for security-critical packages

**File**: `sidecar/requirements.txt`
**STRIDE category**: Supply Chain

**Description**
Several packages use loose version constraints:
- `scikit-learn>=1.5` — no upper bound
- `scikit-image>=0.24` — no upper bound
- `scipy>=1.14` — no upper bound
- `httpx>=0.27.0` — no upper bound
- `open-clip-torch>=2.24.0` — no upper bound

For a production application handling sensitive assets for cultural institutions, unpinned ranges mean that a `pip install --upgrade` or fresh environment creation could silently pull in a breaking or vulnerable version. The `chromadb==0.5.*` and `sentence-transformers==3.*` constraints are also minor-version wildcards.

**Recommended fix**
Pin all production dependencies to exact versions using `pip-compile` (pip-tools) or Poetry lock files. Generate a `requirements.lock` with hashes:

```
pip-compile requirements.txt --generate-hashes --output-file requirements.lock
```

Use `requirements.lock` in production; keep `requirements.txt` as the unpinned source-of-truth for development.

**Verification**: Run `pip-audit -r requirements.lock` in CI to detect known CVEs.

---

### LOW Findings

---

#### [LOW-1] `withGlobalTauri` status not verified — potential window.__TAURI__ exposure

**File**: `src-tauri/tauri.conf.json`
**STRIDE category**: Elevation of Privilege

**Description**
The `tauri.conf.json` does not explicitly set `app.withGlobalTauri = false`. In Tauri v2 this defaults to `false`, but it is good practice to make it explicit so that future Tauri upgrades that might change the default cannot inadvertently expose the global Tauri API to all webview scripts.

**Recommended fix**
Add to `tauri.conf.json`:
```json
"app": {
  "withGlobalTauri": false,
  ...
}
```

---

#### [LOW-2] Error messages from Rust commands propagate raw OS and library errors to the frontend

**File**: `src-tauri/src/lib.rs`, multiple locations (e.g. line 174, 345, 1157)
**STRIDE category**: Information Disclosure

**Description**
Many Tauri commands use `.map_err(|e| e.to_string())` to convert errors for IPC return. Rusqlite errors, IO errors, and library errors may contain filesystem paths, SQL statement fragments, or internal state. For example, a rusqlite `SQLITE_CORRUPT` error would include the database path; an IO error would include the full file path.

**Recommended fix**
Define an application error type with user-friendly messages that do not leak internal paths:

```rust
fn db_err(e: rusqlite::Error) -> String {
    log::error!("Database error: {e}");  // internal details to log only
    "A database error occurred. Please restart the application.".to_string()
}
```

---

#### [LOW-3] Sidecar's FastAPI OpenAPI docs are enabled in production

**File**: `sidecar/main.py`
**STRIDE category**: Information Disclosure

**Description**
FastAPI enables the `/docs` and `/redoc` interactive API documentation endpoints by default. While the sidecar is localhost-only, exposing full API documentation (including schema for all endpoints, supported parameters, and example payloads) to any local process is unnecessary and increases the attack surface.

**Recommended fix**
```python
app = FastAPI(
    title="Jura Trace ML Sidecar",
    version="0.2.0",
    docs_url=None,       # Disable Swagger UI
    redoc_url=None,      # Disable ReDoc
    openapi_url=None,    # Disable OpenAPI schema endpoint
)
```

---

#### [LOW-4] Reverse image search links in the Verify page pass user-controlled URL to external services

**File**: `ui/src/routes/verify/+page.svelte`, lines 476–507
**STRIDE category**: Information Disclosure

**Description**
For URL-sourced verifications, the reverse image search links construct Google Lens, TinEye, and Yandex URLs by encoding the original `urlInput` value. These links are presented to the user for voluntary clicking, not auto-followed. However, if the user submits a URL and then clicks a reverse search link, the original URL is disclosed to a third-party search engine — which may be problematic for sensitive assets verified from private or internal URLs.

The links correctly use `encodeURIComponent` and `rel="noopener noreferrer"`, so there is no XSS risk.

**Recommended fix**
Add a disclosure notice: "Clicking this link will share the image URL with [service name]." This is a UX/privacy concern rather than a technical vulnerability.

---

#### [LOW-5] SQLite database is not encrypted at rest

**File**: `src-tauri/src/db.rs`, lines 18–22
**STRIDE category**: Information Disclosure

**Description**
The SQLite database is stored unencrypted at `<AppData>/jura_archive.db`. For cultural institutions handling sensitive provenance data, the database contains:
- Full file paths to all protected assets
- Perceptual hashes (fingerprints) that could be used to identify images
- C2PA signing metadata including creator names and copyright information
- Forensic scores and audit logs

On macOS the `AppData` directory is within the user's home folder and accessible to other processes running as that user.

**Recommended fix**
Consider SQLCipher (available as `rusqlite` feature `"bundled-sqlcipher"`) with a key derived from the OS keychain. For Phase 3 or v1.0, this would be a meaningful security improvement for institutional deployments.

**Verification**: This is a medium-term architectural decision; document the data-at-rest risk in the deployment guide.

---

## STRIDE Summary Table

| Threat Category | Finding Count | Key Findings |
|---|---|---|
| **Spoofing** | 2 | CRITICAL-1 (SSRF allows identity of target server to be spoofed), HIGH-6 (unprotected signing key) |
| **Tampering** | 4 | CRITICAL-2 (unrestricted filesystem write), HIGH-2 (path traversal), HIGH-4 (LIKE injection), MEDIUM-2 (audit log no chaining) |
| **Repudiation** | 2 | MEDIUM-2 (audit log integrity), HIGH-6 (signing key theft enables false provenance) |
| **Information Disclosure** | 7 | CRITICAL-3 (CSP wildcard), HIGH-2 (path in errors), MEDIUM-5 (URL logging), MEDIUM-3 (data: URI), LOW-2 (raw error propagation), LOW-5 (unencrypted DB) |
| **Denial of Service** | 2 | HIGH-1 (decompression bomb), HIGH-5 (unbounded video upload) |
| **Elevation of Privilege** | 4 | CRITICAL-1 (SSRF), CRITICAL-2 (filesystem), CRITICAL-3 (CSP), MEDIUM-1 (unauthenticated sidecar) |

---

## Recommendations Priority Matrix

| Priority | Finding | Effort | Impact |
|---|---|---|---|
| 1 | CRITICAL-1: SSRF in verify_url | Low — add URL validation function | Prevents SSRF against local services |
| 2 | CRITICAL-3: CSP localhost wildcard | Low — update tauri.conf.json string | Eliminates lateral movement via webview |
| 3 | CRITICAL-2: Unscoped filesystem capability | Medium — add scope to capabilities/default.json | Prevents full filesystem exposure |
| 4 | HIGH-1: No file size / dimension limits | Low — add size check constant and guard | Prevents DoS via crafted files |
| 5 | HIGH-5: Video/audio size check bypass | Low — refactor sidecar endpoint handlers | Prevents sidecar OOM |
| 6 | HIGH-3: Temp file extension traversal | Low — sanitise extension to alphanumeric | Prevents temp-dir escape |
| 7 | HIGH-6: Unprotected C2PA private key | Low — set file permissions 0600 | Hardens signing key storage |
| 8 | HIGH-2: Path traversal in verify_content | Low — add canonicalise + error sanitisation | Prevents filesystem probing |
| 9 | HIGH-4: LIKE injection in search | Low — escape LIKE metacharacters | Prevents wildcard abuse |
| 10 | MEDIUM-1: Unauthenticated sidecar | Medium — add shared-secret middleware | Prevents local sidecar abuse |
| 11 | MEDIUM-2: Audit log no chaining | High — schema migration + hash chain | Enables tamper detection |
| 12 | MEDIUM-7: Unpinned Python dependencies | Low — pin with pip-compile | Prevents silent dependency upgrade |
| 13 | LOW-1: withGlobalTauri not set | Trivial — add config key | Hardens against future default change |
| 14 | LOW-3: FastAPI docs exposed | Trivial — set docs_url=None | Reduces attack surface documentation |
| 15 | LOW-5: Unencrypted SQLite | High — SQLCipher integration | Protects data at rest (v1.0 goal) |

---

## Data Exfiltration Assessment

No evidence of external data transmission was found in the codebase beyond:

1. The `verify_url` command, which issues HTTP GET requests to user-specified URLs. This is intentional functionality but creates the SSRF risk documented in CRITICAL-1.
2. The reverse image search links in the Verify UI, which pass URLs to third-party search engines — but only on explicit user click action (LOW-4).
3. Ollama communication on `127.0.0.1:11434`, which is local-only.

The sidecar correctly binds to `127.0.0.1`, not `0.0.0.0`. The Rust backend's sidecar client is hardcoded to `http://127.0.0.1:8200` (line 1595, `lib.rs`). No telemetry endpoints, analytics libraries, or external CDN references were identified. The local-first principle is architecturally sound; the issues identified are boundary enforcement gaps, not intentional outbound calls.

---

*Report produced by the Jura Trace Security Auditor agent on 2026-03-21. This report covers the codebase at the state reflected in git branch `main` as of the audit date. Security findings should be reassessed after remediation and before any public release.*
