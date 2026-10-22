# BL-C2PA-003: the C2PA library upgrade has not been re-checked, and the next one breaks the API

**Status**: Open. Target v1.3.0.
**Raised**: 22 October 2026, from the pre-v1.3.0 backlog and C2PA reviews.
Security register SR-34.
**Severity**: Critical. The public conformance claim (claims.md row 15)
waits on it, and so does the v1.2.0 announcement.

## What is wrong

Three things, all about `c2pa` (`src-tauri/Cargo.toml:95`, `version = "0.91"`,
0.91.3 in the lockfile and in shipped v1.2.0):

1. **SR-34 is unchecked and its text is stale.** The register still reads
   "c2pa-rs 0.79.3 vs 0.90.16" (`~/jura-brain/security/register.md:77`).
   claims.md row 15 says to re-check conformance after the bump. The
   v1.2.0 changelog records that the test vectors in
   `docs/c2pa-conformance/test-vectors` and the 217-image golden set gave
   unchanged results, but no item owns the re-check and nobody has written
   it down against the conformance record.
2. **0.91.4 is out** (19 October 2026, "Ignore SHA-1 timestamp imprints").
3. **The next breaking release deletes the deprecated API.** The
   c2pa-c-ffi v0.91.0 release notes say the release "planned for
   mid-November 2026 will delete every API still marked deprecated". Trace
   has already moved off the thread-local settings and
   `trust.trust_anchors` (`c2pa.rs:1460-1474`, `:4900`); whether anything
   else it calls is deprecated has not been counted.

Related, from the same review: 0.91.0 made the CAWG trusted identity-issuer
list empty by default, so any CAWG identity assertion now reads
`cawg.ica.untrusted_issuer`. `c2pa.rs` sets no CAWG configuration.

## What would fix it

1. Write the SR-34 re-check: run the conformance test vectors and the
   golden set on the shipped 0.91.3 build, record the results against the
   conformance record (019d8d83-ed1c-787c-920c-8fad67b55cbe), and update
   the register row and claims.md row 15. Until that is done, row 15 stays
   out of public copy.
2. Move to 0.91.4 and repeat step 1's runs.
3. Build with deprecation warnings visible and list every deprecated call.
   Fix them before the mid-November release, so v1.3.0 is not pinned to a
   version that stops getting fixes.
4. Decide what Trace shows for CAWG identity assertions now that none is
   trusted by default. "Not checked" is honest; "untrusted" alone reads as
   a fault in the file.

## What not to do

- Do not loosen the version pin to pick up 0.92 automatically. The
  deprecation removal is a breaking change.
- Do not mark row 15 true on the strength of the changelog alone.
