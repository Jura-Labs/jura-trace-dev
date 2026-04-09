# EXIF injection detection

Design for backlog item #4 — detecting fabricated, templated, or programmatically
injected EXIF metadata blocks that try to impersonate a genuine camera capture.

Added as a set of sub-checks under the existing `exif_anomaly` detector. This is
deliberately **not** a new top-level detector ID — it is additional logic inside
`src-tauri/src/exif_anomaly.rs` that emits `AnomalyFinding`s through the existing
pipeline, alongside the existing software / timestamp / GPS / dimension / MakerNote
checks.

## Literature summary

- **Farid (2009), "Exposing Digital Forgeries from JPEG Ghosts"** — establishes
  that metadata is not authoritative and can be trivially rewritten; motivates
  corroborating pixel-level evidence. We take the inverse lesson: we can still
  extract *shape* signals from a rewrite attempt because fakers rarely
  reproduce vendor-specific quirks.
- **Gloe (2012), "Forensic analysis of ordered data structures on the example of JPEG files"** —
  documents JPEG / EXIF field-ordering and tag-set fingerprints that differ between
  camera firmware and editing libraries (`libexif`, `PIL`, `ImageMagick`). The
  software-field heuristic in this detector is a coarse form of that idea.
- **Kee, Johnson and Farid (2011), "Digital Image Authentication from JPEG Headers"** —
  finds that camera-native JPEG headers (Q-tables, Huffman tables, thumbnail
  presence, EXIF tag ordering) cluster tightly per-model and that edited or
  re-saved files rarely reproduce the native cluster. We do not implement this
  directly — it needs a corpus — but item #4 in the deferred list below
  describes a learned-classifier extension.
- **Thakur and Bajaj (2021), "Review on Passive-Blind Image Forgery Detection"** —
  catalogues metadata-consistency heuristics and notes the high false-positive rate
  of any single metadata rule in isolation. Motivates the "bias away from Critical"
  severity discipline below.
- **Fang et al. (2023), "Detection of AI-generated images with metadata fingerprints"** —
  shows that contemporary generator outputs (Stable Diffusion, DALL-E 3, Midjourney v6)
  often carry a mandatory or configurable Software field naming the library rather
  than a camera firmware string. This maps directly onto the Class 5 heuristic below.

## Threat model recap

EXIF injection, for our purposes, means: metadata fields populated by a process
that is **not** camera firmware. Seven observed forms:

1. Generator outputs with manufactured EXIF (templated field sets)
2. Strip-then-reconstruct via `exiftool` / `PIL` etc.
3. Copy-paste EXIF lifted from an authentic image and grafted on
4. MakerNote absent while other fields declare a camera vendor that always writes one
5. Impossibly precise values (integer-degree GPS, zero-second timestamps)
6. Programmatic-pipeline software fields (`PIL`, `Pillow`, `ImageMagick`, `OpenCV`, `Photopea`)
7. Inconsistent field combinations (iPhone declaring sRGB, lens-body interop mismatch)

## Classes implemented in this iteration

### Class A — Programmatic pipeline software field

**Signal.** `software` field matches a known imaging-library or batch-pipeline name:
`PIL`, `Pillow`, `ImageMagick`, `GraphicsMagick`, `OpenCV`, `skimage`,
`scikit-image`, `Photopea`, `libvips`, `sharp`, `Magick`.

These are distinct from the existing `METADATA_TOOLS` list (exiftool / exiv2 /
jhead / pyexiv — which write metadata without touching pixels) and from the
`IMAGE_EDITORS` list (user-facing GUI apps like Photoshop, GIMP). A pipeline
library in the Software field means the file was assembled by a script, not
exported by a camera and not opened by a human editor.

**Severity: High.** Very low false-positive rate — genuine camera firmware
almost never writes these names, and Lightroom / Capture One / Photoshop write
their own product names, not the underlying library they embed. When this
finding co-occurs with a claimed `Make` / `Model`, the combination is extremely
suspicious.

**Known legitimate triggers.** Scientific and archival workflows that
consciously re-encode through Pillow or ImageMagick (e.g. batch thumbnailing,
colour-space conversion). In those workflows the image is no longer a
primary source — downgrading trust is the correct behaviour, but the finding
text should be descriptive rather than accusatory.

### Class B — Templated "suspiciously round" timestamps

**Signal.** `datetime_original` AND `datetime_modified` are both present, and
**both**:
- end in `:00` seconds, and
- are identical to the minute or to the second.

The conjunction is the point — real captures sometimes have `:00` seconds by
coincidence (roughly 1 in 60), but two independent timestamps simultaneously
hitting `:00` AND being identical is ~1 in 3,600 for coincidence, versus ~1 for
a templated field set where a script wrote the same string into both tags.

