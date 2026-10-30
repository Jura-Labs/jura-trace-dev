# BL-CLAIM-005: the in-app model card shows the previous model's figures

**Status**: Fixed on 30 October 2026, except step 4 (a test that fails
when the page and the metadata disagree), which is still open. Found 12
October 2026, checking figures for an interview against the app and the
model metadata.
**Raised**: 13 October 2026
**Severity**: Medium. The error flatters nothing: the shipped model has a
lower false positive rate than the card says. But the card is where the
product publishes its own error rates, and three of its numbers describe a
model that is no longer the one in the bundle.

## What is wrong

The Help page's model card for the UnivFD probe
(`ui/src/routes/help/model-cards/+page.svelte:318-340`) shows:

| Metric | In the app | `models/univfd_probe_v10onnx_meta.json` |
|---|---|---|
| AUC-ROC | 0.9933 | 0.9929 |
| Authentic false positive rate | 4.12% | 3.87% (119 images) |
| AI detection rate (recall) | 95.70% | 95.77% |

The production probe has been v10onnx since 11 May 2026. The card's own
prose names v10onnx (`:441`, `:527`). The table was not updated when the
model was promoted, and its figures match the earlier probe.

The GBM row at `:171` reads 4.54%, which is the figure used elsewhere for
GBM v4. It was not compared against `deepfake_classifier_v4_meta.json` in
this pass.

## Why it matters

Publishing the false positive rate is one of the three things Jura Labs
says it does about the limits of detection. A published rate that belongs
to a different model undoes that, whichever direction the error runs. The
metadata files are also release assets from v1.2.0 (plan item A5), so the
app and the release would disagree in public.

## What would fix it

1. Correct the three values at `:318-340` to the metadata file's.
2. Compare every other figure on the page against the two metadata files,
   including the GBM table and the per-format and subgroup figures. Do not
   assume the rest is right because these three were the ones noticed.
3. Search the repository and `~/jura-brain/claims.md` for `4.12`, so no
   other document carries the old figure.
4. Better than a correction: have the page read the figures from the
   bundled metadata, or add a test that fails when the page and the
   metadata disagree. This is the BL-SILENT-001 shape, a number that stays
   true only while somebody remembers it.

## What not to do

- Do not round the figures to hide the difference.
- Do not change the model card's stated limits while doing this. They are
  a separate claim with their own evidence.
