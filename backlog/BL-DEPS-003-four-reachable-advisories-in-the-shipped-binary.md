# BL-DEPS-003: five reachable advisories in the shipped product, found the first time anyone looked

**Status**: Open, largely resolved. All five advisories, both of the "November" items and every step in "Order of work" are fixed on `main`; `cargo audit` and `pip-audit` both report zero. Nothing reaches a user until v1.1.0 ships on 13 November 2026, and the item is about the shipped product, so it closes when that release is live. See "Update, 8 September 2026" at the foot of this file. Found 4 September 2026, by the first full supply-chain
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

**The crate-growth question, raised 5 September and resolved the same day.**
The bump appeared to add 28 crates including a full HTML parser, a CSS
parser and selector engine, dbus, and objc2 bindings for core-location and
user-notifications. For a local-first privacy product, a location binding
arriving through a transitive dependency is worth stopping for.

**It does not ship, and neither does most of the rest.** The lockfile adds
26 crates and removes 5. Checked against the actual dependency tree for
each of the three shipping targets, with proc-macro edges excluded so the
result reflects what is linked rather than what is compiled:

| Added crate | macOS | Windows | Linux |
|---|---|---|---|
| `phf`, `phf_shared` | ships | ships | ships |
| `dbus`, `libdbus-sys` | no | no | ships |
| the other 22 | no | no | no |

- **The objc2 location and notification bindings are absent from every
  shipping target.** They arrive under `objc2-ui-kit`, and UIKit is iOS.
  This project has no iOS target, so they are never compiled at all. No
  location capability enters the product.
- **The HTML and CSS stack is build-time only.** `html5ever`,
  `markup5ever`, `cssparser`, `selectors`, `dom_query`, `string_cache`,
  `tendril`, `web_atoms`, `servo_arc` and friends reach us through
  `tauri-macros`, a proc-macro crate. Tauri's codegen parses `index.html`
  at compile time to do its asset and CSP work. None of it is linked into
  the binary.
- **What actually ships is four crates.** `phf` and `phf_shared` are
  compile-time perfect hash maps used by `tauri-utils`, on all three
  platforms. `dbus` and `libdbus-sys` are Linux only, via `tao`, and are
  standard desktop-session integration.

The one line worth carrying forward is the Linux D-Bus dependency, and it
is worth carrying forward only so nobody is surprised by it later: the
Linux build now talks to the session bus. That is local IPC for desktop
integration, not network egress, so it does not disturb BL-CLAIM-001, but
it is a new external surface on one platform and belongs in that item's
evidence rather than being discovered later.

**Method note, because the first two attempts at this were wrong.**
`cargo tree -i <crate>` with no `--target` answers for the host only and
reported these crates absent; with `--target all` it reports the union
across platforms we do not build, which is where "28 crates" came from.
Per-crate queries also fail on ambiguous versions, and a check that reads
"the phrase *nothing to print* is missing" as "the crate is present" turns
that failure into a false positive. That is cause D in BL-SILENT-001, and
it happened twice here. The reliable method is to dump the whole tree once
per real target with `--edges normal,no-proc-macro` and grep it, so there
is no per-query failure mode to misread.

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

## cargo-audit cannot see the advisories this item was written about

Found 5 September, when PR #28 landed the openssl and tauri bumps and
cargo-audit's output did not change by a single line.

It reports **14 vulnerabilities and 26 allowed warnings on the branch, and
exactly the same 14 on main**, where openssl 0.10.75 and tauri 2.10.2 were
both still present. Neither crate appears in its output in either place.

The reason is structural, not a misconfiguration. `cargo audit` reads only
the RustSec advisory database. These advisories live in GHSA and **have no
RUSTSEC identifier at all**. Queried against OSV, openssl 0.10.75 returns
eight entries (CVE-2026-41676, -41677, -41678, -41681, -41898, -45784 among
them) and tauri 2.10.2 returns one (CVE-2026-42184), and every one is
GHSA-only.

