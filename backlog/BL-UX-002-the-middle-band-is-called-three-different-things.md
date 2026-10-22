# BL-UX-002: the middle band is called three different things

**Status**: Open. Target v1.3.0. Decided by Paul, 22 October 2026: the
middle band is called **Uncertain** everywhere.
**Raised**: 22 October 2026, checking the FAQ's trust-score answer against
1.2.0.
**Severity**: Medium. A user following the getting-started guide is told to
expect "Uncertain" and sees "Moderate Trust".

## What is wrong

A score from 0.40 up to 0.70 is the middle band (`ui/src/lib/types.ts:1217-1221`,
`src-tauri/src/verify/verdict.rs`). For the practice pack's
`resaved-uncertain.jpg` (55%, AI detector "authentic", installed 1.2.0, 22 Oct), the
code gives three names:

- The score badge on the verify page says **Moderate Trust**
  (`ui/src/routes/verify/+page.svelte:455`). It says "Uncertain" only when the
  AI detector itself is inconclusive or synthetic (`:449-451`).
- The verdict headline says **Inconclusive. Further review advised.**
  (`ui/src/lib/components/SimpleVerdict.svelte:174`). "Inconclusive" is also
  the label for a result with too little evidence to score (`ceilingApplied:
  insufficientSignal`), which is a different thing.
- The API band is **`uncertain`**, and the note under the result says
  '"Uncertain" does not mean fake.'

The website's FAQ, the getting-started guide, the check-image article and the
practice pack's `readme.txt` all say "Uncertain, around 55 per cent".

## What would fix it

Use "Uncertain" for the middle band wherever a person reads it:

- `ui/src/routes/verify/+page.svelte:455` and `ui/src/lib/types.ts:1250`
  (report and PDF wording): "Moderate Trust" becomes "Uncertain".
- `ui/src/routes/help/methodology/+page.svelte:111`.
- `docs/API_WRAPPER.md:516` and the doc comment at `src-tauri/src/verify/verdict.rs:32`.
- `ui/src/lib/verdict.test.ts` (two assertions).
- `SimpleVerdict.svelte:174`: the middle-band headline stops saying
  "Inconclusive", so that word means only "too little evidence".

Then check a real run of the four practice images on screen.

## What not to do

- Do not rename the API band. `uncertain` is already the name, and the
  schema is a public contract.
- Do not change the thresholds. This is a label, not a scoring change, and
  verdicts must stay identical on the 217-image golden set.
