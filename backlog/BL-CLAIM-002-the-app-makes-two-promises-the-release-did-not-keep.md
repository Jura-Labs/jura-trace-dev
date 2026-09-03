# BL-CLAIM-002: the shipped app makes two promises the release did not keep

**Status**: Open. Found 3 September 2026.
**Raised**: 3 September 2026
**Severity**: Medium, and rising with time. Both statements are inside the
application on 145 machines, both were true as intentions when written, and
both now read as things Jura Labs said it would do and did not.

## What is wrong

### 1. A dated Article 50 commitment, now a month past its date

`ui/src/routes/help/compliance/+page.svelte:370`:

> An EU AI Act Article 50 audit-log export is planned for the v1.0.1 release
> (~early August 2026, as Article 50 binds 2 August 2026).

Article 50 began to bind on 2 August 2026. There is no v1.0.1. A user who
opens the compliance help page today reads a promise whose date has passed.

This wording was a deliberate choice, not an oversight. `CHANGELOG.md:168`
records a sweep across eleven help pages that stripped forward commitments
and internal ticket numbers from public copy, and kept this one: "EU AI Act
Article 50 v1.0.1 reference retained (the only date-anchored forward
commitment that stays, legally bound to 2 August 2026)". Every other
promise was softened to "under evaluation for a future release". This one
was kept precisely because the date was real.

### 2. A model-card reference to a file that was never shipped

`ui/src/routes/help/model-cards/+page.svelte:130` and `:308`, once per
model card:

> Specific generator names are withheld from public documentation;
> per-generator recall is reported in the model-card metadata JSON shipped
> alongside each release.

No such JSON was attached to the v1.0.0 release. The assets are four
installers, four `.sig` files, an updater archive and `SHA256SUMS.txt`.
Nothing else.

The metadata does exist in the repository:
`models/deepfake_classifier_v4_meta.json` and
`models/univfd_probe_v10onnx_meta.json`. It is simply not in the release,
and `models/` is not `src-tauri/models/`, so it is not in the installer
either.

## Why it matters

The sentence at `:130` is doing real work. It is the justification for
withholding the generator names from public documentation, which was itself
a deliberate copyright-exposure decision taken on 7 and 8 June. The trade
offered to the reader is "we will not name the generators, but we will give
you the per-generator recall numbers". Only half of that trade was
honoured, and it is the half that costs nothing to keep.

For a product whose case rests on being checkable, and which is on the C2PA
conforming products list, a pointer to reproducibility evidence that was
never published is the wrong kind of gap. The archive document for the June
retrains cites Berkeley Protocol §6 reproducibility as a reason those
artefacts are retained. The same reasoning applies here.

## What would fix it

Both are cheap, and both should ride on the next release rather than wait
for a feature.

1. **Attach the two model-card metadata files to the release.**
   `deepfake_classifier_v4_meta.json` and `univfd_probe_v10onnx_meta.json`,
   as release assets, listed in the release notes. Check first that they
   contain the per-generator recall the help page promises and nothing that
   the June copyright sweep deliberately removed. If they name generators
   the sweep withheld, that is a decision for Paul, not a mechanical
   upload: the help page promises recall figures, not names.

2. **Add them to the release workflow** so this is not a manual step that
   is forgotten again, next to the `SHA256SUMS.txt` upload.

3. **Correct the Article 50 sentence** to match whatever the next release
   actually does. If the audit-log export ships in it, restate the version
   and drop the date. If it does not, the sentence must stop naming a
   version and a month.

4. **Tidy the stale metadata in `models/`.** `deepfake_classifier_meta.json`
   describes an 80-feature model and `univfd_probe_meta.json` describes v9,
   while production runs the v4 (84-feature) and v10onnx artefacts held in
   the `*_v4_meta.json` and `*_v10onnx_meta.json` files. These stale files
   are not shipped, since `src-tauri/models/` holds only the ONNX files and
   the two joblibs, so this is repository hygiene rather than a user-facing
   fault. It becomes user-facing the moment step 1 attaches the wrong file.

## What not to do

Do not fix the Article 50 sentence by deleting it. It is the only place the
product tells a compliance-minded reader what it does and does not do about
the AI Act, and that reader is a real buyer. Restate it truthfully.

Do not attach the metadata without reading it first. The June sweep moved
four documents out of this repository for naming corpora and generators.
Publishing a JSON that re-exposes what that sweep removed would undo a
deliberate decision by accident.
