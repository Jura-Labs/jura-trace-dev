# FP Telemetry Endpoint — Design Document

**Status**: Design complete, Phase A scaffold landed, solicitor initial response received  
**Author**: API Engineer agent  
**Date**: 2026-04-07 (initial), 2026-04-09 (legal-posture update)  
**Backlog ref**: #1 — "FP telemetry to Jura Labs endpoint"  
**Related**: `docs/compliance/dpia-template.md`, `scripts/train_classifier.py`,  
`src-tauri/src/telemetry/mod.rs`, `docs/legal/fp-telemetry-solicitor-brief.md`,  
`docs/legal/fp-telemetry-solicitor-response-2026-04-09.md`

---

## Executive summary

Jura Trace users can report false positives locally via the "Report false positive" modal. This document designs the optional, opt-in path that uploads those reports to a Jura Labs server endpoint so the training pipeline can consume them for quarterly model retrains.

**Key decisions at a glance**:

| Decision | Choice | Rationale |
|---|---|---|
| Auth model | Per-install anonymous UUID (`jt_inst_<uuid>`) | No account required; enables right-to-erasure without PII |
| Encoding | JSON array of `f32` for feature vector | Compatible with existing `serde_json` stack, no new deps |
| Hosting | Fly.io + Axum binary (same tech as local API) | Matches existing codebase; £4–£15/month at projected volumes |
| Consent | Settings toggle (default off) + one-time in-app notice | Genuine opt-in satisfying GDPR Article 6(1)(a) |
| Rate limit | 10 reports/hour per install UUID | Token bucket; prevents corpus poisoning |
| Idempotency | `report_id` (UUID v4) included in payload; server dedupes | Client can retry safely after network failure |

---

## Part A — API contract

### A.1 Endpoint

```
POST https://telemetry.juralabs.org/v1/fp-reports
```

The server is a separate Axum binary deployed on Fly.io, **not** the local port-8300 API. The local API on 8300 is inward-facing (third-party local integrations); the telemetry endpoint is outward-facing (client → Jura Labs).

### A.2 Authentication

**Model: per-install anonymous UUID transmitted as a bearer token.**

On first launch (or first consent), the client generates a UUID v4 and stores it in `config.json` under the key `telemetry_install_id`. This UUID is prefixed `jt_inst_` to make its provenance clear in server logs.

```
Authorization: Bearer jt_inst_550e8400-e29b-41d4-a716-446655440000
```

**Justification for this model over alternatives:**

- *No auth (pure anonymous)*: Server cannot honour right-to-erasure requests without a client handle. Rejected.
- *Per-user account/email*: Requires PII collection, login flow, and account infrastructure. Disproportionate for an opt-in telemetry feature. Rejected.
- *Per-install UUID (chosen)*: No PII. UUID is not tied to identity. Enables erasure (client sends `DELETE /v1/fp-reports?install_id=<uuid>`). Survives OS reinstall if `config.json` is backed up; generates a fresh ID if config is lost, which is acceptable.

