# BL-CLAIM-006: the detection figures we publish are out of date and out of context

**Status**: Open. Target v1.3.0.
**Raised**: 22 October 2026, from the pre-v1.3.0 review of detection
research.
**Severity**: Critical. Public accuracy figures describe a superseded model,
and none says it was measured only on generators the model was trained on.

## What is wrong

- `docs/user-guide/verify-guide.md:110` and
  `docs/user-guide/getting-started.md:231` quote **UnivFD v9** (AUC 0.9933,
  false positives 4.12%, recall 95.70%). The shipped model is v10onnx
  (0.9929, 3.87%, 95.77%), which the in-app model card already shows
  (BL-CLAIM-005).
- getting-started.md:231 says it "correctly identifies most AI-generated
  images across 14 generator families". The families are 2025 and earlier.
  On 2026 generators the detectors have not been measured (BL-DET-001), and
  2026 benchmarks show this architecture near chance on them.
- `verify-guide.md:128` reasons from "the UnivFD v9 probe".
- Nowhere next to the figures does it say they were measured on a held-out
  split of the training distribution.

## What would fix it

1. Replace the v9 figures with v10onnx's, from the model metadata, in both
   guides and anywhere else a grep for "v9", "0.9933" and "4.12" finds them
   (README, website copy, the press pack).
2. Say where the figures come from: "on a held-out test set drawn from the
   generators it was trained on".
3. Replace "most AI-generated images across 14 generator families" with a
   dated statement: which generator families, up to when, and that newer
   generators have not been measured.
4. When BL-DET-001 has figures for 2026 generators, publish them next to
   these, including if they are poor.
5. Add a claims.md row for the accuracy figures, so the next model change
   has a place to be checked against.

## What not to do

- Do not remove the figures. Stating them with their limits is the point.
