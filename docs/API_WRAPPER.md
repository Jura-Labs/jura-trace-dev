---
title: "Jura Trace local REST API"
description: "What the local REST API on 127.0.0.1:8300 does today, checked against the source: how it starts, authentication, endpoints, request and response shapes, errors, limits, and the planned CLI exit-code contract."
last-updated: 2 October 2026
status: implemented in the desktop application (v1.0 onwards). Headless binary and CLI planned for v1.2.0.
---

# Jura Trace local REST API

The desktop application runs a REST API on `http://127.0.0.1:8300`. It calls
the same verification pipeline as the app's own Verify page, so a result from
the API is the result the app would show for the same file and mode.

This document describes the server as it is in the code today. Every
statement was checked against `src-tauri/src/api/` and the pipeline it calls
on 1 October 2026. Where the server behaves in a way that is surprising or
known to be wrong, this document says so rather than describing what it ought
to do. The changes planned for v1.2.0 are in
[`docs/design/v1.2.0-headless-api-and-cli.md`](design/v1.2.0-headless-api-and-cli.md)
and are marked **(v1.2.0)** where they matter to a caller.

---

## Contents

- [Status at a glance](#status-at-a-glance)
- [How the server runs](#how-the-server-runs)
- [Authentication](#authentication)
- [Investigation modes](#investigation-modes)
- [Response envelope](#response-envelope)
- [Endpoints](#endpoints)
- [The verification result](#the-verification-result)
- [Errors](#errors)
- [Limits](#limits)
- [Schema as a public API contract](#schema-as-a-public-api-contract)
- [CLI exit-code contract (planned)](#cli-exit-code-contract-planned)
- [Examples](#examples)
- [Related documents](#related-documents)

---

## Status at a glance

| | Today (v1.1.0) | v1.2.0 plan |
|---|---|---|
| Where it runs | Inside the desktop app, only while the app is open | Also as a separate `jura-trace-api` binary with no window |
| Address | `127.0.0.1:8300`, fixed | Same by default; `--port`, and an explicit opt-in to listen beyond loopback |
| Getting an API key | **No supported way.** See [Authentication](#authentication) | `jura-trace-api keys add --name <label>` |
| Configuration file | None. Nothing reads one | None planned |
| Readiness endpoint | None; use `GET /api/v1/health` | [`GET /api/v1/ready`](#checking-it-is-up) |
| Trust band in the response | None; computed only on screen | [`verdict`](#verdict) on every verification result |
| CLI | None | `jura verify`, with the exit codes below |
| Licence tier gate | None. Every installation runs the API | No change planned |

There is no `jura-trace-api` binary, no `jura-trace --api` flag and no TOML
configuration in any released version. Earlier versions of this document
described all three; they were never built.

---

## How the server runs

The API starts inside the desktop application's start-up, in `run()` in
`src-tauri/src/lib.rs`, when the application is built with the `api` Cargo
feature, which is on by default. It is not started any other way.

- **Address.** It binds to `127.0.0.1:8300`. The address is fixed in code
  (`src-tauri/src/api/mod.rs`, `start_server`), so it is not reachable from
  other machines and there is no setting that changes this.
- **Lifetime.** It stops when the application quits.
- **Port already in use.** If `8300` is taken, for example by a second copy
  of the app, the server logs a warning and does not start. The app carries
  on without it and nothing tells the user. A caller sees a refused
  connection. **(v1.2.0)** The headless binary makes a failed bind a fatal
  error.
- **The analysis sidecar.** Most detectors run in a Python sidecar that the
  app starts on a loopback port of its own (a random one in release builds). The API reaches it through the app;
  callers never talk to it directly. Until it has started, results come back
  [degraded](#response-envelope).
- **Swagger UI** is served at `http://127.0.0.1:8300/swagger-ui/` and the
  OpenAPI document at `http://127.0.0.1:8300/openapi.json`. The Swagger assets
  are compiled into the application, so the page makes no requests off the
  machine. Neither needs a key.

### Checking it is up

```bash
curl -s http://127.0.0.1:8300/api/v1/health
```

```json
{
  "status": "ok",
  "version": "1.1.0",
  "sidecarAvailable": true,
  "uptimeSeconds": 412
}
```

`status` is always `"ok"` when the server answers. `sidecarAvailable` is
`false` while the sidecar is still starting and whenever it is unreachable;
results produced while it is `false` are degraded. This response is **not**
wrapped in the [envelope](#response-envelope).

`health` asks the sidecar over HTTP on every call and can take seconds, so it
is the wrong thing to poll. **(v1.2.0)** `GET /api/v1/ready` answers from
state the server already holds, needs no key, and is safe to poll:

```bash
curl -s http://127.0.0.1:8300/api/v1/ready
```

```json
{
  "data": {
    "server": "ready",
    "sidecar": "ready",
    "sidecarSince": "2026-11-02T09:14:07Z",
    "sidecarPort": 51873,
    "detail": null
  },
  "apiVersion": "1.0",
  "degraded": false
}
```

| `sidecar` | Meaning |
|---|---|
| `starting` | Started, not yet answering. Verifications run with reduced detectors |
| `ready` | Answering |
| `absent` | None running: not found, not asked for, or stopped by the app's power saver until the next verification |
| `failed` | Its process exited. The state changes within about a second of the exit |

`sidecarSince` is when it entered that state, or `null` when there is no
time to give. `detail` is a sentence when the state needs explaining,
otherwise `null`. `degraded` is `true` unless `sidecar` is `ready`. The
status is always `200`; read the body.

---

## Authentication

Every endpoint except `GET /api/v1/health`, `GET /api/v1/ready` (v1.2.0),
`/openapi.json` and `/swagger-ui/` needs an API key:

```
Authorization: Bearer jt_<key>
```

The `jt_` prefix is required. Keys are stored as SHA-256 hashes in the app's
SQLite database; the raw key is shown once, at creation, and never again.
A missing, malformed, unknown or revoked key gets `401` with the code
`Unauthorized`.

### Obtaining a key: the honest position

**In v1.1.0 there is no supported way for a user to obtain a key.**

- On first start the app creates a key named `bootstrap`. Its full value is
  never stored or displayed: only its first eight characters are written to
  the application log.
- The Settings panel that would show and create keys is switched off in v1.x
  (`V1_SHOW_API_KEYS = false` in `ui/src/lib/featureFlags.ts`).
- `POST /api/v1/auth/keys` creates keys, but itself needs a key.

So the API answers, and every authenticated call fails. **(v1.2.0)**
`jura-trace-api keys add --name <label>` creates a key directly in the
database, prints it once and exits, without a window and without an existing
key.

### Managing keys

With a key, keys can be created, listed and revoked through the
[`/api/v1/auth/keys` endpoints](#key-management). There is no endpoint to
change a key's rate limit after creation.

---

## Investigation modes

The mode decides which detectors run. It is sent as a multipart form field
named `mode` on the upload routes, or a JSON field `mode` on
`/api/v1/verify/url`. **(v1.2.0)** It may be sent as a query parameter
instead, on all three verify routes.

| Value | Runs | Notes |
|---|---|---|
| `quick` | EXIF and C2PA only; the sidecar is not called | `fast` is accepted as a synonym |
| `standard` | Adds the core sidecar detectors | |
| `deep` | The full automatic detector set | `archival` is accepted as a synonym; it was retired on 22 April 2026 and runs exactly what `deep` runs |

Three behaviours a caller needs to know:

1. **The default depends on the route.** With no `mode`, `POST
   /api/v1/verify` and `POST /api/v1/verify/batch` run `deep`, but `POST
   /api/v1/verify/url` runs `standard`. Send `mode` explicitly.
2. **Unrecognised values become `standard` without an error.** That includes
   case variants: `Deep` or `DEEP` runs `standard`. Use the lower-case values
   above. The mode actually used is returned in the result's `mode` field.
3. **In v1.1.0, `mode` as a query parameter is ignored.** `POST
   /api/v1/verify?mode=deep` runs the route default. Earlier versions of this
   document used that form in every example. **(v1.2.0)** The query parameter
   is read. If the query and the body both carry a mode they must be the same
   value; if they differ the request is refused with `400 InvalidParameter`.

---

## Response envelope

Successful responses from every endpoint except `health` and
`protect/sign` are wrapped:

```json
{
  "data": { "...": "the endpoint's result" },
  "apiVersion": "1.0",
  "degraded": false
}
```

- `apiVersion` is the version of this envelope, currently always `"1.0"`. It
  is not the application version; that is `data.provenance.engineVersion` on
  verification results.
- `degraded` is a boolean. There is no reason field.

### What `degraded` means

On `POST /api/v1/verify` and `POST /api/v1/verify/url`, `degraded` is `true`
when **all** of these hold: the content is an image, the mode is not
`quick`, and neither the ELA nor the deepfake classifier produced a result.
In practice that means the sidecar was not available. A degraded response is
still `200` with real results from the detectors that did run, chiefly EXIF
and C2PA, and its `overallTrust` is computed from those.

On `POST /api/v1/verify/batch` the flag is per item, and the rule omits the
image check, so a non-image file in a batch is reported as degraded even
though those detectors were never expected to run. Treat a batch item's
`degraded` as meaningful for images only.

---

## Endpoints

All paths are under `http://127.0.0.1:8300`. Upload routes take
`multipart/form-data`; the others take and return `application/json`.

| Method and path | Purpose |
|---|---|
| `GET /api/v1/health` | Liveness and sidecar status (no key) |
| `GET /api/v1/ready` | **(v1.2.0)** Sidecar state from the server's own record, cheap to poll (no key) |
| `POST /api/v1/verify` | Verify one uploaded file |
| `POST /api/v1/verify/url` | Download a URL and verify it |
| `POST /api/v1/verify/batch` | Verify up to 20 uploaded files |
| `POST /api/v1/protect/sign` | Add C2PA Content Credentials to a file |
| `POST /api/v1/protect/fingerprint` | Perceptual hashes of an image |
| `POST /api/v1/protect/watermark/embed` | Not available. `503` in v1.1.0; **(v1.2.0)** the route is gone, `404` |
| `POST /api/v1/protect/watermark/extract` | Not available. `503` in v1.1.0; **(v1.2.0)** the route is gone, `404` |
| `POST /api/v1/claims/check` | Check a claim against the knowledge base; normally `503` |
| `GET /api/v1/stats` | Counts from the local database |
| `POST /api/v1/auth/keys` | Create a key |
| `GET /api/v1/auth/keys` | List keys |
| `DELETE /api/v1/auth/keys/{keyId}` | Revoke a key |
| `GET /openapi.json` | OpenAPI document (no key) |
| `GET /swagger-ui/` | Swagger UI (no key) |

### POST /api/v1/verify

Multipart fields:

| Field | Required | Notes |
|---|---|---|
| `file` | yes | The media file. The type is detected from its content, not its name |
| `mode` | no | See [modes](#investigation-modes). Default `deep` |

Other fields are ignored. Returns the envelope with a
[verification result](#the-verification-result) in `data`.

| Status | When |
|---|---|
| `200` | Analysed. Includes degraded results and low trust scores |
| `400` | No `file` field, an empty file, or malformed multipart. `BadRequest` for all three in v1.1.0; **(v1.2.0)** `MissingField`, `EmptyFile` and `BadRequest` respectively, plus `InvalidParameter` and `UnsupportedParameter` |
| `401` | Key problem |
| `413` | **(v1.2.0)** `PayloadTooLarge`: the body is over 200 MB. v1.1.0 answers `413` with no JSON body |
| `422` | The file could not be read or processed (`FileSystem`, `C2pa`). **(v1.2.0)** Also `UnsupportedFormat`, for content that is not an image, video, audio or document type the pipeline analyses; v1.1.0 answers `200` with `contentType: "unknown"` and a score no detector stands behind |
| `429` | Rate limit |
| `500` | Internal failure |
| `503` | The analysis service failed in a way the pipeline could not degrade around (`ServiceUnavailable`) |

### POST /api/v1/verify/url

JSON body:

```json
{ "url": "https://example.org/photo.jpg", "mode": "deep" }
```

The server downloads the URL itself, then verifies it as above. Only `http`
and `https` are accepted. A URL whose host is written as a loopback or
private address (`localhost`, `127.0.0.1`, `10.x`, `192.168.x`, `172.16.x` to
`172.31.x`, `169.254.x`) is refused with `400`, and each redirect, up to five,
is checked the same way. In v1.1.0 the check reads the host as written and
does not resolve it, and it misses IPv6 forms such as `[::1]`, so it is not a
complete guard against reaching local services. **(v1.2.0)** IPv4 and IPv6
addresses are parsed, not matched as text, and a hostname is looked up: if it
resolves only to local or private addresses the URL is refused, and the
download connects to the addresses that were checked. Two limits remain. A
name that does not resolve on this machine is passed to the HTTP client,
because behind a proxy only the proxy resolves public names. And on a
redirect the new host is checked but could change its answer before the
connection.
The download times out after 30 seconds.
Default mode **`standard`**. Statuses as for `POST /api/v1/verify`.

### POST /api/v1/verify/batch

Multipart fields: one or more files, each in a field named `files` (a field
named `file` is also accepted), and an optional `mode` (default `deep`)
applied to all of them.

- At most **20** files per request; more is `400`. Empty file parts are
  skipped.
- Files are verified **one after another**, not in parallel.
- One file failing does not fail the request. **(v1.2.0)** That includes a
  file of an unsupported type, which fails alone with its reason in `error`.

```json
{
  "data": {
    "items": [
      {
        "filename": "a.jpg",
        "success": true,
        "result": { "...": "verification result" },
        "error": null,
        "degraded": false
      },
      {
        "filename": "b.bin",
        "success": false,
        "result": null,
        "error": "...",
        "degraded": false
      }
    ],
    "total": 2,
    "succeeded": 1,
    "failed": 1
  },
  "apiVersion": "1.0",
  "degraded": false
}
```

Items are in upload order. The envelope's own `degraded` is always `false`
here; read each item's flag, with the caveat under
[What `degraded` means](#what-degraded-means).

### POST /api/v1/protect/sign

Adds a C2PA manifest to the uploaded file, signed with the installation's
local certificate (the per-install Sovereign mode), with the action
`c2pa.created`.

| Field | Required | Notes |
|---|---|---|
| `file` | yes | The file to sign |
| `creator_name` | yes | Creator or rights holder |
| `license` | no | SPDX licence identifier |

On success the response is the signed file itself, **not** JSON, as
`application/octet-stream` with `Content-Disposition: attachment;
filename="<name>_signed.<ext>"`.

| Status | When |
|---|---|
| `200` | Signed file in the body |
| `400` | Missing `file` or `creator_name`, or an empty file |
| `409` | `TimestampChoiceRequired`: the app is in Standard network mode and nobody has yet chosen whether signatures should carry a trusted timestamp. The API cannot ask, so it refuses. Sign once in the app, or set it in Settings, Network Access, then retry |
| `422` | `C2pa`: the format cannot be signed or the certificate failed |

### POST /api/v1/protect/fingerprint

Multipart field `file`, an image. Returns three 64-bit perceptual hashes as
16-character hex strings:

```json
{
  "data": {
    "hashes": [
      { "algorithm": "aHash", "hashHex": "f0e0c0a080604020" },
      { "algorithm": "dHash", "hashHex": "..." },
      { "algorithm": "pHash", "hashHex": "..." }
    ]
  },
  "apiVersion": "1.0",
  "degraded": false
}
```

A non-image is `422 UnsupportedFormat`; an image that cannot be hashed is
`422 FingerprintFailed`.

### POST /api/v1/protect/watermark/embed and /extract

Invisible watermarking was withdrawn before v1.0 and has not returned. In
v1.1.0 both routes exist, both return `503 ServiceUnavailable`, and both are
listed in the OpenAPI document with their former fields. **(v1.2.0)** They
are compiled out: the paths return `404` and the OpenAPI document does not
list them. The code is kept behind the `watermark` Cargo feature, which is
off.

### POST /api/v1/claims/check

```json
{ "claim": "The photograph was taken in 2023.", "context": "optional text" }
```

Passes the claim to the sidecar's claim checker, which needs a local Ollama
model. Jura Trace does not install Ollama, so on a normal installation this
returns `503 ServiceUnavailable`. Do not build on it.

### GET /api/v1/stats

```json
{
  "data": {
    "totalAssets": 42,
    "totalVerifications": 18,
    "totalFingerprints": 38,
    "c2paSignedCount": 12
  },
  "apiVersion": "1.0",
  "degraded": false
}
```

### Key management

**`POST /api/v1/auth/keys`**, JSON body `{ "name": "CI pipeline", "rate_limit": 100 }`.
`name` must not be empty; `rate_limit` is requests per minute, default 100,
minimum 1. The request fields are **snake case**, unlike every response: the
request type is not renamed, so a body sending `rateLimit` gets the default
of 100. Response:

```json
{
  "data": {
    "keyId": "550e8400-e29b-41d4-a716-446655440000",
    "key": "jt_...",
    "name": "CI pipeline",
    "rateLimit": 100,
    "createdAt": "2026-10-01T12:00:00+00:00"
  },
  "apiVersion": "1.0",
  "degraded": false
}
```

`key` is shown this once.

**`GET /api/v1/auth/keys`** returns `data` as a list of
`{ keyId, name, rateLimit, revoked, createdAt }`. Raw keys and hashes are
never returned.

**`DELETE /api/v1/auth/keys/{keyId}`** revokes a key and returns
`{ "revoked": true }` in `data`. Requests with that key get `401` from then
on. An unknown `keyId` also returns `200`.

---

## The verification result

`data` on the verify routes, and `result` on batch items, is the same
`VerificationResult` the desktop app uses (`src-tauri/src/verify/types.rs`),
serialised in **camelCase**. Fields that do not apply are `null`, and some
are omitted when absent.

### Fields most callers need

| Field | Type | Meaning |
|---|---|---|
| `overallTrust` | number, 0 to 1 | The composite trust score. Higher means fewer concerns found. It is not a probability that the content is genuine |
| `mode` | string | The mode that actually ran: `quick`, `standard` or `deep` |
| `contentType` | string | `image`, `video`, `audio`, `document` or `unknown` |
| `sourceType` | string | `upload` for uploads; the URL route records its own value |
| `detectorsRun` | string[] | The detectors that produced a result for this file |
| `c2paValid` | boolean or null | `null` when the file carries no Content Credentials |
| `c2paManifest`, `c2paChain` | object or null | The active manifest, and the full chain including ingredients |
| `exifAnalysis` | object or null | EXIF findings, each with a severity |
| `metadataFlags` | string[] | Short metadata warnings |
| `inputSha256` | string or null | SHA-256 of the bytes analysed |
| `provenance` | object | What produced this result; see below |

### `verdict`

In v1.1.0 the band the app shows beside the score is computed in the
frontend and is **not** in the API response. **(v1.2.0)** Every verification
result carries it, computed once in `src-tauri/src/verify/verdict.rs`, and
the app's own screen and reports read the same field:

```json
"verdict": {
  "band": "uncertain",
  "score": 0.82,
  "ceilingApplied": "noPositiveAuthenticitySignal",
  "bandBoundaries": { "trusted": 0.7, "uncertain": 0.4 }
}
```

| `band` | On screen | When |
|---|---|---|
| `trusted` | High Trust | Score at or above 0.7 and no ceiling applies |
| `uncertain` | Moderate Trust | Score from 0.4 up to 0.7, or a higher score with a ceiling |
| `untrusted` | Low Trust | Score below 0.4 |
| `inconclusive` | Inconclusive | Image content where neither ELA nor the deepfake detector ran, at any score |

`score` is `overallTrust` again. `ceilingApplied` is `null` when the band is
simply the score's band, and otherwise says why it is not:

| `ceilingApplied` | Meaning |
|---|---|
| `insufficientSignal` | The core image detectors did not run: the sidecar was unavailable, or the mode was `quick`. The band is `inconclusive` |
| `noPositiveAuthenticitySignal` | The score alone would be `trusted`, but nothing positive supports it: no camera MakerNote, no valid Content Credentials, no recognised camera make and model with clean EXIF |
| `deepfakeInconclusive` | The score alone would be `trusted`, but the deepfake detector was inconclusive |
| `deepfakeSynthetic` | The score alone would be `trusted`, but the deepfake detector judged the image synthetic |

A ceiling only ever lowers `trusted` to `uncertain`, or replaces the band
with `inconclusive`. Do not band `overallTrust` yourself: a score of 0.82 can
be `uncertain`, and a client that bands the number alone will disagree with
the app on those files. A band is a summary of evidence for a person to
weigh, not a finding that the content is genuine or false.

### Detector results

Each detector that ran has its own object, otherwise `null`: `elaResult`,
`noiseResult`, `copyMoveResult`, `deepfakeResult`, `clipResult`,
`nprResult`, `jpegGhostResult`, `segmentedElaResult`,
`shadowConsistencyResult`, `colourTemperatureResult`,
`spliceBoundaryResult`, `dctAnalysisResult`, `fourierAnalysisResult`,
`platformFingerprintResult`, `contentTypeResult`, `inputQuality`,
`filenameAnalysis`, `pdfProvenance`, `thumbnailCheck`, `imageMetadata`.
Several also have a flat score: `elaScore`, `noiseScore`, `copyMoveScore`,
`deepfakeScore`. Image-producing detectors return their heat maps as base64
inside their objects, which makes responses large.

`aiGenerator` names the generator when the file's Content Credentials declare
one. Video and audio analysis was withdrawn from v1.0, so `videoMetadata`,
`audioMetadata`, `videoDeepfakeResult`, `transcriptionResult`,
`claimCheckResult` and `claimVerdict` are normally `null`.

The full schema of each object is in `src-tauri/src/verify/types.rs` and
`src-tauri/src/sidecar.rs`, and mirrored for TypeScript in
`ui/src/lib/types.ts`. The OpenAPI document does not describe the
verification result in detail.

### `provenance`

Present on every verification result. Use it to record exactly what produced
a result.

```json
"provenance": {
  "engineVersion": "1.1.0",
  "sidecarVersion": "1.1.0",
  "modelHashes": {
    "deepfakeClassifier": "<sha256>",
    "univfdProbe": "<sha256>"
  },
  "verificationMode": "deep",
  "timestampUtc": "2026-10-01T12:00:00Z"
}
```

| Field | Type | Meaning |
|---|---|---|
| `engineVersion` | string | The application version |
| `sidecarVersion` | string or null | `null` when the sidecar was unavailable |
| `modelHashes.deepfakeClassifier` | string or null | SHA-256 of the classifier model; `null` when not loaded |
| `modelHashes.univfdProbe` | string or null | SHA-256 of the CLIP probe model; `null` when not loaded |
| `verificationMode` | string | `quick`, `standard` or `deep` |
| `timestampUtc` | string | RFC 3339 time the verification completed |

### `methodology`

An older block with overlapping content, kept because the PDF and case
exports read it: `pipelineVersion`, `sidecarVersion`, `classifierModelHash`,
`univfdProbeModelHash`, `analysisMode`, `analysedAt`. New integrations should
read `provenance`.

---

## Errors

Every error has a JSON body:

```json
{
  "code": "BadRequest",
  "message": "Missing 'file' field"
}
```

`code` is the machine-readable part and is stable across releases; branch on
it, not on `message`. The type also defines `requestId`, but the server does
not set it, so it is never present. A `429` adds `rateLimitInfo`.

| `code` | Status | Meaning |
|---|---|---|
| `BadRequest` | 400 | Malformed request, too many batch files, unsafe URL, or a validation failure. In v1.1.0 also a missing field and an empty file |
| `MissingField` | 400 | **(v1.2.0)** A required multipart field was not sent, or a batch had no files |
| `EmptyFile` | 400 | **(v1.2.0)** The uploaded file had no bytes |
| `InvalidParameter` | 400 | **(v1.2.0)** `mode` in the query and in the body disagree |
| `UnsupportedParameter` | 400 | **(v1.2.0)** `mime_type` or `concurrency` was sent; neither is implemented |
| `PayloadTooLarge` | 413 | **(v1.2.0)** The request body is over 200 MB |
| `Unauthorized` | 401 | No usable key |
| `TimestampChoiceRequired` | 409 | Signing in Standard network mode before the timestamp choice is made |
| `FileSystem` | 422 | The file could not be read or written |
| `C2pa` | 422 | A Content Credentials operation failed |
| `UnsupportedFormat` | 422 | Fingerprinting a non-image. **(v1.2.0)** Also verifying content of a type the pipeline does not analyse |
| `FingerprintFailed` | 422 | Fingerprinting could not decode the image |
| `RateLimitExceeded` | 429 | See [Limits](#limits) |
| `Internal` | 500 | An internal failure, including database errors |
| `ServiceUnavailable` | 503 | The analysis service failed, or the feature is not available |

In v1.1.0 `BadRequest` covers several different caller mistakes. From v1.2.0
they have the separate codes above, with the same HTTP statuses as before
except where the table says otherwise. A client written against v1.1.0 that
matches on `BadRequest` for an empty file or a missing field needs the new
codes added.

---

## Limits

| Limit | Value |
|---|---|
| Request body | 200 MB, for single and batch uploads alike |
| Files per batch | 20 |
| URL download timeout | 30 s |
| Rate limit | Per key, a token bucket of `rateLimit` requests (default 100) refilled every 60 s |

When a request is over the limit the response is `429`:

```json
{
  "code": "RateLimitExceeded",
  "message": "Rate limit of 100 requests per minute exceeded. Retry after 37 seconds.",
  "rateLimitInfo": { "limit": 100, "remaining": 0, "resetSeconds": 37 }
}
```

with `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`
(seconds until reset, not a timestamp) and `Retry-After` headers. Successful
authenticated responses also carry the three `X-RateLimit-*` headers.

**Verifications run one at a time.** The pipeline holds a single lock for
the whole of each verification, shared with the desktop app's own Verify page,
so concurrent requests to the verify routes queue behind each other and
behind anything the user is verifying in the app. Set client timeouts with
that in mind. There is no per-request `mime_type` or `concurrency` parameter.
In v1.1.0 sending them has no effect. **(v1.2.0)** Sending either as a query
parameter on a verify route returns `400 UnsupportedParameter` naming it,
instead of being silently ignored. Other unknown query parameters are
ignored.

Browser requests are allowed only from the app's own origins
(`localhost:1420`, `localhost:8300` and their `127.0.0.1` forms); other web
pages cannot call the API from a browser.

---

## Schema as a public API contract

From v1.0 the JSON shape of every response in this document is a public API
contract, and the C2PA Validator Conformant award makes the verification
result an evidence-bearing record. Within v1.x:

- New fields may be added at any time. Clients must ignore fields they do not
  know.
- Existing fields are not renamed and their types do not change. A field that
  is a string does not become nullable.
- A field is removed only after at least one minor release in which it is
  still sent (possibly as `null`) with a `CHANGELOG.md` note, and not before
  v2.0.
- `/api/v1/` paths stay for at least one full release after a v2.0.

The contract is the wire format as implemented. Where this document and the
server disagree, the server is right and the document is the bug. Schema
changes are recorded in `CHANGELOG.md` with the prefix `[schema]`.

---

## CLI exit-code contract (planned)

**(v1.2.0)** The `jura` command-line client does not exist yet. It will be a
thin client of this API, and its exit codes are published now so that scripts
can be written against them. This is the single authoritative table; it
replaces the two inconsistent tables earlier versions of this document
carried. Codes 0 to 6 keep their published meanings, with the two
corrections below. New codes are only ever added, never reassigned.

| Code | Name | Meaning |
|---|---|---|
| 0 | success | The command completed. For `verify`, an analysis was produced, **whatever the verdict** |
| 1 | usage | Bad arguments, an unknown subcommand, or conflicting flags |
| 2 | unreachable | Could not connect to the API |
| 3 | auth | No key, a malformed key, or a rejected key |
| 4 | file | A local input problem: missing, unreadable, empty, or over the size limit |
| 5 | format | The server rejected the content as unsupported |
| 6 | server | The server failed (HTTP 5xx) |
| 7 | timeout | The request exceeded `--timeout`, or `--wait-ready` expired |
| 8 | incomplete | `--require-complete` was given and the response was degraded |
| 20 | verdict | `--fail-on` was given and the verdict met the threshold |

Codes 1 to 8 mean no usable verification was produced; 0 and 20 mean one was.
A low trust score is not a failure: without `--fail-on`, a file that scores
0.02 exits 0 and the caller reads the verdict from the output.

The two corrections to what was published before:

- **6 is server errors only.** An earlier table also mapped a degraded
  response to 6. A degraded response is a `200` with real results, and a
  caller who needs completeness asks for it with `--require-complete` and gets
  8.
- **5 follows the error `code`, not the HTTP status.** An earlier table tied 5
  to HTTP 422 and another to HTTP 400. The server returns 400 for several
  unrelated problems, so the CLI reads `code` and falls back to the status
  only for codes it does not know.

---

## Examples

All assume a key in `JT_KEY`, including its `jt_` prefix. See
[Obtaining a key](#obtaining-a-key-the-honest-position) for why that is not
yet straightforward.

### cURL

```bash
# One file, full detector set
curl -s -H "Authorization: Bearer $JT_KEY" \
  -F "file=@photo.jpg" -F "mode=deep" \
  http://127.0.0.1:8300/api/v1/verify | jq '.data.overallTrust, .degraded'

# A URL, with the mode stated (its default is standard)
curl -s -H "Authorization: Bearer $JT_KEY" -H "Content-Type: application/json" \
  -d '{"url":"https://example.org/photo.jpg","mode":"deep"}' \
  http://127.0.0.1:8300/api/v1/verify/url

# Three files in one request
curl -s -H "Authorization: Bearer $JT_KEY" \
  -F "files=@a.jpg" -F "files=@b.png" -F "files=@c.webp" -F "mode=deep" \
  http://127.0.0.1:8300/api/v1/verify/batch | jq '.data.items[] | {filename, success}'
```

### Python

```python
import os
import requests

API = "http://127.0.0.1:8300/api/v1"
headers = {"Authorization": f"Bearer {os.environ['JT_KEY']}"}

with open("photo.jpg", "rb") as fh:
    r = requests.post(
        f"{API}/verify",
        headers=headers,
        files={"file": fh},
        data={"mode": "deep"},  # a form field, not a query parameter
        timeout=300,
    )

if r.status_code != 200:
    err = r.json()
    raise SystemExit(f"{r.status_code} {err['code']}: {err['message']}")

body = r.json()
result = body["data"]
print(result["overallTrust"], result["mode"], body["degraded"])
print(result["provenance"]["engineVersion"])
```

### JavaScript (Node 18 or later)

```javascript
import { readFile } from 'node:fs/promises';

const form = new FormData();
form.append('file', new Blob([await readFile('photo.jpg')]), 'photo.jpg');
form.append('mode', 'deep');

const res = await fetch('http://127.0.0.1:8300/api/v1/verify', {
  method: 'POST',
  headers: { Authorization: `Bearer ${process.env.JT_KEY}` },
  body: form,
});
const body = await res.json();
if (!res.ok) throw new Error(`${res.status} ${body.code}: ${body.message}`);
console.log(body.data.overallTrust, body.degraded);
```

---

## Related documents

- [`docs/design/v1.2.0-headless-api-and-cli.md`](design/v1.2.0-headless-api-and-cli.md): the v1.2.0 design for the headless binary, the API corrections and the CLI
- [`backlog/BL-API-001-the-headless-api-is-documented-and-does-not-exist.md`](../backlog/BL-API-001-the-headless-api-is-documented-and-does-not-exist.md): why this document was rewritten
- [`src-tauri/src/api/`](../src-tauri/src/api/): the server; `routes.rs` for the handlers, `error.rs` for error codes
- [`src-tauri/src/verify/types.rs`](../src-tauri/src/verify/types.rs): `VerificationResult`, `Provenance` and `MethodologyRecord`
- [`src-tauri/tests/api_integration.rs`](../src-tauri/tests/api_integration.rs): integration tests against the router
- [`docs/ARCHITECTURE.md`](ARCHITECTURE.md): the wider system
