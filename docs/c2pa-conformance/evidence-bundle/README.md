# C2PA Validator Conformance — Evidence Bundle

**Applicant**: Jura Labs CIC
**Product**: Jura Trace
**Record ID**: 019d8d83-ed1c-787c-920c-8fad67b55cbe
**Bundle generated**: 2026-04-15

## Accuracy note (8 September 2026)

The signed artefacts in this bundle (`image_jpeg/jura_signed.jpg`,
`image_png/jura_signed.png`, and the TIFF and WebP equivalents) and the
stored `jura_trace_verify_response.json` files were produced on
2026-04-15 by Jura Trace v0.9.0-rc14 against c2pa-rs 0.79. They are a
point-in-time record of what was submitted to the C2PA Conformance
Programme, and they are deliberately not being regenerated. An evidence
archive should record what was actually submitted, not what the current
build would produce today.

Two things have changed since.

**Jura Trace's signing behaviour changed on 2026-05-22.** A
Generator-track audit moved `digitalSourceType` into the `c2pa.created`
action (it was absent from the action before), replaced the bare
`softwareAgent` string `"Jura Trace 0.9.0"` with a ClaimGeneratorInfoMap
object, and removed the `stds.iptc` assertion entirely. The assertion set
recorded under "Headline values across all four formats" below is the
pre-audit set carried by these 2026-04-15 files. It is not what Jura
Trace embeds now.

**The repo has since moved to c2pa-rs 0.90** (0.90.20 at the time of
writing). c2pa-rs 0.90 enforces the C2PA 2.x rule that a `c2pa.created`
action must carry a `digitalSourceType`. These artefacts were signed
before the 2026-05-22 audit, so their `c2pa.created` action has none.
A validator built on c2pa-rs 0.90 therefore reports a failing check with
code `assertion.action.malformed` and the explanation "c2pa.created
action must have a digitalSourceType", and validates the files as
invalid. A validator built on c2pa-rs 0.79, the version in use when the
bundle was submitted, does not.

**The Conformance award is unaffected.** `SUMMARY.json` records
`api_endpoint_under_test: "POST /api/v1/verify"`. The award is for the
validator role. The signed files here were inputs the validator was
tested against, not a claim that Jura Trace's generator is conformant.
Their present validation status changes what a reader should expect on
re-validating them. It does not change what was certified.

This bundle would not support a future Generator-track conformance
claim. That would need artefacts signed by a current build and validated
under the c2pa-rs version that build ships.

## Executive summary

Jura Trace validates C2PA manifests in **two image formats** with full
bidirectional interoperability against the CAI `c2patool` reference CLI
(c2patool 0.26.47 / c2pa-rs 0.79.3) and Jura Trace (c2pa-rs 0.79):

| MIME type | Validator support | Evidence |
|-----------|------------------|----------|
| **image/jpeg** | ✅ PASS | `image_jpeg/` subfolder |
| **image/png**  | ✅ PASS | `image_png/` subfolder |

Additional formats (TIFF, WebP) are supported by the implementation
but excluded from this submission pending full conformance evidence.
HEIC, HEIF, and AVIF are not yet supported by c2pa-rs 0.79 for signing.

## Format limitations — HEIC, HEIF, AVIF

c2pa-rs 0.79 (our underlying library, also the reference implementation
maintained by CAI) **does not currently support signing HEIC, HEIF, or
AVIF**. Attempting to sign these formats returns "type is unsupported"
at the c2pa-rs Builder level.

Additionally, the CAI Interoperability Testing Files folder (linked in
the programme's intake email) contains only `image/jpeg` samples —
no HEIC, HEIF, or AVIF reference files with embedded C2PA manifests
are publicly available for us to validate against. We also checked the
c2pa-rs GitHub test fixtures: `sample1.heic`, `sample1.heif`, and
`sample1.avif` are present but **unsigned**, intended as source inputs
rather than validator test vectors.

**Question for the C2PA Conformance Programme**: are there published
signed sample files in HEIC, HEIF, or AVIF that we can validate against?
If yes, we will expand this evidence bundle. If not, we propose narrowing
Jura Trace's claimed MIME types to the four formats with full evidence:

- `image/jpeg`
- `image/png`
- `image/tiff`
- `image/webp`

and re-submitting when signed reference samples or c2pa-rs support for
these formats becomes available.

## Legal name clarification

The Intake Form contained a discrepancy between the Applicant field
(`Jura Labs CIC`, with trailing space) and the Product DN `O` field
(`JuraLabs CIC`, no space). Our registered legal name on UK Companies
House (registration number 17117467) is **`Jura Labs CIC`** (with space).

Please treat both the Applicant field and Product DN `O` as:

```
Jura Labs CIC
```

This matches the Apple Developer Program registration (verified against
Companies House and D-U-N-S) used to issue our macOS code signing
certificate (Team ID `Y82C4P9L7F`).

## Per-format evidence — provenance note

**The primary evidence for each format is `jura_trace_verify_response.json`**
— the full, unedited HTTP response body from Jura Trace's REST API verify
endpoint (`POST /api/v1/verify`). This JSON has a schema distinctly Jura
Trace's (camelCase field names, a `detectorsRun` array, `deepfakeResult`
and `clipResult` sibling objects alongside `c2paManifest`) — provably
not c2patool output.

The programme reviewer is expected to run their own validator (c2patool
or the CAI Validation Harness) against each `jura_signed.<ext>` file.
A `c2patool_selfqa.json` file is included in each subfolder but is
labelled as internal QA only — it documents what we saw when we
cross-checked during development, not programme evidence.

### Per-format contents

Each `image_<format>/` subfolder contains:

