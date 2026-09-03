# BL-WM-001: watermarking is two coupled bugs and a bundle problem, not a flag flip

**Status**: Open. Raised 3 September 2026, when watermarking was added to
the v1.1.0 scope.
**Raised**: 3 September 2026
**Severity**: Medium as a defect, high as a scope risk. The feature is
claimed on two published pages and does not exist in the product, which is
claims register row 7, currently `conflict`.

## What is wrong

`ui/src/lib/featureFlags.ts:81` is `V1_SHOW_WATERMARK = false`, and the
comment above it (lines 55 to 80) is the diagnosis, made on 21 May 2026 in
commits `c8b82bf` and `9fc4a22`. Two coupled bugs:

**1. Embed and extract disagree.** The Rust side embeds with the
`blind_watermark` crate (`src-tauri/Cargo.toml:95`, version 0.1). The Python
side extracts with `imwatermark`. They "use incompatible bit placement", so
what one writes the other cannot read.

**2. The extractor drags in torch.** `imwatermark`'s package init eagerly
imports rivaGan, which imports torch. PyInstaller statically analyses that
and pulls torch into the bundle *even with* `excludes = ["torch"]`,
**doubling the bundle to 1.5 GB**.

The second bug has already been acted on. `sidecar/requirements.txt:69-71`
records that `invisible-watermark` was **removed from the runtime
requirements entirely**, precisely so that torch stays out of the lock file
and the bundle. So the extraction dependency is not merely unused, it is
not installed in the shipped sidecar.

That is why this is not a flag flip. Flipping `V1_SHOW_WATERMARK` to true
exposes a UI whose embed writes bits nothing in the shipped product can
read.

## Why the bundle number matters more than it looks

The v1.0.0 macOS artefact is already 815 MB, and the Linux build had to be
cut to deb-only in commit `aa6f562` because "RPM hangs xz-compressing the
607MB onnx". A bundle at 1.5 GB would not be an inconvenience. It would
very likely break the Windows and Linux CI builds outright, on a pipeline
that is currently the subject of BL-CI-001 and has not run since April.

## The finding that matters more than either bug

`docs/watermark-robustness-report.json`, generated 21 March 2026 over 20
images at medium strength with an 11-byte payload:

| Degradation | Survived |
|---|---|
| Original PNG | 100% |
| JPEG Q95 | 100% |
| JPEG Q85 | 100% |
| **JPEG Q70** | **0%** |
| **JPEG Q60** | **0%** |
| Resize 50% then back to 100% | 100% |
| **Resize 75%** | **0%** |
| **Crop 20%** | **0%** |
| **Screenshot simulation** | **0%** |

There is a cliff between Q85 and Q70, and everything below it is total
failure, not degradation.

**Read that against what the feature is for.** The stated value is a
backstop for when a C2PA manifest is stripped: you can still recover proof
that an asset was yours. The way an asset gets stripped in practice is that
somebody downloads it, a platform recompresses it, somebody screenshots it,
or somebody crops it. Those are the four columns reading 0%.

So the algorithm works on lossless and lightly compressed copies, which is
to say on files that mostly still carry their manifest anyway, and fails on
exactly the population the feature exists to serve. That is not a bug to be
fixed by better plumbing; it is the algorithm's operating envelope.

This does not make it worthless. Tamper-evidence within a controlled
archive workflow, where files stay lossless, is a real use. It does mean
the feature cannot be described as scrape detection, and that a forensic
user cannot rely on the absence of a watermark meaning anything at all.

## What the original decision was, and it was not arbitrary

The same comment records that three agents (persona-testing,
content-authenticity-expert, tech-debt-analyst) agreed v1.0 should ship
without it, that **9 of 10 B2B personas do not need it**, and that the
"manifest-strip-backstop value prop is theoretical and not demanded by any
pilot user".

It also names the intended re-enable point: **"v1.1: true"**. The release
now being planned is v1.1.0, so shipping it here is the documented plan,
not a departure from it. The conditions attached are the work.

## What would fix it

The flag comment gives two routes and they are not equivalent.

1. **Vendor `imwatermark` with the rivaGan import made lazy.** The
   content-authenticity-expert's recommendation. Smaller change, keeps the
   existing algorithm, but leaves a vendored fork to maintain and still has
   to be proved not to pull torch through PyInstaller's static analysis,
   which is the thing that defeated `excludes` last time.

2. **Replace with a pure-Rust DWT-DCT-SVD implementation** so embed and
   extract use one library and torch never enters the picture. Larger
   change, removes the whole class of problem, and removes a Python
   dependency from a security-sensitive path.

Route 2 is the better end state and the worse fit for a four-week release.
Route 1 is the faster fit and leaves the bundle risk live until proved.

Whichever is chosen, these ride along if the feature becomes visible:

- **JTV-117**, the robustness claims, which on the numbers above is the
  whole job rather than a copy tweak. The UI has to state the envelope
  plainly: survives lossless and light JPEG, does not survive Q70 or below,
  cropping, resizing or screenshots.
- **JTV-118**, the PNG-only output warning before embedding. `watermark.rs`
  already forces PNG output because lossless is "required for survival",
  which the table explains.
- **A `payload_hex` length cap** in `embed_watermark_asset` before it
  reaches the sidecar, which currently truncates silently at 64 bytes.
- **Pin `blind_watermark`**, currently `"0.1"` unpinned
  (`security-audit-2026-05-16.md`, NEW-LOW-1).

**Correction to an earlier draft of this file:** the JTV-206 path-validation
and path-echo items are already fixed, on 25 March 2026.
`extract_watermark_from_path` no longer exists and the embed command no
longer echoes paths (`lib.rs:2227-2230`). Only the payload cap and the
unpinned crate remain from that group.

## Effort

Eight to twelve working days: three to four to vendor and patch
`imwatermark`, or write the Rust extract, which is riskier; one to two to
verify bundle size on all three platforms; two to re-run robustness and
write honest caveats; two for the payload cap, the pin, the flag flip and
the documentation.

The plumbing is the easy part. The robustness table is the hard part,
because no amount of engineering changes it.

## The cheaper alternative, stated plainly

Claims row 7 is `conflict` because two published pages say Trace applies
invisible watermarks and it does not. There are two ways to resolve that:
ship the feature, or correct the pages. Correcting the pages is already
queued as item 6 of the site-update bundle and costs an afternoon.

Shipping the feature is the better outcome and the more expensive one. That
is a judgement about what the product is for, and it belongs to Paul. What
this file records is that the two are alternatives, and that the release
does not need the feature in order to stop the claim being wrong.

## What not to do

Do not flip the flag to true and ship without fixing the embed and extract
mismatch. It would produce a feature that silently writes watermarks nobody
can recover, which is worse than not offering it, and worse than the
current claim problem.

Do not re-add `invisible-watermark` to `sidecar/requirements.txt` without
measuring the bundle. The line above it exists because somebody already
learned that lesson.
