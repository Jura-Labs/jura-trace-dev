# Discord message to Tim Murphy (CAI / C2PA)

**Date drafted**: 2026-04-22
**Context**: C2PA Conformance admin (round 2 Validator review) referred us to Tim
for guidance on wiring the official C2PA CA Trust List and TSA Trust List into
our validator. Without these, Google-signed test assets resolve as
`signingCredential.untrusted` and surface as "Uncertain (Suspicious)" in our L2
UI — which the Approver would not appreciate.

**Channel**: CAI Discord — find the channel Tim monitors for validator / c2pa-rs
integration questions (likely `#c2pa-rs` or `#developers`).

---

## Message

Hi Tim — I was referred to you by the C2PA Conformance team. I'm Paul, building
**Jura Trace** (a local-first C2PA validator for cultural institutions, UK CIC,
currently in Validator conformance assessment).

The admin reviewed our round-2 evidence and flagged that our validator isn't
loading the official **C2PA CA Trust List** or **TSA Trust List** — so
Google-signed test assets come back as `signingCredential.untrusted` and surface
as "Uncertain" in our UI, when they should resolve as trusted.

We're on **c2pa-rs 0.79** (Rust). Could you point me at:

1. The canonical source/URL for the current C2PA CA trust list bundle (PEM/JSON)?
2. The canonical source/URL for the CAI TSA trust list?
3. The recommended c2pa-rs 0.79 API shape for loading them — is it via `Settings`
   (TOML/JSON), a `Reader::with_trust_anchors`-style builder, or something else?
   Our code still has a stale TODO from the 0.76 era saying no such API existed.
4. Any refresh cadence we should build in (pull on each verify vs. cache +
   periodic refresh)?

Happy to share the validator binary or a test-output capture if useful. Thanks
for the help — trying to clear this ahead of round-3 resubmission.

---

## Optional additions

### Dual-mode framing (if useful for context)
> We ship two signing modes: *Sovereign* (per-install local CA, offline-first —
> our USP for air-gapped museums) and *Conformant* (trust-list cert). The
> trust-list loading only affects verification of third-party manifests, not our
> signer.

### Discord handle (speeds up async)
> I'm `@<your handle>` on the CAI Discord — happy to take it to DM or a thread
> if easier.
