# BL-C2PA-005: revocation is never checked, even when the user allows network access

**Status**: Open. Target v1.3.0.
**Raised**: 22 October 2026, from the pre-v1.3.0 review of C2PA
developments.
**Severity**: Medium. A signing certificate that has been revoked still
reads as valid in Trace.

## What is wrong

Trace does no OCSP or CRL check in either network mode
(`src-tauri/src/network_mode.rs:8,45`) and reports
`signingCredential.ocsp.skipped` (`src-tauri/src/c2pa.rs:5010-5022`).
c2pa-rs 0.91 exposes a setting to fetch OCSP responses
(`verify.ocsp_fetch`, default off), as reported by the review; not yet read
in the crate source.

Revocation is the main published criticism of C2PA validators this year:
Golaszewski, Krawetz et al., arXiv:2604.24890 (April 2026), report that
some conforming validators accept known compromised credentials.

## What would fix it

1. Enhanced mode only: turn on OCSP fetching, through the same outbound
   gate as other Enhanced-mode requests (`network_mode.rs:240`).
2. Three outcomes, shown in plain words (see BL-UX-005): not revoked;
   revoked (the credential fails); could not be checked (no responder,
   timeout or Standard mode).
3. A revoked signing certificate makes the credential invalid in the trust
   score, as an invalid signature does.
4. Re-run the conformance test vectors with fetching on and off.
5. Update the Settings text on what Enhanced mode sends: the certificate's
   serial number goes to the certificate authority's OCSP responder.

## What not to do

- Do not turn it on in Standard mode. Trace's default makes no such
  connection, and that is a public claim.
