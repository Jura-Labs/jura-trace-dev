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
                   │ HTTP (127.0.0.1:8200)
┌──────────────────▼──────────────────────────┐
│      Python ML Sidecar (existing)           │
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
    "capabilities": ["ela", "noise", "copy_move", "deepfake", "rag"]
  },
  "capabilities": {
    "c2pa": true,
    "fingerprint": true,
    "exif_anomaly": true,
    "forensics": true,
    "deepfake": true,
    "rag": true
  }
}
```

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
creator_name: "Juralabs CIC"                    (string, required)
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

```json
{
  "claim": "This photograph shows flooding in Valencia on 29 October 2024.",
  "verdict": "partially_supported",
  "confidence": 0.71,
  "summary": "The DANA floods in Valencia did occur on this date. However, reverse image analysis suggests this specific photograph may originate from an earlier flooding event in a different region.",
  "sources": [
    {
      "title": "Spain floods: Valencia declares state of emergency",
      "url": "https://...",
      "source_type": "verified_news",
      "relevance": 0.89
    }
  ],
  "processing_time_ms": 2341
}
```

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
    "uncertain": 183,
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

```json
{
  "fileInfo": {
    "path": "/tmp/upload_abc.jpg",
    "name": "evidence_photo.jpg",
    "sizeBytes": 2847392,
    "mimeType": "image/jpeg",
    "modifiedAt": "2026-03-20T14:22:00Z"
  },
  "c2paResult": {
    "hasCertificate": true,
    "isValid": true,
    "issuer": "Juralabs CIC",
    "signingTime": "2026-03-15T10:00:00Z",
    "claimsCount": 3,
    "validationStatus": "valid"
  },
  "exifResult": {
    "hasExif": true,
    "anomalyScore": 0.12,
    "anomalies": [],
    "softwareTag": "Adobe Photoshop 2025",
    "gpsPresent": false
  },
  "fingerprintResult": {
    "hashes": [
      { "algorithm": "phash", "hashHex": "a3f2..." },
      { "algorithm": "ahash", "hashHex": "b7c1..." }
    ],
    "matchFound": false,
    "matchAssetId": null
  },
  "forensicsResult": {
    "available": true,
    "elaScore": 0.23,
    "noiseScore": 0.18,
    "copyMoveDetected": false,
    "compressionArtifacts": false
  },
  "deepfakeResult": {
    "available": true,
    "score": 0.08,
    "verdictLevel": "authentic",
    "verdictLabel": "Likely authentic",
    "classifierScore": 0.11,
    "classifierAvailable": true,
    "signalCount": 21,
    "flaggedSignals": []
  },
  "overallTrust": 0.91,
  "trustLabel": "High confidence — likely authentic",
  "processingTimeMs": 3241,
  "mode": "standard",
  "apiVersion": "0.2.0-dev"
}
```

**Verdict levels** (`verdictLevel` field):

| Value        | Display label               | Score range |
|--------------|-----------------------------|-------------|
| `authentic`  | Likely authentic            | 0.00–0.30   |
| `uncertain`  | Cannot determine            | 0.31–0.69   |
| `synthetic`  | Likely AI-generated         | 0.70–1.00   |

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
  "degradedReason": "Python ML sidecar unavailable at http://127.0.0.1:8200",
  "overallTrust": null,
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

# Extract key verdicts
verdict = result["deepfakeResult"]["verdictLevel"]
trust = result["overallTrust"]
anomalies = result["exifResult"]["anomalies"]

print(f"Deepfake verdict: {verdict}")
print(f"Overall trust score: {trust:.2f}")
print(f"EXIF anomalies: {len(anomalies)}")

# Flag for human review if uncertain or synthetic
if verdict in ("uncertain", "synthetic"):
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

// Usage
const result = await verifyFile('./submission_photo.jpg', 'deep');
console.log(`Trust score: ${result.overallTrust}`);
console.log(`Deepfake verdict: ${result.deepfakeResult.verdictLevel}`);
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
