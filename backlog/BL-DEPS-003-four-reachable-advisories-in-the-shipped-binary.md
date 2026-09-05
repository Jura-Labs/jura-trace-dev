# BL-DEPS-003: five reachable advisories in the shipped product, found the first time anyone looked

**Status**: Open, 3 of 5 resolved. Found 4 September 2026, by the first full supply-chain
audit since July.
**Raised**: 4 September 2026
**Severity**: High. These are in the shipped binary and the shipped sidecar,
on code paths that process files supplied by whoever the user is
investigating. Nobody knew, because all three detection routes had been
silent for months.

## How this was missed

The OSV scan wrote no SARIF from 6 July (BL-CI-002). `pip-audit` and
`cargo-audit` live in `ci.yml`, which has not run since 30 April
(BL-CI-001). Dependabot was capped out in all four ecosystems
(BL-DEPS-001). Three independent mechanisms, all quiet at once, for
roughly two months.

The frontend got all the attention because `npm audit` is the one command
somebody ran by hand. It was the wrong place to look: **no npm finding
reaches a user at all** (BL-DEPS-002). Everything below is Rust or Python,
where nobody had looked.

## The five

### 1. openssl 0.10.75, five HIGH memory-safety CVEs

Reached through `c2pa` and `native-tls`, so it is in the shipped binary on
the C2PA signing and verification paths. CVE-2026-41676, -41678, -41681,
-41898 and -42327, all fixed in 0.10.80.

**Resolved: 2026-09-04, branch `fix/openssl-advisories`, commit 3ffc21d.**
openssl 0.10.75 to 0.10.81, openssl-sys 0.9.111 to 0.9.117, **and
openssl-src 300.5.5+3.5.5 to 300.6.1+3.6.3**. Lockfile only, all three
semver-compatible. Verified with `cargo check --all-targets` and the full
Rust suite: 568 lib tests plus 20 API integration tests, zero failures.

**The audit's prescription was incomplete, and the gap is worth recording.**
It said "one `cargo update -p openssl`". This build vendors OpenSSL rather
than linking the system library, so the C library is statically compiled in
through `openssl-src`. That command moves the Rust bindings and leaves
`openssl-src` where it is, so the shipped binary would still have carried
OpenSSL 3.5.5. Memory-safety issues in OpenSSL are usually in the C
library, which means the bindings bump alone could have looked like a fix
while changing nothing that mattered.

The general lesson: when a crate wraps a vendored native library, check
whether the `-src` crate moved too. `cargo update -p <crate>` will not do
it for you.

### 2. Pillow 11.2.1, heap out-of-bounds writes

CVE-2026-59199 reachable from `Image.paste()` and `crop()`, CVE-2026-59205
from ImageCms. Both are in code paths `sidecar/app/services/content_type.py`
and `clip_detector.py` run **on the file being investigated**, which is the
least trustworthy input this product handles. The fixes exist only in 12.x.

This changes the standing decision on Pillow. It was held pending a
decode-drift test, because every forensic feature starts from PIL-decoded
pixels and the thresholds are calibrated against the current decoder. That
reasoning was right about maintenance and wrong about security. **The drift
test still runs, but it now decides whether to recalibrate, not whether to
take the fix.**

Note the likely shape of any drift: Pillow 11.3 and later ship native AVIF,
which can take precedence over the pinned `pillow-avif-plugin` 1.5.5. If
AVIF vectors move while JPEG and PNG hold, that is a decoder swap rather
than drift, and the fix is to pin the decoder explicitly, not to reject the
bump.

**Resolved: 2026-09-05, branch `fix/pillow-12-cves`, PR #29.** Pillow
11.2.1 to 12.3.0. The drift test ran on 84 golden images across five
formats, and found none: 84/84 bit-identical on decode and on every
downstream array, so no recalibration and no change to production weights.
Method and results in `docs/calibration/pillow-12-decode-drift-sep2026.md`.

