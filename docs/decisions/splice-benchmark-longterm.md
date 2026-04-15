---
title: "Long-Term Splice Benchmark Pathway"
status: Draft — decision needed before v1.1 calibration cycle
date: 2026-04-11
author: ml-data-scientist
related:
  - docs/calibration/s28-jpeg-ghost-weight.md
  - docs/decisions/option-c-corpus-strategy.md
  - scripts/build_splice_corpus_v2.py
---

# Long-Term Splice Benchmark Pathway

## Context

Jura Trace's splice / copy-paste forgery calibration currently relies on a
synthetic generator (`scripts/build_splice_corpus_v2.py`) that produces
CC-BY-clean composites for the JPEG Ghost weight sweep and related
manipulation-signal calibration. v2 of that generator (2026-04-11) lifted
JPEG Ghost's AUC from 0.56 to 0.77, confirming the detector is genuinely
discriminative when the corpus exercises its mechanism.

The residual gap is **real-world splice evidence**. Synthetic corpora,
however well-engineered, cannot close the "does it work on actual
forgeries" question. The existing public splice datasets (CASIA v1/v2,
Columbia Uncompressed, CoMoFoD, MICC-F220, IMD2020, DEFACTO, Carvalho
DSO-1, NIST Nimble) are all **research-only** and **not licensed for
commercial use**. Jura Trace is a commercial product (Jura Labs CIC sells
paid tiers under PolyForm Noncommercial, which is a source-availability
licence, not a commercial-use prohibition). Using research-only datasets
for production calibration, shipped benchmark artefacts, or any evaluation
whose output influences released code would breach their terms.

This document lays out the options for closing that gap and recommends a
default pathway.

---

## Option A — Improved synthetic generator ("synthetic plus")

Extend `build_splice_corpus_v2.py` along three axes until it closes as
much of the real-world gap as possible without paying licence fees.

**Axis 1 — Adversarial scenarios**
- Q-delta-near-threshold splices (Δ=5, 10) to test detector sensitivity.
- Progressive vs baseline JPEG source mixing.
- Chroma subsampling mismatches (4:4:4 paste into 4:2:0 bg).
- HEIC/AVIF source conversion to JPEG (smartphone pipeline realism).
- Platform re-encoding passes (WhatsApp Q≈85, Twitter Q≈75) applied to
  the composite before scoring.

**Axis 2 — Larger corpus**
- Scale from 150 to 1,000+ images drawing from the full USB training
  pool (COCO + Open Images + Flickr30k + Unsplash + Wikimedia CC-BY).
- Stratify by camera vendor (consumer phone, DSLR, drone) to match the
  GBM v4 validation strata.

**Axis 3 — Post-processing realism**
- Apply realistic pipeline passes: lens correction, denoise, sharpen,
  social-media crop — not just raw JPEG re-save.
- Mix paste-only, paste-with-blend (alpha-blended edges), and
  paste-with-colour-correction to span forgery difficulty.

**Cost**: ~10-15 engineering hours + CC-BY sources already in hand.
**Licence risk**: zero.
**Output**: benchmark artefact for v1.1 calibration cycle; continued use
of synthetic-only evidence in pilot reports and trust methodology docs.

**Limitation**: still synthetic. Convinces the team, convinces auditors
of the calibration *process*, but does not convince a sceptical outside
reviewer that the detector works on forgeries they did not generate.

---

## Option B — Commissioned gold-standard set

Pay photographers or use explicitly commercial-cleared stock to build a
50-100 image gold set of real, professionally-created forgeries.

**Procedure**
1. Contract 2-3 forensic photographers or retoucher-photographers to
   produce a mix of authentic originals and matched composites. Require
   RAW + JPEG output with full EXIF chain and documented Q history.
2. Pay for commercial usage rights in writing (full buy-out or
   commercial-benchmark-use licence).
3. Catalogue with ground-truth masks, Q-history, and provenance notes.
4. Store under `models/splice_gold_v1/` with a LICENCE.md per-image
   manifest matching `option-c-corpus-strategy.md` format.

**Cost**: £2,000-£5,000 (50-100 images × £30-50 per image for
production + rights, plus project management time).
**Licence risk**: zero once contracts are signed.
**Timeline**: 6-8 weeks from commission to delivery.

**Suitable partners (UK-based, hypothetical starting list, need vetting)**
- Royal Photographic Society members with forensics interest
- University of Edinburgh or Abertay photography / forensics programmes
- Freelance retouchers via commercial illustration agencies (no specific
  names — requires fresh outreach and due diligence)

