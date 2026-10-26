# BL-DOC-007: the in-app help disagrees with the code, and now with the wiki

**Status**: Open. Target v1.3.0 (Paul, 26 October 2026). Do with BL-DOC-003.
**Raised**: 26 October 2026, from the GitHub wiki update for 1.2.0.
**Severity**: Medium. The help is the method we point journalists and
funders to, and parts of it describe a scoring the code does not do.

## What is wrong

The GitHub wiki (Home, Methodology, Format Support, Glossary) was checked
against the v1.2.0 code and rewritten on 26 October 2026 (wiki commit
b9c2d3f). The in-app help pages it mirrors were not, and still say:

- `help/methodology/+page.svelte:103,115`: High Trust starts at 71%, and the
  middle band is "Inconclusive". `verify/verdict.rs:23-25` sets 70% and 40%,
  and Inconclusive is a separate band for when ELA and AI detection did not
  run.
- `help/methodology/+page.svelte:1360`: ELA weight 2.0. `verify/trust.rs`
  pushes it at 1.0.
- Methodology: Standard mode runs noise analysis and copy-move. The mode
  matrix runs them in Deep only. ELA and segmented ELA run on JPEG only
  (`format_router.rs:88-99`), not PNG and TIFF.
- Methodology does not mention the low-resolution cap (short edge under
  768 px or under 1 MP, `trust.rs:78-88`), the cap behind the 55 score on a
  real and a fake image (BL-UX-007), or the 1.2.0 screenshot and document
  rule.
- `help/format-support`: says AVIF and HEIC can be signed (the picker allows
  JPEG, PNG, TIFF and WebP only), says video and audio target v1.0.1, and
  pins "v1.0" six times.
- `help/glossary`: OCSP and CRL "supported in Enhanced network mode" (not
  implemented; 1.2.0 reports `ocsp.skipped`), trust lists "validated in
  Enhanced mode" (they ship with the app and are read locally), CLIP "a
  one-time download of approximately 350 MB" (`:153`; it ships with the
  installer), Ollama planned for v1.0.1, a TSA timestamp on every signature
  (Standard mode asks first since 1.2.0). No entries for API key, `jura`,
  REST API, network modes, Verification Record or Watched Locations.
- Methodology says both AI models are "retrained regularly". They are the
  v1.0.0 models, unchanged in 1.2.0.

## What would fix it

1. Bring the four help pages into line with the wiki text, which carries
   file and line sources for each change. Keep the voice rules.
2. Take the format table from BL-DOC-003 rather than writing it again.
3. Add a test that reads `BAND_TRUSTED_MIN`, `BAND_UNCERTAIN_MIN`, the
   detector weights and the caps from the Rust constants (or a generated
   JSON) and fails when the help pages quote different numbers.
4. After BL-UX-002 settles the middle band's name, use it in both the help
   and the wiki in the same release.
5. At each release, re-check the wiki footer version against the help.

## What not to do

- Do not describe the retrained model (BL-DET-001) before it passes its
  gates.
- Do not state C2PA conformance in the present tense for a build the award
  did not cover (claims.md row 15, SR-34).
