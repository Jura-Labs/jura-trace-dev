# BL-C2PA-004: a manifest that excludes the whole file from its hash

**Status**: Open. Target v1.3.0. Untested: nobody has yet run the attack
against Trace.
**Raised**: 22 October 2026, from the pre-v1.3.0 review of C2PA
developments.
**Severity**: Critical until tested. If Trace shows such a file as validly
signed, it shows a signature over nothing as a signature over the image.

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