**Advantages**
- Defensible in grant applications, academic partnerships, and
  contentious enterprise sales.
- Reproducible forgery provenance (every image has a known author and
  intent).
- Can be made public under CC-BY as a Jura Labs contribution to the
  field — potential PR / community win.

**Limitations**
- 50-100 images is smaller than a full calibration corpus. Useful as a
  **gold validation set** paired with the synthetic training set, not as
  a standalone evaluation corpus.
- Real photographers produce consistent style — may not span the full
  threat surface (e.g. political meme edits, amateur copy-paste
  forgeries). Augment with Option A for breadth.

---

## Option C — News-verification partnership

Approach a fact-checking or news-verification organisation that already
holds a library of real-world verified forgeries and negotiate a
data-sharing agreement permitting commercial benchmark use.

**Likely partners**
- **Full Fact** (UK) — UK fact-checking charity, existing contact via
  civic-tech community.
- **AFP Fact Check** — large international fact-check desk, holds a
  steady stream of verified image manipulations.
- **Bellingcat** — open-source investigations, historically willing to
  share forgery examples for research.
- **Reuters Fact Check** — part of IFCN network.
- **First Draft** (now merged into Information Futures Lab at Brown
  University) — academic partners.

**Procedure**
1. Draft an MOU covering: dataset scope, licensing for commercial
   benchmark use, attribution, redistribution rights, end-user
   constraints, and revocation clauses.
2. Offer reciprocity — e.g. free Jura Trace Team-tier licences for
   partner staff, joint case studies, or a named contributor credit in
   Jura Trace release notes.
3. Engage legal-compliance-advisor to review the MOU before signing,
   with particular attention to the partner's source-of-rights (are they
   licensed to redistribute? did the original photographer consent?).
4. Catalogue delivered images under `models/splice_partner_v1/` with
   per-image provenance metadata.

**Cost**: £0-£2,000 (primarily legal review + partner engagement time;
partners may waive fees in exchange for in-kind contributions).
**Licence risk**: medium — depends heavily on the partner's rights
chain. A fact-checker receiving an image via their audit trail may not
own the redistribution rights to that image. Legal review is essential.
**Timeline**: 2-4 months from first outreach to usable dataset.

**Advantages**
- Real forgeries from real misinformation events — directly relevant to
  Jura Trace's mission statement and grant narratives.
- Strong evidence for regulatory / compliance conversations.
- Partner relationship may open follow-on pilot opportunities.

**Limitations**
- Partner rights chain is often opaque; the fact-checker may have
  informal use rights but not formal commercial-grant rights.
- Unstructured — forgeries come with whatever Q history, resolution, and
  post-processing history they had in the wild. Useful as evaluation
  corpus but hard to use for targeted calibration (no Q_delta ground
  truth).

---

## Option D — Mine existing commercially-cleared sources

Systematic audit of the CC-BY / CC0 / commercial-use-cleared image pools
for images that happen to contain forgeries or composites.

**Potential sources**
- **Wikimedia Commons** — many historical photomontages, editorial
  composites, and known edited images are public domain or CC-BY. Filter
  to `Category:Photomontages` and related subcategories.
- **Unsplash** — no known forgery collections, but commercial stock
  occasionally surfaces obvious edits in rejected submissions (unlikely
  viable).
- **Open Images V7** — annotated image dataset with some `Fake` or
  `Composite` labels in the classification taxonomy (small count).
- **Wikipedia edit histories** — before/after versions of manipulated
  news images, some with explicit tampering notes. Usable if authorship
  is traceable.

**Cost**: ~20-40 engineering hours for scraping, filtering, and
ground-truth annotation.
**Licence risk**: per-image due diligence required. Wikimedia is the
most viable but categories are small (low hundreds of images).
**Output**: a modest (50-200 image) real-world supplement to the
synthetic corpus. Known-provenance, commercially clean, but not
systematically labelled for forgery type.

**Advantages**
- Zero licence cost.
- Composable with Option A as a "real-world spot check" dataset.

**Limitations**
- Likely too small to serve as a primary benchmark.
- Forgery coverage is biased toward historical and editorial composites,
  not modern copy-paste or AI-assisted manipulations.
- Ground-truth annotation (which regions are spliced) requires manual
  labelling.

---

## Comparison

