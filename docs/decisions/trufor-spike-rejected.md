---
title: "TruFor / Noiseprint / MVSS-Net Research Spike — Rejected on Licence Grounds"
status: Closed — do not re-evaluate until upstream licensing changes.
date: 2026-04-11
decision-id: DEC-2026-04-11-001
author: content-authenticity-expert (research spike)
related:
  - docs/backlog.md#9 (TruFor / MVSS-Net splice localisation — rejected)
  - docs/backlog.md#16 (Platform-forwarded augmentation retrain — replaces this)
  - docs/calibration/s28-jpeg-ghost-weight.md (v3 corpus finding that triggered the spike)
  - docs/decisions/option-c-corpus-strategy.md (commercial-use licence requirement)
---

# TruFor / Noiseprint / MVSS-Net Research Spike — Rejected on Licence Grounds

## Decision

**TruFor, Noiseprint++, MVSS-Net, and CAT-Net v2 are all rejected as candidate
additions to Jura Trace's on-demand investigation tier.** The blocker is
licensing, not technical fit. This decision closes backlog item #9 and
replaces it with backlog item #16 (in-house retraining on platform-forwarded
augmented corpus).

**Do not re-open this decision unless the upstream licences change.** The
precedent is documented here so future "add TruFor / add MVSS-Net" suggestions
close in minutes rather than requiring another full research spike.

## Why the spike happened

The v3 JPEG Ghost calibration corpus (2026-04-11, `scripts/build_splice_corpus_v3.py`,
commit `d3b2666`) surfaced a structural limitation: JPEG Ghost is blind to
platform-forwarded splices (p95 = 0.041, indistinguishable from authentic
plain at 0.035, because Twitter/WhatsApp re-encoding wipes the differential
ghost signature entirely).

The content-authenticity-expert agent (2026-04-11 pivot review session)
flagged TruFor as *"the published answer to exactly the gap v3 exposed"* —
a learned camera-fingerprint residual method explicitly designed to survive
social-media re-encoding, with *"a permissive research licence and
ONNX-exportable weights"*. The research spike evaluated whether TruFor could
realistically land in Jura Trace's sidecar as an on-demand investigation
tool alongside NPR, shadow consistency, and splice boundary.

## What the spike found

### TruFor (Guillaro et al., CVPR 2023)

- **Paper**: *"TruFor: Leveraging all-round clues for trustworthy image forgery
  detection and localization"*, arXiv 2212.10957, CVPR 2023 pp. 20606–20615.
- **Repo**: https://github.com/grip-unina/TruFor (246 stars, last push
  2025-05-29, actively maintained).
- **Framework**: PyTorch, SegFormer/MiT-B2 backbone + Noiseprint++ extractor
  + cross-modal CMX fusion + confidence head.
- **Task**: Both image-level integrity score AND pixel-level localisation
  map AND reliability map — exactly the output shape Jura Trace would want
  for the investigation panel.
- **Technical feasibility**: ONNX export would likely work (no custom CUDA
  kernels), weights ~200–300 MB, CPU latency ~2–6s per 1024×1024 image on
  M1/M2 via ONNX Runtime. **Technically a GO if licence were clean.**
- **Licence (hard gate)**: Custom GRIP-UNINA licence at
  https://raw.githubusercontent.com/grip-unina/TruFor/main/test_docker/LICENSE.txt
  explicitly prohibits *"industrial or profit-oriented activities"*. Exact
  text: *"Reproduction, modification, and usage of the software covered by
  this license is allowed free of charge provided that: (i) this software
  should be used, reproduced and modified only for informational and
  nonprofit purposes; any unauthorized use of this software for industrial
  or profit-oriented activities is expressly prohibited"*.
- **Weights licence**: Same LICENSE.txt, same prohibition.
- **Verdict**: **Rejected.** Jura Labs CIC sells paid B2B tiers. This is not
  an ambiguous OSS/research boundary — the licence uses the word
  "prohibited". No legal review needed.

### Noiseprint++ (same group)

- **Repo**: https://github.com/grip-unina/noiseprint
- **Licence**: Same GRIP-UNINA non-commercial licence family as TruFor. SPDX
  shows `NOASSERTION` but the actual LICENSE file carries the same
  commercial-use prohibition.
- **Verdict**: **Rejected.** Same blocker. The entire GRIP-UNINA output is
  on this licence; do not evaluate individual releases from this group in
  isolation — the licence is structural.

