---
title: "Decision: Experimental UI Tag Pattern for In-Development Features"
decision-id: DEC-2026-04-09-001
date: 9 April 2026
status: ADOPTED
decided-by: Paul Griffiths (Director, Jura Labs CIC)
decision-type: UI / Product
related-documents:
  - ui/src/routes/verify/v2/+page.svelte (existing usage)
  - sidecar/app/services/clip_detector.py (docstring flagging EXPERIMENTAL status)
  - docs/backlog.md
review-by: Sprint 31 (pilot feedback cadence)
---

# Decision Memo: Experimental UI Tag Pattern

## Status

**ADOPTED** — 9 April 2026

## Context

Jura Trace is pre-v1.0 (0.9.0-rc14) and under active development. Several
features exist in the codebase that are functionally present but whose
outputs are not yet trustworthy enough for a forensic analyst to quote
in a report without caveat. Examples already in the tree as of 9 April
2026:

- **Verify v2 layout** (`ui/src/routes/verify/v2/+page.svelte`) — parallel
  route gated behind a Settings opt-in, carrying a `"New layout · experimental"`
  lapis-tinted pill in the header.
- **CLIP zero-shot classification** (`sidecar/app/services/clip_detector.py`) —
  file-level docstring already declares the endpoint EXPERIMENTAL and
  says *"produces near-uniform probabilities (~20% per class) — it does
  NOT reliably discriminate between real photos and AI-generated images."*
  The file notes that UnivFD (a trained linear probe) is the production
  path. The verify page currently renders CLIP class probabilities in
  Expert View without any experimental indicator — users have reported
  the flat distribution and asked whether it is correct behaviour.
- **JPEG Ghost weight at 0.5×** — wired into scoring (S28-4, commit
  `63dd511`) but not empirically calibrated. The S28-FU9 sweep (commit
  `497b06a`) could not produce meaningful TPR data because the synthetic
  corpus could not exercise the detector properly. The weight remains a
  cross-review consensus value pending CASIA v2 research benchmark.
- **RAG / Knowledge base retrieval aid** — reframed in commit `4e4af0d`
  with explicit non-warranty language on the model card. The methodology
  page lapis-tinted block documents this clearly but the verify result
  surface does not visually mark individual claim verdicts as advisory.
- **Potential future Phase A features** — seasonal indicators were
  deleted as pseudoscience (`d650c54`), but similar experimental
  investigative tools may land again and need consistent labelling.

Without a consistent visual convention, analysts cannot tell at a glance
which signals are production-grade and which are informational only.
This is a pilot-testing blocker: we cannot ship to museum archivists or
journalists without them being able to distinguish what to trust.

## Decision

Adopt a **single consistent experimental tag pattern** across the UI
using the lapis-tinted pill already in use on the verify v2 header. The
pattern has three visual components and two wording conventions.

### Visual pattern

A small pill / badge adjacent to the feature's heading or within its
result panel:

```
┌─────────────────────────────────┐
│ ⊙ Experimental — informational  │
└─────────────────────────────────┘
```

**Tailwind token for the pill** (consistent with sanctuary brand):

```svelte
<span
  class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase tracking-wider
         bg-lapis/15 text-lapis dark:text-lapis-light border border-lapis/30"
  aria-label="Experimental feature — informational only"
>
  Experimental
</span>
```

**Why lapis**: it's Jura Trace's "information / advisory" accent (the
same tone as `LimitationBanner`, the methodology page's Knowledge Base
Retrieval block, and the v2 header). Amber would imply *caution*, which
is reserved for degraded-input banners and on-demand tools that were
demoted for discriminative power. Cinnabar implies *failure*. Lapis
conveys *informational, not a red flag*.

### Wording convention

Two phrases depending on severity:

1. **"Experimental — informational only"** — for features that run but
   whose output should not be quoted in an evidentiary context. Example:
   CLIP zero-shot class probabilities.
2. **"Experimental — not in scoring"** — for features whose output is
   visible in Expert View but does not contribute to the numeric trust
   score. Example: shadow consistency (demoted), splice boundary (demoted).

### Required companion element

Every pill must link to a plain-English explanation — either:

- An inline tooltip on the pill itself (`title` attribute, WCAG 2.2 AA
  compliant via `aria-describedby` on the parent), or
- A contextual help link (`ContextualHelpLink`) next to the pill
  pointing at the relevant `/help/methodology#...` anchor.

Tooltip-only is acceptable for very short explanations; anything
longer than one sentence must use a help link.

## Consequences

### Positive

- Pilot testers can distinguish scoring signals from advisory signals at
  a glance without reading the methodology page first.
- The existing verify v2 badge is retroactively consistent with a
  documented pattern rather than a one-off.
- Future experimental features have a drop-in component pattern instead
  of inventing bespoke treatments.
- Helps Jura Trace meet the TRIED Pillar 3 (transparency) and Pillar 4
  (reproducibility) requirements — users can see which outputs are
  reproducible scoring signals and which are exploratory.

### Negative

- Slight visual clutter in Expert View where several signals carry the
  pill.
- Requires a discipline step in code review: any new detector PR that
  adds an experimental signal must also add the pill and the help anchor.
- Does not by itself hide the feature — users who ignore the pill may
  still misread the output. The pill is a signal, not a gate.

### Neutral

- Does NOT change what runs in the pipeline. No detector is disabled by
  this decision.
- Does NOT affect the numeric trust score computation.
- Does NOT affect the PDF trust report or the ZIP case export — those
  already carry methodology-version metadata and a "Not run in this
  analysis" row convention.

## Immediate application scope

On adoption, apply the pill to the following surfaces (one focused PR
per surface, not a single giant refactor):

1. **CLIP Classification panel** (`ui/src/routes/verify/+page.svelte`
   around the CLIP result rendering block) — add the `Experimental —
   informational only` pill next to the heading, with an inline help
   link pointing to `/help/methodology#clip-detection`. Update the
   methodology page entry for CLIP to add a "Note" dd explaining that
   zero-shot probabilities are currently uniform because the raw cosine
   similarity is fed into softmax without the CLIP logit scale. The
   UnivFD probe is the production-grade signal.
2. **Verify v2 route header** (`ui/src/routes/verify/v2/+page.svelte`) —
   already using this pattern. Verify wording matches ("New layout ·
   experimental" → keep or switch to the canonical phrase; aesthetic
   call).
3. **JPEG Ghost score row** (in the detector reference list, PDF raw
   scores table, and verify result panel) — add a smaller inline
   `Experimental weight` indicator with a tooltip linking to
   `docs/calibration/s28-jpeg-ghost-weight.md`. Weight is currently
   pinned at 0.5 but empirically uncalibrated per S28-FU9.

## Not in scope

- Full compliance against ISO 25010 or ISO 25023 quality attributes.
  This is a pragmatic UI pattern decision, not a quality management
  framework.
- A feature-flag system for hiding experimental surfaces entirely. The
  pill is a disclosure, not a toggle.
- Backend changes. Every feature tagged experimental continues to run
  exactly as before.

## Review

Revisit this decision at the Sprint 31 pilot feedback cadence. If
pilot testers report that the pill is not doing its job — either
because they miss it or because they find it noisy — refine the visual
treatment rather than abandoning the pattern.
