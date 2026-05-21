# Contributing to Jura Trace

Thank you for your interest in contributing to Jura Trace — a local-first desktop application for forensic media verification, published as open-source public-interest infrastructure.

Jura Trace is published under AGPL-3.0-or-later with a parallel commercial-licence path (see `LICENSE` and `COMMERCIAL.md`). The Contributor Licence Agreement below is what makes that dual-licensing model work.

## How to contribute

- **Bug reports:** open an issue on the canonical repository.
- **Feature ideas:** open a discussion or issue first, before writing code, so we can align on whether the feature fits the roadmap and the local-first principle. See `docs/backlog.md` for the current public roadmap.
- **Pull requests:** small, focused, with tests where appropriate. Match existing code style. Keep architectural changes in a separate PR from feature changes.
- **Documentation:** PRs welcome on user guides, methodology documents, and translation contributions.

## Code style

- **Rust:** `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must pass.
- **Python:** existing patterns in `sidecar/app/` are the reference. Type hints on public functions.
- **Frontend:** SvelteKit 5 runes (`$state`, `$derived`), TailwindCSS, British spelling for user-facing text (American spelling acceptable in code identifiers for framework consistency).
- **Commits** must pass the pre-commit hook: `cargo fmt`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `svelte-check`, `npm test` (vitest). The hook lives in `.githooks/pre-commit` (tracked in the repo) but git does not enable it automatically. After cloning, run **`git config core.hooksPath .githooks`** once per clone to activate it. Without this step the hook is silently inert and commits with clippy/svelte-check failures will land.

## Contributor Licence Agreement

To preserve the dual-licensing model that funds continued development, contributors must agree to the Jura Trace Contributor Licence Agreement before their contributions can be merged.

The CLA is short and standard. By submitting a contribution, you confirm that:

1. **You wrote the contribution yourself**, or you have permission from the original author to submit it. If your employer holds rights in your work, you have your employer's permission to contribute.
2. **You grant Paul Griffiths (the copyright holder) a perpetual, worldwide, non-exclusive, royalty-free, irrevocable licence** to use, reproduce, modify, prepare derivative works of, publicly display, publicly perform, sublicense, and distribute your contribution and any derivative works thereof.
3. **You retain copyright** in your contribution; the licence above is a grant, not an assignment.
4. **The grant includes the right to relicense** your contribution under any future open-source licence (for example, a future GPL revision) and **the right to license it commercially** as part of the dual-licensing model described in `COMMERCIAL.md`.
5. **Your contribution is your original work** or properly attributed under a licence compatible with AGPL-3.0-or-later.

This CLA is required because Jura Trace is published under AGPL-3.0-or-later with a parallel commercial-licence path. Without the CLA, every commercial-licence sale would require unanimous agreement from every contributor, which becomes impractical quickly.

In practice, contributing means agreeing to the CLA. There is no separate document to sign at this stage; submitting a pull request is taken as agreement to the terms above.

## What does NOT change

- **Your copyright in your contribution** — you retain it.
- **Your right to use your own contribution** elsewhere under any terms you wish — granting Paul a non-exclusive licence does not restrict your own use.
- **The AGPL guarantees to users** — your contribution remains available under AGPL-3.0-or-later to every user, forever.

## Code of conduct

Be respectful. Be honest about what your contribution does and doesn't do. Be patient — review may take time as this is a small project with one principal maintainer.

If you encounter behaviour from another contributor that violates ordinary professional norms, contact licensing@juralabs.org.

## AI-tool use

Jura Trace is a forensic verification tool. The detection logic, signing flows, and verification pipeline ARE the credibility surface — a subtle bug in these areas damages the product's purpose in a way that ordinary OSS bugs do not. AI coding assistants (Claude, Copilot, Cursor, Gemini, and similar) are useful for boilerplate, refactors, and documentation, but produce confident-looking code that can be wrong in subtle ways. The policy here is specific rather than blanket.

### Acceptable AI-tool use

- AI as an inference-time assistant to a human author who reads, understands, and substantively edits the output before committing
- Boilerplate generation, refactors with clearly-bounded scope, documentation drafting, test scaffolds
- Mechanical translations (YAML ↔ JSON, schema ↔ types, diff inspection, search across the codebase)
- Drafting plain-English versions of technical content for help / methodology pages, with the human author verifying every factual claim against the source

### Not acceptable

- AI-only authorship without human substantive editing (creates ungrantable rights under the CLA above; potentially licence-pollutes the codebase)
- AI-generated changes to high-risk files (see list below) without explicit human review of every line
- AI-suggested dependency upgrades accepted without checking the upstream changelog and the relevant advisory database
- AI-generated security-research reports without a working reproducer
- Concealment of AI-tool use in a PR description (grounds for closure)

### High-risk files — manual review of every line, no exceptions

These files affect signing, parsing, IPC, key handling, the auto-updater, or the sidecar process. Changes here require human-only verification of the security-critical paths even when AI assistance was used to draft the change:

- `src-tauri/src/c2pa.rs`
- `src-tauri/src/sidecar.rs`
- `src-tauri/src/ris.rs` (BYOK keystore)
- `src-tauri/src/db.rs` (migrations + backup paths)
- `src-tauri/src/metadata.rs` (parser surface)
- `src-tauri/src/api/` (REST surface)
- `src-tauri/Cargo.toml` (dependency upgrades)
- `.github/workflows/release.yml` and `.forgejo/workflows/release.yml` (release pipeline + signing)
- `src-tauri/tauri.conf.json` (updater + bundle config)
- `sidecar/app/services/clip_detector.py` and `sidecar/app/services/deepfake.py` (model loading + scoring)
- `sidecar/jura-sidecar.spec` (PyInstaller bundling)

If your change touches any file in this list and used AI assistance, the PR description must include a brief note describing what you manually verified.

### Disclosure in pull requests

The pull-request template includes an `AI-Disclosure` section. Fill it in honestly:

- Which AI tool(s) were used (or none)
- For which parts of the change
- What you manually verified

Honest disclosure earns reviewer time; concealment is grounds for closure as a CLA violation.

### Why this is asymmetric

The maintainer uses AI tooling for a substantial portion of day-to-day work and discloses it via `Co-Authored-By:` trailers in commits. The policy above is symmetric: it applies equally to outside contributors and the maintainer. Different surfaces (high-risk vs peripheral) have different review bars regardless of who authored the change.

### Triage policy

Limited maintainer hours mean triage is necessarily strict:

- Bug reports without a working reproducer will be closed with a request to add one
- PRs that look LLM-generated and touch high-risk files without disclosure will be closed with an explanation
- PRs that disclose AI use and target peripheral surfaces (documentation, tests, non-security code) get fair-priority review
- First-time contributors are encouraged to start with peripheral changes; signing / detection / parser changes from new contributors require demonstrated context and a clear test plan

This is not gatekeeping for its own sake. The maintainer is solo and pre-revenue; review time is the binding constraint, and the trade-offs above optimise that constraint while preserving the forensic-credibility surface.

## Security

If you discover a security vulnerability, **do not open a public issue.** Email licensing@juralabs.org with details. We will acknowledge within 7 days and aim to ship a fix or mitigation within 30 days for medium-severity issues, sooner for high-severity ones. Coordinated disclosure is preferred.

### Supply-chain scanning on PRs

Three overlapping scanners run automatically on every pull request to catch known-vulnerable dependencies before they reach `main`:

1. **cargo-audit** (Rust). Runs in `ci.yml` against `src-tauri/Cargo.lock` and the RustSec advisory database.
2. **pip-audit** (Python). Runs in `ci.yml` against the installed sidecar environment.
3. **OSV-Scanner** (cross-ecosystem). Runs in `osv-scanner.yml` against every lockfile (Rust, Python, npm) and posts a SARIF report to the Security tab. Also fires weekly so newly disclosed advisories surface on quiet branches.

If you add a dependency that triggers one of these, the recommended order is: bump the dependency to a patched version, otherwise document the rationale for accepting the advisory in the PR description (with link to the advisory and any compensating control).

#### Socket.dev (optional, recommended for maintainers)

[Socket.dev](https://socket.dev/) catches a class of supply-chain risks that CVE-based scanners miss: typosquats, install-script payloads, telemetry beacons, and packages that newly request network or filesystem capabilities. It is a free GitHub App for open-source repositories.

To install on a fork or downstream:

1. Visit https://socket.dev/install/github and authorise the Socket app for the target repository.
2. Socket runs automatically on every PR that touches `package.json`, `requirements*.txt`, or `Cargo.toml`.
3. Reports appear as a PR check and inline review comments. No CI changes required.

For the upstream `juralabs/jura-archive` repository this is installed at the org level by the maintainer.

## Questions

For questions about contributing that aren't covered here, open a discussion or email licensing@juralabs.org.
