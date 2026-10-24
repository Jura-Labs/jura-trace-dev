# BL-DOC-003: the per-format detector list is written by hand, and three sources disagree

**Status**: Open. Target v1.3.0. The website session is waiting on it (FAQ
item B10, brain `inbox/2026-10-21-vps-php2shell-patch.md:175`).
**Raised**: 22 October 2026, from the pre-v1.3.0 backlog review.
**Severity**: Medium. Users cannot tell which checks ran on a WebP, AVIF or
HEIC file.

## What is wrong

- `ui/src/routes/help/format-support/+page.svelte:31-39` says WebP, AVIF
  and HEIC get "Deepfake, EXIF anomaly, CLIP, segmented ELA", with ELA and
  JPEG Ghost off.
- The website FAQ says these formats get metadata only.
- `CHANGELOG.md` v1.2.0 says GAN fingerprint analysis now runs on AVIF,
  which the help list does not mention.

All three are written by hand. The real answer is in the sidecar's codec
gates and the detector registry.

## What would fix it

1. Derive a table, format by detector, from the code: run each detector
   gate against one sample per format (JPEG, PNG, TIFF, WebP, AVIF, HEIC)
   and record ran, skipped, or failed.
2. Generate the help page's rows from that table at build time, or add a
   test that fails when the help text and the table disagree.
3. Send the table to the website session for the FAQ.
4. State the C2PA conformance scope next to it: the validator record
   covers JPEG, PNG, TIFF and WebP, not AVIF or HEIC.

## What not to do

- Do not hand-edit the help page again without the table. That is how the
  three sources drifted.
