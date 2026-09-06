# BL-DOC-001: PDFs already half-work, by drag and drop, and the help page says they do not

**Status**: Open. Found 3 September 2026, assessing document support for
v1.1.0.
**Raised**: 3 September 2026
**Severity**: Medium as a correctness problem, low as a build problem. Most
of the capability already exists, which makes this the cheapest new feature
on the list and also a live inconsistency today.

## What is wrong

**The file picker excludes PDFs. Drag and drop does not.**

`ui/src/routes/verify/+page.svelte:1458-1463` filters PDF out of the file
picker, with the comment "PDF dropped 2026-05-11". The drag-and-drop
handler at lines 1419 to 1447 has **no extension gate at all**. A user who
drags a PDF onto the Verify page today gets a real analysis run.

What they get back is not nothing. `src-tauri/src/format_router.rs:109-136`
routes it to `ContentType::Document`, and `src-tauri/src/pdf_provenance.rs`,
387 lines built on `lopdf` 0.34, already runs for PDFs from
`verify/pipeline.rs:811-828`. The UI already renders the result panel at
`verify/+page.svelte:4074`. It reports producer software, incremental
saves, signature fields, redactions, PDF/A conformance and embedded fonts,
alongside filename analysis and a SHA-256.

Meanwhile `ui/src/routes/help/format-support/+page.svelte:70` tells the
user:

> Not supported in this release. PDF files are excluded from the Verify
> file picker. Jura Trace v1.0 analyses still images only.

The middle sentence is true. The first and last are not, for anyone who
drags rather than clicks.

## Why it matters

Two different users get two different products depending on which gesture
they use, and the documentation describes only one of them. A journalist
who drags a PDF gets provenance findings the help page says do not exist,
with no framing to tell them what the analysis did and did not cover. There
is no C2PA reading and no forensic analysis in that path, so the result is
thinner than an image result while looking like the same thing.

This is the same failure class as BL-CI-001 and BL-REL-002: a thing that
looks like it works, is partly wired, and has no gate telling anyone which.

## What is missing for real PDF support

**C2PA reading is one feature flag away.** The `c2pa` crate at 0.79.3
(`Cargo.lock:740`) ships a PDF asset handler behind an off-by-default `pdf`
feature. `src-tauri/Cargo.toml:63` enables only `file_io`, so PDF manifests
currently error as an unsupported format. Enabling it gives **read-only**
support; the crate's own `pdf_io.rs` says PDF write "will be added in a
future release", so signing PDFs is not available at this version whatever
we do.

**The forensic detectors do not apply.** Of the thirteen, only C2PA is
meaningful on a PDF. EXIF-anomaly is effectively replaced by the existing
PDF provenance analysis. The other eleven are raster analyses. Running them
on images *embedded inside* a PDF needs an extraction layer that does not
exist, plus per-image result plumbing. That is the difference between a
three-to-five-day feature and a fifteen-to-twenty-five-day one.

## What would fix it

The minimum that is honest and useful, estimated at three to five days:

1. **Enable the `pdf` feature** on the `c2pa` dependency, giving C2PA
   manifest reading on PDFs.
2. **Add PDF to the file picker**, so the two entry points agree.
3. **Frame the result as a document mode**, explicitly: this is provenance
   and structure analysis plus Content Credentials if present, and it is
   not the forensic stack. A user must not read a thin PDF result as a
   clean forensic result.
4. **Correct `help/format-support`** to describe what the mode actually
   does.
5. **Tests**, including a PDF carrying a C2PA manifest and one without.

If PDF support is not wanted in this release, the cheap fix is the opposite
one: **gate drag and drop to the same extensions as the picker**, so the
product does one thing and the documentation is true. That is under a day
and it should happen either way, because today the two paths disagree.

## The constraint that must not be broken

Jura Trace is on the C2PA conforming products list as a Validator for
specification 2.2, and that record is **scoped to still images**: JPEG,
PNG, TIFF and WebP. It does not cover PDF.

So if PDFs are accepted: do not show the conformance badge or claim on a
PDF result, do not describe PDF validation as covered by the conformance
listing, and keep "C2PA Conformant Validator" scoped to the four image
formats in all copy. PDF is C2PA reading outside the assessed scope until a
re-assessment says otherwise. This is claims register row 15 and it is
currently `true`; extending the claim by implication is the fastest way to
make it false.

## What not to do

Do not enable the `pdf` feature and leave the help page saying PDFs are
unsupported. That converts a documentation error into a larger one.

Do not let a PDF result render in the same visual frame as a full image
verdict without a label. The trust score means something different when
eleven of thirteen detectors did not run, which is the same problem
JTV-215 raises for degraded image results.
