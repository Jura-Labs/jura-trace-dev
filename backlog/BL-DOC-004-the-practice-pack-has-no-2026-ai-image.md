# BL-DOC-004: the practice pack has no 2026 AI image

**Status**: Open. Target v1.3.0. Follows BL-DET-001's measurement.
**Raised**: 22 October 2026, from the pre-v1.3.0 review of detection
research.
**Severity**: Low. It is how new users learn what the bands mean, and it
shows only the easy case.

## What is wrong

The practice pack (linked from juralabs.org/download and the getting
started guide; the files are held on the website, not in this repository)
has four images. Its AI example is an OpenAI image that scores 0.149, Low
Trust, with the AI detector saying "synthetic" (installed 1.2.0, 22 Oct).
It does not show what users will meet most in 2026: an image from a
current generator, which the detectors were not trained on (BL-DET-001).

## What would fix it

1. After BL-DET-001 has measured the 2026 slice, add one image from an
   Apache-2.0 generator run locally (Qwen-Image or FLUX.2 [klein] 4B), with
   whatever result Trace gives it.
2. Say in the readme and the getting-started guide what that result means,
   honestly: if it scores in the middle band or higher, say so, and say
   why the other evidence on the page matters.
3. Keep the source, prompt and generator version in the readme, so the
   image can be reproduced.

## What not to do

- Do not pick an image because Trace catches it. The point is to show a
  current case as it is.
- Do not use an image from a provider whose terms have not been checked
  (BL-DET-001 step 1).
