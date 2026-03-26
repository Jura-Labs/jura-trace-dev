# Development Workflow — Jura Trace

**Prepared**: 26 March 2026
**Audience**: Paul Griffiths (primary), future co-founder or hire
**Scope**: Development, testing, staging, release, backup, database migrations, onboarding

---

## Contents

1. [Assessment — What is Working and What is Not](#1-assessment)
2. [Recommended Changes (Prioritised)](#2-recommended-changes)
3. [Make Commands](#3-make-commands)
4. [Backup Strategy](#4-backup-strategy)
5. [Database Migration Strategy](#5-database-migration-strategy)
6. [Branching Strategy](#6-branching-strategy)
7. [Pre-commit Hook Configuration](#7-pre-commit-hook-configuration)
8. [Staging and Beta Channel Design](#8-staging-and-beta-channel-design)
9. [Onboarding Guide for a New Contributor](#9-onboarding-guide)

---

## 1. Assessment

### What is working well

**CI pipeline is solid.** The three-job CI workflow (Rust, Frontend, Python) with concurrency cancellation, cache keying on lock files, and `pip-audit` is professional-grade. The separation of lint from test in Python (ruff before pip install) is exactly right. The Cargo cache paths are complete. This is not a pain point.

**Release workflow is thorough.** The 4-platform matrix with `fail-fast: false`, draft-then-publish pattern, SHA-256 checksum generation, and `latest.json` auto-updater manifest is more than most small desktop projects produce. The environment gate on `publish-release` is a meaningful safety check.

**Graceful degradation architecture.** The sidecar-offline path, optional CLIP/Ollama, and FFmpeg detection pattern mean the app continues working when components are absent. This is the right design for a local-first tool aimed at non-technical users.

**Test coverage is meaningful.** 235 Rust tests, 308 Python tests, and type-checking at every layer catch real regressions. The CI test suite has already caught issues (the Sprint 15 cargo fmt failure). This investment pays off.

**Security posture is above average for a solo project.** Audit hash chain, API key auth, scoped filesystem capability, pinned Python dependencies with pip-audit, and CSP locked to localhost origins are all appropriate for a tool that handles institutional content.

### What is not working well

**The `make dev` command does not actually start development.** It prints instructions rather than running them. A solo developer opening the project for the first time (or returning after a week) has to read the output, open two terminals, and type two commands manually. With a Python sidecar also needed, it is effectively three commands across three terminals. This is friction at the most common entry point.

**Single-branch workflow lacks any safety net for experiments.** All Claude Code agent work, all in-progress features, and all fixes land directly on `main`. There is no place to put exploratory work without it being in the main line. The agent worktrees in `.claude/worktrees/` help, but they do not push to separate branches — they are temporary local workspaces only. If an agent pushes a broken state to `main`, CI is the only gate.

**No pre-commit hook.** `cargo fmt`, `ruff format`, and basic Rust compilation errors are caught only in CI, typically minutes after push. Fast local feedback would catch the most common errors (formatting, obvious type errors) in under 5 seconds before they hit the remote.

**Database schema has no versioning.** The current approach — `CREATE TABLE IF NOT EXISTS` plus ad-hoc `ALTER TABLE ADD COLUMN` for new columns — works for additive changes during development but will break silently on non-additive changes (column renames, type changes, constraint additions, data transformations). A user upgrading from v0.9 to v1.0 with an existing database could hit this. There is currently no mechanism to detect which schema version a database is at.

**macOS Intel builds are fragile.** The `macos-13` runner (last Intel GHA runner) is a known stability issue — the runner is end-of-life in Q3 2026 per GitHub's roadmap. Builds are already noted as failing intermittently. The path forward is universal binaries, but that is a Sprint 19+ item.

**Linux release builds fail due to torch dependencies.** The PyInstaller freeze on Linux includes `torch` (CPU variant, ~600 MB) from the full `requirements.lock`. The `requirements-ci.txt` fallback in the release workflow attempts to handle this, but if that file does not exclude torch correctly or if open-clip-torch is included, the freeze will either OOM or produce a binary too large for the 90-minute timeout. This needs explicit attention.

**No automated smoke test after build.** After `cargo tauri build`, there is no verification that the produced `.app`/`.dmg`/`.exe` launches successfully. A silent link failure or missing sidecar binary in the bundle would only be discovered by a user.

**No corpus backup.** The `scripts/build_corpus.py` and `scripts/expand_corpus.py` scripts download from external sources (Guardian API, HuggingFace, COCO). If those sources become unavailable or the corpus is accidentally deleted, the 545-image training set cannot be reconstructed quickly. The trained model (`models/deepfake_classifier.joblib`) is in the repo, but the training data is not.

**No documented `.env` / environment setup for development.** New contributors need to know about `JURA_SIDECAR_KEY`, `JURA_DB_PATH`, and `JURA_MODELS_DIR`. These are scattered across `DEPLOYMENT.md` and inline comments in the code. There is no `.env.example` file.

**Plane is not linked to GitHub.** Sprint work items in Plane have no automatic connection to commits or PRs. A co-founder reviewing Plane cannot see what code implements a given item.

---

## 2. Recommended Changes

Priority is **High / Medium / Low** for a solo developer context. High means blocking real risk or daily friction. Medium means worth doing before v1.0. Low means nice-to-have.

### Priority: High

#### H1 — Fix `make dev` to actually start development

The Makefile `dev` target must start the sidecar, SvelteKit, and Tauri together or make it genuinely easy to do so. See [Section 3](#3-make-commands) for the proposed implementation using `make dev-sidecar`, `make dev-ui`, and `make dev-tauri` in separate terminals, plus a `Procfile`-based single-command option.

#### H2 — Add a pre-commit hook for fast feedback

A 10-second local check for `cargo fmt --check`, `ruff format --check`, and `ruff check` prevents the most common CI failures before they reach GitHub. See [Section 7](#7-pre-commit-hook-configuration).

#### H3 — Add PRAGMA user_version to the database for migration versioning

Before v1.0, introduce a schema version number so the application can detect an old database and run the appropriate migration steps. This is the minimum required to prevent upgrade data loss. See [Section 5](#5-database-migration-strategy).

#### H4 — Create a `.env.example` file documenting all environment variables

Document `JURA_SIDECAR_KEY`, `JURA_DB_PATH`, `JURA_MODELS_DIR`, and `JURA_KB_DIR` in a committed `.env.example` file. This is the first thing a new contributor needs and currently requires reading multiple files to reconstruct.

#### H5 — Fix Linux release build: pin requirements-ci.txt explicitly

The `requirements-ci.txt` file must explicitly exclude `open-clip-torch` and `faster-whisper` (both are gracefully-degraded optional dependencies). If it is not present or is incomplete, the Linux PyInstaller freeze will OOM or timeout. Verify this file exists and is correct.

### Priority: Medium

#### M1 — Introduce a lightweight branching convention for agent work

Use short-lived feature branches for non-trivial Claude Code agent sessions: branch from `main`, push the branch, let CI run, merge if green. The overhead for a solo developer is minimal (one extra push command). The benefit is that CI runs on an isolated change before it is on `main`. See [Section 6](#6-branching-strategy).

#### M2 — Add a beta release channel for the auto-updater

Tags matching `v*-beta.*` should produce pre-releases that only update clients who have opted into the beta channel. This allows pilot users to test new releases before public rollout. See [Section 8](#8-staging-and-beta-channel-design).

#### M3 — Add a `make test-all` command that runs all four test suites

`cargo test`, `python -m pytest`, `npm test`, and `npx playwright test` (headed on local, skipped in CI) should be runnable with one command. See [Section 3](#3-make-commands).

#### M4 — Automate the corpus backup to a cloud object store

Run `scripts/build_corpus.py` once and upload the corpus to a private S3 or Backblaze B2 bucket. A one-time cost of ~$0.02/month. Without this, the trained classifier cannot be retrained if the corpus is lost. See [Section 4](#4-backup-strategy).

#### M5 — Add a post-build smoke test step

After `cargo tauri build`, verify that the produced binary reports `--version` and that the sidecar binary is present in the bundle. A 30-second shell check catches missing-binary issues before release. This can be added as an additional step in the release workflow.

#### M6 — Document GitHub Secrets required for CI/release

Create `docs/ci-secrets.md` listing every secret that CI/release workflows reference, their purpose, and how to create them. Currently a new maintainer cannot set up CI from scratch without reading both workflow files line by line. (Contents: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`, `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.)

### Priority: Low

#### L1 — Add a `make lint` target that runs all linters

Currently `make lint` only runs `cargo clippy` and `npx svelte-check`, missing `ruff check` for Python and `cargo fmt --check`. See [Section 3](#3-make-commands).

#### L2 — Connect Plane issues to commits via commit message convention

Prefix commit messages with `[JT-NNN]` where NNN is the Plane issue number. This is a convention only — no tooling required. It creates a navigable audit trail between plan and implementation.

#### L3 — Add `CODEOWNERS` file

A `.github/CODEOWNERS` file listing `@paulgriffiths` as the owner of all paths ensures that if the repo is ever opened for contribution, review requirements are clear from the start.

---

## 3. Make Commands

The current Makefile has the right top-level targets but `dev` is a no-op and several targets are incomplete. The proposed replacement is below.

### Proposed Makefile (replace the existing one)

The key change is providing three composable `dev-*` targets (sidecar, UI, Tauri) for the three-terminal development flow, plus a `Procfile` for developers who prefer a process manager.

```makefile
.PHONY: dev dev-sidecar dev-ui dev-tauri \
        test test-rust test-python test-frontend test-e2e test-all \
        build check lint fmt clean install \
        docker-up docker-down docker-build docker-logs

# ── Development ──────────────────────────────────────────────────────────────
#
# Three-terminal development flow:
#   Terminal 1: make dev-sidecar
#   Terminal 2: make dev-ui       (or wait for Tauri to start it automatically)
#   Terminal 3: make dev-tauri
#
# Alternatively, install foreman or overmind and run:
#   foreman start -f Procfile.dev
#
# Tauri's beforeDevCommand already runs `cd ui && npm run dev`, so `make dev-ui`
# is provided for convenience when you want the SvelteKit server without Tauri.

dev:
	@echo ""
	@echo "Jura Trace — three-terminal development flow:"
	@echo ""
	@echo "  Terminal 1 (Python sidecar):"
	@echo "    make dev-sidecar"
	@echo ""
	@echo "  Terminal 2 (Tauri + SvelteKit, starts UI automatically):"
	@echo "    make dev-tauri"
	@echo ""
	@echo "  Or, with foreman installed:"
	@echo "    foreman start -f Procfile.dev"
	@echo ""

dev-sidecar:
	cd sidecar && uvicorn main:app --host 127.0.0.1 --port 8200 --reload

dev-ui:
	cd ui && npm run dev

dev-tauri:
	cd src-tauri && cargo tauri dev

# ── Install dependencies ──────────────────────────────────────────────────────

install:
	cd ui && npm install
	cd src-tauri && cargo fetch
	cd sidecar && pip install -r requirements.txt

# ── Tests ────────────────────────────────────────────────────────────────────

test-rust:
	cd src-tauri && cargo test

test-python:
	cd sidecar && python -m pytest tests/ -v

test-frontend:
	cd ui && npm test -- --run

test-e2e:
	cd ui && npx playwright test

# Run all test suites (excludes Playwright — requires running dev server).
# Use test-e2e separately when the app is running.
test:
	$(MAKE) test-rust
	$(MAKE) test-python
	$(MAKE) test-frontend

# Run everything including Playwright (requires dev server on port 1420).
test-all: test test-e2e

# ── Build ─────────────────────────────────────────────────────────────────────

build:
	cd ui && npm run build
	cd src-tauri && cargo tauri build

# ── Check ─────────────────────────────────────────────────────────────────────

check:
	cd ui && npx svelte-kit sync && npx svelte-check
	cd src-tauri && cargo check

# ── Lint ──────────────────────────────────────────────────────────────────────

lint:
	cd src-tauri && cargo clippy -- -D warnings
	cd src-tauri && cargo fmt --check
	cd ui && npx svelte-check
	cd sidecar && ruff check .
	cd sidecar && ruff format --check .

# ── Format ────────────────────────────────────────────────────────────────────

fmt:
	cd src-tauri && cargo fmt
	cd ui && npx prettier --write "src/**/*.{svelte,ts,js,css}"
	cd sidecar && ruff format .

# ── Clean ─────────────────────────────────────────────────────────────────────

clean:
	cd ui && rm -rf node_modules .svelte-kit build
	cd src-tauri && cargo clean

# ── Docker (sidecar + Ollama local stack) ─────────────────────────────────────

docker-up:
	docker compose up -d

docker-down:
	docker compose down

docker-build:
	docker compose build

docker-logs:
	docker compose logs -f
```

### Procfile.dev

Create `Procfile.dev` in the project root for `foreman start -f Procfile.dev` (or `overmind start -f Procfile.dev`). This is optional — the three-terminal approach works fine. The Procfile is for developers who prefer a single terminal with process multiplexing.

```
sidecar: cd sidecar && uvicorn main:app --host 127.0.0.1 --port 8200 --reload
ui:      cd ui && npm run dev
tauri:   cd src-tauri && cargo tauri dev
```

Note: foreman does not handle Ctrl-C cleanly on macOS when child processes hold ports. Overmind (`brew install overmind`) is a better choice on macOS and is the preferred tool. Without either installed, the three-terminal approach is equally valid.

---

## 4. Backup Strategy

As a local-first application, Jura Trace does not have a server to back up. The backup responsibilities are different from a web application.

### What needs backing up

| Asset | Current location | Risk without backup | Recommended backup |
|-------|-----------------|--------------------|--------------------|
| Source code | GitHub (juralabs/jura-archive) | Low — GitHub is the backup | None needed beyond pushing regularly |
| Agent memories | `.claude/agent-memory/` (in repo) | Low — in repo, pushed to GitHub | Already covered by git push |
| `models/deepfake_classifier.joblib` | In repo | Low — in repo | Already covered |
| Training corpus (545 images) | Gitignored, local only | **High** — re-download takes hours; some sources may become unavailable | Cloud object store (see below) |
| Plane project data | Hosted by Plane | Medium — Plane manages backups; export quarterly | Export to JSON quarterly: Settings > Export |
| User SQLite databases | User's machine | Owned by user — document recommendation only | Advise users to include app data dir in Time Machine / OS backup |
| Signing certificates (Sprint 19+) | GitHub Secrets + local Keychain | **Critical** — loss = re-apply for cert | 1Password or similar vault; export `.p12` file to encrypted backup |

### Corpus backup procedure

Run once after any significant corpus expansion and upload to a private Backblaze B2 or AWS S3 bucket:

```bash
# From project root — requires rclone configured with your cloud provider
rclone sync ./scripts/corpus/ remote:jura-trace-corpus/
rclone sync ./scripts/expanded_corpus/ remote:jura-trace-corpus-expanded/
```

Cost estimate: 545 images at ~2 MB average = ~1 GB. Backblaze B2 charges $0.006/GB/month = ~$0.006/month. Effectively free. Set up once; costs nothing to maintain.

If the corpus must be rebuilt from scratch, `scripts/build_corpus.py` requires a Guardian API key. Store that key in 1Password or equivalent alongside the signing certificates.

### SQLite database — user guidance

Add the following to `DEPLOYMENT.md` and the Settings page:

> Your Jura Trace database is located at `~/Library/Application Support/Jura Trace/jura_archive.db` (macOS). This file contains all your imported assets, verification history, and audit logs. Include this directory in your system backup (Time Machine on macOS, File History on Windows) to protect your data. Jura Trace does not back up your data automatically — backup is your responsibility.

The database is not large (typically under 50 MB for institutional use). It is safe to copy the file while the application is not running.

---

## 5. Database Migration Strategy

### Current state

The current `init_schema()` function creates all tables with `CREATE TABLE IF NOT EXISTS` and adds new columns via `ALTER TABLE ADD COLUMN`. This is safe for additive changes only. There is no schema version number, so the application cannot distinguish a fresh database from a v0.5 database from a v0.9 database.

The single existing migration (adding `prev_hash` and `entry_hash` to `audit_log`) silently ignores errors, which means it silently ignores already-having those columns. This pattern works once but does not scale.

### Recommended approach: PRAGMA user_version

SQLite has a built-in integer field `PRAGMA user_version` (default: 0) specifically designed for application-managed schema versioning. It is stored in the database header, requires no extra table, and is supported by rusqlite.

Implement this before v1.0 using the following pattern in `db.rs`:

```rust
const SCHEMA_VERSION: u32 = 2; // Increment for every schema change

fn init_schema(&self) -> SqliteResult<()> {
    let conn = self.conn.lock().unwrap();

    // Read current version. Returns 0 on a fresh database.
    let current_version: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(0);

    if current_version == SCHEMA_VERSION {
        return Ok(()); // Already at current version — nothing to do.
    }

    // Run all migrations in sequence from the current version forward.
    // Each migration is idempotent or explicitly guarded by the version check.
    if current_version < 1 {
        conn.execute_batch(MIGRATION_V1)?;
    }
    if current_version < 2 {
        conn.execute_batch(MIGRATION_V2)?;
    }
    // Add further migrations here as: if current_version < N { ... }

    // Update the stored version only after all migrations succeed.
    conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))?;
    Ok(())
}

// Migration 1: Initial schema creation (replaces the current init_schema body).
// All tables are created here exactly once, not with IF NOT EXISTS guards.
const MIGRATION_V1: &str = "
    CREATE TABLE IF NOT EXISTS assets ( ... );
    CREATE TABLE IF NOT EXISTS fingerprints ( ... );
    -- etc.
";

// Migration 2: Add hash chain columns to audit_log (currently done ad-hoc).
const MIGRATION_V2: &str = "
    ALTER TABLE audit_log ADD COLUMN prev_hash TEXT;
    ALTER TABLE audit_log ADD COLUMN entry_hash TEXT;
";
```

### Migration rules

1. Never modify a past migration constant. Add new `MIGRATION_V(N+1)` constants for every change.
2. Never rename a column or change a column type in place — SQLite does not support this natively. Instead, create a new table, copy data, drop the old table, and rename the new one. This is all one migration step.
3. For destructive migrations (dropping data), add an explicit warning in the release notes.
4. Test migrations with an existing v0.9 database before releasing v1.0. Keep a copy of the v0.9 database in `tests/fixtures/` for regression testing.

### Before v1.0: set the initial version

Run `PRAGMA user_version = 1` on the current schema once, before Sprint 20 releases. Any existing user databases get assigned version 0 by default, which will trigger the full migration chain (safe, since `CREATE TABLE IF NOT EXISTS` handles existing tables).

---

## 6. Branching Strategy

### Current approach: main-first for small changes

For a solo developer, committing directly to `main` is a valid and pragmatic choice for small, well-understood changes. It is the current approach and it is fine for:

- Documentation updates and configuration changes
- Single-file bug fixes where the change is obvious
- Dependency version bumps with green CI

This is not a problem — it keeps the workflow simple and there is no bureaucratic overhead.

### When to use a feature branch

Use a short-lived branch (`feat/`, `fix/`, `chore/`) when:

- The change spans multiple files or multiple commits (multi-day work)
- The change is exploratory or risky (e.g., a new ML detector, a database migration)
- A Claude Code agent session is handling the work (isolates agent output from `main` until CI passes)
- You want to be able to abandon the work without reverting commits on `main`

The overhead is minimal: one extra `git checkout -b` and `git push -u origin` at the start, one merge at the end.

### Recommended branch naming

```
feat/sprint-17-monitor-ui
feat/sprint-17-export-zip
fix/linux-build-torch-exclude
chore/update-rust-1.89
```

### Why not single-branch only?

The single-branch approach works until an agent session produces a broken commit or two agent sessions overlap. The Claude Code worktrees in `.claude/worktrees/` are local only — they do not push to separate branches automatically. A branch per multi-day session means CI validates each session's output independently before it lands on `main`.

### Release branches

For v1.0, create `release/v1.0` from `main` at code freeze. Patch releases (`v1.0.1`) are tagged from `release/v1.0` and cherry-picked back to `main`. This is only necessary once there is a public release to maintain; before v1.0, tagging from `main` is sufficient.

### GitHub branch protection

Branch protection rules (Settings > Branches > Add rule for `main`) require **GitHub Pro or a public repository**. The current private repository on the free plan cannot enforce branch protection.

**Current workaround (free plan, private repo):** Rely on the pre-commit hook (Section 7) and CI as the safety nets. Push to `main` directly for small changes; use feature branches for risky work.

**When to enable branch protection:** Upgrade to GitHub Pro (~$4/month) or make the repository public. Once enabled, configure:
- Require status checks to pass before merging: `Rust`, `Frontend`, `Python`
- Do not require pull request reviews (too much overhead for solo work)
- Do not allow force pushes

### When a co-founder joins

When a second developer joins:

1. **Upgrade to GitHub Pro or make the repo public** — this unlocks branch protection, which becomes essential with two developers working concurrently.
2. **Require CI to pass on all branches before merge** — the three CI jobs (`Rust`, `Frontend`, `Python`) are the gate.
3. **Use PRs for all changes** — enables code review, keeps `main` clean, and creates a navigable history.
4. **Consider `CODEOWNERS`** — a `.github/CODEOWNERS` file ensures the right person reviews changes to sensitive areas (e.g., `src-tauri/src/c2pa.rs`, `sidecar/app/services/`).

The current workflow is intentionally lightweight for solo development. The transition to a two-person workflow requires only a GitHub plan upgrade and enabling branch protection — the CI pipeline, test suite, and documentation are already in place.

---

## 7. Pre-commit Hook Configuration

### What to check

A pre-commit hook should be fast (under 10 seconds) and check only what is trivially fixable locally:
- `cargo fmt --check` — Rust formatting (fast; ~1 second)
- `ruff format --check sidecar/` — Python formatting (fast; ~0.5 seconds)
- `ruff check sidecar/` — Python lint (fast; ~0.5 seconds)

Do NOT run in pre-commit: `cargo test`, `cargo clippy`, `pytest`, `svelte-check`, `playwright`. These are slow, belong in CI, and running them locally before every commit creates friction that leads to bypassing the hook.

### Implementation

The simplest implementation uses plain shell scripts (no third-party tooling required):

**`.git/hooks/pre-commit`** (create this file; `chmod +x` it):

```bash
#!/usr/bin/env bash
set -e

# Fast pre-commit checks — formatting and Python lint only.
# Full test suite runs in CI. Do not add slow checks here.

echo "pre-commit: checking Rust formatting..."
cd src-tauri && cargo fmt --check
cd ..

echo "pre-commit: checking Python formatting..."
cd sidecar && ruff format --check .
cd ..

echo "pre-commit: running Python lint..."
cd sidecar && ruff check .
cd ..

echo "pre-commit: all checks passed."
```

To install:

```bash
cp docs/pre-commit.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

Store `docs/pre-commit.sh` (a copy of the script above) in the repository so new contributors can install it.

### Optional: lefthook

If the project ever gains more than two contributors, consider `lefthook` (a fast Go-based hook manager). It supports parallel hook execution and is easy to configure via `lefthook.yml`. For a solo developer, the plain shell approach is adequate and has no dependencies.

---

## 8. Staging and Beta Channel Design

### What "staging" means for a desktop app

A traditional staging environment (a separate server) does not apply to a local-first desktop application. For Jura Trace, the equivalent of staging is:

1. **A beta release channel** — users can opt in to receive pre-release builds before the public release.
2. **A local smoke-test build** — running `cargo tauri build` locally and manually testing the produced installer before tagging a release.
3. **An RC (release candidate) tag** — a `v1.0.0-rc.1` tag triggers the full release workflow but produces a GitHub pre-release that does not auto-update production clients.

### Beta channel implementation

The Tauri auto-updater `latest.json` endpoint drives the update channel. The simplest beta channel approach does not require a separate CDN — it uses two `latest.json` files:

- `latest.json` — the stable channel (served from `/releases/latest/download/latest.json`)
- `latest-beta.json` — the beta channel (served from `/releases/latest/download/latest-beta.json`)

In `tauri.conf.json`, the updater endpoint for stable builds remains:
```json
"endpoints": ["https://github.com/juralabs/jura-archive/releases/latest/download/latest.json"]
```

Beta builds use:
```json
"endpoints": ["https://github.com/juralabs/jura-archive/releases/latest/download/latest-beta.json"]
```

Beta-channel builds are a separate Tauri binary variant, built from the same source with a different `tauri.conf.json`. The simplest implementation is a `tauri.beta.conf.json` file that overrides only the updater endpoint, which is passed to `cargo tauri build` via `--config`.

### Release tagging convention

| Tag pattern | GitHub release type | Auto-update channel | Who receives it |
|-------------|--------------------|--------------------|-----------------|
| `v1.0.0` | Full release | Stable | All users |
| `v1.0.0-rc.1` | Pre-release | None (no `latest.json` written) | Manual download only |
| `v1.0.0-beta.1` | Pre-release | Beta | Opted-in beta users |

The release workflow already handles pre-releases via `prerelease: /-(rc|beta|alpha)/.test(tag)`. The `latest.json` upload step should be conditioned on `!prerelease` so that RC and alpha tags do not update stable clients.

### Local smoke test before tagging

Before tagging any release:

```bash
# 1. Build locally
make build

# 2. Verify the .app launches and the sidecar starts
open "src-tauri/target/release/bundle/macos/Jura Trace.app"

# 3. Check the sidecar binary is present in the bundle
ls "src-tauri/target/release/bundle/macos/Jura Trace.app/Contents/MacOS/"

# 4. Verify version number matches the tag you intend to push
grep '"version"' src-tauri/tauri.conf.json
```

Add a `make release-check` target that automates steps 1, 3, and 4 (step 2 requires human eyes).

---

## 9. Onboarding Guide

### For a new co-founder or hire

This section assumes the reader is a developer with general experience but no prior knowledge of this codebase.

### Prerequisites

Install these tools before anything else:

| Tool | Install | Version |
|------|---------|---------|
| Rust | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` | 1.88+ |
| Node.js | `brew install node` or nvm | 20+ |
| Python | `brew install python@3.13` | 3.12+ |
| Tauri CLI | `cargo install tauri-cli --version '^2'` | 2.x |
| FFmpeg | `brew install ffmpeg` | any recent |

On Linux, also install:
```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev libjavascriptcoregtk-4.1-dev \
  libsoup-3.0-dev
```

### First-time setup

```bash
git clone https://github.com/juralabs/jura-archive.git
cd juralabs
make install                  # installs Node, Cargo, and Python deps
cp docs/pre-commit.sh .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit
```

### Reading list (in order)

Before writing code, read these documents in order:

1. `CLAUDE.md` — architecture, tech stack, design principles, current status
2. `PROJECT_SPEC.md` — objectives, KPIs, phased delivery plan
3. `docs/ARCHITECTURE.md` — detailed technical architecture
4. `docs/DEPLOYMENT.md` — how the app is installed and configured by users
5. `CHANGELOG.md` — development history (skim the most recent 3–4 sprints)
6. `docs/security-audit-report.md` — known security issues and their status
7. `docs/sprint-plans/sprint-15-to-v1.0-plan.md` — the roadmap to v1.0

This takes roughly 2–3 hours and will give a complete picture of where the project is and where it is going.

### Codebase mental model

The application has four independently testable layers:

1. **Rust (`src-tauri/src/`)** — the Tauri shell, all file I/O, C2PA, fingerprinting, EXIF, SQLite, and the HTTP client that talks to the Python sidecar. Entry point: `lib.rs`. Tests: `cargo test` (235 tests).

2. **SvelteKit (`ui/src/`)** — the UI. SPA mode (no SSR). Svelte 5 runes. All Tauri IPC calls go through `ui/src/lib/api.ts` (which has a browser mock fallback for development without Tauri). Entry point: `routes/+layout.svelte`. Tests: vitest unit tests + Playwright e2e (209 files, 0 svelte-check errors).

3. **Python sidecar (`sidecar/`)** — the ML forensics engine. FastAPI on port 8200. Each detector is a separate service in `app/services/`. Entry point: `main.py`. Tests: pytest (308 tests). The sidecar is stateless — it receives a file path or base64 image and returns a result JSON.

4. **Database (`src-tauri/src/db.rs`)** — SQLite via rusqlite. All DB access goes through the `Db` struct. Schema defined in `init_schema()`. No ORM.

The layers communicate as follows:
- UI → Rust: Tauri IPC (`invoke('command_name', { args })`)
- Rust → Sidecar: HTTP POST to `http://127.0.0.1:8200` with `X-Jura-API-Key` header
- Rust → DB: direct `Db` struct method calls (no network)
- Sidecar → Ollama: HTTP POST to `http://127.0.0.1:11434` (optional)

### Running the development environment

```bash
# Terminal 1: Python sidecar
make dev-sidecar

# Terminal 2: Tauri app (starts SvelteKit automatically via beforeDevCommand)
make dev-tauri
```

The application opens in a Tauri window. Changes to SvelteKit files hot-reload instantly. Changes to Rust files require the dev server to recompile (typically 5–30 seconds depending on which file changed).

If the sidecar is not running, the application functions with all ML forensics features showing "Analysis service offline". This is by design — start the sidecar if you are working on forensics features, skip it if you are working on UI or Rust-only features.

### Running tests

```bash
make test-rust      # Rust unit tests (~10 seconds)
make test-python    # Python pytest suite (~30 seconds)
make test-frontend  # Vitest unit tests (~5 seconds)
make test-e2e       # Playwright (requires running dev server on port 1420)
make test           # All except Playwright
```

### Making a change

1. Create a branch: `git checkout -b feat/your-description`
2. Make the change.
3. Run `make lint` to check for formatting and lint errors.
4. Run the relevant test suite (e.g., `make test-rust` for Rust changes).
5. Commit: `git commit -m "[JT-NNN] brief description of what and why"`
6. Push: `git push -u origin feat/your-description`
7. CI runs automatically. Verify it passes before merging.
8. Merge to `main` when CI is green: `git checkout main && git merge feat/your-description && git push`
9. Delete the branch: `git branch -d feat/your-description && git push origin --delete feat/your-description`

### Key things that are not obvious

**The frontend must be built before Rust compiles.** `tauri::generate_context!()` embeds the frontend dist path at compile time. If `ui/build/` does not exist, `cargo check` and `cargo test` fail. Running `make dev-tauri` handles this automatically. If you are running `cargo test` standalone, run `cd ui && npm run build` first.

**The sidecar API key is auto-generated.** In development, `JURA_SIDECAR_KEY` is generated as a UUID at Tauri startup and passed to the sidecar process. If you start the sidecar manually via `make dev-sidecar`, the key does not match what Tauri expects. For development, this is not a problem — the Tauri dev server starts the sidecar itself via the `Command` plugin. If you want the sidecar running independently (e.g., for sidecar-only development), set `JURA_SIDECAR_KEY=dev-key` in both terminals.

**Playwright tests require a running dev server.** Run `make dev-tauri` (or just `make dev-ui`) in a separate terminal before running `make test-e2e`. The Playwright config points to `http://localhost:1420`.

**The deepfake classifier model must be present for full Python tests.** `models/deepfake_classifier.joblib` is in the repo. If tests report "model not found", check that the file is at that path.

**Optional dependencies that affect test counts.** Running Python tests without `ffprobe` installed skips 5 video/audio tests. Running without `open-clip-torch` skips 14 CLIP tests. The CI test run uses Python 3.12 (not 3.13) because ubuntu-latest does not yet ship 3.13.

### Where to find things quickly

| Question | Where to look |
|----------|--------------|
| What does `invoke('verify_asset', ...)` return? | `ui/src/lib/types.ts` + `src-tauri/src/lib.rs` |
| How does a forensic detector work? | `sidecar/app/services/<detector>.py` |
| What does `verify_asset` do end-to-end? | `src-tauri/src/lib.rs` — find `verify_asset` command |
| What is stored in the database? | `src-tauri/src/db.rs` — `init_schema()` |
| What are the CSP rules? | `src-tauri/tauri.conf.json` — `app.security.csp` |
| What CI secrets are needed? | `docs/ci-secrets.md` (to be created per M6 above) |
| What sprint are we on? | `CHANGELOG.md` — first section heading |
| What is the v1.0 plan? | `docs/sprint-plans/sprint-15-to-v1.0-plan.md` |

### Bus factor mitigation

The project is well-documented for a solo codebase. The main risk is that some decisions and trade-offs exist only in Claude Code agent memories (`.claude/agent-memory/*/`) and conversation context. These are committed to the repository, but they are structured for agent consumption, not human reading.

The mitigation is the reading list above. The combination of `CLAUDE.md`, `ARCHITECTURE.md`, `CHANGELOG.md`, and the security audit report gives a new contributor enough context to be productive in a day. The code itself is well-commented, the test suite is large enough to catch regressions, and the Makefile provides clear entry points.

If Paul is unavailable: the CI and release pipelines are fully automated. A tagged release (`git tag v1.0.0 && git push origin v1.0.0`) will produce signed installers for all four platforms without any manual intervention, once code signing is configured in GitHub Secrets.

---

*This document should be reviewed and updated at each major release milestone. The next scheduled review is Sprint 20 (v1.0 release).*
