---
title: "DPIA Phase B Supplement — FP Telemetry"
decision-id: DPIA-2026-04-09-001
date: 9 April 2026
status: DRAFT — awaiting solicitor review
parent: docs/compliance/dpia-template.md
related:
  - docs/design/fp-telemetry-endpoint.md
  - docs/legal/fp-telemetry-solicitor-response-2026-04-09.md
owner: Paul Griffiths (Director, Juralabs CIC)
---

# DPIA Phase B Supplement — FP Telemetry

**Parent document**: `docs/compliance/dpia-template.md` (template version 1.0, 25 March 2026)
**Processing activity**: Voluntary false-positive telemetry upload (Phase B pilot)
**Data controller**: Juralabs Community Interest Company (UK)
**Data processor**: Fly.io, Inc. (Frankfurt region) under standard Data Processing Agreement
**Applicable law**: UK GDPR (retained EU law) and the Data Protection Act 2018
**ICO guidance followed**: ICO "Data protection impact assessments" (seven-step structure)

---

## 1. Supplement scope

This document extends the parent DPIA template (`docs/compliance/dpia-template.md`) with a processing-activity assessment specific to the Phase B false-positive telemetry pilot. The parent template covers the local-first desktop application where no personal data leaves the user's device during core operation. This supplement covers the narrowly-scoped Phase B deviation from that baseline: an opt-in, client-initiated upload of pseudonymous false-positive diagnostic data from consenting pilot users to a Juralabs-controlled endpoint for the purpose of detector retraining. It is prepared as a separate reviewable document so it can be attached to the Phase B solicitor review bundle without altering the parent skeleton. The processing activity is named throughout as **"Voluntary false-positive telemetry upload (Phase B pilot)"**.

This supplement does not alter, override, or replace any risk assessment in the parent template. Risks identified here are additional to those in Section 5 of the parent.

---

## 2. Describe the processing

### 2.1 Nature of processing

The Phase B pilot introduces a single outbound data flow to the otherwise local-first Jura Trace application. The processing activities performed are:

1. **Collection** — when a consenting user marks a verification result as a false positive via the existing in-app "Report false positive" modal, the client assembles a telemetry payload from fields already stored in the local `false_positive_reports` SQLite table.
2. **Client-side anonymisation** — the client strips the user-supplied free-text `reason_note` field and retains only the structured fields described in §2.4 below. This step is implemented in `build_report_from_fp_row` in `src-tauri/src/telemetry/mod.rs`.
3. **Transmission** — the payload is uploaded by a background Tokio task to `https://telemetry-staging.juralabs.org/v1/fp-reports` over TLS, authenticated by a per-install pseudonymous UUID carried as a bearer token.
4. **Server storage** — the payload is persisted by the Fly.io-hosted Axum server into a Postgres table (`fp_reports`) in the Frankfurt region.
5. **Use for model retraining** — at the next quarterly retrain cycle, unprocessed rows are exported, validated, and ingested as training examples by the classifier retraining pipeline (`scripts/train_classifier.py`). Contribution to a trained model artefact is irreversible by design; see §3.4 and risk R-T6 below.
6. **Erasure** — a user who withdraws consent may delete all server-side reports associated with their install UUID via `DELETE /v1/fp-reports?install_id=<uuid>`.

### 2.2 Scope of processing

- **Population**: opt-in Phase B pilot users only. Default state is off. No scale target has been set for Phase B; the design document models cost up to 100 reports/day but makes no commitment to achieve that volume.
- **Frequency**: event-driven — a payload is generated only when a user explicitly marks a verification as a false positive. There is no periodic or background sampling. Per-install rate limit of 10 reports/hour; global rate limit of 500 reports/minute.
- **Volume per record**: approximately 1.2 KB JSON payload, dominated by the 80-to-84-element feature vector.
- **Geographical scope**: clients are located wherever Jura Trace is installed; server-side storage is restricted to the EU (Frankfurt). No transfers outside the UK/EEA adequacy perimeter.

### 2.3 Context of processing