Additionally we flag canonical template values specifically: `2023:01:01 00:00:00`,
`2024:01:01 00:00:00`, `2024:01:01 12:00:00`, `2025:01:01 00:00:00`,
`2000:01:01 00:00:00` — values that appear in `exiftool` tutorials and in
several public injection scripts.

**Severity: Low on the coincidence case, Medium on the canonical-template case.**
Timestamps are too easily explained by clock errors and sub-minute camera bursts
to warrant High on their own.

**Known legitimate triggers.** Timer-triggered cameras, security cameras, and
certain batch import tools normalise timestamps. Scientific time-lapse rigs
occasionally use round-second intervals. The Low severity and descriptive text
absorb this.

### Class C — Integer-degree GPS coordinates

**Signal.** Both latitude and longitude, in decimal degrees, are within
`1e-6` of an **integer value** AND the non-integer fractional part of each is
exactly zero after rounding to 6 decimal places. Real consumer GPS chips
produce coordinates with residuals well beyond the sixth decimal place; even
when a capture is on a surveyor's marker, the recorded coordinates carry the
chip's natural jitter. A lat/lon pair at `(51.000000, -1.000000)` is almost
certainly hand-written.

We additionally require the coordinates not to be at `(0, 0)` — the existing
`gps_null_island` check already owns that case at Medium severity.

**Severity: Medium.** Two independent values simultaneously hitting exact
integers is improbable enough on its own, but the signal is weaker than Class A
because some mapping / planning tools do emit integer coordinates for mocked
points.

**Known legitimate triggers.** Manually entered coordinates for illustrative
figures in reports, test fixtures exported into the wild, some flight-planning
tools writing mission waypoints. The finding text names "manually entered" as
the most likely explanation — keeping the user's agency rather than jumping
straight to fabrication.

### Class D — MakerNote absent on a vendor that always writes one

**Signal.** `camera_make` is present and matches a vendor in a new
`MAKERNOTE_MANDATORY_VENDORS` subset of the existing `KNOWN_CAMERA_VENDORS`
list — vendors whose firmware has been observed to write a MakerNote on
**every** capture: `apple`, `canon`, `nikon`, `sony`, `fujifilm`, `olympus`,
`om digital`, `panasonic`, `leica`, `hasselblad`, `phase one`, `ricoh`,
`pentax`. AND `has_maker_note` is `false` OR `maker_note_length < 16`.

Deliberately conservative: only the camera brands where MakerNote is
essentially universal. Smartphone vendors (Google Pixel, Samsung) are
excluded from the mandatory list because Google Photos backup, WhatsApp,
Telegram and similar pipelines routinely strip MakerNote without otherwise
touching the file, and this detector must not fire on every re-shared phone
photo.

**Severity: Medium.** This is a compound signal — the claimed vendor is
inconsistent with the absence of the vendor's proprietary block. But the
sharing-platform-strip pathway is common enough that High would over-fire.
Note specifically that **MakerNote absence alone is never fired** — there must
also be a `Make` claim matching a vendor that always writes one. This honours
the CLAUDE.md-level constraint "do not fire on MakerNote absence alone".

**Known legitimate triggers.** Files that have been through Google Photos
re-encoding, iCloud download, WhatsApp, Telegram, Signal, or any social media
platform that strips non-essential tags. Also some RAW-to-JPEG conversions
through Lightroom / Capture One when the user has MakerNote stripping enabled.
We accept the medium FP rate here because the interaction with
`camera_authenticity_bonus` (see below) means a genuinely-authentic file with
MakerNote intact receives +0.25 trust, so the *net* effect on a clean file is
unchanged.

### Class E — iPhone declaring sRGB colour space

**Signal.** `camera_make` contains "apple" (case-insensitive) AND `camera_model`
contains "iphone" AND `color_space` equals "sRGB" AND `has_maker_note` is false.

Modern iPhones (iPhone 7 and later) write the Display P3 profile by default,
represented in EXIF as `Uncalibrated` with an accompanying ICC profile or as
an explicit Display P3 tag. An "iPhone" claim paired with a literal `sRGB`
colour space and no MakerNote strongly suggests a hand-rolled EXIF block.

**Severity: Low.** This is a narrow heuristic — older iPhone models and
certain third-party camera apps (Halide, ProCam) can and do write sRGB
intentionally. The requirement that MakerNote also be absent is what makes
the compound safe; a genuine iPhone with MakerNote intact never fires here.

**Known legitimate triggers.** Screenshots taken on an iPhone (these carry a
Software tag of "iOS <version>" rather than a camera capture signature — and
usually lack `camera_make` entirely, which blocks this check); iPhone 6 and
earlier, which predate Display P3; third-party camera apps that write sRGB
intentionally. Low severity absorbs all three.

## Classes considered and rejected

