# UnivFD v9 — ONNX Embedding Drift Validation

**JTV-143 hard gate: PASS**
**Date**: 2026-05-03
**Reproducer**: `scripts/validate_clip_onnx_drift.py` against
`/Volumes/Samsung USB/Training Data/corpus/authentic`

## Summary

The ONNX-exported CLIP ViT-B/32 (`laion2b_s34b_b79k`) vision encoder produces
embeddings indistinguishable from the original PyTorch model on the v9
training corpus. The trained UnivFD LogReg probe will operate against the
ONNX-derived embeddings without retraining.

| Metric | Value | Threshold | Status |
|---|---|---|---|
| Vision encoder mean cosine similarity | 1.000000 | ≥ 0.998 | ✅ PASS |
| Vision encoder min cosine similarity | 1.000000 | ≥ 0.995 | ✅ PASS |
| Vision encoder p99 drift (1 − cosine) | 0.000000 | ≤ 0.005 | ✅ PASS |
| L2 diff mean | 0.000064 | informational | — |
| L2 diff max | 0.000163 | informational | — |
| Text encoder mean cosine similarity | 1.000000 | ≥ 0.998 | ✅ PASS |
| Text encoder min cosine similarity | 1.000000 | ≥ 0.995 | ✅ PASS |

Verdict per the sprint plan decision rule: **Ship Option B-full as planned.**
No probe retraining required.

## Methodology

100 randomly-selected authentic images (deterministic seed `0xC11B`) from the
training corpus were processed through both the PyTorch reference model
(`open_clip.ViT-B-32 laion2b_s34b_b79k` FP32) and the ONNX-exported FP32
vision encoder loaded via `onnxruntime CPUExecutionProvider`. Cosine similarity
was computed between paired 512-dim image embeddings.

Five fixed text prompts (the `_TEXT_PROMPTS` constants from
`sidecar/app/services/clip_detector.py`) were tokenised with
`open_clip.SimpleTokenizer` and processed through both backends. Cosine
similarity was computed between paired 512-dim text embeddings.

Sample size of 100 was chosen as the gate-decision sample. A larger sample
(1,000+) is recommended before production cut to confirm uniformity at the
tail; given the perfect-cosine result on n=100 the tail risk is judged low.

## Latency

ONNX runtime is ~2.3× slower than PyTorch CPU on this checkpoint
(45.2 ms vs 19.4 ms per image), but PyTorch is excluded from the production
sidecar bundle so the meaningful comparison is ONNX (45 ms) vs scikit-learn
LogReg-only (the v1.0 fallback path is to skip the embedding step entirely
and lose the UnivFD probe). The 45 ms additional latency per image verify is
acceptable given verify is not a high-throughput operation (typically one
image per several seconds).

## Conclusions for the sprint

1. ONNX export reproduces PyTorch reference to within numerical precision.
   The sprint plan's worst-case "+2-3 day retrain" contingency is not needed.
2. Both encoders ship as planned: Sprint 31 Day 1 deliverable complete on
   day 0 of pre-sprint.
3. Latency budget unchanged from the spec — verify pipeline already tolerates
   per-detector budgets in the seconds.

## Open follow-ups

- Re-run validation on a larger AI-generated sample (DiffusionDB, Civitai,
  flux, sdxl) to confirm cosine perfection holds across the harder-to-classify
  tails before RC15 cut.
- Text encoder ONNX has a reshape constraint: batch>1 fails with the current
  export. The runtime workaround (loop one prompt at a time, ~125 ms one-shot
  on first model load) is acceptable since prompt embeddings cache; revisit
  in v1.0.x if a re-export with proper dynamic axes is desired.