The wider relationship is that of a desktop software vendor (Juralabs CIC) and voluntary pilot participants who have installed Jura Trace and elected to help improve its AI detection models. Jura Trace is marketed as "local-first"; the Phase B feature is a narrow, opt-in exception to that posture, and user expectations must be managed accordingly. This informs the decision (captured in the design document, Part C.2, and confirmed by the solicitor's initial response) to rely on **consent** (Article 6(1)(a) UK GDPR) rather than legitimate interests (Article 6(1)(f)), even though legitimate interests would likely be defensible.

### 2.4 Purposes of processing

The sole purpose is the improvement of Jura Trace's AI detection models through quarterly retraining of the GBM deepfake classifier and, in subsequent cycles, the UnivFD probe. The existing training corpus is drawn from synthetic generators and curated public datasets; it does not cover the edge-case real-world false positives encountered by pilot users (modern codecs, social-media recompression, consumer scanner output, computational photography artefacts). Telemetry data addresses that gap.

The data will not be used for marketing, user profiling, product personalisation, behavioural analytics, advertising, or any secondary purpose. This limitation is recorded in Part C.5 of the design document ("Non-warranty statement") and is reproduced in the consent notice.

### 2.5 Fields transmitted

The payload fields, as specified in Part A.3 of the design document, are:

| Field | Type | Notes |
|---|---|---|
| `report_id` | UUID v4 | Reused from local `false_positive_reports.id`; supports idempotent retries |
| `schema_version` | integer | Currently always `1` |
| `reason_code` | string | One of a fixed five-value set: `modern_codec`, `social_media`, `scanner`, `computational_photography`, `other` |
| `mime_type` | string or null | e.g. `image/jpeg` — no filename or path |
| `deepfake_score` | number or null | Scalar score at time of report |
| `deepfake_verdict` | string or null | `authentic` \| `inconclusive` \| `synthetic` |
| `feature_vector` | number array or null | 80–84 `f32` values extracted from the deepfake pipeline |
| `feature_vector_length` | integer or null | Server-side schema-drift detection |
| `reported_date` | string (`YYYY-MM-DD`) | Date-only; no hour, minute, second, or timezone |
| `app_version` | string | Jura Trace version string |
| (transport) install UUID | bearer token | `jt_inst_<uuid>` in the `Authorization` header; not part of the JSON body |

### 2.6 Fields explicitly NOT transmitted

The following categories are excluded by design and enforced client-side before upload:

- Raw image, video, audio, or PDF content (pixels, frames, waveforms, document text)
- File name or file path
- Local file hashes (SHA-256 or perceptual hashes)
- EXIF metadata of any kind, including GPS coordinates, device serial numbers, photographer identity, and embedded thumbnails
- Full ISO-8601 timestamps with sub-day precision
- The user's free-text `reason_note` (stored locally only; stripped during payload construction)
- Client IP address as a persisted field (IP will appear in Fly.io edge logs transiently but is not stored in the `fp_reports` table; see R-T3 below)
- Any identifier that links the install UUID to a person, account, email address, or device serial

---

## 3. Necessity and proportionality

### 3.1 Is the processing necessary to achieve the purpose?

**Yes.** The GBM classifier (currently v4, AUC-ROC 0.9868) is retrained quarterly on a corpus of 10,709 images. The false positives reported in the current corpus are exclusively synthetic or curator-selected examples. Real-world false positives encountered by users in deployment — for example, heavily recompressed social-media JPEGs misclassified as AI-generated — are not represented in the corpus. Without user-reported telemetry, the classifier cannot learn from these edge cases and the false-positive rate cannot be systematically reduced.

An alternative mechanism (asking pilot users to manually email screenshots or files) would collect considerably more personal data than the structured telemetry design and would be less reliable. The feature-vector approach collects the minimum needed to improve the model.

### 3.2 Is there a less intrusive way?

The less intrusive baseline is **no telemetry at all**, which is the current pre-Phase-B state and will remain the default-off state for all non-consenting users. The Phase B pilot is a deliberate, bounded, opt-in departure from that baseline, justified by the product-improvement purpose and tempered by the data-minimisation measures in §3.3. Users who do not consent continue to experience Jura Trace as a fully local-first application with no outbound data flow.

Alternative architectures considered and rejected (recorded in the design document, Part A.2):

- **No authentication / pure anonymous upload** — rejected because it would make the right to erasure impossible to honour.
- **Per-user account or email-based identifier** — rejected as disproportionate; it would introduce personal data collection solely to enable a feature whose current design avoids it.
- **Per-install pseudonymous UUID** (chosen) — enables erasure without collecting personal data.

### 3.3 Is data minimisation adequate?

**Yes.** Data minimisation is achieved through multiple mechanisms:

- The transmitted payload consists of approximately 84 floating-point numbers plus a fixed-vocabulary reason code, a MIME type, a date, and a version string.
- The user's free-text field (`reason_note`) is stripped before upload.
- The date field is truncated to day precision so that reports cannot be linked to individual sessions.
- The install UUID is randomly generated on the client, stored only locally and on the Juralabs server, and is never cross-referenced with any other data source.
- The client IP address is not persisted (see R-T3).
- No pixel data, file hash, file path, or EXIF data is transmitted.

The ml-data-scientist reconstruction-feasibility assessment (to be filed at `docs/design/fp-telemetry-feature-vector-privacy.md`) is a follow-up item required before the Phase B bundle is considered complete. This supplement's risk assessment (§5, R-T2) is provisional pending that assessment.

### 3.4 Is the retention period proportional?

**Yes, with a caveat.** The retention schedule, per Part C.4 of the design document:

| Artefact | Retention | Rationale |
|---|---|---|
| Raw `fp_reports` rows (server) | 18 months from receipt | Covers two quarterly retrain cycles with headroom; sufficient for audit and error investigation |
| Erasure audit log (`install_id` + timestamp only) | 90 days | Long enough to confirm erasure has completed; short enough to avoid becoming a secondary dataset |
| Trained model weights derived from ingested data | Indefinite | A trained classifier artefact is not practically decomposable into the individual reports that contributed to its weights |

The caveat is the third row: once a report has contributed to a retrained model, its contribution cannot be surgically removed even if the user subsequently exercises the right to erasure. This is addressed in two ways:

1. The consent notice will state this clearly in plain English (design document Part C.2 draft consent wording; full privacy notice still to be drafted).
2. The ICO's published position on aggregated statistical analyses (see ICO guidance on the right to erasure) acknowledges that data incorporated into aggregate analyses may not be individually removable, and that this does not of itself defeat Article 17.

Residual exposure is limited by the 18-month raw-data retention window: after that window, the only remaining artefact is the trained classifier, in which individual contributions are not recoverable.

---

## 4. Consultation

### 4.1 Internal consultation

| Party | Role | Input |
|---|---|---|
| API Engineer agent | Endpoint and server design | Authored `docs/design/fp-telemetry-endpoint.md` |
| ml-data-scientist agent | Reconstruction-feasibility assessment | Engaged; assessment due at `docs/design/fp-telemetry-feature-vector-privacy.md` (follow-up item — flagged as dependency for bundle completeness) |
| Data controller (Paul Griffiths, Director, Juralabs CIC) | Final decision authority | Reviewed and approved Phase A scaffold; supplement awaiting sign-off (§7) |

### 4.2 External consultation

| Party | Role | Status |
|---|---|---|
| UK solicitor | Initial legal review of the Phase B design | **Initial response received** 9 April 2026 — captured in `docs/legal/fp-telemetry-solicitor-response-2026-04-09.md`. Marked INITIAL; not final advice |
| UK solicitor | Formal review of the Phase B bundle | **Pending**: awaits the full bundle (this supplement, privacy notice draft, Fly.io DPA cover sheet, reconstruction assessment) |
| UK solicitor | Formal DPIA review | **Scheduled**: required before Phase C production launch (§8) |
| Fly.io, Inc. | Data processor | Standard Data Processing Agreement to be countersigned following solicitor review of the signature document (per solicitor's initial response, item 2) |

### 4.3 Data subject consultation

No direct data-subject consultation has been conducted at the time of writing. This is consistent with ICO guidance: consultation of data subjects is proportionate to the risk of the processing, and for a low-volume, opt-in pilot relying on explicit consent, advance consultation is not strictly required. **Recommendation**: a feedback item should be added to the Phase B pilot testing protocol so that the first cohort of consenting pilot users are asked whether the consent notice is clear and whether they feel adequately informed. Their feedback should be incorporated into the privacy notice before Phase C.

---

## 5. Risks identified and assessed

The risk assessment uses the ICO's qualitative approach: each risk is described, assigned likelihood and severity ratings, paired with mitigations, and assigned a residual rating. Risks are numbered with the `R-T` prefix to distinguish them from the parent template's `R` series.

| # | Risk | Likelihood | Severity | Inherent | Mitigation | Residual |
|---|---|---|---|---|---|---|
| R-T1 | Unintentional capture of personal data in the user-supplied `reason_note` free-text field | Medium | Moderate | MEDIUM | `reason_note` is stripped client-side in `build_report_from_fp_row` (`src-tauri/src/telemetry/mod.rs`) before payload construction; never leaves the device | LOW |
| R-T2 | Feature-vector reconstruction — inversion of the 80–84 floats into approximate pixel content, which would re-introduce personal data | Low (pending assessment) | Moderate-to-major (contingent) | MEDIUM | Vector is derived from hand-engineered forensic features (ELA summaries, noise statistics, frequency descriptors), not a learnt embedding trained for reconstruction. Formal assessment pending at `docs/design/fp-telemetry-feature-vector-privacy.md`. Solicitor initial response indicates "appears to be not personal information" subject to confirmation | LOW *(provisional)* |
| R-T3 | Install UUID linkage to other sources, enabling re-identification | Low | Moderate | LOW-MEDIUM | UUID is generated client-side from a cryptographically random source, stored only in `config.json` and the Juralabs server; not logged in the local SQLite audit log; not linked to any Juralabs account, licence key, or email address; user can delete and regenerate at any time | LOW |
| R-T4 | Server compromise at Fly.io exposing the `fp_reports` table | Low | Moderate | MEDIUM | Data volume is small; exposed fields are pseudonymous and non-content; TLS in transit; EU-region hosting (Frankfurt); standard Fly.io DPA in place; 18-month retention cap limits historical exposure; no pixel data or file hashes to disclose | LOW |
| R-T5 | Sub-processor chain — Fly.io engages a sub-processor that Juralabs has not vetted | Low | Moderate | LOW-MEDIUM | Standard Fly.io DPA includes sub-processor notification clauses; Juralabs to review the executed DPA and maintain a sub-processor register; solicitor to review the DPA signature document before countersignature (per solicitor's initial response, item 2) | LOW |
| R-T6 | Retain-after-erasure — user erases their reports but the data has already been incorporated into a trained classifier | Medium | Minor | LOW-MEDIUM | Consent notice and privacy notice will disclose this limitation in plain English (drafted in design document Part C.2); the ICO's guidance on aggregated statistical analyses applies; raw rows are deleted on request within the 18-month window; trained model files retain only aggregate statistical contribution which is not individually recoverable | LOW (with disclosure) |
| R-T7 | Corpus poisoning — malicious actor submits fake reports to degrade the classifier | Low-Medium | Moderate | MEDIUM | 10 reports/hour per install UUID rate limit (token bucket); 500 reports/minute global rate limit; `feature_vector_length` schema validation; outlier detection during quarterly retrain review; training pipeline weights self-reported FP rows at reduced confidence versus curated examples (exact weighting to be set by ml-data-scientist at Phase C) | LOW |

**Summary**: all residual risks are rated LOW. R-T2 is marked provisional pending the ml-data-scientist reconstruction assessment — if that assessment reaches a different conclusion, this supplement must be revised and re-submitted to the solicitor.

---

## 6. Measures to reduce risk

### 6.1 Technical measures

| Measure | Implementation |
|---|---|
| Opt-in gate (default off) | Setup wizard checkbox (unchecked) + Settings toggle; `telemetry_enabled` flag in `config.json` |
| Client-side anonymisation | `build_report_from_fp_row()` in `src-tauri/src/telemetry/mod.rs` strips `reason_note` and limits payload to the fields in §2.5 |
| Pseudonymous authentication | Per-install UUID v4 bearer token (`jt_inst_<uuid>`); no account, email, or licence-key linkage |
| Transport security | TLS 1.2+ only; HTTPS endpoint `telemetry-staging.juralabs.org` (Phase B) and `telemetry.juralabs.org` (Phase C) |
| EU-region hosting | Fly.io Frankfurt (`fra`) region; no storage outside the UK/EEA adequacy perimeter |
| Schema validation | Server rejects payloads with `feature_vector_length` outside `[80, 100]`; schema version check |
| Rate limiting | 10 reports/hour per install UUID (token bucket); 500 reports/minute global |
| UUID-based erasure endpoint | `DELETE /v1/fp-reports?install_id=<uuid>` — returns count of deleted rows; 90-day audit log |
| Retention cap | 18-month hard delete on raw rows; 90-day erasure audit retention |
| Idempotency | Server maintains unique index on `report_id`; duplicate submissions return 200, not 201; prevents double-counting and supports safe client retry |

### 6.2 Organisational measures

| Measure | Implementation |
|---|---|
| DPIA review cycle | Annual review of this supplement and the parent template by the data controller; formal review by external solicitor before Phase C |
| Solicitor engagement at each Phase gate | Phase B bundle submitted to solicitor before Phase B user sessions; Phase C bundle submitted before production launch |
| Privacy notice transparency | Plain-English consent notice presented in the setup wizard and Settings; full privacy notice at `juralabs.org/privacy` (to be published before Phase B user sessions) |
| Rights contact | The data controller (Director, Juralabs CIC) is the single point of contact for all UK GDPR rights requests; contact details published in the privacy notice |
| Data Processing Agreement | Fly.io standard DPA reviewed by solicitor and countersigned by the data controller before any Phase B user data is collected |
| Article 30 records | Juralabs' Article 30 records-of-processing register is updated with the entry shown in Part C.6 of the design document |
| Non-warranty disclosure | Consent notice and privacy notice will state that individual reports are not evaluated, acted upon, or responded to |

---

## 7. Sign-off

This supplement is submitted to the data controller for approval and to the UK solicitor for formal review as part of the Phase B review bundle. Approval is required before any Phase B user-session data is collected.

| Field | Value |
|---|---|
| **Data controller name** | Paul Griffiths |
| **Role** | Director, Juralabs Community Interest Company |
| **Approval date** | |
| **Signature** | |
| | |
| **External reviewer** | UK solicitor (formal review pending Phase B bundle) |
| **Review date** | |
| **Reviewer advice reference** | |

**Decision** (to be completed after solicitor review):

- [ ] Proceed with Phase B pilot — conditions, if any, recorded below
- [ ] Proceed with Phase B pilot subject to conditions
- [ ] Do not proceed — Phase B is blocked pending resolution

**Conditions** (if applicable):

1.
2.
3.

---

## 8. Review schedule

| Trigger | Action |
|---|---|
| Phase B pilot begins | This supplement becomes the live governance record for the processing activity |
| Any material change to payload fields, retention, or processor | Re-submit to solicitor before the change is deployed |
| 3 months before Phase C launch decision | **Formal DPIA review** by UK solicitor; this supplement is either incorporated into a Phase C version or superseded |
| Annually from approval | Data-controller review; update versioned in §9 |
| Any data-protection incident involving the telemetry endpoint | Triggers immediate re-review; incident log appended |
| ml-data-scientist reconstruction assessment lands | R-T2 re-scored; supplement revised if assessment changes the residual rating |

The Phase B pilot is explicitly **not** a substitute for the formal DPIA review required before Phase C production launch. This supplement is a proportionate governance instrument for a low-volume, opt-in pilot; a wider assessment will be required before any scale-up.

---

## 9. Cross-references

| Document | Relevance |
|---|---|
| `docs/compliance/dpia-template.md` | Parent DPIA skeleton that this supplement extends |
| `docs/design/fp-telemetry-endpoint.md` | Authoritative technical design (API contract, payload, privacy posture, server ingestion, client wiring, phased rollout) |
| `docs/design/fp-telemetry-feature-vector-privacy.md` | ml-data-scientist reconstruction-feasibility assessment — **pending**; required for Phase B bundle completeness; blocks final residual rating for R-T2 |
| `docs/legal/fp-telemetry-solicitor-brief.md` | Seven-question legal brief submitted to the solicitor |
| `docs/legal/fp-telemetry-solicitor-response-2026-04-09.md` | Solicitor's initial (preliminary) response, 9 April 2026 |
| `src-tauri/src/telemetry/mod.rs` | Client-side Phase A scaffold (`FpTelemetryReport`, `build_report_from_fp_row`, `serialize_report`) |

---

## Version history

| Version | Date | Author | Changes |
|---|---|---|---|
| 0.1 (DRAFT) | 9 April 2026 | Legal Compliance Advisor (on behalf of Juralabs CIC) | Initial draft for solicitor review as part of Phase B review bundle |
