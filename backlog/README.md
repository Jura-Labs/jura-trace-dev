# Jura Trace backlog

This directory is the system of record for Jura Trace backlog items, in the
same shape ROOTED uses. One file per item, committed to git, written for
somebody who has not seen the code.

The convention is in `AGENTS.md` under "Backlog convention". The short
version:

- A BL file states what is wrong, how it showed up, why it matters, what
  would fix it and what not to do, with file paths and line numbers.
- Implement on a branch, never on `main`.
- When it merges, append `Resolved: YYYY-MM-DD <commit> — what shipped` to
  the file. Do not delete it. The history is the point.
- Every merge gets a line in `~/jura-brain/log.md` with a deliverable ID.

## Open items

Statuses reconciled 8 September 2026 against `origin/main` at `d0ca411d`,
after the fourteen merges of that day. Each item's own file carries a dated
"Update, 8 September 2026" section with the evidence.

| Item | What it is | Status | Severity |
|---|---|---|---|
| [BL-REL-002](BL-REL-002-updater-manifest-carries-urls-not-signatures.md) | The updater manifest held URLs where signatures belong; the live manifest is now correct, the fallback resolves, and the re-download notice and the live update on a real v1.0.0 install remain | Open, largely resolved server-side | Highest |
| [BL-DEPS-001](BL-DEPS-001-dependabot-queue-is-saturated.md) | The Dependabot queue was at the cap in every ecosystem; cap raised, queue moving, 18 open across pip and cargo and actions; steps 4 to 6 remain. Security updates were never subject to the cap | Open, largely resolved | High |
| [BL-CLAIM-001](BL-CLAIM-001-nothing-leaves-the-device-is-nearly-true.md) | "No data is uploaded to external servers" is nearly true, and nearly is not good enough | Open, narrowed | High |
| [BL-DEPS-002](BL-DEPS-002-frontend-advisories-one-ships.md) | Ten frontend advisories (was twelve), none of which reaches a user | Open, partly resolved | Medium |
| [BL-UX-001](BL-UX-001-tool-object-object-in-the-c2pa-panel.md) | The C2PA panel shows "Tool: [object Object]" | Open | Low to fix |
| [BL-CLAIM-002](BL-CLAIM-002-the-app-makes-two-promises-the-release-did-not-keep.md) | Two promises shipped inside the app that the release did not keep. The Article 50 sentence and eleven frozen "v1.0" strings corrected 10 September, before v1.1.0; the model-card metadata file is still open | Open, partly resolved | Medium |
| [BL-DOC-001](BL-DOC-001-pdfs-already-half-work-and-the-help-page-denies-it.md) | PDFs already half-work by drag and drop, and the help page denies it | Open | Medium |
| [BL-WM-001](BL-WM-001-watermarking-is-two-bugs-and-a-bundle-problem.md) | What enabling watermarking actually requires, and the robustness cliff | Open | Scope risk |
| [BL-DEPS-003](BL-DEPS-003-four-reachable-advisories-in-the-shipped-binary.md) | Five reachable advisories in the shipped product; every one is fixed on `main`, `cargo audit` and `pip-audit` report zero, and nothing reaches a user until v1.1.0 ships | Open, largely resolved (fixed in tree, not for users) | High |
| [BL-DEPS-004](BL-DEPS-004-a-sidecar-dependency-can-ship-untested.md) | CI tests the locked version while the release ships the bumped one; the sync guard now compares versions, the other three options and the macOS script remain | Open, guard in place | High |
| [BL-REL-003](BL-REL-003-the-release-page-advertises-a-linux-appimage-that-was-never-built.md) | The release page advertised a Linux AppImage that was never built; recurrence closed, and the AppImage now builds on this repo (smoke run, 10 September). Linux auto-update starts with v1.1.0 to v1.1.1 | Open, waiting for v1.1.0 | Medium |
| [BL-REL-004](BL-REL-004-the-macos-update-installs-and-then-does-not-restart.md) | The macOS update installed, said "restarting shortly", and never restarted; fixed in tree (PR #53) and proven on the staging leg 10 September. v1.0.0 users still need the quit-and-reopen sentence in the notes; the non-boot-volume failure remains | Open, largely resolved | Medium |
| [BL-TEST-001](BL-TEST-001-nothing-tests-the-installed-product.md) | Nothing tested the installed product; the updater leg now runs green on this repo (10 September, Windows, request path only), the macOS leg is proven by hand, the Windows verdict and Linux remain | Open, partly resolved | High, v1.1.0 floor |
| [BL-TEST-003](BL-TEST-003-the-e2e-suite-runs-nowhere.md) | 314 end-to-end tests ran nowhere; 296 now run in CI, the 18 visual tests run only on a Mac | Open, partly resolved | Medium |
| [BL-SILENT-001](BL-SILENT-001-mechanisms-that-report-success-while-doing-nothing.md) | The class behind five defects: mechanisms reporting success while doing nothing. Two more instances fixed 8 September | Open | High as a class |
| [BL-CLAIM-003](BL-CLAIM-003-the-pdf-report-asserts-checks-that-never-ran.md) | The exported forensic report asserts OCSP/CRL checks that never run | Open | High, legal |
| [BL-API-001](BL-API-001-the-headless-api-is-documented-and-does-not-exist.md) | The headless API is documented and does not exist; it gates the CLI | Open | Medium |
| [BL-SIZE-001](BL-SIZE-001-242mb-of-download-computes-10kb-of-constants.md) | 242 MB of every download computes 10 KB of constants | Open | High for adoption |

## Closed items

Kept in the table because the history is the point. Each file carries its
`Resolved:` line.

| Item | What it was | Closed |
|---|---|---|
| [BL-REL-001](BL-REL-001-stray-model-backup-in-the-bundle.md) | A stray classifier backup sat inside the bundled resources glob and shipped in the first local v1.1.0 build. Moved out; the build script now refuses to bundle with any stray model file present, using CI's own pattern | 9 September 2026 |
| [BL-CI-001](BL-CI-001-test-gate-has-not-run-since-april.md) | The test gate had not run since 30 April and only fired on pull requests. Push trigger, pinned ruff, lint cleared, `cargo test` and `pytest` green on every push to `main`; four further CI gaps closed in #34 | 8 September 2026 |
| [BL-CI-002](BL-CI-002-osv-scanner-has-scanned-nothing-since-july.md) | The weekly supply-chain scan had scanned nothing since July. `--skip-git` removed, SARIF asserted present, upload working; 1,451 packages across seven lockfiles scanned on every push | 8 September 2026 |
| [BL-TEST-002](BL-TEST-002-timing-dependent-tests-fail-under-load.md) | Two timing-dependent Rust integration tests failed under runner load. Both replaced with signal-based waits (#30, #33) | 8 September 2026 |

The first four items as originally listed (BL-REL-002, BL-CI-001, BL-CI-002,
BL-DEPS-001) were one story: no working channel through which a newly
disclosed vulnerability reached anybody, and none through which a fix
reached a user. On 8 September the two CI channels are closed as fixed, the
Dependabot channel is moving, and the updater channel is fixed on the server
and unproven on a real install. Reading them in that order is still the
order to repair what remains.

## Two standing instructions from Paul, 3 September 2026

- **This branch stays unmerged** until D1 to D3 in
  `~/jura-brain/inbox/2026-09-03-trace-plane-triage.md` are answered.
- **v1.0.1 is not cut** before BL-REL-002 has a scheduled fix. That item
  also does not close on the code fix alone: it is paired with a
  re-download notice, because at least 21 installs cannot be reached by any
  updater repair.

## Where the rest of the backlog is, for now

Plane's `JTV` project holds 248 work items, 156 of them open, none touched
since 22 June 2026. It is the historical record of everything planned
between March and June and is not being used day to day.

A proposal for what carries over into this directory, and whether Plane
retires for Trace as it has for ROOTED, is with Paul at
`~/jura-brain/inbox/2026-09-03-trace-plane-triage.md`. Nothing in Plane has
been or will be deleted.

Until that is decided, the items above are the ones found by reading the
code and the live systems from 3 September 2026 onwards, not by mining
Plane. (This paragraph said "the eight items" when the directory held
eight; it holds twenty-one as of 8 September.)