- **Class 3 (copy-paste EXIF)** — requires either a known-authentic corpus to
  fingerprint field sets against, or a learned model. Out of scope for a
  rules-only iteration. Could be revisited as a learned classifier following
  Kee / Johnson / Farid 2011.
- **Lens-body interoperability** — too many real-world exceptions (third-party
  adapters, manual lenses, lens databases that disagree with the camera's
  reported lens). False-positive rate too high for a production rule.
- **Field-ordering fingerprint** (Gloe 2012) — kamadak-exif does not currently
  preserve the on-wire tag order after parsing, and the reorder step is lossy.
  Would require a separate raw-bytes pass at the metadata-extraction layer.
  Flagged as a future iteration.
- **Quantisation table fingerprint** — out of scope for this module; belongs
  in the sidecar JPEG Ghost / splice-boundary detectors, not in EXIF.
- **Generator-specific XMP tags** (`Stable Diffusion`, `AI-generated: true`)
  — XMP is not currently parsed by `metadata.rs`. Added as a follow-up TODO
  rather than extending the parser in this PR.

## Severity assignment summary

| Class | Finding ID                           | Severity | Justification                                                   |
|-------|--------------------------------------|----------|-----------------------------------------------------------------|
| A     | `software_pipeline_library`          | High     | Near-certain signal when combined with a camera Make / Model claim |
| B     | `timestamp_templated_identical`      | Low      | Coincidence possible; compound against Class B-canonical         |
| B     | `timestamp_canonical_template`       | Medium   | Hitting a known `exiftool` tutorial literal is near-certain      |
| C     | `gps_integer_degrees`                | Medium   | Two independent integer values is rare but planning tools exist |
| D     | `maker_note_mandatory_vendor_missing`| Medium   | Compound signal, but sharing-platform strip keeps this off High |
| E     | `iphone_colour_space_mismatch`       | Low      | Narrow; absorbs older iPhones and third-party camera apps       |

None of these are `Critical`. That tier remains reserved for the existing
`software_ai_generator` check, which is a ground-truth signal rather than an
inferred-from-context one.

## Interaction with `camera_authenticity_confidence`

The positive counter-signal added in Sprint 29 Track 1 (`camera_authenticity_confidence`
in `metadata.rs`) scores MakerNote + vendor-match on `[0.0, 1.0]` and is applied
as a **trust bonus** in the deepfake scoring path (up to 0.25 score reduction
on the AI probability).

This new injection detector is the negative counterpart. The interaction is:

- `camera_authenticity_bonus` is applied to the **deepfake classifier output**
  in `compute_trust` via `camera_authenticity_bonus` as a score deduction on
  the AI probability.
- The new injection findings emit through the existing `ExifAnalysis.findings`
  list and therefore flow into `exif_trust` via the existing severity-to-deduction
  table in `Severity::deduction()`.
- **They are not mutually exclusive by construction but they are mutually
  exclusive in practice**: a file with MakerNote ≥ 16 bytes and a known vendor
  is given the full positive bonus AND will not trigger Class D (which requires
  MakerNote absent). Classes A, B, C, E can in principle co-fire with the
  positive bonus — e.g. a genuine Canon photo that has been batch-resaved
  through ImageMagick will have both `authentic_maker_note` (Info) and
  `software_pipeline_library` (High). The behaviour is correct: the file
  originated on a real camera but its current bytes were rewritten by a library,
  which is exactly what a downstream verifier should be told.
- **Class D is actively guarded** against double-counting: it requires
  `has_maker_note` false OR `maker_note_length < 16`, the same gate that
  returns 0.0 from `camera_authenticity_confidence`. So the two can never
  both be non-zero on the same file, and there is no interaction in
  `compute_trust` to reconcile.

In code, `compute_trust` does not need to change. The new findings are
absorbed into the existing `exif_trust` term through their severity weights,
and the `camera_authenticity_bonus` term continues to apply independently.

## Deferred follow-ups

1. **XMP parsing** — modern AI generators increasingly write provenance signals
   in XMP (`photoshop:Credit`, `dc:creator`, `xmp:CreatorTool`, and
   `Iptc4xmpExt:DigitalSourceType`). `metadata.rs` does not currently parse XMP.
   This would let us detect Class 1 at much higher confidence and is the single
   highest-value future extension.
2. **Field-ordering fingerprint (Gloe 2012)** — requires a second metadata
   extraction pass that preserves on-wire order. Worth investigating alongside
   a corpus of native-camera JPEGs per vendor.
3. **Learned field-set classifier** — given a corpus of known-authentic files
   per camera model, train a small classifier on `(fields_present, vendor)` and
   flag combinations outside the authentic cluster. Directly follows
   Kee-Johnson-Farid 2011. Defer until corpus is assembled.
4. **Lens-body interoperability** — possible with a curated lens/body matrix,
   but the maintenance burden is significant. Reconsider if community
   contributors want to maintain the table.
