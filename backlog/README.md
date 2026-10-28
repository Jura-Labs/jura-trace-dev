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

**Reconciled 13 October 2026** against `origin/main` at `a7b58e65`, after
the v1.2.0 Tier A and Tier B work. Six items moved to Closed, seven rows
were rewritten and three items were added. Rows not mentioned in that
sentence were not re-checked and still carry their September wording.

Statuses reconciled 8 September 2026 against `origin/main` at `d0ca411d`,
after the fourteen merges of that day. Each item's own file carries a dated
"Update, 8 September 2026" section with the evidence.

The last seven items were added on 11 September 2026, from defects found
while planning v1.2.0 rather than while looking for them. They were
verified against `main` at `d9eb2da1` before filing, and four of them are
also recorded as instances in BL-SILENT-001.

| Item | What it is | Status | Severity |
|---|---|---|---|
| [BL-REL-002](BL-REL-002-updater-manifest-carries-urls-not-signatures.md) | The updater manifest held URLs where signatures belong; the live manifest is now correct, the fallback resolves, and the re-download notice and the live update on a real v1.0.0 install remain | Open, largely resolved server-side | Highest |
| [BL-DEPS-001](BL-DEPS-001-dependabot-queue-is-saturated.md) | The Dependabot queue was at the cap in every ecosystem; cap raised, queue moving, 18 open across pip and cargo and actions; steps 4 to 6 remain. Security updates were never subject to the cap | Open, largely resolved | High |
| [BL-CLAIM-001](BL-CLAIM-001-nothing-leaves-the-device-is-nearly-true.md) | "No data is uploaded to external servers" was nearly true. The README text, the vendored Swagger assets and the public download README are all fixed on `main`; closes when v1.2.0 is published | Open, fixed in tree | High |
| [BL-DEPS-002](BL-DEPS-002-frontend-advisories-one-ships.md) | Ten frontend advisories (was twelve), none of which reaches a user | Open, partly resolved | Medium |
| [BL-CLAIM-002](BL-CLAIM-002-the-app-makes-two-promises-the-release-did-not-keep.md) | Two promises shipped inside the app that the release did not keep. Both corrected; the Settings paragraph about the API was the last (#157) | Resolved 8 Oct, ships in v1.2.0 | Medium |
| [BL-DOC-001](BL-DOC-001-pdfs-already-half-work-and-the-help-page-denies-it.md) | PDFs already half-work by drag and drop, and the help page denies it. Target v1.3.0 | Open | Medium |
| [BL-DOC-002](BL-DOC-002-the-cli-and-api-guides-are-not-reachable-from-anything-a-user-sees.md) | The CLI and API guides exist but nothing a user sees links to them: release page cites bare dev-repo paths, public GETTING_STARTED is stale, website silent. Target v1.3.0 | Open | Medium |
| [BL-WM-001](BL-WM-001-watermarking-is-two-bugs-and-a-bundle-problem.md) | What enabling watermarking actually requires, and the robustness cliff. Target v1.3.0 | Open | Scope risk |
| [BL-DEPS-003](BL-DEPS-003-four-reachable-advisories-in-the-shipped-binary.md) | Five reachable advisories in the shipped product; every one is fixed on `main`, `cargo audit` and `pip-audit` report zero, and nothing reaches a user until v1.1.0 ships | Open, largely resolved (fixed in tree, not for users) | High |
| [BL-DEPS-004](BL-DEPS-004-a-sidecar-dependency-can-ship-untested.md) | CI tests the locked version while the release ships the bumped one; the sync guard now compares versions, the other three options and the macOS script remain | Open, guard in place | High |
| [BL-REL-003](BL-REL-003-the-release-page-advertises-a-linux-appimage-that-was-never-built.md) | The release page advertised a Linux AppImage that was never built; recurrence closed, and the AppImage now builds on this repo (smoke run, 10 September). Linux auto-update starts with v1.1.0 to v1.1.1 | Open, waiting for v1.1.0 | Medium |
| [BL-REL-004](BL-REL-004-the-macos-update-installs-and-then-does-not-restart.md) | The macOS update installed, said "restarting shortly", and never restarted; fixed in tree (PR #53) and proven on the staging leg 10 September. v1.0.0 users still need the quit-and-reopen sentence in the notes; the non-boot-volume failure remains | Open, largely resolved | Medium |
| [BL-TEST-001](BL-TEST-001-nothing-tests-the-installed-product.md) | Nothing tested the installed product; the updater leg now runs green on this repo (10 September, Windows, request path only), the macOS leg is proven by hand, the Windows verdict and Linux remain | Open, partly resolved | High, v1.1.0 floor |
| [BL-TEST-003](BL-TEST-003-the-e2e-suite-runs-nowhere.md) | 314 end-to-end tests ran nowhere; 296 now run in CI, the 18 visual tests run only on a Mac | Open, partly resolved | Medium |
| [BL-SILENT-001](BL-SILENT-001-mechanisms-that-report-success-while-doing-nothing.md) | The class behind five defects: mechanisms reporting success while doing nothing. Two more fixed 8 September; on 10 September the pre-commit hook itself turned out never to have been a gate, four of its five checks piped through `tail` | Open | High as a class |
| [BL-REL-005](BL-REL-005-the-macos-build-ships-whatever-is-installed-on-one-laptop.md) | The macOS build froze the sidecar against whatever was installed on one laptop. It now freezes from a venv built from `requirements-ci.txt` (#158); the `excludes` half was not added | Open, largely resolved | High |
| [BL-REL-006](BL-REL-006-the-only-thing-keeping-a-release-a-draft-is-a-failing-ci-job.md) | The only thing keeping a release a draft was the macOS CI job failing. The publish decision now asserts what is attached (#87); the macOS and Linux freshness gate is still missing. Target v1.3.0 | Open, partly resolved | High |
| [BL-REL-007](BL-REL-007-the-checksum-file-omits-the-one-file-the-updater-downloads.md) | `SHA256SUMS.txt` omitted the updater archive. The workflow now fails on a missing line; the published v1.1.0 file still lists five of six | Open, partly resolved | Medium |
| [BL-API-002](BL-API-002-the-api-server-announces-a-port-it-has-not-bound.md) | The API server logged that it was listening before it bound. The log line now follows the bind; the desktop still cannot show that the API is not running. Target v1.3.0 | Open, partly resolved | Medium |
| [BL-SIDECAR-001](BL-SIDECAR-001-the-sidecar-contract-disagrees-with-everything-that-describes-it.md) | The sidecar's contract disagreed with its client, its settings, its docstrings and its auth test. The auth test is fixed (#101); the dead settings and client methods remain. Target v1.3.0 | Open, partly resolved | Medium |
| [BL-REL-008](BL-REL-008-a-deb-install-is-never-offered-an-update-it-can-install.md) | A `.deb` install is offered an update it cannot install: the manifest's one Linux entry is the AppImage. Fires for the first time when v1.2.0 is published | Open | Medium |
| [BL-CLAIM-005](BL-CLAIM-005-the-in-app-model-card-shows-the-previous-models-figures.md) | The in-app model card shows 4.12% false positives for the UnivFD probe; the shipped model's metadata says 3.87% | Open | Medium |
| [BL-LIC-001](BL-LIC-001-what-the-licensing-audit-left.md) | What the licensing audit left: one sentence in `TRAINING.md` that still reads as a licence term, no `license` field in `ui/package.json`, 51 files without an SPDX header | Open | Low |

## Closed items

Kept in the table because the history is the point. Each file carries its
`Resolved:` line.

| Item | What it was | Closed |
|---|---|---|
| [BL-API-001](BL-API-001-the-headless-api-is-documented-and-does-not-exist.md) | The headless API was documented and did not exist. `docs/API_WRAPPER.md` rewritten against the source, `jura-trace-api` and the `jura` client built (v1.2.0 B2a stages 0 to 6, B2b) | 5 Oct |
| [BL-API-003](BL-API-003-an-authenticated-api-runs-on-every-install-and-no-user-can-get-a-key.md) | An authenticated API ran on every install and no user could get a key. Keys are issued in Settings on every tier (#157) | 8 Oct |
| [BL-SIZE-001](BL-SIZE-001-242mb-of-download-computes-10kb-of-constants.md) | 242 MB of every download computed 10 KB of constants. The CLIP text encoder is precomputed and no longer shipped (#158) | 8 Oct |
| [BL-API-004](BL-API-004-the-swagger-crate-is-compiled-into-every-build-and-never-used.md) | The Swagger crate was compiled in and never used while the page fetched from a CDN. The crate now serves vendored assets from the binary (#99) | 21 Sept |
| [BL-UX-001](BL-UX-001-tool-object-object-in-the-c2pa-panel.md) | The C2PA panel showed "Tool: [object Object]". It shows the generator's name (#95) | 17 Sept |
| [BL-UX-002](BL-UX-002-the-middle-band-is-called-three-different-things.md) | The middle band reads "Moderate Trust", "Inconclusive" and "Uncertain" in different places; Paul decided "Uncertain" (22 Oct). Target v1.3.0 | Open | Medium |
| [BL-CLAIM-003](BL-CLAIM-003-the-pdf-report-asserts-checks-that-never-ran.md) | The exported report asserted OCSP/CRL checks that never ran. The report, `api.ts` and the settings toast no longer claim them (#35) | 29 Sept |
| [BL-PERF-001](BL-PERF-001-five-seconds-of-every-windows-launch-is-taskkill.md) | About 5 s of every Windows launch was the app waiting on a `taskkill` child, and each sidecar `/health` cost 2 s on Ollama. Replaced with an in-process sweep and `/health/ready` (#109); warm launch 3.7 s with the B1 port | 24 Sept |
| [BL-LOG-001](BL-LOG-001-the-app-log-file-records-errors-only.md) | The app's log file recorded errors only. Default filter now `warn,jura_trace_lib=info`, millisecond timestamps (#109) | 24 Sept |
| [BL-CLAIM-004](BL-CLAIM-004-standard-mode-does-not-cover-the-signing-timestamp.md) | Signing sent a hash to DigiCert even in Standard mode. Option 3: ask once, remember, refuse until answered (#112) | 24 Sept |
| [BL-REL-001](BL-REL-001-stray-model-backup-in-the-bundle.md) | A stray classifier backup sat inside the bundled resources glob and shipped in the first local v1.1.0 build. Moved out; the build script now refuses to bundle with any stray model file present, using CI's own pattern | 9 September 2026 |
| [BL-REL-009](BL-REL-009-the-release-page-and-the-download-page-recommend-different-windows-installers.md) | The release page recommends the `.msi`, the download page the `.exe`. Target v1.3.0, after BL-TEST-004 | Open | Low |
| [BL-CI-001](BL-CI-001-test-gate-has-not-run-since-april.md) | The test gate had not run since 30 April and only fired on pull requests. Push trigger, pinned ruff, lint cleared, `cargo test` and `pytest` green on every push to `main`; four further CI gaps closed in #34 | 8 September 2026 |
| [BL-CI-002](BL-CI-002-osv-scanner-has-scanned-nothing-since-july.md) | The weekly supply-chain scan had scanned nothing since July. `--skip-git` removed, SARIF asserted present, upload working; 1,451 packages across seven lockfiles scanned on every push | 8 September 2026 |
| [BL-TEST-002](BL-TEST-002-timing-dependent-tests-fail-under-load.md) | Two timing-dependent Rust integration tests failed under runner load. Both replaced with signal-based waits (#30, #33) | 8 September 2026 |
| [BL-TEST-004](BL-TEST-004-the-real-updater-has-never-updated-an-exe-install.md) | The real updater has never updated a Windows `.exe` install; only a hand-run `.msi` stands in for it. Target v1.3.0 | Open | Medium |
| [BL-C2PA-001](BL-C2PA-001-the-trust-lists-are-six-months-old.md) | The C2PA trust lists were fetched 22 April: 19 signing CAs and 14 timestamp authorities, against 32 and 23 upstream on 17 Oct. Refresh script still TODO. SR-23. Target v1.3.0 | Open | Critical |
| [BL-C2PA-002](BL-C2PA-002-trace-cannot-say-which-trust-list-a-signer-is-on.md) | All four trust bundles are one anchor set, so Trace cannot tell Interim Trust List signers from official-list signers, which the CAI says conforming validators must. Target v1.3.0 | Open | Critical |
| [BL-C2PA-003](BL-C2PA-003-the-c2pa-library-upgrade-has-not-been-re-checked.md) | SR-34 re-check after c2pa 0.91.3 not done (claims row 15 waits on it); 0.91.4 out; the mid-November release deletes the deprecated API; CAWG identities now untrusted by default. Target v1.3.0 | Open | Critical |
| [BL-C2PA-004](BL-C2PA-004-a-manifest-that-excludes-the-whole-file.md) | A published attack (Buchanan, 2 Oct) signs a manifest whose hash excludes the whole file. Untested against Trace. Target v1.3.0 | Open | Critical until tested |
| [BL-DET-001](BL-DET-001-the-ai-detectors-have-never-seen-a-2026-generator.md) | The AI detectors were trained on 2025-and-earlier generators and never measured on 2026 ones; three 2026 benchmarks put this architecture near chance on them. Measure, then retrain on 2026 generators; ships only if it passes the gates. Target v1.3.0 | Open | Critical |
| [BL-CLAIM-006](BL-CLAIM-006-the-detection-figures-we-publish-are-out-of-date-and-out-of-context.md) | The user guides quote the superseded UnivFD v9 figures and "most AI images across 14 generator families", with no note that the figures are in-distribution. Target v1.3.0 | Open | Critical |
| [BL-CLAIM-007](BL-CLAIM-007-a-result-does-not-say-what-kind-of-evidence-it-rests-on.md) | An ML-only result says "Treat as AI-generated content", and no headline says whether it rests on credentials, a watermark or statistics; the EU Code of Practice (June 2026) calls forensic detection not mature enough. Target v1.3.0 | Open | Critical |
| [BL-UX-003](BL-UX-003-the-signing-mode-is-called-local-signing-everywhere.md) | The website says "Local Signing" (Paul, 21 Oct); the app says Sovereign and Bedrock. Labels only; the stored value stays. Target v1.3.0 | Open | Medium |
| [BL-UX-004](BL-UX-004-a-missing-label-is-not-evidence-of-a-real-photo.md) | "No Content Credentials attached" says nothing about what that means; with AI labelling duties new, readers will read absence as authenticity. Target v1.3.0 | Open | Medium |
| [BL-UX-005](BL-UX-005-checks-trace-did-not-do-should-say-not-checked.md) | Revocation, CAWG identity and the trust list's date: show "not checked" and the date, not silence or "untrusted". Target v1.3.0 | Open | Medium |
| [BL-REL-010](BL-REL-010-release-notes-have-no-known-limitations-section.md) | v1.1.0 and v1.2.0 have no "Known limitations" section; two real limits are buried inline. Target v1.3.0 | Open | Low |
| [BL-DOC-003](BL-DOC-003-the-per-format-detector-list-is-written-by-hand-and-disagrees.md) | Help, FAQ and changelog disagree on which detectors run on WebP, AVIF and HEIC; derive the table from the code. Target v1.3.0 | Open | Medium |
| [BL-REC-001](BL-REC-001-the-verification-record-cannot-show-when-it-was-made.md) | The Verification Record's time is the local clock; offer an optional RFC 3161 timestamp. Target v1.3.0 | Open | Medium |
| [BL-DOC-004](BL-DOC-004-the-practice-pack-has-no-2026-ai-image.md) | The practice pack has no image from a 2026 generator; add one after BL-DET-001, with whatever result it gets. Target v1.3.0 | Open | Low |
| [BL-C2PA-005](BL-C2PA-005-revocation-is-never-checked.md) | No OCSP check in either mode; add it in Enhanced mode only, with three plain outcomes. Target v1.3.0 | Open | Medium |
| [BL-DEPS-005](BL-DEPS-005-three-deferred-major-upgrades.md) | reqwest 0.13 (#25), onnxruntime 1.30 (#91) and Tailwind 4 were deferred; each needs its own check. Target v1.3.0 | Open | Medium |
| [BL-RIS-001](BL-RIS-001-reverse-image-search-is-built-and-unreachable.md) | Bring-your-own-key reverse image search (`ris.rs`, 33 tests) is not compiled in; the app tells users to search elsewhere. Enhanced mode, per-search consent. Target v1.3.0 | Open | Medium |
| [BL-MON-001](BL-MON-001-watched-locations-is-built-and-hidden.md) | Watched Locations is built, flagged off, and its scheduler runs only on a Pro tier that does not exist. Needs Paul's Community limit. Target v1.3.0 | Open | Medium |
| [BL-UX-006](BL-UX-006-first-run-is-four-screens-and-no-sample.md) | First run is an overlay, a wizard, a reminder and two cards, and no sample to try. One screen and a "Try a sample image" button. Target v1.3.0 | Open | Medium |
| [BL-DOC-005](BL-DOC-005-the-getting-started-guide-describes-a-product-that-never-shipped.md) | `docs/user-guide/getting-started.md` (March 2026) promises watermarking, video, audio and old verdict names, and says "no data leaves your device". Rewrite. Target v1.3.0 | Open | High |
| [BL-REC-002](BL-REC-002-the-verification-record-can-only-be-made-in-the-desktop-app.md) | The CLI and API cannot make a Verification Record, and it has no analyst or case fields. First step for Uwazi and team use. Target v1.3.0 | Open | Medium |
| [BL-DOC-006](BL-DOC-006-nothing-shows-a-team-how-to-share-one-trace.md) | No guide for one server and many desks; the only team advice is SQLite on a shared drive. Target v1.3.0 | Open | Medium |
| [BL-DOC-007](BL-DOC-007-the-in-app-help-disagrees-with-the-code-and-the-wiki.md) | The in-app help (methodology, format support, glossary) quotes bands from 71%, ELA weight 2.0, OCSP support and AVIF signing; the code and the 26 Oct wiki say otherwise. Fix and test the numbers. Target v1.3.0 | Open | Medium |
| [BL-A11Y-001](BL-A11Y-001-no-accessibility-audit-since-may.md) | No accessibility audit since May, no automated check in CI, no accessibility statement. Target v1.3.0 | Open | Medium |
| [BL-I18N-001](BL-I18N-001-the-app-can-only-speak-english.md) | English only, no i18n library. Start the process: library, core-path strings, glossary, one pilot language. Target v1.3.0 (groundwork) | Open | Medium |
| [BL-DIST-001](BL-DIST-001-not-in-any-package-manager.md) | In no package manager. winget and a Homebrew tap; not Flathub (its AI-content rule). Target v1.3.0 | Open | Medium |
| [BL-DIST-002](BL-DIST-002-not-on-the-digital-public-goods-registry.md) | Not on the Digital Public Goods registry; needs an app privacy policy and an ownership statement first. Target v1.3.0 (preparation) | Open | Medium |
| [BL-INT-001](BL-INT-001-a-uwazi-recipe-test.md) | HURIDOCS/Uwazi: a recipe script and template, tested on sample images, before anything further. After BL-REC-002. Target v1.3.0 | Open | Medium |
| [BL-CLI-001](BL-CLI-001-jura-is-not-on-the-path-after-a-mac-or-windows-install.md) | `jura` is not on the PATH after a Mac or Windows install; the first command fails with "command not found". A Settings button on macOS, PATH in the Windows installers. Target v1.3.0 | Open | Medium |
| [BL-UX-007](BL-UX-007-a-capped-score-does-not-say-it-was-capped.md) | An AI image and a real photo both scored 55: the low-resolution cap (under 1 MP or short edge under 768 px) and the inconclusive cap fire silently, and `ceilingApplied` is null. Report every cap and say it in plain words. Target v1.3.0 | Open | High |
| [BL-REL-011](BL-REL-011-no-intel-mac-build.md) | No Intel Mac build; no request on record. Target later | Open | Low |
| [BL-OBS-001](BL-OBS-001-nobody-reads-the-signals-we-already-have.md) | Update-check counts and download-click events exist and go unread. Target later | Open | Low |

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
eight; it holds thirty-five as of 13 October.)