The install UUID is **not** logged in the local SQLite audit log (that log is the user's own record; the UUID is a telemetry infrastructure detail only).

### A.3 Request

**`POST /v1/fp-reports`**

```
Content-Type: application/json
Authorization: Bearer jt_inst_<install-uuid>
X-Jura-Schema-Version: 1
```

```typescript
// TypeScript interface form — canonical definition
interface FpTelemetryPayload {
  // ── Identity & deduplication ────────────────────────────────────────────
  /** UUID v4 generated locally when the report was created. Used for idempotent retries. */
  report_id: string;

  /** Schema version for forward compatibility. Currently always 1. */
  schema_version: number;

  // ── Classification context ──────────────────────────────────────────────
  /** Reason code from the in-app modal. One of:
   *  modern_codec | social_media | scanner | computational_photography | other */
  reason_code: string;

  /** MIME type of the analysed file (e.g. "image/jpeg"). No filename or path. */
  mime_type: string | null;

  /** Deepfake score (0.0–1.0) at the time the false-positive was flagged. */
  deepfake_score: number | null;

  /** Three-way verdict string: "authentic" | "inconclusive" | "synthetic". */
  deepfake_verdict: string | null;

  // ── Feature vector ──────────────────────────────────────────────────────
  /** 80–84 element float array from the deepfake pipeline's feature extractor.
   *  Order and semantics are versioned with schema_version.
   *  null when the deepfake detector did not run (quick mode, video, PDF). */
  feature_vector: number[] | null;

  /** Number of elements in feature_vector. Used by the server to detect
   *  schema drift between client versions. */
  feature_vector_length: number | null;

  // ── Temporal ─────────────────────────────────────────────────────────────
  /** Calendar date of the report (ISO 8601, date-only: YYYY-MM-DD).
   *  Truncated to day to avoid linking reports to individual sessions. */
  reported_date: string;

  /** Jura Trace application version that produced this report. */
  app_version: string;
}
```

**What is intentionally absent from the payload:**

| Field | Reason omitted |
|---|---|
| Filename / file path | Direct personal data; could identify the user's subject matter |
| Raw image bytes | Would make the endpoint a covert upload mechanism; not needed for training |
| EXIF GPS coordinates | Special category adjacent; even approximate location is personal data under UK DPA 2018 |
| Full ISO-8601 timestamp | Hour/minute precision could link reports to identifiable sessions; date-only is sufficient for seasonality analysis in training |
| `reason_note` (free text) | User-supplied; may accidentally contain PII. Excluded from telemetry. Stored locally only. |
| `verification_id` / `file_hash` | Local identifiers; useless to the server and create a linkage risk |
| Signal scores JSON (full) | Replaced by the normalised `feature_vector`; raw scores are an implementation detail |

### A.4 Response

**Success (201 Created)**

```json
{
  "accepted": true,
  "report_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Report received. Thank you for helping improve Jura Trace detection accuracy."
}
```

**Duplicate (200 OK — idempotent)**

```json
{
  "accepted": false,
  "report_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Report already received."
}
```

**Rate limited (429 Too Many Requests)**

```json
{
  "error": "rate_limited",
  "retry_after_seconds": 3600,
  "message": "Too many reports from this installation. Please try again later."
}
```

**Invalid schema (400 Bad Request)**

```json
{
  "error": "invalid_payload",
  "details": "schema_version 0 is no longer supported. Upgrade Jura Trace.",
  "message": "Payload rejected."
}
```

**Error codes table**:

| HTTP | `error` field | Meaning |
|---|---|---|
| 201 | — | Report accepted |
| 200 | — | Already received (idempotent retry) |
| 400 | `invalid_payload` | Malformed JSON or unsupported schema version |
| 401 | `unauthorized` | Missing or malformed install UUID |
| 429 | `rate_limited` | Burst limit exceeded |
| 500 | `server_error` | Server-side ingestion failure; client should retry later |

### A.5 Erasure endpoint

```
DELETE /v1/fp-reports?install_id=<uuid>
Authorization: Bearer jt_inst_<uuid>
```

Deletes all reports associated with the install UUID. Responds 200 with a count of deleted records. This is the right-to-erasure mechanism. The client exposes a "Delete my telemetry data" button in Settings.

### A.6 Rate limiting

- **Limit**: 10 reports per install UUID per hour (token bucket, refills at 10/hour).
- **Why per-install rather than per-IP**: mobile/roaming users change IP; the install UUID is the stable identity for rate enforcement.
- **Corpus poisoning mitigation**: the server also applies a global rate limit of 500 reports/minute across all clients. Reports with a `feature_vector_length` outside `[80, 100]` are rejected (400) as schema drift rather than silently ingested.

### A.7 Idempotency

The client includes `report_id` (the same UUID stored in the local `false_positive_reports.id` column). The server maintains a unique index on `report_id`. On receipt of a duplicate, it returns 200 (not 201) with `"accepted": false`. The client treats both 200 and 201 as success — it marks the report as uploaded in the local DB.

---

## Part B — Payload design

### B.1 Field-by-field justification

**`report_id`**: The UUID already exists in the local `false_positive_reports` table. Reusing it for idempotency avoids generating a second identifier.

**`schema_version`**: Integer starting at 1. When the feature vector grows (e.g. 80→84 elements in Sprint 29 Track 3), increment to 2. The server refuses version 0 with a descriptive error; it accepts the current version and logs unknown future versions for forward-compatible processing. This is a simple integer rather than semver because the payload schema is a single flat object — major changes are breaking; there are no minor/patch semantics.

**`feature_vector`**: JSON array of `f32` values. Alternatives considered:

- *Base64-encoded little-endian binary*: smaller on the wire (~80×4 = 320 bytes → 430 bytes base64 vs ~1.2 KB JSON). Rejected — the volume is low enough that saving 800 bytes per report is not worth the added complexity of encoding/decoding and the loss of human-readability during debugging.
- *Protobuf*: no new dependencies in Rust without adding `prost`; rejected per constraints.
- *JSON array of f32 (chosen)*: works with existing `serde_json`. Server can ingest with any language. Human-readable in logs during Phase B staging.

**`feature_vector_length`**: Sent separately so the server can detect drift without parsing the array. If `feature_vector.len() != feature_vector_length`, the report is flagged for manual review rather than rejected outright (the user's intent is still valid even if the client version has a bug).

**`reported_date`**: `YYYY-MM-DD` only. The full `created_at` timestamp stored locally is not transmitted. This is a deliberate privacy measure: date-granularity is sufficient for the training pipeline (seasonal distribution analysis) and does not allow session linkage.

**`app_version`**: Allows the training pipeline to bucket reports by model version, so a report from an install running v0.9.0 (GBM v4 + UnivFD v8) is not mixed with reports from a future v1.1.0 with different thresholds.

### B.2 Schema versioning strategy

When a breaking change is needed (new required field, changed encoding, feature vector length change):

1. Increment `schema_version` in the client's `FpTelemetryReport` struct.
2. Server adds handling for the new version in an `ingest_v{N}` function; retains `ingest_v{N-1}` for the transition period (one major release cycle, approximately 6 months).
3. Old schema versions are deprecated with a 400 response after the transition period, instructing users to upgrade.

Non-breaking additions (new optional fields) do not require a version bump — the server ignores unknown fields via `#[serde(deny_unknown_fields)]` disabled (i.e., unknown fields are tolerated on the server).

---

## Part C — Privacy and legal posture

> **Legal status (updated 2026-04-09)**: the seven questions in this
> Part and in the "Questions for review" section at the bottom of
> this document were sent to a UK solicitor as
> `docs/legal/fp-telemetry-solicitor-brief.md`. The solicitor's
> **initial preliminary response** is captured in
> `docs/legal/fp-telemetry-solicitor-response-2026-04-09.md`. That
> response is marked INITIAL and will be superseded by a formal
> written advice memo once the full Phase B review bundle (privacy
> notice draft, DPIA supplement, Fly.io DPA cover sheet, reconstruction
> assessment) is prepared. The engineering direction below reflects
> the preliminary response and is sufficient to proceed with Phase B
> design work; it is not sufficient to actually collect telemetry
> from real users until the formal memo is received.

### C.1 Applicable law

Jura Labs is a UK Community Interest Company. UK GDPR (retained) and the UK Data Protection Act 2018 apply. The telemetry feature involves transmission of data from users' machines to a Jura Labs server.

The per-install UUID is a **pseudonymous identifier** under Article 4(5) UK GDPR — the solicitor's initial response explicitly confirmed this classification. The 80–84-element feature vector is **not in itself personal data** per the solicitor's initial inspection (subject to an ml-data-scientist reconstruction-feasibility assessment — see Q2 follow-up). However, because the install UUID is a pseudonymous handle that groups reports from the same user, the overall dataset still constitutes personal data under Article 4(1) and must have a lawful basis.

### C.2 Lawful basis

**Chosen basis: Consent (Article 6(1)(a) UK GDPR).** *Confirmed by solicitor initial response 2026-04-09.*

The solicitor noted that Article 6(1)(f) **legitimate interests** could also defensibly support the processing given the product-improvement purpose and the minimal data collected, but recommended **consent anyway** on the grounds that consent gives stronger legal cover and clearer user control. Rationale for taking the solicitor's recommendation rather than defaulting to legitimate interests:

- Consent is unambiguous and removes the need for the Article 6(1)(f) three-part balancing test (necessity, legitimate interest, not overridden by user rights).
- Users of a product explicitly marketed as "local-first" would not reasonably expect data to leave their device under a legitimate-interest basis. Consent aligns with user expectations.
- Consent is clear, specific, and freely given (the feature is off by default; the app continues to function fully without it). Consent can be withdrawn at any time via the Settings toggle.
- Withdrawing consent triggers the erasure path described in C.3, which is much cleaner than the balancing-test unwind required under legitimate interests.

The consent notice must be:

1. Presented before the feature is activated (Settings toggle, or the setup wizard opt-in checkbox).
2. Written in plain English.
3. Specific about what is collected (feature vectors, reason codes, app version, install UUID) and why (quarterly model retrains to reduce false positives).
4. Linked to the full privacy notice at `https://juralabs.org/privacy`.

**Draft consent notice** (for Settings toggle tooltip and setup wizard):

> Help improve Jura Trace by sharing anonymous detection data. When you report a false positive, the system score for that image — but not the image itself, filename, or any personal details — is sent to Jura Labs. This is used to improve future detection accuracy. You can withdraw consent and delete your data at any time. [Learn more at juralabs.org/privacy]

### C.3 Right to erasure

Per Article 17 UK GDPR. Mechanism:

1. User clicks "Delete my telemetry data" in Settings → Jura Trace (Reports section).
2. Client sends `DELETE /v1/fp-reports?install_id=<uuid>` to the telemetry endpoint.
3. Server deletes all rows with that `install_id`. Returns 200 with a count.
4. Client clears the `telemetry_install_id` from `config.json` and generates a fresh UUID on next upload (breaking the link to deleted data).
5. If the telemetry endpoint is unreachable, the client queues the erasure request and retries. The user is informed that deletion is pending.

Note: data that has already been included in a quarterly retrain batch (incorporated into a trained model file) cannot be surgically removed. The privacy notice must state this clearly. The ICO's guidance on the right to erasure acknowledges that data incorporated into aggregated statistical analyses may not be individually removable; this position applies here.

### C.4 Data retention policy

- Raw reports: **18 months** from receipt date. After 18 months, reports are deleted from the server database. This covers two quarterly retrain cycles with headroom.
- Retrained model files: retained indefinitely as part of the production model artefacts. The contribution of individual reports cannot be removed from a trained model.
- Erasure requests: retained as a deletion audit record (install_id + deletion timestamp) for 90 days, then deleted.

### C.5 Non-warranty statement

The telemetry feature is an investigative aid for model improvement, not a statement about the user's specific use case. Consistent with the pattern established in `sidecar/app/services/claim_checker.py`:

> False positive reports submitted via Jura Trace are used solely for the purpose of improving detection models in future releases. Submission of a report does not constitute a warranty, guarantee, or representation by Jura Labs CIC that the submitted analysis result was incorrect. Reports are reviewed in aggregate; individual reports are not evaluated, acted upon, or responded to.

This statement should appear in:
- The consent notice (condensed).
- The full privacy notice on juralabs.org.
- The `docs/compliance/dpia-template.md` telemetry supplement (see C.7 below).

### C.6 GDPR Article 30 records of processing

Juralabs must add a new entry to its Article 30 records-of-processing register for the telemetry endpoint. Key fields:

| Field | Value |
|---|---|
| Processing activity | Voluntary FP telemetry upload |
| Controller | Jura Labs CIC |
| Data categories | Pseudonymous diagnostic data (install UUID, feature vector, reason code, app version, report date) |
| Lawful basis | Consent (Article 6(1)(a)) |
| Purpose | Training data improvement for AI detection models |
| Retention | 18 months |
| Recipients | No third parties. Server hosted on Fly.io (data processor). DPA with Fly.io required. |
| Transfers | Data stored in EU/EEA Fly.io region (Frankfurt); no transfers outside UK/EEA adequate countries |

### C.7 DPIA requirement

A full DPIA as per `docs/compliance/dpia-template.md` is **recommended but not legally required** for the telemetry endpoint in isolation. *Confirmed by solicitor initial response 2026-04-09: the existing DPIA skeleton is sufficient for the Phase B pilot, with a formal DPIA review gated before Phase C production launch.*

Assessment against ICO triggers:

| Trigger | Applies? |
|---|---|
| Systematic and extensive profiling | No — aggregate model training, not individual profiling |
| Large-scale processing of special category data | No — feature vectors are numeric, not biometric |
| Systematic monitoring of publicly accessible areas | No |
| New technologies | Partially — AI training pipeline |
| Automated decision-making with legal/significant effects | No |

Fewer than two triggers apply. However, because this feature changes Jura Trace from a fully local-first product (no outbound data) to one with an optional outbound channel, completing a proportionate DPIA supplement is required before the Phase C production launch. The supplement should be appended to `docs/compliance/dpia-template.md` under a "Telemetry Processing Activity" section and submitted to the solicitor for review alongside the Phase B review bundle.

**Resolved (2026-04-09)**: the previously flagged question about whether the ICO's "public interest" ground for AI research (Schedule 1, para 4 DPA 2018) could apply is **moot** — the solicitor recommended consent anyway for its stronger legal cover, so the research-ground analysis no longer gates Phase B. Consent is the sole lawful basis; see C.2.

---

## Part D — Server-side ingestion sketch

### D.1 Hosting recommendation: Fly.io + Axum binary

**Chosen**: Fly.io (Frankfurt region) running a standalone Axum binary — the same tech stack as the local port-8300 API.

**Rationale**:

- *Cloudflare Workers + D1*: D1 SQLite is read-optimised; bulk inserts during a batch upload would be slower. Workers are JS-first; writing Axum Rust for Workers requires `worker-rs` which is not battle-tested. Rejected.
- *Self-hosted VPS*: Requires Juralabs to manage TLS, OS updates, and uptime monitoring. At the current team size (solo + agents), operational overhead is too high for a non-revenue-critical endpoint. Rejected.
- *Fly.io + Axum (chosen)*: Existing Rust expertise. Shared Cargo workspace patterns. £4–£10/month on a shared-cpu-1x 256 MB instance. Fly.io is GDPR-compliant with an EU region (fra). Data Processing Agreement available.

### D.2 Storage schema (Postgres via fly-postgres)

SQLite is used locally because it requires no server. For the server, Postgres is chosen over SQLite because:

1. Concurrent writes from many clients would require WAL mode and careful locking on SQLite; Postgres handles this natively.
2. `fly-postgres` is a managed offering with daily backups.
3. The quarterly retrain script can query Postgres directly or pull a JSONL export.

```sql
CREATE TABLE fp_reports (
    id              UUID PRIMARY KEY,            -- report_id from client
    install_id      TEXT NOT NULL,               -- jt_inst_<uuid>
    schema_version  SMALLINT NOT NULL DEFAULT 1,
    reason_code     TEXT NOT NULL,
    mime_type       TEXT,
    deepfake_score  DOUBLE PRECISION,
    deepfake_verdict TEXT,
    feature_vector  JSONB,                       -- array of f32
    feature_vector_length SMALLINT,
    reported_date   DATE NOT NULL,               -- YYYY-MM-DD from client
    app_version     TEXT NOT NULL,
    received_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retrain_batch   TEXT                         -- set to "YYYY-QN" when included in a retrain
);

CREATE UNIQUE INDEX idx_fp_reports_id ON fp_reports(id);
CREATE INDEX idx_fp_reports_install ON fp_reports(install_id);
CREATE INDEX idx_fp_reports_date ON fp_reports(reported_date);
CREATE INDEX idx_fp_reports_retrain ON fp_reports(retrain_batch) WHERE retrain_batch IS NULL;

-- Erasure audit log (install_id only; no content retained)
CREATE TABLE erasure_log (
    install_id      TEXT NOT NULL,
    deleted_count   INTEGER NOT NULL,
    deleted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

The `retrain_batch` column is set to `"2026-Q3"` (or similar) when a report is included in a retrain. Reports with `retrain_batch IS NOT NULL` can be archived or deleted after the 18-month retention window without risk of losing unprocessed data.

### D.3 Feeding into the quarterly retrain pipeline

The training pipeline lives in `scripts/train_classifier.py`. The quarterly retrain process:

1. Run a Postgres query to export unprocessed reports:
   ```sql
   SELECT reason_code, mime_type, deepfake_score, deepfake_verdict,
          feature_vector, app_version, reported_date
   FROM fp_reports
   WHERE retrain_batch IS NULL
     AND schema_version = 1
     AND array_length(feature_vector::json, 1) BETWEEN 80 AND 100
   ORDER BY received_at;
   ```

2. These reports are **false positives** — the model scored something as synthetic/suspicious when it was authentic. They are added to the training corpus as authentic-class examples with their feature vectors, bypassing the image-based feature extraction step.

3. A new script `scripts/ingest_telemetry_reports.py` (Phase C work) reads the Postgres export, validates feature vectors, and writes them into the corpus format consumed by `train_classifier.py`.

4. After the retrain, set `retrain_batch = '2026-Q3'` on all ingested rows.

5. The new model is evaluated on the held-out validation set (`scripts/build_validation_test_set.py`) before promotion to production.

**Important caveat**: FP reports submitted by users are self-reported — the user believes the image is authentic, but there is no ground-truth verification. The training pipeline should weight these reports at 50% of a verified authentic training example, or require a minimum of 5 reports with the same `reason_code` for a given image type before including them. The exact weighting strategy is a decision for the ml-data-scientist agent at Phase C.

### D.4 Monitoring and alerting minimum

- **Fly.io metrics**: CPU, memory, request count. Threshold: >1000 reports/minute triggers a PagerDuty/email alert (corpus poisoning attempt).
- **Error rate**: >5% 4xx/5xx over a 5-minute window → alert.
- **Database size**: daily check that `fp_reports` row count is within expected range for the user base.
- **Erasure lag**: if an erasure request cannot be confirmed within 24 hours, log and alert.

At Phase B (staging, <100 reports/day), a simple Fly.io log-based alert via `fly logs --app jura-telemetry | grep ERROR` piped to email is sufficient. Full Prometheus/Grafana setup is deferred to Phase C.

### D.5 Cost estimate

| Volume | Fly.io instance | Postgres | Egress | Total/month |
|---|---|---|---|---|
| 100 reports/day | shared-cpu-1x 256MB (£3.50) | fly-postgres free tier | <1 GB | ~£4 |
| 1,000 reports/day | shared-cpu-1x 256MB (£3.50) | fly-postgres performance-2x (£7) | ~10 GB (£0.90) | ~£12 |
| 10,000 reports/day | shared-cpu-2x 512MB (£14) | fly-postgres performance-4x (£29) | ~100 GB (£9) | ~£52 |

At the projected user base for Phase B (pilot institutions, <200 users), 100 reports/day is a generous estimate. Cost is not a concern at this stage.

---

## Part E — Client-side wiring plan

### E.1 Where the upload happens

After `mark_false_positive` succeeds (local SQLite write confirmed), the client:

1. Checks `telemetry_enabled()` — returns false if the user has not opted in.
2. Builds a `FpTelemetryReport` from the local DB row via `build_report_from_fp_row()`.
3. Adds the report to an in-memory queue (bounded to 100 items).
4. A background Tokio task drains the queue, sending one report at a time.

Upload is **not** blocking the IPC response to the frontend. The user sees "False positive reported" immediately regardless of upload success.

### E.2 Opt-in mechanism

Two touchpoints:

1. **Setup Wizard step 5** ("Ready" summary screen): checkbox "Help improve detection accuracy by sharing anonymous data" — unchecked by default. Links to the consent notice. If checked, sets `telemetry_enabled = true` in `config.json` and generates the install UUID.

2. **Settings page** — "Detection improvement" section: toggle "Share anonymous false positive data with Jura Labs" with a two-sentence explanation and a "Learn more" link to `juralabs.org/privacy`. The toggle reflects the current state and can be turned off at any time.

Turning the toggle off:
- Sets `telemetry_enabled = false` in `config.json`.
- Clears the in-memory queue.
- Does **not** automatically delete server-side data (user must click "Delete my telemetry data" separately — this is a deliberate separation so users understand the two distinct actions).

### E.3 Network failure handling

The background upload task uses exponential backoff:

| Attempt | Delay before retry |
|---|---|
| 1 (initial) | Immediate |
| 2 | 30 seconds |
| 3 | 5 minutes |
| 4 | 1 hour |
| 5+ | 24 hours (max) |

After 7 days of failed retries, a report is marked `upload_failed_permanently` in the local DB and removed from the queue. The user is **not** notified (this is a background telemetry feature; surfacing network failures would be noise). The report remains in the local `false_positive_reports` table.

The upload queue is persisted to a separate SQLite table `telemetry_upload_queue` (Phase B work — not in the current scaffold). Until that table exists, the queue is in-memory and is lost on restart. This is acceptable for Phase B where upload reliability is not a hard requirement.

### E.4 User data visibility and control

In Settings → Reports (the existing section that shows `get_false_positive_stats` count):

- Show count of locally stored FP reports.
- Show count uploaded vs pending vs failed.
- "Delete my telemetry data" button (sends the erasure DELETE request).
- "Export my reports" button (downloads a JSON file of the local `false_positive_reports` table — this is the local data only, not the server copy).

### E.5 Interaction with the local `false_positive_reports` table

The local table is the source of truth. Telemetry upload is additive:

- `false_positive_reports.id` becomes the `report_id` in the telemetry payload.
- A new column `uploaded_at TEXT` is added to the local table in a schema migration (Phase B). `NULL` means not yet uploaded; an ISO-8601 timestamp means uploaded successfully.
- The `get_false_positive_reports()` DB function is extended to return this field.

The Tauri `mark_false_positive` command signature is **not changed** — the upload is triggered by a separate mechanism (the background task watching the local DB for rows where `uploaded_at IS NULL` and `telemetry_enabled = true`).

---

## Part F — Phased rollout plan

### Phase A: Local-only queue (current state)

**Status**: In progress — scaffold landed.

What exists:
- Local `false_positive_reports` SQLite table.
- `mark_false_positive` Tauri IPC command.
- `src-tauri/src/telemetry/mod.rs` scaffold with `FpTelemetryReport`, `build_report_from_fp_row`, `serialize_report`, and `telemetry_enabled` stub.
- No network code. No opt-in UI. Default off.

Phase A is the permanent state until the Phase B gate is passed.

**Phase A → B gate**: All of the following must be true:

1. Jura Trace v1.0 has shipped (pilot testing complete, public release).
2. The privacy notice at `juralabs.org/privacy` has been updated to cover telemetry and reviewed by a solicitor (or at minimum by the Jura Labs CIC board).
3. The DPIA supplement has been appended to `docs/compliance/dpia-template.md`.
4. The Fly.io server binary is deployed to a staging environment with a distinct URL (`https://telemetry-staging.juralabs.org`).
5. At least one pilot user has provided informed consent in a test session.
6. A DPA with Fly.io has been signed.

### Phase B: Opt-in upload to staging endpoint

What to build:
- Settings toggle and setup wizard checkbox (UI).
- Install UUID generation and `config.json` storage.
- Background upload task with exponential backoff.
- HTTP client in `src-tauri/src/telemetry/mod.rs` (replace the TODO stub).
- Server binary deployed to Fly.io staging.
- Erasure endpoint and "Delete my telemetry data" button.
- Schema migration adding `uploaded_at` to `false_positive_reports`.

Target: Phase B ready for 3–5 pilot users to test the full consent + upload + erasure flow before any production data is collected.

**Phase B → C gate**: All of the following must be true:

1. Staging endpoint has handled at least 50 real reports without data loss or schema errors.
2. Erasure flow tested end-to-end (submit → delete → confirm deletion).
3. Privacy notice live on juralabs.org.
4. No open legal questions (see Part C "Questions for review" below).
5. Quarterly retrain pipeline integration tested with synthetic server-side data.
6. Decision made on tier gating (see Phase C note below).

### Phase C: Production endpoint

What to build:
- Switch client to production URL (`https://telemetry.juralabs.org`).
- `scripts/ingest_telemetry_reports.py` Postgres → corpus ingestion script.
- Monitoring/alerting (Fly.io metrics, error rate alert).
- Full Prometheus/Grafana if volume justifies it.

**Tier gating decision** (open question): should telemetry upload be available to all tiers including Community, or gated to Professional and above?

Arguments for open to all: the more reports, the better the model. Community users are the largest group and most likely to see edge-case FP conditions.

Arguments for gating: Enterprise/Professional users have signed a licence agreement, giving Jura Labs a stronger legal basis for data relationships; Community users may not expect even opt-in telemetry.

**Recommendation**: open to all tiers, but only after the Phase B staging validation. The opt-in consent mechanism is the primary control, not tier gating.

---

## Questions for review

Status of each item after the solicitor's initial response (2026-04-09).
The full response is captured at
`docs/legal/fp-telemetry-solicitor-response-2026-04-09.md`.

1. ~~**Legal posture (solicitor call required)**~~ — **Resolved.** Solicitor confirmed consent is the recommended lawful basis despite legitimate interests being defensible. The Schedule 1 para 4 research ground is moot. See C.2.

2. ~~**DPA with Fly.io**~~ — **Deferred with engineering action.** Solicitor wants to review the actual signature document before the founder countersigns. Engineering action: download the Fly.io standard DPA, prepare a Juralabs-side cover sheet with CIC registration number, submit to the solicitor alongside the Phase B review bundle. Do NOT sign yet.

3. ~~**Feature vector privacy**~~ — **Open with specific engineering follow-up.** Solicitor said "appears to be not personal information" but explicitly disclaimed technical expertise on reconstruction feasibility. New work item: commission an ml-data-scientist assessment on whether the 80–84-element feature vector can be inverted into approximate pixel content. File at `docs/design/fp-telemetry-feature-vector-privacy.md`. Attach the assessment to the Phase B review bundle for solicitor confirmation.

4. ~~**Tier gating for Phase C**~~ — **Open.** Product-owner decision, not a legal question. Current Part F recommendation is "open to all tiers, opt-in is the primary control, tier gating is not a control". Default stands unless product owner overrides.

5. ~~**Retrain weighting for self-reported FP rows**~~ — **Open.** ml-data-scientist decision; not a legal question. Not blocking Phase B; must be specified before the first retrain cycle that ingests telemetry data.

**New items from the solicitor's initial response (2026-04-09)**:

6. **Privacy notice draft** — Juralabs prepares the first draft at `docs/legal/fp-telemetry-privacy-notice-draft.md`, solicitor reviews before publication. Must cover the six elements named in Q6 of the brief.

7. **DPIA Phase B supplement** — short addendum to `docs/compliance/dpia-template.md` under a "Telemetry Processing Activity" section. Solicitor considers the existing skeleton sufficient for Phase B pilot but wants a formal review before Phase C.

8. **Fly.io DPA + cover sheet** — see item 2 above.

9. **Formal DPIA review before Phase C** — solicitor to conduct. Schedule 3 months before Phase C launch decision.

---

## Appendix — Related files

| File | Relevance |
|---|---|
| `src-tauri/src/telemetry/mod.rs` | Client-side scaffold (Phase A) |
| `src-tauri/src/db.rs` | `false_positive_reports` table schema and `FalsePositiveReport` struct |
| `src-tauri/src/lib.rs` | `mark_false_positive` and `get_false_positive_stats` Tauri commands |
| `docs/compliance/dpia-template.md` | DPIA template; a telemetry supplement should be appended for Phase C |
| `scripts/train_classifier.py` | Quarterly retrain pipeline; telemetry reports feed in as Phase C work |
| `scripts/build_validation_test_set.py` | Held-out validation set; must be re-evaluated after each retrain |
| `sidecar/app/services/claim_checker.py` | Non-warranty notice pattern (lines 104–113) |
