# BL-C2PA-004: a manifest that excludes the whole file from its hash

**Status**: Fixed for `c2pa.hash.data` (JPEG, PNG, TIFF, WebP) on 12 November
2026; box and BMFF hashes still to check. Step 4 (report to C2PA) is open.
**Raised**: 22 October 2026, from the pre-v1.3.0 review of C2PA
developments.
**Severity**: Critical until tested. If Trace shows such a file as validly
signed, it shows a signature over nothing as a signature over the image.

## What testing found (12 November 2026)

- **Whole-file exclusion** (Buchanan's own sample, and a fixture built the
  same way): c2pa-rs 0.91.3 rejects it with `assertion.dataHash.mismatch`,
  so `c2paValid` was already false. But the validation summary also saw the
  `assertion.dataHash.match` pass that c2pa-rs reports beside the failure,
  and said "Data integrity confirmed: file has not been modified" under the
  red badge. Fixed in #241.
- **Exact manifest exclusion plus a range over the scan data**, with pixels
  changed after signing: c2pa-rs passes it (`is_valid` true, only the
  informational `additionalExclusionsPresent`), and Trace showed it as
  valid, with the provenance bonus. Fixed by measuring the excluded bytes
  against the manifest store (`unsigned_bytes` in `src-tauri/src/c2pa.rs`).
- Calibration: every Trace-signed JPEG, PNG, TIFF and WebP, and all six
  conformance test vectors, exclude 4 to 12 bytes beyond the store.
- The code that builds both attack files is kept out of this repository
  until C2PA has responded (decided by Paul, 13 November 2026), as "What
  not to do" below asks. The public tests cover the measurement
  (`unsigned_gap_allows_container_overhead_only`) and the calibration
  (`trace_signed_files_have_no_unsigned_bytes`).

## Still open

- `c2pa.hash.boxes` and `c2pa.hash.bmff` exclusions are not measured.
- Step 4 below: report the second layout to C2PA / c2pa-rs (issue #2645
  asks for central hardening of `verify_hash_binding`).
- A file whose manifest is rejected still gets a forensic-only verdict,
  which can be "trusted" (Buchanan's file scored 0.78). Whether a rejected
  or partly unsigned manifest should cap the verdict is a trust-weighting
  decision, not taken here.

## What is wrong

David Buchanan, "How to Hack Time, With C2PA" (2 October 2026,
da.vidbuchanan.co.uk/blog/hacking-time.html), builds a `c2pa.hash.data`
assertion whose exclusion range covers the whole file. The signature and
its RFC 3161 timestamp are valid, but they cover nothing. He writes: "We're
effectively signing nothing at all, but as of today all the C2PA
verification tools I can find don't flag anything as unusual."

The review read c2pa-rs 0.91.3's source and found a check added in
September (PR #2653) that should reject a single whole-file exclusion with
`assertion.dataHash.mismatch` for JPEG and PNG. It also found that a
manifest listing the correct manifest range plus a large extra range would
still pass, with only an informational `additionalExclusionsPresent`. That
is a reading of the code, not a test. Trace does not show exclusion ranges
at all.

## What would fix it

1. Build both variants as test files (a whole-file exclusion; the correct
   range plus an extra range covering most of the image) for JPEG and PNG,
   and add them to the regression tests.
2. Run them through Trace and record what it shows.
3. If either reads as validly signed with no warning, flag it: when the
   excluded bytes go beyond the manifest store, say in plain words that
   part of the image is not covered by the signature, and do not let the
   result count as positive provenance evidence in the trust score.
4. Report the result to the C2PA conformance contacts if Trace, as a
   conforming validator, accepted either file.

## What not to do

- Do not publish the test files outside the repository's test fixtures
  before the C2PA has responded, if the library turns out to accept them.
