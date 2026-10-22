# BL-DOC-005: the getting-started guide describes a product that never shipped

**Status**: Open. Target v1.3.0: rewrite, not retire (Paul, 22 October 2026).
**Raised**: 22 October 2026, from the pre-v1.3.0 adoption review.
**Severity**: High as a claims problem. It promises features a user will
not find, and two statements in it are false.

## What is wrong

`docs/user-guide/getting-started.md` (241 lines, "last-updated: 25 March
2026", "status: published"):

- `:31`: Protect embeds credentials in "images, documents, audio, and
  video" and adds "an invisible watermark". Watermarking is off
  (`V1_SHOW_WATERMARK = false`); video and audio are disabled.
- `:62`: FFmpeg "required for video and audio file analysis".
- `:93`: "start the sidecar service before running a verification". The
  engine starts by itself.
- `:115`, `:223`, `:225`: GIF, BMP, MP4, MOV, MP3, WAV, PDF as supported.
- `:140`: verdicts "Authentic", "Inconclusive" and "Synthetic". The bands
  are High Trust, Uncertain (BL-UX-002) and Low Trust.
- `:182-192`: a watermark step, and `:190` says it "survives JPEG
  compression, resizing, and 30% cropping". Our own report
  (`docs/watermark-robustness-report.json`) shows 0% survival at a 20% crop.
- `:13`: "no data leaves your device". claims.md row 6 marks that wording
  stale.
- `:231`: the superseded UnivFD v9 figures (BL-CLAIM-006).

## What would fix it

Rewrite it against v1.3.0 as shipped, in plain English for a
non-technical reader:

1. Install, the one first-run screen (BL-UX-006), and the sample image.
2. Check a photo: what the three bands mean, what Content Credentials
   show, what a missing label does not mean (BL-UX-004).
3. Save a Verification Record and pass it on.
4. Sign a file with Local Signing (BL-UX-003).
5. A short "For developers" pointer to the CLI and API guides
   (BL-DOC-002).
6. What Trace sends over the network, and when, matching the README
   disclosure.

Every feature in it must be one a user can reach in v1.3.0 (see the
"what actually ships" table in the brain's `products/trace.md`). Then
check the public `GETTING_STARTED.md` on Jura-Labs/jura-trace and the
website guide say the same.

## What not to do

- Do not keep any sentence about watermarking until BL-WM-001 ships and
  its robustness is re-measured.