The AVIF hazard predicted two paragraphs above was real and did not bite.
`PIL.features.check("avif")` does flip False to True across the bump, so
Pillow 12 can decode AVIF natively, but `pillow-avif-plugin` still handled
every AVIF file with byte-identical output. Worth recording that the
prediction was correctly shaped and correctly bounded: it named the risk,
named the tell, and named the fix if the tell fired.

### 3. lopdf 0.34.0, stack overflow on hostile PDFs

RUSTSEC-2026-0187: a crash via deeply nested PDF objects. Direct
dependency, `src-tauri/Cargo.toml:92`, used by `pdf_provenance.rs` and
`verify/pipeline.rs`.

**Read this against BL-DOC-001.** That item records that the Verify file
picker excludes PDFs but drag-and-drop has no extension gate, so a dropped
PDF already runs the pipeline today. The two findings together mean a
hostile PDF can already reach a parser with a known crash, through a path
the help page says does not exist. Neither item is serious alone. Together
they are the most direct "attacker-supplied file reaches vulnerable code"
route in the product.

The fix is 0.42.0 and it is API-breaking, so budget for it. Gating
drag-and-drop, which BL-DOC-001 already proposes as its cheap option,
narrows the exposure in the meantime and costs under a day.

### 4. tauri 2.10.2, origin confusion in the IPC boundary

CVE-2026-42184, moderate: remote pages can invoke local-only IPC. Fixed in
2.11.1; current is 2.11.5. Practical risk is low, because the webview never
navigates to a remote origin. It is on the list anyway because it is the
IPC trust boundary, which is the one place where "low practical risk"
should not be the end of the conversation.

**Resolved: 2026-09-05, branch `fix/rust-advisories`, PR #28.** tauri
2.10.2 to 2.11.5, and `tauri-build` 2.5.5 to 2.6.3 alongside it. The audit
predicted `tauri-build` would need a separate deliberate bump; it moved on
its own with the lockfile update, so that prediction was wrong in a
harmless direction.

**One thing Paul should look at rather than wave through.** This bump adds
28 crates and removes 7, including a full HTML parser, a CSS parser and
selector engine, dbus, and objc2 bindings for core-location and
user-notifications. Linking a binding is not calling an API, and nothing in
this codebase requests location or posts notifications. But a local-first
privacy product growing a location binding through a transitive dependency
is the kind of thing that should be noticed on the way in rather than
discovered by somebody reading our dependency tree back to us.

### 5. pillow-heif 1.2.0, added 2026-09-05

PYSEC-2026-2258, fixed in 1.3.0. Not in the original audit because nothing
was scanning; it surfaced on the first `pip-audit` run that actually
reported (see "What the first run actually looked like" below).

This belongs beside item 2, not in the November pile. `pillow-heif` decodes
HEIC, which means it runs on the file being investigated, on exactly the
untrusted-input path that made the Pillow bump urgent. It was deliberately
left out of PR #29 so that the decode-drift evidence in that PR stays
attached to the single variable it actually tested. It needs the same
treatment: a small bump, re-running the drift harness over the HEIC subset.
Half a day, and it should go in this release rather than November.

## Two more, for the November release

**quick-xml 0.38.4 and 0.39.2**, RUSTSEC-2026-0194 and -0195, denial of
service reachable through XMP parsing of hostile files. Arrives via `c2pa`,
so the fix is gated on bumping that crate.

**c2pa 0.79.3 against 0.90.18.** No advisory against 0.79.3 itself, but it
carries the vulnerable quick-xml and eleven minor versions of upstream
validation fixes, on the crate that holds C2PA Validator conformance. This
is a conformance task with a test plan, not a dependency bump: the
conformance vectors must be re-run afterwards, and `Cargo.lock` diffed for
c2pa, coset and x509. That is SR-34.

## Accepted, with reasons

- **rsa, RUSTSEC-2023-0071**, the Marvin timing attack. No fix exists
  upstream. Exploiting it needs an observer positioned to time operations,
  and this is a local-first desktop application. Accepted.
- **The unmaintained GTK3 crates**, roughly eighteen of them, arriving
  through `wry` on Linux. Ecosystem-wide, upstream's to solve, and OSV
  reports them as findings where `cargo-audit` calls them warnings.
