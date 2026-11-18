# BL-DET-002: AI text watermarks are to be investigated, not built

**Status**: Open. Target v1.3.0 for the investigation only. No
implementation in v1.3.0 (Paul, 5 November 2026).
**Raised**: 5 November 2026, from the v1.3.0 plan.
**Severity**: Low as a defect. It keeps a public statement true: the live
1.2.0 post says "We're also investigating text watermarks for AI
generators".

## What this is

A written investigation, not code. It answers whether Trace, or Jura
Check, could ever honestly report that a text carries an AI provider's
watermark, and on what terms.

## What the investigation covers

1. **Which provider text watermarks exist and who can check them.** Google
   SynthID Text (an open implementation exists; detection needs the
   provider's watermark configuration), Anthropic's watermark API (private
   preview per the October review), and any others found. For each: can a
   local app check it without the provider's keys, and what would it send
   where.
2. **The false-positive evidence.** The October review cites an EMNLP 2026
   study of 135,389 non-native and edited texts with human-text false
   positives from 0 to 100% across 13 statistical detectors. Read it and
   record what it says about watermark checks as opposed to statistical
   detection.
3. **Where it would live.** Trace (local, no keys) or Jura Check (cloud),
   and in which network mode.
4. **The only wording it could support.** "A provider watermark was found"
   or "not found", never a statement about who wrote the text.
5. **A recommendation** for v1.4.0 or later: build, wait, or drop.

Output: `docs/research/text-watermarks-2026.md`, with sources and the date
each was read.

## What not to do

- Do not add statistical AI text detection. The review recommends against
  it on false-positive evidence, especially for non-native English
  writers.
- Do not write code for this in v1.3.0.
- Do not describe the investigation publicly as a planned feature.