So the five headline findings in this item were never within cargo-audit's
reach. **OSV-Scanner is the check that covers them**, and it does scan
`Cargo.lock`; it has been producing SARIF again since the `--skip-git` fix
in BL-CI-002. Which means the two months when OSV silently scanned nothing
were two months with **no effective advisory coverage of the Rust
dependency tree at all**, because the other Rust check structurally could
not have caught these. That is worse than BL-CI-002 recorded at the time.

The 14 cargo-audit does report are genuinely RustSec-tracked and are
triaged above: lopdf and quick-xml are scheduled, rsa and rustls-webpki and
tar are accepted with reasons, and quinn-proto, h2 and crossbeam-epoch join
the accepted set pending review.

**What this changes about how the checks should be read.** "cargo-audit is
clean" does not mean "the Rust dependencies are clean"; it means "nothing
RustSec has written up is outstanding". The two are far apart, and the gap
is invisible from the output. The same asymmetry does not apply to
`pip-audit`, which queries PyPI's advisory data via OSV and did report the
Pillow findings.

Two consequences worth acting on rather than just noting:

1. **OSV-Scanner is now load-bearing for Rust and should be treated that
   way.** Its SARIF-absence assertion (BL-CI-002) is the only thing
   standing between us and a repeat.
2. **Consider whether cargo-audit earns its place.** It costs minutes of
   billable time building from source on every run, per the note at
   `ci.yml:128-132`, to check a strict subset of what OSV already covers.
   Either keep it and label it accurately in the workflow, so nobody reads
   it as full Rust coverage, or drop it and let OSV be the Rust gate.

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
and that needed a decision rather than a habit.** A check that is
permanently red teaches everyone to scroll past it, which is the same
failure as a check that is permanently green for the wrong reason
(BL-SILENT-001).

**Decided 5 September 2026 by Paul: fix the remaining 23 before the
release**, rather than scoping the gate to shipped-code reachability. The
gate stays as strict as it is and the backlog moves to meet it, so a green
`pip-audit` will mean the dependency set is clean rather than that the
question was narrowed until it went quiet. Tracked as item 7 in the order
of work.

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
7. **Clear the remaining 23 `pip-audit` findings before the release.**
   Paul's decision, 5 September: fix them rather than narrow the gate, so
   the check ends up genuinely green instead of scoped until it is quiet.
   starlette and python-multipart are the substantial half.
8. lopdf to 0.42, with time budgeted for the API break.
9. November: c2pa, quick-xml, python-multipart, starlette.

## Update, 8 September 2026

Reconciled against `origin/main` at `d0ca411d`, `src-tauri/Cargo.lock`,
the three sidecar manifests, and CI run `34284960670` on that commit.

A note on numbering first. "PR #28" and "PR #29" in the resolutions above
are `Jura-Labs/jura-archive` numbers (openssl and tauri; Pillow). The
`jura-trace-dev` repository created on 8 September has its own #28 and #29,
which are the two pull requests below. Same numbers, different changes.

### Fixed in tree versus fixed for users

Everything in this section is **fixed on `main`**. None of it is in any
installer a user can download. v1.0.0, built on 18 to 20 June, still
carries openssl 0.10.75, Pillow 11.2.1, lopdf 0.34.0, tauri 2.10.2,
pillow-heif 1.2.0, quick-xml 0.38.4 and c2pa 0.79.3, and will until v1.1.0
ships on 13 November 2026 and the updater path in BL-REL-002 delivers it.
The item's title describes the shipped binary, and the shipped binary is
unchanged.

### The five, and the two for November

