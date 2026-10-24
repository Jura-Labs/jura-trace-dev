# BL-UX-007: a capped score does not say it was capped

**Status**: Open. Target v1.3.0.
**Raised**: 23 October 2026. Paul checked an AI-generated image and a
genuine photograph in v1.2.0 and both scored 55.
**Severity**: High for trust in the result. Two opposite images get the
same number, and nothing on screen or in the API says why.

## What happened

Both files run through an ephemeral v1.2.0 `jura-trace-api`, deep mode,
23 October 2026:

| | AI image (`hero-fake-image-verdict-1024w.webp`) | Real photo (geograph 996575, Canon PowerShot A640) |
|---|---|---|
| Size | 1024 x 593 WebP, 0.61 MP, no metadata | 640 x 480 JPEG, 0.31 MP, camera MakerNote |
| AI detector | `inconclusive`, low confidence, classifier 0.30, 0 of 19 signals | `authentic`, high confidence, classifier 0.05 |
| `overallTrust` | 0.55 | 0.55 |
| `verdict.band` / `ceilingApplied` | `uncertain` / `null` | `uncertain` / `null` |

Both hit the **low-resolution cap**, `LOW_RESOLUTION_CAP = 0.55`
(`src-tauri/src/verify/trust.rs:78`), which fires when the short edge is
under 768 px or the image is under 1 megapixel (`:83`, `:88`, applied at
`:448-464`). The AI image also hits `VERDICT_CEILING_INCONCLUSIVE = 0.55`
(`:71`, `:425-433`).

These are score caps inside `compute_trust`. `ceilingApplied` reports only
the band ceilings in `src-tauri/src/verify/verdict.rs:100-116`
(`InsufficientSignal`, `NoPositiveAuthenticitySignal`,
`DeepfakeInconclusive`, `DeepfakeSynthetic`), which apply only when the raw
band would be `trusted`. A score already capped to 0.55 is `uncertain`
before that check, so `ceilingApplied` is `null` and neither the app nor
the API says a cap fired.

Most images people check are web-sized, so this is the common case, not
an edge case: under 1 MP, every image lands at 55 or lower whatever the
detectors found.

## What would fix it

1. Report every cap that bounded the score: add `scoreCaps` (or extend
   `ceilingApplied`) with `lowResolution`, `deepfakeInconclusive`,
   `deepfakeSynthetic`, `compositeEvidence`, `selfDeclaredAi`, each with
   its value. Mark it `[schema]`, additive.
2. Say it in plain words under the score: "Capped at 55 because this
   image is smaller than 1 megapixel. Jura Trace will not give a small
   image a high score." and, where it applies, "The AI check could not
   tell either way."
3. When the score is capped, lead the result with what actually differs:
   for these two files, "AI check: looks like a camera photo (high
   confidence)" against "AI check: could not tell". Today both read as the
   same 55.
4. Add both files (or equivalents with clear licences) to the golden set
   as a regression pair: same capped score, different explanations.

## What not to do

- Do not raise or remove the cap to make the numbers differ. It exists so
  a small image cannot look more certain than the detectors are; that
  decision stands. The fault is that it is silent.
