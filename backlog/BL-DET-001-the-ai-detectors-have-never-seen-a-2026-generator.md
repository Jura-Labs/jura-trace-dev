# BL-DET-001: the AI detectors have never seen a 2026 generator

**Status**: Open. Target v1.3.0 (measurement); any model change follows the
measurement.
**Raised**: 22 October 2026, from the pre-v1.3.0 review of detection
research.
**Severity**: Critical. Trace's AI-detection figures are measured on the
generators it was trained on, and three 2026 benchmarks show this kind of
detector falling close to chance on newer ones.

## What is wrong

The two AI detectors are the UnivFD probe `v10onnx` (a linear probe on CLIP
ViT-B/32, May 2026) and GBM v4. Their training families
(`models/generator_family_labels.json`) are all 2025 or earlier. None of
the generators released this year is in them, and none has been measured:
GPT Image 2 (April 2026), Nano Banana 2 (February 2026), Midjourney V8
(March 2026), FLUX.2 (January 2026), Seedream 5 (2026), Qwen-Image.

The published figures (AUC 0.9929, false positives 3.87%, recall 95.77%)
come from a held-out split of the same training distribution.

2026 evidence on this architecture, all read on arXiv on 22 October 2026
(figures as reported by the review; the papers exist and are dated as
shown, the figures are inside the papers, not the abstracts):

- Ren et al., arXiv:2602.07814 (8 Feb 2026): open-source detectors average
  75% at best out of the box, 18 to 30% on Flux Dev, Firefly v4 and
  Midjourney v7. Already cited on the website (claims.md row 26).
- DailyBench, arXiv:2607.24016 (27 Jul 2026): UniFD at 49.8% on a set
  covering FLUX.2, Qwen-Image, Nano Banana 2 and GPT-Image 2.
- LAION-Mobile, arXiv:2609.11134 (10 Sep 2026): UnivFD AUC 0.600 on a
  modern mix; refitting thresholds on modern content raises false alarms on
  smartphone photos sharply.

## What would fix it

Measure first, then decide.

1. Build a "2026 generators" evaluation slice. Generate locally with
   Apache-2.0 models (Qwen-Image; FLUX.2 [klein] 4B). Add GPT Image 2 and
   Nano Banana 2 samples only after checking each provider's terms allow it.
   Evaluation only, under the same licence bar as the corpus (the CASIA
   decision of April 2026).
2. Build a current-phone slice: recent flagship photos, including AI zoom
   and object-removal edits (Magic Eraser, Clean Up), from people who give
   permission. This is the false-positive guard.
3. Run v10onnx and GBM v4 on both slices and the 217-image golden set.
   Publish the figures, whatever they are, in the model card.
4. Evaluate a backbone change inside the existing probe pipeline: Meta's
   Perception Encoder (PE-Core, Apache-2.0) in place of CLIP ViT-B/32. Check
   the training data licence of any pretrained detector before testing it
   (the PE-Core-based ONNX models on GitHub and Smaug both need that check).
5. Gate any new model on: better on the 2026 slice, no rise in false
   positives on the phone slice, golden set within agreed tolerance.

## What not to do

- Do not retune thresholds towards 2026 generators without the phone
  slice. LAION-Mobile shows that is how false positives on real photos go
  up.
- Do not ship a model whose training or calibration data cannot be used
  commercially.
- Do not add AI text detection as part of this. The 2026 review
  recommends against it on false-positive evidence, especially for
  non-native English writers. Whether to follow it is for Paul; see
  `~/jura-brain/inbox/2026-10-22-trace-v130-review.md`.
