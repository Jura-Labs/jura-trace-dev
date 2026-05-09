<!--
  Jura Trace pull-request template.
  Active on Codeberg / Forgejo. Mirror of .github/PULL_REQUEST_TEMPLATE.md
  during the AGPL launch's source-host migration window.
-->

## Summary

<!-- 1-3 sentences on what this PR does and why. Link to a Plane work item
     or issue if one exists. -->

## AI-Disclosure

<!-- Required. See CONTRIBUTING.md "AI-tool use" for the full policy.
     Concealment of AI use is a CLA violation and grounds for closure. -->

- [ ] **No AI assistance was used** for this change
- [ ] **AI assistance was used.** Tool(s):
  - For: <!-- which parts of the diff did the AI help draft? -->
  - I manually verified: <!-- what did you read, run, and check by hand? -->

## High-risk surfaces

<!-- See CONTRIBUTING.md for the file list. Tick if any of the following
     are touched: c2pa.rs, sidecar.rs, ris.rs, db.rs migrations / backup
     paths, metadata.rs, api/, Cargo.toml, release.yml, tauri.conf.json,
     clip_detector.py, deepfake.py, jura-sidecar.spec -->

- [ ] This change does **not** touch any high-risk file
- [ ] This change touches a high-risk file. I attest I have manually
      reviewed every line of the diff in those files for security and
      correctness, regardless of whether AI assistance was used

## Test plan

<!-- Tick all that apply. -->

- [ ] `cargo check --all-targets` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `svelte-check` passes (no new errors)
- [ ] `npm test` (vitest) passes
- [ ] Existing Playwright e2e tests pass on the affected surfaces
- [ ] New tests added for new functionality
- [ ] Manually verified on: <!-- macOS aarch64 / macOS x86_64 / Windows x86_64 / Linux x86_64 -->

## Other notes

<!-- Anything reviewers should know: backwards-compat concerns, schema
     migrations, performance impact, known edge cases. -->
