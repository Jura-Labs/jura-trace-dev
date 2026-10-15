# BL-SILENT-002: screenshot and document AI suppression has not run since April

**Status**: Open. Found 15 October 2026, while running the 217-image golden
verdict set for the C1 step 4 bump. Not fixed: fixing it changes verdicts.
**Severity**: High. A shipped safeguard is off, and nothing reports it
except a `WARN` line in the log.
**Raised**: 15 October 2026

## What happens

`POST /forensics/content-type` classifies an image as photograph,
screenshot, document, artwork or unknown, and says whether the AI detectors
are reliable on it. The Rust pipeline is meant to suppress the GBM and CLIP
scores when they are not (`src-tauri/src/verify/pipeline.rs:1353`), which is
the fix `672bba5f` (14 April 2026) made for laundered screenshots.

The reply never parses. `ContentTypeResult` in `src-tauri/src/sidecar.rs:744`
carries `#[serde(rename_all = "camelCase")]`, so it expects
`aiDetectionSuitable`. The sidecar's model,
`sidecar/app/models/content_type.py:20`, has no alias generator and sends
`ai_detection_suitable`. serde reports a missing field, `reqwest` reports
"error decoding response body", and `pipeline.rs:957-960` logs a warning
and returns `None`.

`None` then defaults to `ai_detection_suitable = true` at
`pipeline.rs:985-989`, which means no suppression. So every image is scored
as a photograph, and `contentTypeResult` is absent from every verification
result.

## Evidence

Two golden runs on 15 October, 217 images each, debug `jura-trace-api`
against the sidecar from source: 217 of 217 requests to
`/forensics/content-type` returned 200, and 217 of 217 logged
`Sidecar content-type classification failed: Failed to parse content-type
response: error decoding response body`. The struct's attributes have not
changed since `672bba5f`, so this has been the behaviour of every release
since then. That release history has not been checked against an installed
build.

## Why nothing caught it

The same shape as BL-SILENT-001: failure is a log line and a default that
looks like success. No test sends the sidecar's real JSON through the Rust
type. The Rust tests build `ContentTypeResult` directly, and the sidecar
tests check the Python model, so each side agrees with itself.

## The fix, and why it is not a quiet one

The code change is small: match the casing on one side (drop
`rename_all` on the Rust struct, or add a camelCase alias generator to the
Python model, checking which other consumers read either), and add a test
that deserialises a captured sidecar reply into the Rust type.

But it switches suppression on for the first time. Screenshots, documents
and unknown images will stop getting AI-detection scores in their verdict,
so trust bands move on exactly the images the April change targeted. That
needs:

- a decision on whether it ships in v1.2.0 or waits, given the date rule in
  `docs/release/v1.2.0-plan.md`;
- a golden-verdict run before and after, with the moved verdicts listed
  and checked by eye rather than accepted;
- a CHANGELOG entry saying verdicts change on screenshots and documents;
- a look at `docs/calibration/univfd-v10-screenshot-retrain-plan.md`,
  which may assume the suppression works.

The `contentTypeResult` field also starts appearing in API responses,
which `docs/API_WRAPPER.md` should describe if it does not already.