- **rustls-webpki 0.103.9**, a panic on a malformed CRL. Only reachable in
  Enhanced network mode, which is not the local-first default.
- **tar 0.4.45**, via `tauri-plugin-updater`. Updater payloads are
  signature-verified before extraction.
- **vitest**, the one critical. A development tool that never ships.

## What the first OSV run will look like

Expect roughly **130 to 140 results across about 60 packages**, of which
Pillow alone contributes 22, python-multipart 8, starlette 8, openssl 8 and
undici 13, plus the eighteen Rust unmaintained informationals.

**A large red first run is two months of backlog, not a regression. A small
green one would mean the scanner broke again**, which is what the
SARIF-size assertion added in BL-CI-002 exists to catch. Nobody should read
the first run as bad news, and nobody should read a quiet one as good.

## What the first run actually looked like

`pip-audit` reported for the first time on 5 September 2026, on the first
push-triggered CI run after the cadence fix landed: **50 findings across 10
packages**, of which Pillow was 27 lines covering 16 distinct advisories.
The per-package estimate above held closely — starlette 8 exactly,
python-multipart 6 against a predicted 8 — which is worth recording,
because it means the audit's reachability reasoning can be trusted at
roughly this resolution next time.

After PR #29 that is **23 findings across 9 packages**, with Pillow gone.
The 23 break down as: starlette 8, python-multipart 6, and one each from
pillow-heif, click, idna, pydantic-settings, pygments, pytest and
python-dotenv. Of those, only pillow-heif (now item 5) and python-multipart
touch untrusted input; the rest are development and tooling surface.

**`pip-audit` therefore stays red after both of this week's dependency PRs,
and that needs a decision rather than a habit.** A check that is
permanently red teaches everyone to scroll past it, which is the same
failure as a check that is permanently green for the wrong reason
(BL-SILENT-001). Two honest options: fix the remaining 23 before the
release, or scope the gate to fail only on findings that reach shipped code
and report the rest without failing. Paul's call, but it should not simply
sit red for two months.

## Not an advisory, but the worst pin in the repository

`tauri-apps/tauri-action@v0` (`release.yml:760`, `:777`, `:1118`) is a
moving major-zero tag on the action that **builds and signs the release
artefacts**. Whoever controls that tag controls what is signed with the
Apple and Azure certificates. No action in this repository is SHA-pinned;
`dtolnay/rust-toolchain@stable` moves too. SHA-pin the release-signing
actions at minimum. This is a bigger exposure than most of the advisories
above, and it has no CVE to make it visible.

`blind_watermark = "0.1"` (`Cargo.toml:95`) also still floats, on a crate
with a few hundred lifetime downloads. That is BL-WM-001's problem too.

## Clean, and worth recording as such

Zero git-sourced crates in `Cargo.lock`. Every npm package resolves to
registry.npmjs.org. No install or postinstall scripts in the UI lock file.
`requirements.lock` is hash-pinned with `--generate-hashes`. The Cloudflare
worker has no runtime dependencies at all.

## Order of work

1. ~~`cargo update -p openssl`~~ **Done**, `fix/openssl-advisories`, and it
   needed `openssl-src` too.
2. ~~Gate drag-and-drop to the picker's extensions~~ **Done**,
   `fix/gate-drag-and-drop`, awaiting merge.
3. ~~Pillow to 12.x with the decode-drift test~~ **Done**, PR #29. No
   drift, no recalibration.
4. ~~tauri to 2.11.5~~ **Done**, PR #28, with the crate-growth question
   above open for Paul.
5. SHA-pin `tauri-apps/tauri-action`. Branch `fix/sha-pin-actions` ready,
   awaiting merge.
6. **pillow-heif to 1.3.0** with the drift harness re-run over the HEIC
   subset. New item 5; half a day; belongs in this release.
7. Decide what `pip-audit` should gate on, so the check stops being
   permanently red. See "What the first run actually looked like".
8. lopdf to 0.42, with time budgeted for the API break.
9. November: c2pa, quick-xml, python-multipart, starlette.
