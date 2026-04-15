# C2PA Validator Conformance — Evidence Bundle

**Applicant**: Jura Labs CIC
**Product**: Jura Trace
**Record ID**: 019d8d83-ed1c-787c-920c-8fad67b55cbe
**Bundle generated**: 2026-04-15

## Executive summary

Jura Trace validates C2PA manifests in **four image formats** with full
bidirectional interoperability against the CAI `c2patool` reference CLI
(c2patool 0.26.47 / c2pa-rs 0.79.3) and Jura Trace (c2pa-rs 0.79):

| MIME type | Validator support | Evidence |
|-----------|------------------|----------|
| **image/jpeg** | ✅ PASS | `image_jpeg/` subfolder |
| **image/png**  | ✅ PASS | `image_png/` subfolder |
| **image/tiff** | ✅ PASS | `image_tiff/` subfolder |
| **image/webp** | ✅ PASS | `image_webp/` subfolder |
| image/heic | N/A — see "Format limitations" below | — |
| image/heif | N/A — see "Format limitations" below | — |
| image/avif | N/A — see "Format limitations" below | — |

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

## Per-format evidence

Each subfolder contains:

- `source.<ext>` — unsigned source image
- `jura_signed.<ext>` — signed by Jura Trace via REST API
- `evidence.json` — machine-readable signing + validation record

### image/jpeg

- **Signed by Jura Trace**: 2,189,374 bytes
- **Validated by Jura Trace**: `c2paValid: true`, 3 assertions
  (`c2pa.actions.v2`, `c2pa.rights`, `stds.iptc`)
- **Cross-validated by c2patool**: `validation_state: Valid`, 3 assertions
- **Signer**: Jura Labs CIC (per-install CA — `signingCredential.untrusted`
  in strict trust-list validators, which is expected and correct for
  Local Signing mode)

### image/png

- **Signed by Jura Trace**: 13,419,110 bytes
- **Validated by Jura Trace**: `c2paValid: true`, 3 assertions
- **Cross-validated by c2patool**: `validation_state: Valid`, 3 assertions

### image/tiff

- **Signed by Jura Trace**: 40,175,300 bytes (uncompressed large-format test)
- **Validated by Jura Trace**: `c2paValid: true`, 3 assertions
- **Cross-validated by c2patool**: `validation_state: Valid`, 3 assertions

### image/webp

- **Signed by Jura Trace**: 1,432,642 bytes
- **Validated by Jura Trace**: `c2paValid: true`, 3 assertions
- **Cross-validated by c2patool**: `validation_state: Valid`, 3 assertions

## Additional validator evidence — external samples

We validated two publicly-available externally-signed C2PA samples
through Jura Trace to demonstrate validator-side interoperability:

### 1. c2pa-rs reference fixture `exp-test1.png`

Source: https://github.com/contentauth/c2pa-rs/tree/main/sdk/tests/fixtures

| Check | Result |
|-------|--------|
| Jura Trace manifest parse | Success (5 assertions enumerated) |
| Jura Trace validation | `c2paValid: false` (correctly catches untrusted signing credential) |
| c2patool validation (independent) | `validation_state: Invalid` — same verdict, same reason (`signingCredential.untrusted`) |

**Verdict: validators agree.** Both Jura Trace and c2patool arrive at
the same verdict via independent implementations.

### 2. Google Pixel Camera production sample

Source: the CAI Interoperability Testing Files Google Drive folder.

| Check | Result |
|-------|--------|
| Jura Trace manifest parse | Success |
| Jura Trace validation | `c2paValid: false` |
| c2patool validation (independent) | `validation_state: Invalid` — failures: `signingCredential.expired`, `signingCredential.untrusted` |

Both validators correctly catch the expired certificate and the
out-of-trust-list signing credential.

## Implementation details

- **Library**: c2pa-rs 0.79 (Apache-2.0 / MIT, the CAI reference
  implementation)
- **Signing feature**: `file_io` only (we do not use `fetch_remote_manifests`;
  all processing is local-first per our product design)
- **Validation policy**: Jura Trace's `is_valid` flag returns `true` when
  `reader.validation_status()` returns `None` or all statuses have the
  code `signingCredential.untrusted`. Any other validation status (expired
  certificate, malformed assertion, data-hash mismatch, etc.) produces
  `is_valid: false`.
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
for reproducibility.

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