### MVSS-Net (Dong et al.)

- **Repo**: https://github.com/dong03/MVSS-Net (324 stars, last push
  2023-04-13, lightly maintained).
- **Licence**: No LICENSE file in repo. `license: null` in GitHub metadata.
  README silent on licensing.
- **Verdict**: **Rejected.** Under standard copyright law, code published
  without an explicit licence grant is "all rights reserved" — not usable
  commercially even for on-demand inference. Would need upstream to add an
  SPDX licence before re-evaluation.

### CAT-Net v2 (Kwon et al.)

- **Repo**: https://github.com/mjkwon2021/CAT-Net (303 stars, last push
  2025-07, actively maintained).
- **Licence**: `license: null` in GitHub metadata. README defers to MS COCO /
  RAISE licensing for *data* only, no code licence specified.
- **Verdict**: **Rejected.** Same reasoning as MVSS-Net.

## Structural observation

The entire learned-splice-localisation academic ecosystem is either
GRIP-UNINA-adjacent (and therefore non-commercial-licensed) or
licence-silent. This is not accidental — splice localisation research
originates predominantly from European academic media-forensics groups that
operate under research-only licence terms by institutional default. There is
no drop-in commercially-licensed equivalent as of this spike.

The academic literature's commercial-use rate is roughly inverse to its
technical quality: the best-performing learned detectors are all
research-licensed, and the commercially-licensed alternatives are
significantly behind the state of the art.

## What we do instead

**Backlog item #16 — Platform-forwarded augmentation retrain.** Jura Labs
owns its training corpus, its training pipeline, and its licence terms.
The same gap (platform-forwarded splices invisible to JPEG Ghost) can be
closed by retraining UnivFD v8 (and optionally GBM v4) on a platform-
forwarded augmentation of the existing 10,709-image training corpus:

- For each training image, generate 2-3 variants via Pillow JPEG Q=75
  (Twitter-equivalent) and Q=85 (WhatsApp-equivalent) re-saves.
- The augmented corpus teaches the learned detectors to recognise
  platform-forwarded synthetic content via learned features rather than
  a differential JPEG ghost signature that platform re-encoding erases.
- Retains commercial licence cleanliness (CC-BY training data; PIL
  augmentation is pure Python operations we already have in the pipeline).
- Estimated effort: 1 week for the augmentation pipeline + retrain +
  validation + model card update.
- Published methodology satisfies TRIED Pillar 5 (Durable) retraining
  cadence and is EMIF-narrative-pitchable.

This path is **technically weaker than TruFor** (UnivFD / GBM are
image-level classifiers, not pixel-level localisation) but **strategically
stronger** because it requires no third-party licence negotiation, owns
the entire pipeline, and reinforces the Rooted philosophy (local-first,
sovereign by design).

## Conditional re-open paths (for future reference only)

None of these are being pursued, but documenting them so future sessions
do not re-derive them:

1. **Direct commercial-licence negotiation with Verdoliva / GRIP-UNINA.** The
   group has industry collaborations (the vera.ai Horizon Europe project,
   Google acknowledgements in the TruFor paper). A negotiated licence grant
   is not implausible but is a multi-month legal track, not a Phase B
   engineering ticket. Would require leadership sponsorship, framing around
   Jura Labs' civil-society / CIC status, and pro-bono legal support.

2. **Upstream LICENSE file addition**. If MVSS-Net or CAT-Net v2 add an
   SPDX licence (Apache-2.0 or MIT would be the typical progression), the
   spike can be re-opened for those specific projects. Check the repos
   annually.

3. **Reimplementation from the paper under a clean-room approach.** The
   TruFor paper is published and the architecture is reproducible. A
   clean-room reimplementation is legally different from using the
   original code, but the weights would still need to be trained from
   scratch on a commercial-licence-clean corpus. Estimated effort:
   2-3 months full-time for a trained ML engineer — not a Phase B ticket.

## Summary for the lazy reader

- **TruFor, Noiseprint++, MVSS-Net, CAT-Net v2**: all rejected, licence block.
- **Entire learned splice-localisation ecosystem**: structurally blocked
  for commercial use as of April 2026.
- **Jura Trace's response**: in-house platform-forwarded augmentation
  retrain on existing UnivFD v8 / GBM v4 (backlog item #16).
- **Do not re-evaluate** these specific projects unless their upstream
  licences change. Check annually.
