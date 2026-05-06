# Contributing to Jura Trace

Thank you for your interest in contributing to Jura Trace — a local-first desktop application for forensic media verification, published as open-source public-interest infrastructure.

Jura Trace is published under AGPL-3.0-or-later with a parallel commercial-licence path (see `LICENSE` and `COMMERCIAL.md`). The Contributor Licence Agreement below is what makes that dual-licensing model work.

## How to contribute

- **Bug reports:** open an issue on the canonical repository.
- **Feature ideas:** open a discussion or issue first, before writing code, so we can align on whether the feature fits the roadmap and the local-first principle. See `docs/backlog.md` and `PROJECT_SPEC.md` for current direction.
- **Pull requests:** small, focused, with tests where appropriate. Match existing code style. Keep architectural changes in a separate PR from feature changes.
- **Documentation:** PRs welcome on user guides, methodology documents, and translation contributions.

## Code style

- **Rust:** `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must pass.
- **Python:** existing patterns in `sidecar/app/` are the reference. Type hints on public functions.
- **Frontend:** SvelteKit 5 runes (`$state`, `$derived`), TailwindCSS, British spelling for user-facing text (American spelling acceptable in code identifiers for framework consistency).
- **Commits** must pass the pre-commit hook: `cargo fmt`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `svelte-check`, `npm test` (vitest).

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

## Security

If you discover a security vulnerability, **do not open a public issue.** Email licensing@juralabs.org with details. We will acknowledge within 7 days and aim to ship a fix or mitigation within 30 days for medium-severity issues, sooner for high-severity ones. Coordinated disclosure is preferred.

## Questions

For questions about contributing that aren't covered here, open a discussion or email licensing@juralabs.org.