| Option | Licence risk | Cost (£) | Timeline | Size | Realism | Reproducibility |
|---|---|---|---|---|---|---|
| A — Synthetic plus | None | 0 | 2-3 weeks | 1,000+ | Low | High |
| B — Commissioned gold | None (contracts) | 2k-5k | 6-8 weeks | 50-100 | High | High |
| C — News partnership | Medium (rights chain) | 0-2k | 2-4 months | Variable | Very high | Low-medium |
| D — CC-BY mining | Per-image | 0 (time) | 3-4 weeks | 50-200 | Medium | Medium |

---

## Recommendation

**Phased combination. Primary: A + D. Stretch: B. Defer: C.**

### Phase 1 (v1.0 → v1.1, 0-8 weeks post-release)
- **Option A extension**: expand the synthetic generator to cover
  adversarial cases (Section 6.1 of the calibration doc), larger corpus,
  and platform re-encoding passes. Low cost, immediate leverage.
- **Option D in parallel**: spend 2-3 days mining Wikimedia Commons
  photomontage categories for a 50-100 image real-world supplement.
  Treat as a spot-check dataset, not a primary benchmark.
- **Output**: v1.1 calibration report with synthetic primary + CC-BY
  real-world supplement. Strong enough for pilot testing and first-tier
  enterprise conversations.

### Phase 2 (v1.1 → v1.2, 2-6 months post-release)
- **Option B (commissioned gold set)** contingent on:
  1. Positive pilot feedback justifying the spend.
  2. A specific use case requiring defensible real-world evidence (e.g.
     a cultural heritage institution pilot, a regulatory conversation,
     or a grant application).
  3. Legal-compliance review of photographer contracts.
- Budget line item: £3,000 in v1.2 development cycle.
- Output: `models/splice_gold_v1/` — 50-100 image benchmark, released
  under CC-BY as a Jura Labs open contribution.

### Phase 3 (v1.2+, opportunistic)
- **Option C (news partnership)** pursued opportunistically when a
  partnership conversation arises naturally — do not chase it as primary
  source. The rights chain complexity makes it high-effort, high-risk.
- Use as supplementary evaluation, never as primary calibration.

---

## Decision points

The following choices require explicit sign-off before work proceeds:

**D1. Phase 1 scope**
- Default: A + D, two-week sprint.
- Alternative: A only (drop D as not worth the annotation effort).
- Required by: v1.1 calibration kickoff.
- Owner: ml-data-scientist + pm.

**D2. Phase 2 budget allocation**
- Default: reserve £3,000 in v1.2 cycle for Option B.
- Alternative: defer indefinitely pending pilot outcomes.
- Required by: v1.1 release planning.
- Owner: pm + finance.

**D3. Photographer commissioning brief**
- If Phase 2 proceeds, draft a commissioning brief specifying:
  - Image count and diversity requirements
  - Required metadata (Q history, ground-truth masks, EXIF chain)
  - Licence terms (CC-BY public release vs internal-only)
  - Deliverables timeline and acceptance criteria
- Required by: Phase 2 kickoff.
- Owner: ml-data-scientist + legal-compliance-advisor.

**D4. Open-source release strategy**
- Default: release commissioned gold set under CC-BY with Jura Labs
  attribution. Community contribution narrative for grant applications.
- Alternative: keep internal as a competitive differentiator.
- Required by: Phase 2 contract negotiation.
- Owner: pm + grant-writer.

---

## Non-goals

- **CASIA v1/v2**: rejected 2026-04-11, non-commercial licence. Do not
  download, do not cite in product materials, do not use for any
  evaluation whose output influences released code. See
  `docs/calibration/s28-jpeg-ghost-weight.md` Section 5.1.
- **Columbia Uncompressed**: rejected as primary FP corpus (PNG
  short-circuit makes it structurally unsuitable for JPEG-ghost work).
- **Any dataset without written commercial-use clearance**: no
  exceptions, regardless of how widely cited in academic literature.

---

## Related work

- v1 calibration run: `docs/calibration/s28-jpeg-ghost-weight.md`
  Section 4 (2026-04-07 results).
- v2 calibration run: `docs/calibration/s28-jpeg-ghost-weight.md`
  Section 8 (2026-04-11 results, synthetic-plus generator landed).
- Corpus strategy: `docs/decisions/option-c-corpus-strategy.md`.
- TRIED Pillar 1 (real-world adaptability) and Pillar 5 (durability)
  both require ongoing calibration evidence that is defensible under
  commercial-use scrutiny.
