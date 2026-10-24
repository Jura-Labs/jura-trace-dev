# BL-C2PA-001: the trust lists are six months old

**Status**: Open. Target v1.3.0.
**Raised**: 22 October 2026, from the pre-v1.3.0 review of C2PA
developments. Security register SR-23.
**Severity**: Critical. Content Credentials from signers who joined the
C2PA trust list since April show as untrusted, and Trace holds a C2PA
Validator conformance record.

## What is wrong

Trace compiles four trust bundles into the binary (`src-tauri/src/c2pa.rs:1450`,
`concatenated_trust_bundle`). The two live ones were fetched on 22 April
2026 and have not changed since (`src-tauri/trust-list/README.md`, "Last
fetched: 2026-04-22").

Counted 22 October 2026:

| Bundle | In Trace | Upstream (`c2pa-org/conformance-public`, synced 17 Oct 2026) |
|---|---|---|
| `C2PA-TRUST-LIST.pem` (signing CAs) | 19 | 32 |
| `C2PA-TSA-TRUST-LIST.pem` (timestamp authorities) | 14 | 23 |

The README's own policy is "refresh on every minor release". Neither
v1.1.0 nor v1.2.0 did. The refresh script it names,
`scripts/refresh-trust-lists.sh`, is still marked "TODO: add".

## Why it matters

- A file signed under one of the 13 newer CAs gets
  `signingCredential.untrusted` in Trace and a valid result in up-to-date
  validators. A user reads that as a problem with the file.
- The CAI's conformance guidance says "Conforming validator products must
  refer to the C2PA trust list to determine whether Content Credentials
  were signed with a valid certificate"
  (opensource.contentauthenticity.org/docs/conformance/trust-lists/, read
  22 Oct 2026). A six-month-old copy is a weak reading of that.
- v1.2.0 made the revocation report depend on the responder chaining to
  these roots, so a missing root changes that report too.

## What would fix it

1. Add `scripts/refresh-trust-lists.sh`: download the two live bundles,
   print the before and after counts and the names of added and removed
   certificates. The ITL bundles are frozen and stay as they are.
2. Refresh, update the counts in `trust-list/README.md` and in the tests
   that assert them (`c2pa.rs:4844` expects 175; `:4881` too).
3. A CI check, weekly, that fails when upstream differs from the vendored
   copy by more than the date. It must name the differing certificates, not
   just fail.
4. Re-run the conformance test vectors in `docs/c2pa-conformance/` and the
   217-image golden set. Results should change only where a newly trusted
   signer is present.
5. Show the trust list's fetch date in the Content Credentials panel's
   details, so a reader knows how current "untrusted" is.

## What not to do

- Do not fetch the trust list at run time by default. Trace is local-first;
  a live fetch, if ever added, belongs in Enhanced mode only.
- Do not drop the ITL bundles. Content signed before January 2026 still
  chains to them. See BL-C2PA-002 for how they should be reported.
