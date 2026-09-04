# BL-DEPS-003: four reachable advisories in the shipped product, found the first time anyone looked

**Status**: Open. Found 4 September 2026, by the first full supply-chain
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

## The four

### 1. openssl 0.10.75, five HIGH memory-safety CVEs

Reached through `c2pa` and `native-tls`, so it is in the shipped binary on
the C2PA signing and verification paths. CVE-2026-41676, -41678, -41681,
-41898 and -42327, all fixed in 0.10.80.

**This is the cheapest fix in the audit and should be done first.** 0.10.80
is semver-compatible with what is in the lock file, so it is one
`cargo update -p openssl` and no manifest change.

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

1. `cargo update -p openssl`. Minutes, five HIGHs, no API change.
2. Gate drag-and-drop to the picker's extensions (BL-DOC-001), narrowing
   the lopdf exposure for under a day of work.
3. Pillow to 12.x with the decode-drift test, recalibrating if it moves.
4. tauri to 2.11.5.
5. SHA-pin `tauri-apps/tauri-action`.
6. lopdf to 0.42, with time budgeted for the API break.
7. November: c2pa, quick-xml, python-multipart, starlette.
