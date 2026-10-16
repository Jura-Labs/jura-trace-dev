# BL-SILENT-002: screenshot and document AI suppression has not run since April

**Status**: Fixed 15 October 2026 for v1.2.0, Paul's decision the same day.
Found while running the 217-image golden verdict set for the C1 step 4 bump.
See "Fixed" at the foot of this file.
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

## Fixed, 15 October 2026

`#[serde(alias = "ai_detection_suitable")]` on the field, the pattern the
other structs in `sidecar.rs` already use, with no `default`, so a reply
missing the field still fails rather than choosing a suppression value. The
new test `test_content_type_result_deserialise_sidecar_reply` deserialises a
reply captured from the sidecar, checks the `false` case, and checks the
field still serialises as `aiDetectionSuitable` for the frontend. With the
alias removed it fails with the production error, `missing field
aiDetectionSuitable`.

The suppression flag text no longer begins "Screenshot", because it is also
used for documents. The three AI-card test mocks used `contentType` where
the type has `category`; corrected (no snapshot changed).

### What it changes, on the 217-image golden set

Two debug `jura-trace-api` builds from the same tree, without and with the
fix, against the same sidecar environment (the lock merged in #173). The
"with" build predates only the reworded suppression flag text, which no
compared field reads.
Classification failed on 217 of 217 before and succeeded on 217 of 217
after: 192 photograph, 21 screenshot, 4 document.

- **195 images unchanged.** Every photograph is unchanged.
- **22 moved**, all of them classified screenshot or document, all from
  the corpus folders `authentic/screenshots` and
  `edge_cases/edge_double_screenshot`, so all authentic.
  - 18 from **untrusted to uncertain**. Before, the AI detectors had scored
    these authentic screenshots 0.65 to 0.82 synthetic and capped them at
    0.25 to 0.35. This is the false positive the April change was written
    for.
  - 4 stay uncertain at a new score: `screenshot_011`, `_017` and `_019`
    from 0.87 to 0.59, and `screenshot_007` from 0.54 to 0.59. Pinning the
    AI score to neutral removes the benefit an authentic-looking AI score
    gave as well as the penalty.
- 3 more screenshots (`screenshot_009`, `_013`, `_015`) were suppressed
  with no change. Their AI score was already 0.15, "authentic", and their
  trust sat at 0.55 in both runs, so a limit other than the AI score set it
  before and after.
- No deepfake, CLIP or UnivFD score changed on any image. Suppression
  works in trust scoring only; the detector results are still reported.

Two checked by eye (`screenshot_014`, classified document, and
`edge_double_screenshot_000`): both synthetic window-style screenshots, as
their folder says.

### What this does not show

The golden set has no AI-generated screenshot or document. Suppression
pins the AI score to neutral for anything classified that way, so an
AI-generated image the classifier calls a screenshot would also lose its
untrusted verdict. That is the trade-off the April design accepted, and
these 217 images cannot measure it. A set of AI-generated screenshots and
documents would.