| Item | State on `main` | Where |
|---|---|---|
| 1 openssl | Fixed 4 September (recorded above) | `Cargo.lock`: openssl 0.10.81 |
| 2 Pillow | Fixed 4 September (recorded above) | `requirements*.txt`, `requirements.lock`: Pillow 12.3.0 |
| 3 lopdf | **Fixed 8 September**, `jura-trace-dev` #28 (`cc0cbc8a`) | `Cargo.toml:92` `lopdf = "0.42"`; `Cargo.lock` 0.42.0. The API break the item budgeted for did not bite: `Document::load` and `Object` compiled without source changes |
| 4 tauri | Fixed 5 September (recorded above) | |
| 5 pillow-heif | **Fixed** | 1.6.0 in `requirements.txt:38`, `requirements-ci.txt:44`, `requirements.lock:368` |
| quick-xml | **Fixed 8 September**, #28 and #29 | `Cargo.lock`: 0.41.0 only. #28 moved the `plist` copy; #29 moved the `c2pa` copy |
| c2pa 0.79.3 | **Fixed 8 September**, #29 (`c1718c6d`) | `Cargo.toml:63` `c2pa = "0.90"`; `Cargo.lock` 0.90.20. Not a version bump: 0.90 deprecates the thread-local settings API, so `src-tauri/src/c2pa.rs` now parses the trust bundle once into `trust_settings()` (`c2pa.rs:1447`) and builds an explicit `c2pa::Context` per reader and builder through `trust_context()` (`c2pa.rs:1468`). The per-thread initialiser that had to be remembered is gone. The conformance vectors in `docs/c2pa-conformance/test-vectors/` were diffed between 0.79 and 0.90 in the PR; two stale fixtures were regenerated and the evidence bundle README given an accuracy note in #32 (`3ac697c7`) |

### What the two auditors say now

`cargo audit` on `main`, run 34284960670: **0 vulnerabilities** over 921
crates, "16 allowed warnings found". The 16 are 10 unmaintained (core2,
fxhash, paste, proc-macro-error, ttf-parser, five `unic-*` crates), 4
unsound (anyhow RUSTSEC-2026-0190, glib RUSTSEC-2024-0429, rand
RUSTSEC-2026-0097 twice) and 2 yanked (core2, spin). It was 14
vulnerabilities and 26 warnings when this section was written; #28 took it
to 2 and #29 to 0. Of the 14, twelve were fixed by lockfile bumps
(crossbeam-epoch, h2, quinn-proto, rustls-webpki, plist's quick-xml) and by
lopdf, so the "pending review" set above (quinn-proto, h2, crossbeam-epoch)
and rustls-webpki were fixed rather than accepted.

The one suppression is `src-tauri/.cargo/audit.toml`, ignoring
RUSTSEC-2023-0071 (rsa, Marvin) only. No fixed version exists, `rsa` is a
required dependency of every c2pa release 0.79 through 0.90.20, and the
file records why the exposure does not apply and when the entry comes back
out. Approved by Paul at the terminal, 8 September. This is the same
acceptance recorded under "Accepted, with reasons" above, now enforced in
one place that CI and a developer's terminal both read.

`pip-audit` on `main`, same run: **"No known vulnerabilities found"**. The
23 findings that stayed red after Pillow (starlette, python-multipart,
pillow-heif, click, idna, pydantic-settings, pygments, pytest,
python-dotenv) are cleared; starlette is 1.6.0 and python-multipart 0.0.32
in all three manifests. Paul's 5 September decision to fix them rather than
narrow the gate is done.

The caveat in "cargo-audit cannot see the advisories this item was written
about" still holds and is unchanged by any of this: `cargo audit` reads
RustSec only, the openssl and tauri advisories are GHSA-only, and OSV is the
check that covers them. OSV is scanning again (BL-CI-002, closed).

### Order of work, checked

1 to 4 done as recorded. 5, SHA-pin `tauri-apps/tauri-action`: **done**,
`release.yml:824` and `:841` pin `84b9d35b5fc46c1e45415bdb6144030364f7ebc5`
(v0.6.2), and the Repo hygiene job fails on any unpinned action
(`ci.yml:382-395`). 6, pillow-heif: **done**. 7, the 23 pip-audit
findings: **done**. 8, lopdf: **done**, #28. 9, the November four:
**done early**, c2pa and quick-xml in #29, python-multipart and starlette
in the pip-audit clearance.

### What remains

Ship it. The advisories reach users when v1.1.0 does, and BL-REL-002's
live-update leg is what decides whether v1.0.0 installs receive it or need
the re-download notice. Until then every "fixed" in this file means fixed
for whoever builds from `main`.
