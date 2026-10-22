# BL-UX-005: checks Trace did not do should say "not checked"

**Status**: Open. Target v1.3.0. Pairs with BL-C2PA-001 (trust-list date)
and BL-C2PA-003 (CAWG).
**Raised**: 22 October 2026, from the pre-v1.3.0 review of C2PA
developments.
**Severity**: Medium. A check that was not run must not read as a fault in
the file, and a "trusted" signer should say how current that judgement is.

## What is wrong

Three parts of a Content Credentials result are things Trace did not, or
could not, check:

1. **Revocation.** Trace never contacts certificate authorities
   (`src-tauri/src/network_mode.rs:8,45`). The API returns
   `signingCredential.ocsp.skipped` as information
   (`src-tauri/src/c2pa.rs:5010-5022`). The verify page shows nothing about
   revocation at all.
2. **CAWG identity.** Since c2pa-rs 0.91.0 no identity issuer is trusted
   unless listed, and Trace lists none (BL-C2PA-003), so an identity
   assertion comes back `cawg.ica.untrusted_issuer`.
3. **The trust list's age.** "Trusted" means "on our copy of the list",
   fetched on a date the user cannot see (BL-C2PA-001).

## What would fix it

In the Content Credentials details:

- "Revocation: not checked. Jura Trace does not contact certificate
  authorities." (If BL-C2PA-005 lands, "checked, not revoked" or "could
  not be checked" in Enhanced mode.)
- "Identity claims: not checked" for a CAWG assertion whose issuer is not
  listed, instead of "untrusted".
- "Trust list dated 17 October 2026" (the fetch date, from the build).

## What not to do

- Do not turn "not checked" into a warning colour. It is information, as
  the API already reports it.
