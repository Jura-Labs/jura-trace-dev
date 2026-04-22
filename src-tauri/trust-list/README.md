# C2PA Trust Lists — Vendored PEM Bundles

This directory contains the four official C2PA trust list PEM bundles that
Jura Trace compiles into the validator binary via `include_str!` in
`src/c2pa.rs`. They are loaded into `c2pa::Settings` at Reader construction
so well-known signers (Google, Adobe, Truepic, etc.) resolve as trusted
rather than `signingCredential.untrusted`.

## Sources

| File | Source | Purpose |
|------|--------|---------|
| `C2PA-TRUST-LIST.pem` | `raw.githubusercontent.com/c2pa-org/conformance-public/main/trust-list/C2PA-TRUST-LIST.pem` | Current signing CAs (19 certs) |
| `C2PA-TSA-TRUST-LIST.pem` | `raw.githubusercontent.com/c2pa-org/conformance-public/main/trust-list/C2PA-TSA-TRUST-LIST.pem` | Current timestamp authorities (14 certs) |
| `ITL-anchors.pem` | `raw.githubusercontent.com/contentauth/verify-site/main/static/trust/anchors.pem` | Interim Trust List anchors — legacy, frozen Jan 2026, required for content signed before then (27 certs) |
| `ITL-allowed.pem` | `raw.githubusercontent.com/contentauth/verify-site/main/static/trust/allowed.pem` | Interim Trust List end-entity allowlist (115 certs) |

## Refresh Policy

**Last fetched**: 2026-04-22

The current C2PA Trust List and TSA Trust List are live documents — new CAs
are added as vendors complete the Conformance Program. Refresh cadence:

- Refresh on every minor release (e.g. 0.9 → 1.0, 1.0 → 1.1).
- Refresh out-of-band if a Validator conformance round flags a missing CA.
- Run `scripts/refresh-trust-lists.sh` (TODO: add) to re-download all four
  from their canonical sources.

The ITL bundles are frozen as of January 2026 and should not change, but we
still ship them because third-party assets signed before Jan 2026 continue
to chain to them.

## Verification

After refreshing, verify each file parses cleanly:

```bash
for f in *.pem; do
  echo "$f: $(grep -c 'BEGIN CERTIFICATE' $f) certs"
done
```

Expected counts at time of vendoring (2026-04-22): 19 / 14 / 27 / 115
(total 175 trust anchors).