| File | Purpose |
|------|---------|
| `source.<ext>` | Unsigned source image (pre-signing) |
| `jura_signed.<ext>` | Signed by Jura Trace — **hand this to your own validator** |
| `jura_trace_verify_response.json` | **PRIMARY EVIDENCE** — raw response from our verify endpoint |
| `c2patool_selfqa.json` | Internal QA cross-check (not programme evidence) |
| `index.json` | Per-format summary with headline values |

### Headline values across all four formats

Source images are resized to 1024 px on the longest side — a pragmatic
size for fast reviewer inspection while still exercising the full
format round-trip (header, colour profile, lossy/lossless encoding,
container structure).

| MIME | Signed bytes | Jura `c2paValid` | Jura assertions | Jura `detectorsRun` includes `c2pa` |
|------|-------------:|:----------------:|:---------------:|:-----------------------------------:|
| image/jpeg | 176,641 | ✅ true | 3 | ✅ |
| image/png  | 932,192 | ✅ true | 3 | ✅ |

**Note on `jura_trace_verify_response.json`**: the response has been
filtered to C2PA-relevant fields only (`c2paManifest`, `c2paValid`,
`detectorsRun`, `methodology`, etc.). The full unfiltered response from
the verify endpoint also contains ML forensic detector outputs
(ELA/deepfake/noise heatmap base64 images) which are not relevant to
C2PA conformance and added ~20 MB per file to the bundle. If the reviewer
wants the full response, it can be regenerated by running
`build_evidence_bundle.py` without the filter step. As of 8 September
2026 that reproduces the bundle's structure and the unfiltered response
shape, but not the values recorded here: the script signs with the
current build (the post-2026-05-22 assertion set) and validates against
c2pa-rs 0.90, so both the signed files and the verify responses it
produces differ from the stored ones. See the accuracy note at the top
of this file.

All four signed files carry the three assertions Jura Trace embedded on
signing when they were produced on 2026-04-15: `c2pa.actions.v2`
(c2pa.created), `c2pa.rights` (the supplied licence), and `stds.iptc`
(IPTC metadata block). That describes these artefacts, not current
product behaviour. The 2026-05-22 Generator-track audit changed the set,
removing `stds.iptc` (see the accuracy note at the top of this file).

The signer on all files is the per-install local certificate authority
("Jura Labs CIC") — the Local Signing mode default. This is expected to
trigger `signingCredential.untrusted` in strict trust-list validators.
Jura Trace's validator policy treats that specific status as acceptable
because our architecture deliberately uses per-install CAs for the
offline-first use case. The optional Conformant Signing mode accepts
institution-imported trust-list certificates for cross-tool validation.

## Additional validator evidence — external samples

We validated two publicly-available externally-signed C2PA samples
through Jura Trace to demonstrate inbound (external-signed) validator
interop:

- `external_samples/c2pa-rs_exp-test1.png` — the c2pa-rs reference
  fixture from https://github.com/contentauth/c2pa-rs/tree/main/sdk/tests/fixtures
- `external_samples/CAI_PixelCameraProd_PXL_20250708.jpg` — a Pixel
  Camera sample from the Interoperability Testing Files folder linked
  in the intake email

In both cases Jura Trace successfully parsed the manifest, enumerated
the assertions, and returned a validation verdict (`c2paValid: false`
in both cases — in both files the signing credential was either
untrusted on our policy or expired). The programme reviewer can verify
the expected verdicts by running their own validator against these
files.

## Implementation details

- **Library**: c2pa-rs 0.79 (Apache-2.0 / MIT, the CAI reference
  implementation)
- **Signing feature**: `file_io` only (we do not use `fetch_remote_manifests`;
  all processing is local-first per our product design)
- **Validation policy**: Jura Trace derives a tri-state validity from the
  c2pa-rs `Reader::json()` validation results:
  - **Valid** — no failures, or failures only contain `signingCredential.untrusted`
    (expected for self-signed / per-install CA certificates).
  - **Valid at signing** — failures are cert-soft only (`signingCredential.expired`
    and/or `signingCredential.untrusted`) AND active-manifest success list
    includes `timeStamp.validated` or `timeStamp.trusted` AND `claimSignature.validated`.
    This handles short-lived signing certificates (e.g. Google Pixel Camera)
    where the trusted timestamp proves the signature was issued during cert validity.
  - **Invalid** — any other failure (data hash mismatch, claim signature invalid,
    expired cert without trusted timestamp, etc.).
- **Supported outputs**: every validation result returns the active
  manifest ID, claim generator, assertion list with JSON bodies, signature
  timestamp, AI-content declaration extraction from `c2pa.actions`
  `digitalSourceType`, and issuer information.

## Reproducing this bundle

1. Start Jura Trace with the REST API enabled on port 8300
2. Set `JURA_API_KEY` env var to a valid API key
3. From repo root: `python3 /tmp/c2pa-interop/build_evidence_bundle.py`
4. Inspect each `image_*/evidence.json` for the full record

The script `build_evidence_bundle.py` is vendored alongside this bundle
for reproducibility. It reproduces the method, not the artefacts.
Re-running it on 8 September 2026 produces files signed by the current
build and validated under c2pa-rs 0.90, which differ from the 2026-04-15
artefacts stored here. See the accuracy note at the top of this file.

## Provenance of source imagery

The base source image for all 7 format conversions is a 2015 Samsung
Galaxy S5 photograph from the Jura Trace development corpus. It contains
no identifiable persons and carries an open licence allowing test use.
The same source image is converted into each target format via Pillow
11.x with pillow-heif and pillow-avif plugins, ensuring all formats are
derived from identical pixel data for controlled comparison.

## Contact

- paul@juralabs.org
- Record ID: 019d8d83-ed1c-787c-920c-8fad67b55cbe
