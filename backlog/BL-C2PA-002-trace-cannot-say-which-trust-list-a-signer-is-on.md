# BL-C2PA-002: Trace cannot say which trust list a signer is on

**Status**: Open. Target v1.3.0.
**Raised**: 22 October 2026, from the pre-v1.3.0 review of C2PA
developments.
**Severity**: Critical. It is a stated requirement for conforming
validators during the trust-list transition.

## What is wrong

`concatenated_trust_bundle()` (`src-tauri/src/c2pa.rs:1450-1458`) joins the
official C2PA trust list, the TSA list, the Interim Trust List (ITL)
anchors and the ITL allow-list into one string, registered as a single
anchor set under the URI `system_anchors` (`c2pa.rs:1501-1504`). After
validation, Trace knows a signer is trusted but not which list trusted it.

The CAI's conformance guidance (opensource.contentauthenticity.org/docs/
conformance/trust-lists/, read 22 October 2026):

> During the transition period, validator products may consult both the
> frozen ITL and the C2PA trust list, but must distinguish between Content
> Credentials signed with certificates on the ITL and those from conforming
> products using the official C2PA trust list.

The ITL was frozen in January 2026. Several camera makers' roots are on
the ITL only, so a camera file and a file from a conforming product can
both read "trusted" in Trace with nothing to tell them apart.

## What would fix it

1. Register the official list and the ITL as separate anchor sets with
   their own URIs, or match the signing certificate against each list after
   validation.
2. Carry the result in the response, for example
   `signingCredential.trustSource`: `c2pa-trust-list`, `interim-trust-list`
   or `none`. Mark it `[schema]` in the changelog; it is additive.
3. Show it in the Content Credentials panel in plain words: "Signed with a
   certificate on the C2PA trust list" or "Signed with a certificate on the
   older Interim Trust List, which closed in January 2026".
4. Re-run the conformance test vectors (`docs/c2pa-conformance/`). The code
   comment at `c2pa.rs:1471-1474` explains why the single anchor set was
   chosen; the split must not change any validity result.

## What not to do

- Do not treat ITL signers as untrusted. The guidance allows consulting
  both; it requires telling them apart.
- Do this with or after BL-C2PA-001, so the official list being reported is
  a current one.
