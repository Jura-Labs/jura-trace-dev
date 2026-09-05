# Agent rules

The Jura Labs steering rules apply: see ~/jura-brain/AGENTS.md.

## 1. Before you start, and before you finish

1. **Read `~/jura-brain/strategy/weeks/current.md` before starting any
   work.** Work only on deliverables listed there.
2. **Every action carries that deliverable's ID.** Commit messages,
   branch names, output files, work items.
3. **Append one line to `~/jura-brain/log.md` for every action**, in the
   format below. Do this even when the work happened entirely in this
   repository. A session whose work never reaches `log.md` is invisible
   to the Friday audit, and the system cannot see what it cannot read.

```
2026-09-01 | session-name | id: W37-3 | what you did | inputs: files used | model: name
```

   The `id:` label is not decoration. A line missing it is an orphan and
   the audit will report it; a line missing an *unlabelled* field just
   shifts the others along and every model then fills the gap with a
   guess. That failure was found on 30 August 2026 and is why the label
   exists.

4. If no deliverable ID applies, write a proposal to `~/jura-brain/inbox/`
   with the evidence and stop. Do not act on it.

Nothing monitors this repository. There is no watcher and no queue. Every
workflow is at L0: it runs when Paul types a command. These three habits
are the whole integration, and they are conventions, not enforcement.

## Repo-specific rules

- Releases follow docs/release/, not ad-hoc tagging.
- Personal data never leaves the server or local/.
- Source is `Jura-Labs/jura-archive` (private, `origin`). The public
  `Jura-Labs/jura-trace` repo carries installers only. Never push source
  to the release repo.
- A detection verdict never comes from a language model. The verdict
  pipeline stays detectors plus human-set thresholds (models.md, "Never").
- Never write that Trace can "verify", "prove" or "guarantee" a fact.
  Trace surfaces evidence, provenance and uncertainty so a person can
  judge (mission.md, "Evidence").
- Material from human-rights evidence holders is analysed locally and is
  never sent to a cloud model (rule 8).
- Work carries a deliverable ID from ~/jura-brain/strategy/weeks/current.md.
  Trace work sits under Q4-2 and Bet 2.

### Backlog convention (Paul, 3 Sep 2026; mirrors ROOTED)

The system of record for Jura Trace backlog items is **this repo**:
`backlog/BL-*.md`, committed to git, with the full write-up. Not Plane.
Plane's JTV project is the historical record and is not being used day to
day; see `backlog/README.md` for what carried over and what did not.

For a session reviewing or implementing a backlog item:

1. Read the BL file in full; it is the spec.
2. Implement on a branch (rule 4: code goes to a branch, never to main).
3. When merged, append to the BL file:
   `Resolved: YYYY-MM-DD <merge commit> — one line on what shipped`
   Do not delete the file; the history is the point.
4. Log the work to `~/jura-brain/log.md` with the deliverable ID, as the
   rules above already require. A merged backlog item nobody logged is
   invisible to the Friday audit.
5. A new backlog item is a new BL file with the same shape, and its own
   log line.

A BL file states what is wrong, how it showed up, why it matters, what
would fix it, and what not to do. It is written for someone who has not
seen the code, and it names files and line numbers so they can check.

---


This file provides guidance to Claude Code when working with code in this repository.

## Jura Labs Ecosystem

Jura Trace is one of two products built by **Jura Labs** (UK Community Interest Company — Companies House 17117467, registered 25 March 2026). The full technology stack, infrastructure, and strategic context are documented in:

- **Tech stack reference**: `../jura-labs-docs/JURA-LABS-TECH-STACK-UPDATED.md`
- **Project management**: [Plane workspace](https://app.plane.so/jura-labs/) — connected to Claude Code via MCP (user scope)
- **Sister product**: ROOTED (carbon emissions guidance) — see `../ecoadvisor/`
- **Shared docs**: `../jura-labs-docs/`

## Project Overview

**Jura Trace** is a local-first desktop application for content verification and protection. In a world of synthetic media, verification matters. It helps cultural institutions protect their digital assets from unauthorised AI extraction, and helps communities verify content authenticity.

**Developed by**: Juralabs Community Interest Company (UK) — https://juralabs.org
**Licence**: AGPL-3.0-or-later (with commercial-licence path on request — see `COMMERCIAL.md`). Switched 2026-05-06 from PolyForm Noncommercial 1.0.0; risk accepted on solicitor scoping per memory `project_ip_architecture_dual_entity.md`.
**Current Version**: 0.9.0-rc.25 cut 2026-05-21 (Phase A — v1.0 in launch prep, target **live public release Mon 22 June 2026** (revised 2026-05-09 from 31 May; CPL embargo lifts 31 May silently — Jura Trace makes no public comment between 31 May and 22 June launch; see memories `project_v1_live_release.md` + `project_v1_marketing_plan_may2026.md`))
**Source repo**: `Jura-Labs/jura-archive` (private)
**Release repo**: `Jura-Labs/jura-trace` (public — installers only, no source)
**Windows signing**: Azure Trusted Signing (certificate ID a7e35def-628b-4980-8785-2e535f709418)
**macOS signing**: Apple Developer ID Application — Jura Labs CIC (Team ID `Y82C4P9L7F`), G2 Sub-CA, valid through 15 April 2031. Cert installed in login keychain. See `docs/install-guides/macos-signing-setup.md` for full setup, notarisation, and CI integration.

## Core Architecture

Four-layer local-first stack. See `docs/ARCHITECTURE.md` for the full diagram and data flow.

1. **Tauri v2 desktop shell** (Rust) — macOS / Windows / Linux.
2. **SvelteKit 5 frontend** on port 1420 — tabs: Protect, Verify, Monitor, Settings, Help. Communicates with Rust via Tauri IPC.
3. **Rust core engine** — C2PA signing/verification, perceptual fingerprinting, EXIF anomaly + MakerNote authenticity, format routing, DWT-DCT-SVD watermarking, video/audio metadata, SQLite database, Axum REST API on port 8300.
4. **Python ML sidecar** (FastAPI) on port 8200 — image forensics (ELA, noise, copy-move, NPR, chromatic aberration, JPEG ghost, segmented ELA, shadow consistency, colour temperature, splice boundary), GBM deepfake classifier, CLIP ViT-B/32 + UnivFD zero-shot AI detection, RAG claim checker, watermark embed/extract, video frame extraction, video deepfake (per-frame + temporal), audio/video transcription via faster-whisper.
5. **Ollama** on port 11434 (optional) — LLaVA for Tier 3 descriptions, Qwen2.5 for RAG claim verification.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | Tauri v2 (Rust) |
| Frontend | SvelteKit 5 + TailwindCSS (SPA mode, static adapter, Svelte 5 runes) |
| Core engine | Rust (c2pa-rs, rusqlite, image_hasher, reqwest) |
| Auto-catalogue | Tier 1: EXIF (Rust), Tier 2: CLIP/ONNX (optional), Tier 3: Ollama (optional) |
| ML sidecar | Python 3.13 + FastAPI (ELA, noise, copy-move, deepfake, NPR, chromatic aberration, JPEG ghost, segmented ELA, shadow consistency, colour temperature, splice boundary, CLIP detection, watermark embed/extract, video metadata, audio metadata, video frame extraction, video deepfake per-frame analysis) |
| CLIP detection | open_clip ViT-B/32 — optional (~350 MB, lazy-loaded, graceful degradation) |
| LLM runtime | Ollama — optional (LLaVA for Tier 3 descriptions, Qwen2.5 for RAG claim verification) |
| Database | SQLite (via rusqlite in Rust) |

## Development Commands

```bash
# Start development. IMPORTANT: the app and the sidecar must share the same
# non-empty JURA_SIDECAR_KEY or the sidecar 503s every /forensics/* call and
# verify silently runs with only the Rust-native detectors (C2PA, EXIF).
# The make targets and Procfile.dev handle the key for you (jura-dev-local):
make dev-sidecar          # Terminal 1: Python ML sidecar (port 8200)
make dev-tauri            # Terminal 2: Tauri app (spawns Vite on port 1420)

# Or manually — export the key on BOTH sides:
cd sidecar && JURA_SIDECAR_KEY=jura-dev-local uvicorn main:app --host 127.0.0.1 --port 8200 --reload
cd src-tauri && JURA_SIDECAR_KEY=jura-dev-local cargo tauri dev

# Build production app
make build

# Type check SvelteKit
cd ui && npx svelte-check

# Check Rust compilation
cd src-tauri && cargo check

# Run Rust tests (568 lib + 20 API integration)
cd src-tauri && cargo test

# Run Rust linter
cd src-tauri && cargo clippy -- -D warnings

# Run Python sidecar tests (425 tests: 354 pass / 71 skip in a clean venv;
# skips are ffprobe-dependent video/audio and CLIP when open_clip is absent)
cd sidecar && python -m pytest tests/ -v

# Run Vitest unit tests (182 tests)
cd ui && npm test

# Run Playwright e2e tests (314 tests: 157 specs x 2 browser projects)
# The browser binary is NOT installed by `npm ci` and is not in the repo.
# Without this first step every test fails with "Executable doesn't exist",
# which looks like a catastrophic regression and is not one.
cd ui && npx playwright install chromium   # once per machine

# CI runs 296 of the 314. The 18 visual-regression baselines are macOS-only
# (*-darwin.png) so the pixel comparisons only happen on a Mac. See
# BL-TEST-003.
cd ui && npx playwright test

# Run SvelteKit type check
cd ui && npx svelte-check
```

### Disk pressure — periodic cargo clean

`src-tauri/target/` grows steadily during active Rust development and can
reach 15 GB or more with incremental build artefacts. On laptops with
tight disk budgets this will fill the root filesystem and break subsequent
`cargo check` / `cargo test` / `cargo tauri dev` runs with `ENOSPC` (no
space left on device). It can also fail non-cargo writes — mid-session
disk exhaustion has previously taken out Claude Code's own scratch
directory and the editor's staging area for in-flight file edits.

**Practice**: run `cargo clean --manifest-path src-tauri/Cargo.toml`
whenever free space on `/` drops below ~5 GB. It frees the target dir
cleanly; the next `cargo check` takes ~1–2 minutes to rehydrate caches
and ~10 minutes for a cold full rebuild. Also delete `sidecar/build/`
(PyInstaller output, not tracked) which can add another 200–300 MB.

Check disk usage with `df -h /` and target size with
`du -sh src-tauri/target`.

## Project Structure

Top-level layout (browse directories directly for file listings):

- `src-tauri/` — Tauri v2 Rust backend. Core modules in `src/`: `lib.rs` (entry: Tauri command definitions + registration glue), `verify/` (pipeline, trust scoring incl. `verify/trust.rs::compute_trust`, input quality, result types), `state.rs` (AppState, LicenceTier), `config.rs`, `startup.rs`, `c2pa.rs`, `db.rs`, `exif_anomaly.rs`, `fingerprint.rs`, `format_router.rs`, `metadata.rs`, `watermark.rs`, `sidecar.rs`, `error.rs`, `sun_position.rs`, `api/` (REST).
- `ui/` — SvelteKit 5 frontend (SPA/static adapter). Routes in `src/routes/` (protect, verify, monitor, settings, help); shared code in `src/lib/` (`types.ts`, `api.ts`, `pdf.ts`, `zip.ts`, `blob.ts`, `components/`, `stores/`); Playwright e2e in `tests/`.
- `sidecar/` — Python 3.13 FastAPI ML sidecar on port 8200. Routers in `app/api/`, detection/forensics logic in `app/services/`, Pydantic schemas in `app/models/`, pytest suite in `tests/`. `main.py` entry, `requirements.txt` / `requirements-ci.txt` / `requirements.lock`, `jura-sidecar.spec` for PyInstaller.
- `scripts/` — training, corpus, and calibration scripts; `scripts/agents/` for corpus crawl/protect/verify pipeline.
- `models/` — trained model artefacts (on external USB during development).
- `docs/` — public-facing documentation only (install guides, user guides, methodology, calibration, decisions, C2PA conformance, branding, compliance, ARCHITECTURE.md, BRAND_GUIDELINES.md, backlog.md). **Internal-consultation docs (strategy, funding drafts, sprint plans, internal architecture / corpus / deployment notes, project spec) live in `../jura-labs-docs/` per the doc-hygiene pattern (memory `project_repo_doc_hygiene`).**
- `.github/workflows/` — CI, Release (4-platform matrix), Dependabot.
- `.github/PULL_REQUEST_TEMPLATE.md` + `.github/ISSUE_TEMPLATE/` (and `.forgejo/` mirrors) — contributor-facing forms with mandatory `AI-Disclosure` field per `CONTRIBUTING.md` "AI-tool use" section.
- Root: `CHANGELOG.md`, `CLAUDE.md`, `LICENSE`, `COMMERCIAL.md`, `TRAINING.md`, `CONTRIBUTING.md`, `README.md`, `Makefile`.

## Two-Repo Release Architecture

Jura Trace uses a split-repo model to keep source code private while distributing installers publicly:

| Repo | Visibility | Purpose |
|------|-----------|---------|
| `Jura-Labs/jura-archive` | Private | Source code, CI, development, all branches and tags |
| `Jura-Labs/jura-trace` | Public | Release installers only — no source code |

**How it works:**
1. Tags are pushed to `Jura-Labs/jura-archive` (the source repo where the workflow lives).
2. The release workflow runs in `jura-archive` using the default `GITHUB_TOKEN` only for checkout.
3. All GitHub Releases API calls (create, upload assets, publish) target `Jura-Labs/jura-trace` via the `RELEASE_PAT` secret (a PAT with `repo` scope on `jura-trace`).
4. `tauri-action` is called without `releaseId` on all platforms — this prevents it from uploading to `jura-archive`. Assets are uploaded manually via `gh release upload --repo Jura-Labs/jura-trace` instead.
5. The Tauri auto-updater endpoint in `tauri.conf.json` also points to `Jura-Labs/jura-trace` so end-user update checks resolve against the public repo.

**Required secrets in `Jura-Labs/jura-archive`:**
- `RELEASE_PAT` — GitHub PAT with `repo` scope on `Jura-Labs/jura-trace`
- `GITHUB_TOKEN` — standard Actions token (used only for source checkout)

**CI/CD workflows**: `.github/workflows/` — CI (Rust + Python + Frontend with pip-audit), Release (4-platform matrix), Dependabot, OSV-Scanner (cross-ecosystem SCA, SARIF to Security tab)

### Mac release builds are LOCAL, not CI (2026-05-21)

The release workflow's macOS "Phase 5 — per-file sign nested sidecar-bundle code" step silences codesign stderr (`>/dev/null 2>&1`) inside a `set -euo pipefail` loop, so when any single codesign call across the ~610 nested .dylib/.so files fails, the only visible error is `sort: stdout: Broken pipe`. CI Mac jobs have been unreliable since this surfaced.

**Workaround (the current shipping pattern):**
1. Push the release tag — CI runs Windows (works) + Mac (will fail, but that creates the draft release as a side effect).
2. On Apple Silicon: `bash scripts/build-local-mac.sh`. Produces `.app`, signed DMG, `.app.tar.gz`, `.app.tar.gz.sig` under `$CARGO_TARGET_DIR/aarch64-apple-darwin/release/bundle/`.
3. **Verify the DMG was notarised** (search the build log for `Notarisation accepted` + `Stapled + Gatekeeper-validated`). If not, see the notarisation-setup block below — the build is still installable on the building Mac but other Macs will hit a Gatekeeper warning.
4. Upload Mac artefacts: `gh release upload v0.9.0-rc.X --repo Jura-Labs/jura-trace <files>`.
5. Publish the draft once both platforms are populated.

Prerequisites to avoid the recurring "bundle_dmg.sh failed" error: before running the local build, detach any leftover hdiutil mounts (`for d in $(hdiutil info | awk '/^\/dev\/disk/ {print $1}'); do hdiutil detach "$d" -force; done`) and quit any running Jura Trace + sidecar process. See memory `project_mac_local_release_build.md` for the full procedure and the cargo macro-cache caveat (the build-local-mac.sh patch in commit 162f338 invalidates the cache automatically).

### Notarisation must not be skipped at tag-cut

This has been forgotten on more than one rc. The DMG carrying only Developer ID signing (without notarisation) installs cleanly on the building Mac but throws a Gatekeeper warning on every other Mac, which silently kills testers' confidence. Notarisation is part of the release, not optional.

`scripts/build-local-mac.sh` resolves Apple notarisation credentials in three places, in this order. The build picks the first one that has all three pieces (`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`); credentials are GitHub Secrets on `Jura-Labs/jura-archive` (set 2026-04-23) but **GitHub Secrets are write-only** so they cannot be pulled back — they must be re-established locally.

1. **Shell env** — `APPLE_ID + APPLE_PASSWORD + APPLE_TEAM_ID` already exported. Honoured first so CI semantics match local.
2. **`.env.local`** at the repo root (gitignored). Sourced before the env-var check. Convenient but plaintext-on-disk.
3. **macOS keychain profile `jura-trace-notary`** — most secure. Set up once with:
   ```
   xcrun notarytool store-credentials "jura-trace-notary" --team-id Y82C4P9L7F
   ```
   It prompts interactively for the Apple ID email and the 16-char app-specific password (generated at https://appleid.apple.com/account/manage → App-Specific Passwords). The profile then lives in the login keychain at service `com.apple.gke.notary.tool`. Phase 8 of the build script then uses `--keychain-profile jura-trace-notary`, no env-var plumbing required.

If none of the three resolves, the build script warns and still completes — DMG is signed but **not** notarised. The build log will say `Apple notarisation creds not set — DMG signed but NOT notarised.` near the top of pre-flight. Treat that line as a release-blocker: re-set creds and rebuild.

**Pre-tag-cut check**: `xcrun notarytool history --keychain-profile jura-trace-notary` should return at least an empty `JSON history` payload (proves the creds talk to Apple's servers). If it errors with "could not retrieve the credentials", set the profile up again before pushing the tag.

**Post-build check**: `spctl --assess --verbose=2 --type install "<path-to-dmg>"` should report `source=Notarized Developer ID`. If it says `source=Developer ID` (no "Notarized" prefix), the DMG was signed but not notarised — do not publish.

## Codeberg Public Source Push (clean-snapshot ruleset)

The public AGPL source lives at `codeberg.org/jura-labs/jura-trace` (the Codeberg mirror), separate from the private GitHub dev repo `Jura-Labs/jura-archive` (`origin`). Remote: `codeberg` = `git@codeberg.org:jura-labs/jura-trace.git`.

**CRITICAL SAFETY RULE.** `codeberg/main` is a scrubbed public history with NO common ancestor to local `main`. NEVER `git push codeberg main`, and NEVER `--force` to it. A force-push overwrites the clean public history with the full private dev history (639+ commits), exposing everything the scrub avoided and wiping the public release commits + tags. This is destructive and unrecoverable. The public repo is updated by a curated catch-up sync, not a raw push.

**Safe method (curated sync of a release onto the clean history):**
1. `git worktree add <dir> codeberg/main` (check out the clean public history).
2. Lay the included source tree (from the release tag) over the worktree, applying the exclusions below. Do NOT copy the dev `.git`.
3. Secret-scan the result (no hardcoded keys/tokens; `.env` absent; `src-tauri/binaries/*` are tiny stubs, not real binaries).
4. Single clean commit ("Jura Trace vX.Y.Z public source release (AGPL-3.0-or-later)"). Create the tag ON the snapshot commit only: `git push codeberg HEAD:refs/tags/vX.Y.Z`. NEVER `git push codeberg vX.Y.Z` using the existing LOCAL release tag: that tag points into the private dev history, so pushing it uploads dev commit objects to Codeberg (reachable via the tag until deleted). If it happens, `git push codeberg :refs/tags/vX.Y.Z` then re-create on the snapshot commit, and request a Codeberg GC.
5. Fast-forward push to `codeberg` (builds on the existing clean history, no force).
6. Push the wiki separately to `git@codeberg.org:jura-labs/jura-trace.wiki.git` (pages from `../jura-labs-docs/codeberg-wiki-draft/`).
7. Make the repo public in Codeberg settings if still private.

**INCLUDE** (AGPL Corresponding Source must be buildable):
- `src-tauri/`, `ui/`, `sidecar/`, `scripts/` (build + training scripts)
- Dependency/lock manifests: `Cargo.toml`+`Cargo.lock`, `ui/package.json`+`ui/package-lock.json`, `sidecar/requirements*.txt`+`requirements.lock`+`jura-sidecar.spec`, `Makefile`, root `package.json`
- `src-tauri/binaries/` stubs (needed for `cargo check`; verify they ARE stubs), `src-tauri/trust-list/*.pem` (public C2PA anchors), `sidecar/app/services/bpe_simple_vocab_16e6.txt.gz` (CLIP tokenizer)
- `.forgejo/` (Codeberg CI)
- Root docs: LICENSE, README.md, COMMERCIAL.md, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, GENAI_USE_POLICY.md, TRAINING.md, NOTICE, THIRD-PARTY-LICENSES.txt, CHANGELOG.md
- From `docs/`, ONLY `docs/ARCHITECTURE.md` and `docs/BRAND_GUIDELINES.md`

**EXCLUDE** (never publish):
- `docs/` (everything except the two files above)
- `data/`, `infrastructure/`, `test-results/`, `corpus/`, `models/` (trained weights, treated as data/IP), `agents/` (corpus crawl pipeline)
- `CLAUDE.md`, `.claude/`, `.github/` (GitHub-specific; exposes the private two-repo model + `RELEASE_PAT`), `.env`

**MUST FIX before any push** (or stale/wrong copy goes public): source README staleness, and any PolyForm / "noncommercial" / "not open source" licence claims in INCLUDED files (licence is AGPL-3.0-or-later since 6 May 2026). Note: the corpus-dataset "non-commercial" notes in `scripts/*.py` are accurate dataset-licence statements and stay; the CC-BY-NC option in the Protect UI is a user content-licence choice and stays. See memories `project_codeberg_fresh_release_snapshot` + `project_repo_doc_hygiene`.

## Key Files

Top-level entry points and non-obvious files. Sidecar services live under `sidecar/app/services/`, UI components under `ui/src/lib/components/`, help pages under `ui/src/routes/help/` — browse those directories directly rather than tracking individual files here.

- **Reference docs**: `CHANGELOG.md`, `docs/ARCHITECTURE.md`, `docs/BRAND_GUIDELINES.md`, `CONTRIBUTING.md` (includes the AI-tool-use policy + high-risk-file list)
- **Internal-only docs (NOT in repo)**: `../jura-labs-docs/jura-trace-strategy/`, `../jura-labs-docs/jura-trace-funding/`, `../jura-labs-docs/jura-trace-internal/` — see memory `project_repo_doc_hygiene` for the discipline + what belongs where
- **Rust entry**: `src-tauri/src/lib.rs` — Tauri commands + registration glue. Verify pipeline lives in `src-tauri/src/verify/` (`pipeline.rs`, `trust.rs::compute_trust`, `types.rs::VerificationResult`)
- **Sidecar client**: `src-tauri/src/sidecar.rs` — HTTP client for Python sidecar
- **REST API module**: `src-tauri/src/api/` — Axum REST API on port 8300 (routes, types, auth, rate_limit, error)
- **Error module**: `src-tauri/src/error.rs` — `AppError` enum with structured IPC serialisation `{ code, message }`
- **Tauri config**: `src-tauri/tauri.conf.json` — app name, window, permissions, auto-updater endpoint
- **Sidecar entry**: `sidecar/main.py` — FastAPI application
- **CI requirements** (non-obvious): `sidecar/requirements-ci.txt` — the file CI actually uses for PyInstaller builds (excludes torch/whisper/chromadb). Separate from `requirements.txt`.
- **PyInstaller spec**: `sidecar/jura-sidecar.spec` — parameterised for macOS/Windows/Linux
- **Frontend contract**: `ui/src/lib/types.ts` (mirrors Rust structs) and `ui/src/lib/api.ts` (Tauri IPC wrapper with browser mock fallback)
- **Main UI**: `ui/src/routes/verify/+page.svelte`, `ui/src/routes/protect/+page.svelte`, `ui/src/routes/+layout.svelte`
- **Models** (on external USB; paths are where sidecar loads them from):
  - `models/deepfake_classifier.joblib` — GBM v4, 84 features, AUC 0.9868, FP 4.54%
  - `models/univfd_probe.joblib` — UnivFD v9, LogReg on CLIP ViT-B/32, AUC 0.9933, FP 4.12%
- **Training scripts**: `scripts/train_classifier.py`, `scripts/train_univfd_probe.py` (`--C` for regularisation), `scripts/agents/` (corpus crawl/protect/verify/validate pipeline)
- **CI/CD**: `.github/workflows/` — CI (Rust + Python + Frontend with pip-audit), Release (4-platform matrix, workflow_dispatch enabled, Linux paused), Dependabot
- **macOS entitlements**: `src-tauri/Entitlements.plist`
- **Sprint plans / strategy / funding drafts / internal architecture**: NOT in this repo. Live in `../jura-labs-docs/jura-trace-strategy/`, `../jura-labs-docs/jura-trace-funding/`, `../jura-labs-docs/jura-trace-internal/`. See memory `project_repo_doc_hygiene` for the rule.
- **User/install docs**: `docs/user-guide/`, `docs/install-guides/`, `docs/pilot-testing/`

## Design Principles

1. **Local-first**: All processing on-device. No cloud calls. No telemetry.
2. **Rust for core**: C2PA, hashing, EXIF analysis, file I/O in Rust for performance.
3. **Python for ML**: Image forensics, deepfake detection, RAG in Python sidecar.
4. **Graceful degradation**: Sidecar offline = forensics skipped, app still works.
5. **Tiered AI**: Core features work without AI. CLIP tagging is optional download. Ollama is optional enhancement.
6. **SvelteKit for UI**: SPA mode via static adapter for Tauri webview. Svelte 5 runes ($state, $derived).

## Current Status

**Version**: 0.9.0-rc.25 cut 2026-05-21, on `main` at `0333e87` 2026-05-22 (Phase A — v1.0 in launch prep, target **live public release Mon 22 June 2026** (revised 2026-05-09 from 31 May; CPL embargo lifts 31 May silently — no public comment between 31 May and 22 June; see memories `project_v1_live_release.md` revised 2026-05-09 + `project_v1_marketing_plan_may2026.md`); pivot from pilot-cohort framing confirmed 2026-05-06, see memory `project_v1_live_release`). Phases 1–3 complete. Sprint 28 tech-debt sweep closed 8 April 2026; Sprint 29 shipped 7 April 2026; Sprint 30 backlog sweep 9 April 2026 (SIFT copy-move, EXIF injection detection, FP telemetry Phase B review bundle, URL watchlist scheduler, Tauri race-condition fix, Experimental UI tag rollout, XMP AI-provenance detection). rc.25 batch (commits `41a2b8b` + `162f338`, 2026-05-21): P0 tag-cut fixes (C2PA chain-walk Gemini fix, sidecar auth empty-key bypass closed with PYTEST_CURRENT_TEST carve-out, REST watermark routes return 503, ELA caption rewrite, 9 Playwright snapshots regenerated) plus the v1.0 UI gating sweep (Deployment Profiles, Ollama Service-Status card, watermark capability chip + per-asset button + "No Watermark" badge — all behind feature flags). SCA tooling added (OSV-Scanner workflow + cargo-audit in CI + Socket.dev docs in CONTRIBUTING). For full sprint-by-sprint history see `CHANGELOG.md` and git log.

**Post-rc.25 sweep (2026-05-22, on `main` head-of-line, commits `604b5f0` → `0333e87`)** — the work that ships in rc.26+ if/when cut, currently installed locally via `bash scripts/build-local-mac.sh` direct copy:

* **C2PA Generator-track 11-item audit closed** (commits `604b5f0` `aeef3a7` `c5362e6` `cc8ee79` `7d2ecb4` `2b1903f`). Licence-value mismatch fixed (single-asset dropdown silently dropped schema-org assertion — shipping bug); spec-correct manifest restructure (dropped non-conformant `c2pa.rights`, added `c2pa.training-mining` per §18.18 with CC0-aware policy, `claim_generator_info` array, `softwareAgent` as `ClaimGeneratorInfoMap`, `digitalSourceType` moved into action); ingredient handling on re-sign (`parentOf` preserved); action selector (`c2pa.created` vs `c2pa.published`) with backend `SignAction` enum + UI radios on single + batch panels; pre-seal disclosure block listing what gets embedded; version-string env macro (CARGO_PKG_VERSION + c2pa::VERSION). New tests `every_ui_license_option_resolves` + `resigning_preserves_parent_provenance`. 87 c2pa::tests pass.
* **C2PA UX followups** (commit `0333e87`). Verify-side `c2pa.training-mining` policy row added under digitalSourceType (round-trip parity); pre-seal disclosure extended with TSA URL + cert SHA-256 fingerprint + claim-generator string (new Tauri command `get_signing_disclosure`, new helper `c2pa::signing_cert_fingerprint_hex`, new `c2pa::TSA_URL` const); action radio contextual hints for institutional edge cases; signing-mode row added to disclosure block.
* **Trust-score doc drift remediation** (commit `259bf6d`). Backend was 20% EXIF / 80% forensic (reduced from 40/60 after security audit) but methodology page, Berkeley Protocol page, glossary, and PDF report all still claimed 40/60 — Berkeley Protocol §6 reproducibility violation. Adopted content-authenticity-expert's restructured copy across all four surfaces: methodology now walks the 5-component algorithm (forensic primary, EXIF corroborating at 20% cap, C2PA adjustment, composite cap 0.55, deepfake verdict ceiling), PDF cites `src-tauri/src/lib.rs::compute_trust` as the AGPL-3.0 reproducibility anchor. New "How this score is calculated" link under the trust ring on verify panel.
* **Verify drop-zone + protect format scope** (commit `604b5f0`). Verify copy dropped MP4/MOV/PDF mention (Tauri filter already excluded them); Protect file picker narrowed to JPEG/PNG/TIFF/WebP (validator-conformant scope only — HEIC/HEIF/AVIF removable since c2pa-rs can sign them but they are outside our Validator submission scope). `canSignC2pa()` UI predicate aligned.
* **Verify watermark surface gated** (commit `4d0c1a9`). Watermark Detection row + Watermark dot in detector grid behind `V1_SHOW_WATERMARK`. Closes the pilot-tester report that "No Jura Trace watermark detected" was appearing on every verify result.
* **Help copy sweep** (commit `44ff5d6`). 10 P0 items across help/verify, help/ollama, help/bedrock-signing, help/glossary, help/compliance, help/settings, help/protect, and the protect drop-zone subtitle. Trust-score formula text aligned across every surface; v1.0 gate notice added to Conformant section of help/bedrock-signing; deferred-feature mentions (watermark, audio, video, PDF, Ollama, profiles) all corrected.
* **Tauri updater key rotation** (commits `c8d4f12` + `aea8063`). May-10 keypair `98980FC45EF9FD71` was effectively lost (in GitHub Secrets but unusable); rotated to fresh empty-password keypair `1AED7E4A127C6230` at `~/.tauri/jura-trace-v10.key`. `bundle.createUpdaterArtifacts: true` added to tauri.conf.json — without it the updater bundle silently produced nothing. `scripts/build-local-mac.sh` auto-loads the key from disk so local builds produce signed `.app.tar.gz` + `.sig` without manual env-var plumbing. rc.25 installers carry the old (lost) pubkey and cannot auto-update; rc.26+ installers carry the new pubkey. See memory `project_tauri_updater_key_rotation.md`.

**Models in production** (as of 7 April 2026):
- **GBM Deepfake Classifier v4** — 10,709 images (5,724 authentic + 4,985 AI), 84-feature vector, AUC-ROC 0.9868, authentic FP 4.54%, AI recall 92.52%, threshold 0.49. SHA-256 `512def7ec62cbeb023c5343859a15606a02742d48b1fca31df11667c0b9ba14a` (re-pickled under sklearn 1.8.0; numerically identical to the original 1.7.2 pickle, max |Δ predict_proba| = 0, and the value pinned in `sidecar/app/services/deepfake.py`).
- **UnivFD probe v9** — LogisticRegression on CLIP ViT-B/32 embeddings, 39,016 training samples (10,712 original + 32,142 platform-forwarded augmentation via Q=75/85/2× re-saves), AUC-ROC 0.9933, FP 4.12%, recall 95.70%. Platform-forwarded-specific AUC: plt75 0.9947, plt85 0.9949, plt2x 0.9937. DiffusionDB recall 67.6% → 97.3%, Civitai SFW 75.8% → 98.7%. Known trade-off: flux_dev 88.9% (−11.1 pp), sdxl_turbo 91.1% (−8.9 pp). SHA-256 `ed691b45cbe2903a7e0530fd0ec78ab91eef9f15133af4c1a5c8cf172086dacd`. Promoted 2026-04-12 from candidate trained 2026-04-11. Full validation in `docs/calibration/univfd-v9-platform-augmentation.md`.
- Training corpus: 6,571 authentic + 5,005 AI = 11,576 images total (stored on external USB).
- Camera FP after MakerNote authenticity bonus + `KNOWN_CAMERA_VENDORS` (36 vendors, 11 Global Majority brands): consumer (Pixel/iPhone) 8.81%, high-end (DJI/DSC) 10.32%. All AI generator families pass 100% recall except DALL-E 3 (91.4%), Civitai SFW (75.8%), DiffusionDB (67.6% — weakest, older SD).

**Detector lineup**: 13 forensic detectors in v1.0. 10 run automatically (EXIF anomaly, C2PA, ELA, noise, copy-move, deepfake GBM+UnivFD ensemble, JPEG Ghost at 0.5× weight, segmented ELA, colour temperature, CLIP) and 3 are on-demand investigation tools (NPR, shadow consistency, splice boundary). The 3 on-demand tools inform investigator judgement but do not contribute to the numeric trust score. Watermark detection (feature-flagged off in v1.0) and video deepfake (not in v1.0 scope) are not counted in the 13. Chromatic aberration removed entirely (forensic audit 1/5). Diffusion artefact detector removed (superseded by UnivFD v8). Schema v6 `detectors_run` column persists the exact lineup per verification so PDF / ZIP renderers can distinguish "detector ran and returned null" from "detector not run in this build / mode". Full Sprint 28 audit trail in `docs/backlog.md` and project memory.

**EXIF anomaly detector (as of Sprint 30)**: the `exif_anomaly` detector now includes both an injection-detection suite (5 sub-checks: programmatic pipeline library in Software field, templated timestamps, integer-degree GPS, MakerNote absent on mandatory-vendor camera, iPhone sRGB mismatch — all landed in commit `d03f338`) and an XMP provenance suite (2 sub-checks: `xmp_ai_digital_source` for `Iptc4xmpExt:DigitalSourceType` values like `trainedAlgorithmicMedia`, and `xmp_ai_creator_tool` for known AI generators in `xmp:CreatorTool` — landed in commit `ce06f7f`). Both are High severity and flow through the existing `exif_anomaly` scoring path with no new detector ID.

**Known issues / carry-over from Sprint 29**:
- Track 2 partial: ~1,000 of target 1,200–1,400 Global Majority handset photos still outstanding.
- Track 4 validation set samples from training corpus — not a true held-out set.
- `demosaic_inter_channel_coherence` feature has DC-dominated sampling, needs iteration.
- Composite border detector: 21 candidates flagged on full corpus, HTML preview pending user review.
- **JPEG Ghost 0.5× weight calibration closed 2026-04-07** — Option 3 (synthetic CC-BY generator) executed; weight retained at 0.5, no code change. CASIA v2 **rejected 2026-04-11** as non-commercial-licensed and unusable for Jura Trace (commercial product); a commercial-cleared splice benchmark remains an open future need per `docs/calibration/s28-jpeg-ghost-weight.md` Section 6.

**Test counts** (verified 2026-09-05; the previous figures had drifted on every line): 568 Rust lib tests + 20 Rust API integration, 425 Python sidecar (354 pass / 71 skip in a clean venv), 314 Playwright e2e across 11 spec files and 2 browser projects (18 visual baselines, macOS-only, so CI runs 296 of them), 182 Vitest unit tests via @testing-library/svelte + happy-dom, 0 svelte-check errors, clippy + fmt clean. Pre-commit hook now runs `cargo fmt`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `svelte-check`, and `npm test` (vitest) — hardened across two commits: `0912b14` (Sprint 30 test-fixture regressions) and `6d2161a` (vitest gate added 2026-04-28).

## Backlog

See `docs/backlog.md` for open items, UI audit deferrals, and resolved-archive. Track in Plane; that file is a lightweight mirror.

## British Spelling

- **User-facing text**: British spelling (Organisation, Colour, Catalogue)
- **Code**: American spelling (organization, color, catalog) for framework consistency

## Brand Identity — Sanctuary Theme

- **Name**: Jura Trace (formerly Jura Archive)
- **Logo**: Eye mark — concentric circles (lapis outer, cream iris, dark pupil)
- **Palette**: Warm mineral tones — Obsidian (#1E2128), Graphite (#272B34), Quartz (#EDEAE4), Flint (#78756D / #9B9890), Lapis (#376399 / #5A85B5), Malachite (#5B8A5F), Amber, Cinnabar
- **Metaphor**: Geology — permanence, layers, provenance — with human-centred warmth
- **Tagline**: "Know What's Real"
- **Philosophy**: "Keep people at the heart of every decision. Use technology to support and guide, not to take over."
- **Theme**: Sanctuary — warm dark backgrounds, cream text, Georgia serif headings, editorial layout (900px width), earth-line gradient dividers, generous whitespace
- **Design**: System fonts, no emojis, WCAG 2.2 AA, Lighthouse 97% accessibility

## Product Architecture (Post-v1.0)

Two products sharing a common detection engine:

```
                    SHARED CORE
                    (Rust library crate — extracted Phase A Sprint 23)
                   /                     \
    Jura Trace Desktop              Jura Check API (post-June gate)
    (Tauri v2 + SvelteKit)          (Axum, cloud-hosted)
    - Local-first, offline           - JWT + Redis
    - Python sidecar (local 8200)    - Python sidecar (container)
    - B2B tiers (5-tier)             - Consumer tiers (4-tier)
    - Port 8300 (local API)          - verify.juralabs.org
```

**Architecture decisions (27 March 2026):**
- Python sidecar stays — rewriting 21 ML services in Rust rejected (110-160 pts risk)
- Tauri desktop shell stays — the product ships in April/May 2026
- Two separate APIs: local desktop (API key) + hosted consumer (JWT/Redis) — decided at June gate
- Team tier retained — £79/seat/mo fills the £199→£6K gap
- Consumer tiers (Free/Personal/Family/Creator) scoped under Jura Check, not Jura Trace
- 14 personas total: 10 existing B2B + 4 new consumer (Ravi, Sarah M, Jordan, Priya)

## Relationship to ROOTED

Jura Trace is a **sibling product**, not a fork. Shares Juralabs' philosophy and some frontend patterns (SvelteKit, Tailwind, dark mode) but has its own codebase, brand, tech stack (Tauri/Rust), and release cycle. Both use nature-inspired design — ROOTED through organic/growth imagery, Jura Trace through geological/mineral imagery with a warm, editorial aesthetic.
