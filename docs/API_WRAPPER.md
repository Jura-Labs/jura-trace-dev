---
title: "Jura Trace API Wrapper — Architecture and Specification"
description: "Design, endpoint reference, authentication, and integration guide for the Jura Trace local HTTP API, enabling external applications to access the VERIFY and PROTECT pipelines programmatically."
last-updated: 21 March 2026
status: planned — Phase 2 / 3 feature, not yet implemented
phase: 3
---

# Jura Trace API Wrapper

> **Note**: The API wrapper is a planned feature. It is not available in the current release (0.2.0-dev). Implementation is estimated at 2–3 weeks of development effort. This document records the intended architecture for planning and commercial roadmap purposes.

The Jura Trace API wrapper is a local HTTP server that runs alongside the desktop application and exposes the VERIFY and PROTECT pipelines via REST endpoints. It enables external applications — learning management systems, eCommerce platforms, practice management software, and corporate workflow tools — to submit content for verification and receive structured results, without sending any data to a cloud service.

All processing remains on the user's device. The API wrapper is local-only: it binds to `127.0.0.1` and is never accessible from the network.

---

## Table of Contents

1. [Design Principles](#design-principles)
2. [Architecture](#architecture)
3. [Running the API Wrapper](#running-the-api-wrapper)
4. [Authentication](#authentication)
5. [Endpoints — Verification](#endpoints--verification)
6. [Endpoints — Protection](#endpoints--protection)
7. [Endpoints — Claim Checking](#endpoints--claim-checking)
8. [Endpoints — Management](#endpoints--management)
9. [Response Schemas](#response-schemas)
10. [Error Handling](#error-handling)
11. [Rate Limiting](#rate-limiting)
12. [Integration Examples](#integration-examples)
13. [Licence Tier Capabilities](#licence-tier-capabilities)
14. [Implementation Notes](#implementation-notes)
15. [Related Documents](#related-documents)

---

## Design Principles

1. **Local-only**: Binds to `127.0.0.1:8300` exclusively. The OS will not route this address to any network interface. There is no configuration option to expose it externally.
2. **No cloud dependency**: The wrapper calls the same Rust core engine and Python ML sidecar (port 8200) as the desktop UI. No data leaves the device.
3. **Same pipeline**: The wrapper does not implement its own verification logic. It is a thin HTTP layer over existing Tauri commands. Results are identical to those produced by the desktop interface.
4. **API key authentication**: Bearer token authentication using locally generated keys stored in the application's SQLite database. No OAuth, no external identity provider.
5. **OpenAPI 3.1**: The full specification is auto-generated and available at `/openapi.json`. Clients can generate typed SDKs from it.
6. **Graceful degradation**: If the Python ML sidecar (port 8200) is offline, the wrapper returns partial results from the Rust engine (C2PA verification, EXIF anomaly detection, perceptual hash lookup) and flags the omission clearly in the response.

---

## Architecture

```
┌─────────────────────────────────────────────┐
│  External Application (LMS, CMS, Pipeline)  │
│  e.g. Moodle, Canvas, practice management   │
└──────────────────┬──────────────────────────┘
                   │ HTTP (127.0.0.1:8300)
                   │ Bearer token auth
┌──────────────────▼──────────────────────────┐
│      Jura Trace API Wrapper (Rust/Axum)     │
│                                             │
│  REST endpoints + OpenAPI 3.1 spec          │
│  Auth: API key (SQLite-backed)              │
│  Rate limiting: configurable per key        │
│  Multipart parsing: file uploads            │
│  Response: JSON (application/json)          │
└──────────────────┬──────────────────────────┘
                   │ Direct function calls
                   │ (shared library, not IPC)
┌──────────────────▼──────────────────────────┐
│         Jura Trace Core Engine (Rust)        │
│                                             │
│  C2PA signing and verification (c2pa-rs)    │
│  Perceptual hashing (aHash, dHash, pHash)   │
│  EXIF anomaly detection                     │
│  Format router (MIME detection)             │
│  SQLite database (rusqlite)                 │
└──────────────────┬──────────────────────────┘
                   │ HTTP (127.0.0.1, port from `[sidecar].url` below)
┌──────────────────▼──────────────────────────┐
│      Python ML Sidecar (existing)           │
│      Note: when the wrapper is co-deployed   │
│      with Jura Trace Desktop, the desktop's  │
│      sidecar uses an ephemeral port (Option C, │
│      May 2026) — point the wrapper at a       │
│      separately-managed sidecar instead.      │
│                                             │
│  21-signal deepfake ensemble                │
│  ELA, noise analysis, copy-move detection   │
│  GBM classifier (AUC-ROC 0.945)            │
│  RAG claim checker (Phase 2 Wk 19-20)      │
└─────────────────────────────────────────────┘
```

The wrapper is implemented as either:

- **Separate binary**: `jura-trace-api` compiled from the same workspace, started independently of the desktop application.
- **Feature flag in the main binary**: `jura-trace --api --port 8300` starts the desktop application and the API server together.

The separate binary approach is preferred for enterprise deployments where the API must run in a headless server context (CI/CD pipelines, background services) without launching a desktop window.

---

## Running the API Wrapper

### Starting the server

```bash
# Separate binary (preferred for headless/server use)
jura-trace-api --port 8300 --config /etc/jura-trace/api.toml

# Feature flag on main binary
jura-trace --api --port 8300

# Development (from workspace root)
cargo run --bin jura-trace-api -- --port 8300
```

### Configuration file (TOML)

```toml
# /etc/jura-trace/api.toml (or ~/.config/jura-trace/api.toml)

[server]
host = "127.0.0.1"   # Must not be changed to 0.0.0.0
port = 8300
workers = 4

[auth]
require_key = true   # Set to false only for development

[rate_limit]
default_requests_per_hour = 100   # Professional tier default
burst = 10                        # Requests allowed in rapid succession

[sidecar]
url = "http://127.0.0.1:8200"
timeout_seconds = 30

[logging]
level = "info"   # trace | debug | info | warn | error
file = "/var/log/jura-trace-api.log"
```

### Verifying the server is running

```bash
curl http://127.0.0.1:8300/api/v1/health
```

Expected response (sidecar available):

```json
{
  "status": "ok",
  "version": "0.2.0-dev",
  "sidecar": {
    "available": true,
    "url": "http://127.0.0.1:8200",
    "capabilities": {
      "ela": true,
      "noise": true,
      "copy_move": true,
      "deepfake": true,
      "jpeg_ghost": true,
      "npr": true,
      "chromatic_aberration": true,
      "segmented_ela": true,
      "shadow_consistency": true,
      "colour_temperature": true,
      "splice_boundary": true,
      "watermark": true,
      "clip_detect": false,
      "rag": false,
      "video_metadata": false,
      "audio_metadata": false,
      "video_frames": false
    }
  },
  "capabilities": {
    "c2pa": true,
    "fingerprint": true,
    "exif_anomaly": true,
    "forensics": true,
    "deepfake": true,
    "rag": false
  }
}
```

> **Note**: JSON field names in the API wrapper follow snake_case convention throughout (matching REST and Python sidecar conventions). The desktop application receives the same data serialised to camelCase via Tauri's `serde(rename_all = "camelCase")` — but clients calling the API wrapper directly should expect snake_case field names in all responses.

---

## Authentication

### Generating an API key

API keys are generated through the management endpoint or through the Jura Trace desktop interface (Settings → API Keys).

```
POST /api/v1/auth/keys
Content-Type: application/json

{
  "name": "moodle-integration",
  "rate_limit": 100
}
```

**Response:**

```json
{
  "key": "jt_a1b2c3d4e5f6...",
  "key_id": "key_7f8g9h",
  "name": "moodle-integration",
  "rate_limit": 100,
  "created_at": "2026-03-21T09:00:00Z",
  "expires_at": null
}
```

The `key` value is returned once and not stored in plaintext. Record it immediately. If lost, generate a new key and revoke the old one.

### Using API keys

Include the key as a Bearer token in the `Authorization` header on all subsequent requests:

```
Authorization: Bearer jt_a1b2c3d4e5f6...
```

### Key management

```
GET    /api/v1/auth/keys             List all keys (name, key_id, created_at, last_used_at)
DELETE /api/v1/auth/keys/{key_id}    Revoke a key immediately
PATCH  /api/v1/auth/keys/{key_id}    Update rate limit or name
```

Keys are stored in the application's SQLite database (`~/.local/share/jura-trace/jura_trace.db` on Linux, `~/Library/Application Support/jura-trace/jura_trace.db` on macOS). They are scoped to localhost — they cannot be used from another machine.

---

## Endpoints — Verification

### POST /api/v1/verify

Submit a single file for verification. Returns the full `VerificationResult` matching the schema produced by the desktop VERIFY pipeline.

**Request:**

```
POST /api/v1/verify?mode=standard
Content-Type: multipart/form-data
Authorization: Bearer jt_...

file: (binary, required)
```

**Query parameters:**

| Parameter  | Type   | Default    | Description |
|------------|--------|------------|-------------|
| `mode`     | string | `standard` | Investigation depth: `standard`, `deep`, or `archival` |
| `mime_type`| string | auto       | Override MIME detection. If omitted, detected from file magic bytes. |

**Response:** `200 OK`, `application/json`

Returns a `VerificationResult` object. See [Response Schemas](#response-schemas).

**Example:**

```bash
curl -X POST "http://127.0.0.1:8300/api/v1/verify?mode=deep" \
  -H "Authorization: Bearer jt_abc123..." \
  -F "file=@/path/to/evidence.jpg"
```

---

### POST /api/v1/verify/url

Submit a URL for verification. The file is fetched locally (the request originates from the user's machine, not a cloud intermediary).

**Request:**

```
POST /api/v1/verify/url?mode=standard
Content-Type: application/json
Authorization: Bearer jt_...

{
  "url": "https://example.com/image.jpg"
}
```

**Query parameters:** Same as `/api/v1/verify`.

**Response:** `200 OK`, `application/json` — `VerificationResult` object.

**Notes:**

- The URL is fetched using the local network connection. If the URL requires authentication or is behind a VPN, ensure the network context is configured before making this call.
- URLs are logged in the application audit trail with timestamp and requesting API key name.

---

### POST /api/v1/verify/batch

Submit multiple files for verification in a single request.

**Request:**

```
POST /api/v1/verify/batch?mode=standard&concurrency=3
Content-Type: multipart/form-data
Authorization: Bearer jt_...

files[]: (binary, required, multiple)
```

**Query parameters:**

| Parameter     | Type    | Default    | Description |
|---------------|---------|------------|-------------|
| `mode`        | string  | `standard` | Investigation depth: `standard`, `deep`, or `archival` |
| `concurrency` | integer | `3`        | Parallel processing limit. Range: 1–5. Higher values increase CPU load. |

**Response:** `200 OK`, `application/json`

```json
{
  "results": [
    {
      "filename": "photo1.jpg",
      "result": { /* VerificationResult */ }
    },
    {
      "filename": "photo2.png",
      "error": "Unsupported file format: image/bmp",
      "result": null
    }
  ],
  "total": 2,
  "succeeded": 1,
  "failed": 1,
  "processing_time_ms": 4521
}
```

Partial failures do not abort the batch. Each file result includes either a `result` or an `error` field.

---

## Endpoints — Protection

### POST /api/v1/protect/sign

Sign a file with a C2PA Content Credential, embedding provenance metadata (creator, licence, timestamp) into the file.

**Request:**

```
POST /api/v1/protect/sign
Content-Type: multipart/form-data
Authorization: Bearer jt_...

file: (binary, required)
creator_name: "Jura Labs CIC"                    (string, required)
license: "CC BY 4.0"                            (string, required)
rights_statement: "All rights reserved"         (string, optional)
contact_url: "https://juralabs.org"             (string, optional)
```

**Response:** `200 OK`

Returns the signed file as a binary download (`Content-Disposition: attachment`), alongside a JSON header block:

```
X-Jura-Manifest: {"manifest_label":"...","signing_time":"...","creator":"...","hash_alg":"sha256"}
```

The signed file is a valid JPEG/PNG/PDF with the C2PA manifest embedded in the standard location. It can be opened by any C2PA-compatible viewer, including Adobe's Content Credentials viewer and the Jura Trace VERIFY pipeline.

---

### POST /api/v1/protect/fingerprint

Compute perceptual hashes for a file without signing or modifying it. Useful for registering an asset in the fingerprint database before publication.

**Request:**

```
POST /api/v1/protect/fingerprint
Content-Type: multipart/form-data
Authorization: Bearer jt_...

file: (binary, required)
```

**Response:** `200 OK`, `application/json`

```json
{
  "filename": "painting_scan.tiff",
  "mime_type": "image/tiff",
  "hashes": [
    { "algorithm": "phash", "hash_hex": "a3f2..." },
    { "algorithm": "ahash", "hash_hex": "b7c1..." },
    { "algorithm": "dhash", "hash_hex": "f4d9..." }
  ],
  "asset_id": "ast_9k2m...",
  "registered_at": "2026-03-21T09:15:00Z"
}
```

The fingerprint is stored in the local SQLite database and is immediately available for lookup via the VERIFY pipeline.

---

## Endpoints — Claim Checking

### POST /api/v1/claims/check

Submit a text claim for verification against the local RAG knowledge base.

> **Note**: This endpoint requires the Python ML sidecar (port 8200) to be running with the RAG pipeline enabled (Phase 2, Weeks 19–20). If the sidecar is unavailable, the endpoint returns `503 Service Unavailable`.

**Request:**

```
POST /api/v1/claims/check
Content-Type: application/json
Authorization: Bearer jt_...

{
  "claim": "This photograph shows flooding in Valencia on 29 October 2024.",
  "context": "Shared on social media with caption claiming it shows the DANA floods."
}
```

**Response:** `200 OK`, `application/json`

The response schema mirrors `ClaimCheckResponse` from the Python ML sidecar (`sidecar/app/models/schemas.py`). Claim checking uses local TF-IDF retrieval against text files in `sidecar/knowledge_base/` — there is no web retrieval and no external URLs in the response.

```json
{
  "overall_verdict": "disputed",
  "claims": [
    {
      "claim": "This photograph shows flooding in Valencia on 29 October 2024.",
      "verdict": "disputed",
      "explanation": "The DANA floods in Valencia did occur on this date. However, cross-referencing against the local knowledge base finds no corroborating material specifically placing this image at that location and time.",
      "confidence": 0.68
    }
  ],
  "model_used": "qwen2.5:7b",
  "methodology": "TF-IDF retrieval from local knowledge base, followed by LLM-assisted reasoning. No web retrieval is performed — results reflect only locally indexed material.",
  "summary": "One claim assessed. The event is documented in the knowledge base but the specific image origin could not be verified from available local sources."
}
```

**Possible `verdict` values for individual claims**: `supported`, `disputed`, `unverified`, `unavailable`

**Possible `overall_verdict` values**: `supported`, `disputed`, `unverified`, `mixed`, `unavailable`

---

## Endpoints — Management

### GET /api/v1/health

Returns service status and capability availability.

**Response:** `200 OK`, `application/json` — see [Running the API Wrapper](#running-the-api-wrapper) for example response.

---

### GET /api/v1/stats

Returns aggregate statistics for the local installation.

**Response:** `200 OK`, `application/json`

```json
{
  "assets_registered": 4201,
  "verifications_total": 1847,
  "verifications_today": 23,
  "deepfake_verdicts": {
    "authentic": 1609,
    "inconclusive": 183,
    "synthetic": 55
  },
  "false_positive_rate": 0.006,
  "calibration_corpus_size": 627,
  "database_size_mb": 142
}
```

---

### GET /openapi.json

Returns the auto-generated OpenAPI 3.1 specification for all available endpoints.

**Response:** `200 OK`, `application/json`

Use this endpoint to generate typed client SDKs in any language using tools such as `openapi-generator` or `fern`.

---

## Response Schemas

### VerificationResult

The primary verification response schema. Mirrors the `VerificationResult` struct in `src-tauri/src/lib.rs` and the TypeScript interface in `ui/src/lib/types.ts`.

All field names are snake_case in API wrapper responses. The schema mirrors the `VerificationResult` struct in `src-tauri/src/lib.rs` (annotated `serde(rename_all = "camelCase")` for the desktop app, but serialised to snake_case for the REST API).

Optional fields are omitted when not applicable (for example, `ela_result` is `null` if the sidecar was offline or the file is not an image). All responses include a top-level `api_version` field for client compatibility checks.

```json
{
  "api_version": "0.2.0-dev",
  "source_type": "file",
  "content_type": "image",
  "mode": "standard",
  "overall_trust": 0.91,
  "c2pa_valid": true,
  "ela_score": 0.14,
  "noise_score": 0.09,
  "copy_move_score": 0.03,
  "deepfake_score": 0.08,
  "metadata_flags": [],
  "ai_generator": null,
  "claim_verdict": null,
  "exif_analysis": {
    "findings": [],
    "trust_score": 0.95,
    "fields_populated": 24,
    "fields_total": 30,
    "has_exif": true
  },
  "c2pa_manifest": {
    "title": "evidence_photo.jpg",
    "format": "image/jpeg",
    "claim_generator": "Jura Archive/0.2.0",
    "assertions": [
      { "label": "c2pa.actions", "value": "c2pa.created" }
    ],
    "is_valid": true,
    "signed_at": "2026-03-15T10:00:00Z"
  },
  "ela_result": {
    "ela_image_base64": "<base64>",
    "max_difference": 18.4,
    "mean_difference": 3.1,
    "score": 0.14,
    "suspicious": false
  },
  "noise_result": {
    "heatmap_base64": "<base64>",
    "block_variances": [2.1, 1.8, 2.4],
    "global_variance": 2.1,
    "anomalous_blocks": 0,
    "total_blocks": 64,
    "score": 0.09,
    "suspicious": false
  },
  "copy_move_result": {
    "visualisation_base64": "<base64>",
    "clone_regions": [],
    "matched_pairs": 0,
    "score": 0.03,
    "suspicious": false
  },
  "deepfake_result": {
    "score": 0.08,
    "suspicious": false,
    "confidence": "high",
    "verdict_level": "authentic",
    "signals": [
      {
        "name": "noise_residual",
        "description": "Natural sensor noise level detected",
        "weight": 3.0,
        "triggered": false
      }
    ],
    "heatmap_base64": "<base64>",
    "summary": "Image appears authentic (1 of 21 signals triggered)",
    "watermarks": [],
    "classifier_score": 0.11,
    "classifier_available": true
  },
  "npr_result": {
    "score": 0.07,
    "suspicious": false,
    "hv_correlation": 0.82,
    "diff_variance_ratio": 1.1,
    "hf_energy_ratio": 0.042,
    "heatmap_base64": "<base64>",
    "summary": "Neighbouring pixel relationships consistent with camera capture"
  },
  "jpeg_ghost_result": {
    "score": 0.04,
    "suspicious": false,
    "ghost_quality": 75,
    "quality_variance": 0.02,
    "deviating_blocks": 1,
    "total_blocks": 256,
    "heatmap_base64": "<base64>",
    "summary": "No JPEG ghost artefacts detected"
  },
  "ca_result": {
    "r_squared": 0.97,
    "is_consistent": true,
    "score": 0.05,
    "suspicious": false,
    "sample_count": 120,
    "summary": "Chromatic aberration pattern consistent across the frame"
  },
  "segmented_ela_result": null,
  "shadow_consistency_result": null,
  "colour_temperature_result": null,
  "splice_boundary_result": null,
  "watermark_extract_result": {
    "extracted_payload": "JL-2026-evidence_photo",
    "extracted_hex": "4a4c2d323032362d...",
    "has_watermark": true,
    "confidence": 0.94,
    "success": true,
    "message": "Watermark extracted successfully"
  },
  "video_metadata": null,
  "audio_metadata": null
}
```

**Verdict levels** (`verdict_level` field on `deepfake_result`):

| Value           | Display label       | Score range   |
|-----------------|---------------------|---------------|
| `authentic`     | Likely authentic    | 0.00–0.29     |
| `inconclusive`  | Cannot determine    | 0.30–0.64     |
| `synthetic`     | Likely AI-generated | 0.65–1.00     |

Score thresholds are calibrated in `sidecar/app/services/deepfake.py`. The `synthetic` verdict is also triggered when one or more invisible AI watermarks are detected (Stable Diffusion, SDXL, or Flux watermark patterns), regardless of the numeric score.

### `methodology` — reproducibility provenance block

Every `VerificationResult` carries a `methodology` block that pins the exact engine, sidecar, and model artefacts used to produce the result. Downstream tooling — including the v1.0.1 `jura` CLI, audit-report generators, and external reproducibility harnesses — should cite these values verbatim when archiving evidence.

```json
{
  "methodology": {
    "pipeline_version": "1.0.0",
    "sidecar_version": "0.9.0",
    "classifier_model_hash": "2931f197cba6f376e85b1cbcfd584e6802f36e4fbf68ff00c83d61d4d655db18",
    "univfd_probe_model_hash": "ed691b45cbe2903a7e0530fd0ec78ab91eef9f15133af4c1a5c8cf172086dacd",
    "analysis_mode": "standard",
    "analysed_at": "2026-06-22T10:00:00Z"
  }
}
```

| Field | Type | Meaning |
|------|------|---------|
| `pipeline_version` | string | Jura Trace application version (`CARGO_PKG_VERSION` at build time) |
| `sidecar_version` | string \| null | Python ML sidecar version, `null` if the sidecar was offline |
| `classifier_model_hash` | string \| null | SHA-256 of `models/deepfake_classifier.joblib`, `null` if not installed |
| `univfd_probe_model_hash` | string \| null | SHA-256 of `models/univfd_probe.joblib`, `null` if the optional CLIP probe is not installed (added in JTV-181, v1.0) |
| `analysis_mode` | string | `quick`, `standard`, or `deep` |
| `analysed_at` | string | ISO 8601 timestamp (UTC, RFC 3339) |

When the same input is verified twice with matching `methodology`, the result MUST be bit-identical except for `analysed_at`. Any deviation indicates a non-determinism bug — report via the issue tracker.

---

## CLI Exit-Code Contract

The forthcoming `jura` CLI (v1.0.1, JTV-182) is a thin Rust client over this REST API. Its exit-code contract is pre-locked in v1.0 so newsroom and forensic-audit automation written against the v1.0.1 release can be authored against this REST API today.

| Exit code | Name | Meaning |
|-----------|------|---------|
| `0` | success | Operation completed; result emitted to stdout |
| `1` | usage | Invalid command-line arguments (missing required flag, unknown subcommand) |
| `2` | unreachable | Cannot reach the local API at `http://127.0.0.1:8300` (server not running) |
| `3` | auth | Authentication failure — missing or rejected API key |
| `4` | file | Local file error — input file not found, unreadable, or empty |
| `5` | format | Server rejected the input as an unsupported format (HTTP 422) |
| `6` | server | Server-side error (HTTP 5xx) or partial-availability degraded response |

The numeric codes are stable across all v1.x releases. Wrapper scripts and CI pipelines may rely on them indefinitely. New conditions get new codes; existing codes are never re-purposed.

Server-side handlers that want to influence the CLI exit code should map to the corresponding HTTP status code listed below — the CLI's translation table follows the HTTP status, not the response body.

---

## Schema as Public API Contract

Starting with v1.0 (22 June 2026), the JSON shape of every response in this document is a **public API contract**.

**What this means in practice:**

- New fields MAY be added at any time. Clients MUST ignore unknown fields and not error on them.
- Existing field names MUST NOT be renamed within a v1.x major version. A rename is a v2.0 break.
- Existing field types MUST NOT change. Widening a `string` to `string | null` is a break and requires a v2.0.
- Field-removal requires a deprecation cycle of at least one minor release (`v1.x` → `v1.x+1`) during which the field is still emitted (possibly `null`), accompanied by a `CHANGELOG.md` note. Hard removal lands no earlier than v2.0.
- `methodology.pipeline_version` is the authoritative engine-version string. The legacy top-level `api_version` field is retained for back-compat and reflects the *envelope* version, not the engine.

**Why this matters:**

Pilot newsroom partners are already integrating against this API. The C2PA Validator Conformant award (recordId `019d8d83-…`, spec 2.2, awarded 2026-05-06) makes the verification result an evidence-bearing record. Schema drift breaks both groups silently.

**Versioning model:**

- `v1.x.y` — non-breaking additions and bug fixes. Schema stays compatible.
- `v2.0` — breaking changes. Will be cut only when the cost of carrying compatibility shims exceeds the value. Old endpoints under `/api/v1/` will be retained for one full release cycle.

All schema changes MUST be entered in `CHANGELOG.md` under the relevant release heading with the prefix `[schema]`.

---

## Error Handling

All errors return a JSON body with a consistent structure:

```json
{
  "error": "unsupported_format",
  "message": "The submitted file type (image/bmp) is not supported by the verification pipeline.",
  "status": 422,
  "request_id": "req_x7k2m9"
}
```

### Status codes

| Code | Meaning |
|------|---------|
| `200` | Success |
| `400` | Bad request — malformed request body or missing required field |
| `401` | Unauthorised — missing or invalid API key |
| `413` | Payload too large — file exceeds 100MB limit |
| `422` | Unprocessable — valid request but unsupported file format or content |
| `429` | Rate limit exceeded — see `Retry-After` header |
| `500` | Internal server error — pipeline failure |
| `503` | Service unavailable — sidecar offline (partial results may follow with `503` on forensics/deepfake fields) |

### Partial availability

When the Python ML sidecar (port 8200) is offline, the API wrapper returns `200 OK` with partial results rather than a top-level error. The `forensicsResult` and `deepfakeResult` fields will have `"available": false`, and a `"degraded"` flag appears at the top level:

```json
{
  "degraded": true,
  "degraded_reason": "Python ML sidecar unavailable at http://127.0.0.1:8200",
  "overall_trust": null,
  ...
}
```

This matches the graceful degradation behaviour of the desktop application.

---

## Rate Limiting

Rate limits are enforced per API key, not per IP address.

| Tier         | Default limit       | Burst |
|--------------|---------------------|-------|
| Professional | 100 requests/hour   | 10    |
| Enterprise   | Unlimited (default) | N/A   |

When a rate limit is exceeded, the response is `429 Too Many Requests` with a `Retry-After` header indicating when the limit resets:

```
HTTP/1.1 429 Too Many Requests
Retry-After: 1800
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1742810400
```

Enterprise licences can configure per-key rate limits to prevent any single integration from consuming all available processing capacity:

```bash
PATCH /api/v1/auth/keys/key_7f8g9h
Content-Type: application/json

{
  "rate_limit": 500
}
```

---

## Integration Examples

### Python (requests)

```python
import requests

# Submit a file for verification
with open("evidence_photo.jpg", "rb") as f:
    response = requests.post(
        "http://127.0.0.1:8300/api/v1/verify",
        files={"file": f},
        params={"mode": "deep"},
        headers={"Authorization": "Bearer jt_abc123..."},
    )

result = response.json()

# Extract key verdicts — API wrapper uses snake_case field names
verdict = result["deepfake_result"]["verdict_level"]  # authentic | inconclusive | synthetic
trust = result["overall_trust"]
findings = result["exif_analysis"]["findings"] if result.get("exif_analysis") else []

print(f"Deepfake verdict: {verdict}")
print(f"Overall trust score: {trust:.2f}")
print(f"EXIF anomaly findings: {len(findings)}")

# Flag for human review if inconclusive or synthetic
if verdict in ("inconclusive", "synthetic"):
    print("Flagged for human review.")
```

### cURL

```bash
# Single file verification
curl -X POST "http://127.0.0.1:8300/api/v1/verify?mode=deep" \
  -H "Authorization: Bearer jt_abc123..." \
  -F "file=@evidence_photo.jpg"

# Batch verification with three files
curl -X POST "http://127.0.0.1:8300/api/v1/verify/batch?mode=standard&concurrency=3" \
  -H "Authorization: Bearer jt_abc123..." \
  -F "files[]=@photo1.jpg" \
  -F "files[]=@photo2.jpg" \
  -F "files[]=@photo3.png"

# Claim checking
curl -X POST "http://127.0.0.1:8300/api/v1/claims/check" \
  -H "Authorization: Bearer jt_abc123..." \
  -H "Content-Type: application/json" \
  -d '{"claim": "This image shows the 2024 Valencia floods.", "context": "Social media post."}'
```

### JavaScript / Node.js

```javascript
import { createReadStream } from 'fs';

// Single file verification
async function verifyFile(filePath, mode = 'standard') {
  const form = new FormData();
  form.append('file', createReadStream(filePath));

  const response = await fetch(
    `http://127.0.0.1:8300/api/v1/verify?mode=${mode}`,
    {
      method: 'POST',
      body: form,
      headers: { 'Authorization': 'Bearer jt_abc123...' },
    }
  );

  if (!response.ok) {
    const error = await response.json();
    throw new Error(`Verification failed: ${error.message}`);
  }

  return response.json();
}

// Usage — API wrapper uses snake_case field names
const result = await verifyFile('./submission_photo.jpg', 'deep');
console.log(`Trust score: ${result.overall_trust}`);
console.log(`Deepfake verdict: ${result.deepfake_result?.verdict_level}`);
```

### Moodle plugin (PHP)

```php
<?php
// Verify a student submission image via Jura Trace API
function jura_verify_submission(string $filepath): array {
    $ch = curl_init('http://127.0.0.1:8300/api/v1/verify?mode=deep');

    $postFields = [
        'file' => new CURLFile($filepath, mime_content_type($filepath)),
    ];

    curl_setopt_array($ch, [
        CURLOPT_POST           => true,
        CURLOPT_POSTFIELDS     => $postFields,
        CURLOPT_RETURNTRANSFER => true,
        CURLOPT_HTTPHEADER     => [
            'Authorization: Bearer ' . get_config('mod_jura', 'api_key'),
        ],
    ]);

    $response = curl_exec($ch);
    $status   = curl_getinfo($ch, CURLINFO_HTTP_CODE);
    curl_close($ch);

    if ($status !== 200) {
        throw new moodle_exception('apierror', 'mod_jura', '', $status);
    }

    return json_decode($response, true);
}
```

---

## Licence Tier Capabilities

| Capability | Community | Professional | Enterprise |
|-----------|-----------|--------------|------------|
| API wrapper access | — | Single key | Multiple keys with scoping |
| Rate limit | — | 100 req/hr | Unlimited (configurable) |
| Batch endpoint | — | Yes | Yes |
| Concurrency limit | — | 3 | 5 |
| Watched folder mode | — | — | Yes |
| Custom RAG knowledge base | — | — | Yes |
| Key management console | — | Limited | Full |
| OpenAPI spec download | — | Yes | Yes |
| SLA on API availability | — | — | 24hr response |

Community users access the VERIFY and PROTECT pipelines through the desktop interface. The API wrapper is a Professional and Enterprise feature.

---

## Implementation Notes

### Technology choices

- **Axum** (Rust async HTTP framework) — already in the ecosystem via `tokio`. Shares memory space with the core engine; no serialisation overhead for internal calls.
- **Tower middleware** — rate limiting, request tracing, and authentication implemented as Tower layers. Composable and testable.
- **Multipart parsing** — `axum-multipart` for file uploads. Files are streamed to temporary storage and cleaned up after processing.
- **OpenAPI generation** — `utoipa` crate for auto-generating the OpenAPI 3.1 spec from Rust struct annotations. Served at `/openapi.json`.

### Estimated implementation effort

| Component | Effort |
|-----------|--------|
| Axum server setup, middleware stack, configuration | 3–4 days |
| Endpoint implementations (wrapping existing pipeline functions) | 5–7 days |
| API key management and rate limiting | 2–3 days |
| OpenAPI spec generation and documentation | 2–3 days |
| Integration testing | 2–3 days |
| **Total** | **2–3 weeks** |

### Security considerations

- The server binds exclusively to `127.0.0.1`. This is enforced in code, not just configuration. Any attempt to bind to `0.0.0.0` or a specific network interface will fail at startup.
- API keys are stored as SHA-256 hashes in SQLite. The plaintext key is returned once at creation and never stored.
- All API activity is written to the application audit log with key name, endpoint, timestamp, and result summary.
- File uploads are validated for MIME type and size (100MB limit) before processing begins. Files are written to a temporary directory isolated from the application's data directory.
- The server does not support HTTPS (TLS termination on localhost is not meaningful for security and adds operational complexity). All traffic is loopback-only.

### Configuration for air-gapped deployments

Enterprise deployments in air-gapped environments (no internet access) should configure `auto_update = false` in the TOML configuration and manage updates through the organisation's existing software distribution channel (SCCM, Jamf, Ansible, etc.).

---

## Related Documents

- [`docs/ARCHITECTURE.md`](./ARCHITECTURE.md) — full system architecture, data flow, and module specifications
- [`docs/FINANCIAL_ROADMAP.md`](./FINANCIAL_ROADMAP.md) — commercial expansion strategy, persona analysis, and tier structure
- [`src-tauri/src/lib.rs`](../src-tauri/src/lib.rs) — `VerificationResult` struct and Tauri command implementations the API wrapper calls
- [`ui/src/lib/types.ts`](../ui/src/lib/types.ts) — TypeScript interfaces mirroring the Rust structs (useful for JavaScript client development)
- [`sidecar/main.py`](../sidecar/main.py) — Python ML sidecar entry point (port 8200)

---

*Last updated: 21 March 2026*
*Phase 3 planned feature — not available in current release (0.2.0-dev)*
