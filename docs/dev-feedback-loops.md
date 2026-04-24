# Tight feedback loops — testing without burning CI

Two patterns that cut CI cost and wait time during iteration.  Use these
**before** tagging anything; reserve formal `vX.Y.Z-rc.N` tags for builds
the approver / pilots will actually receive.

## 1. Test in `cargo tauri dev` first

The default mode for **anything that doesn't change install-time
behaviour**.  Free, hot-reload, instant feedback.

```sh
# Terminal 1 — sidecar
make dev-sidecar

# Terminal 2 — Tauri shell + SvelteKit hot reload
make dev-tauri
```

Catches:

| Bug class | Example |
|---|---|
| Trust-score calibration | rc.22 composite-AI ceiling work |
| UI changes (any) | rc.22 Enhanced banner, Protect hide |
| Sidecar API behaviour | All forensics + classifier output |
| C2PA verifier output | Pixel manifest reading |
| Verify-mode logic | Standard / Deep mode differences |

Does **not** catch:

| Bug class | Why |
|---|---|
| Installer behaviour | NSIS / MSI / DMG only run on a packaged build |
| Code-signing chain | Dev builds aren't signed |
| Tauri permissions in production | Dev mode uses looser CSP / fs scopes |
| Auto-updater | Only present in packaged builds |
| First-run setup wizard logic | Sometimes; depends on the flag check |

For the gap cases, fall back to pattern 2.

## 2. `workflow_dispatch` from a branch — no tag burned

Already enabled on `release.yml`.  Triggers a full CI build off any
branch (or commit SHA), produces installers as **workflow artefacts**
attached to the run page — NOT as a public GitHub Release.

```sh
# Build the current main without tagging
gh workflow run release.yml --repo juralabs/jura-archive --ref main

# Or build a specific branch
gh workflow run release.yml --repo juralabs/jura-archive --ref my-fix-branch

# Or a specific commit
gh workflow run release.yml --repo juralabs/jura-archive --ref <sha>

# Watch progress
gh run watch <run-id> --repo juralabs/jura-archive

# Download the artefacts when done (don't have to publish a release)
gh run download <run-id> --repo juralabs/jura-archive --dir ~/Desktop/test-build
```

**Cost is the same as a tag-triggered build** (still uses CI minutes).
The win is no version-number burn and no public release clutter — you
keep `vX.Y.Z-rc.N` tags for builds you actually share.

### When to use which

| Scenario | Use |
|---|---|
| Calibration tweak you want to verify | `cargo tauri dev` |
| UI redesign | `cargo tauri dev` |
| Single Rust function change | `cargo tauri dev` + `cargo test` |
| Want to spot-check the installer flow before tagging | `workflow_dispatch` from branch |
| Want to send a build to the approver / a pilot tester | Tag (`vX.Y.Z-rc.N`) |
| Production release | Tag |

### Don't tag for

- "I changed one line in CSS, let me see how it looks" → dev mode
- "I think this trust-score formula is right" → dev mode + cargo test
- "I want to confirm Windows installer still works after my fix" →
  workflow_dispatch (no tag), download artefact, install in your VM
- "I need a build the approver can install" → tag

### Workflow_dispatch run on a non-main branch — typical flow

```sh
git checkout -b fix-the-thing
# … hack hack hack …
git commit -am "trying X"
git push -u origin fix-the-thing

# Build + sign Windows + macOS off this branch
gh workflow run release.yml --repo juralabs/jura-archive --ref fix-the-thing

# Wait, download, test in VM
gh run download <run-id> --repo juralabs/jura-archive

# If it works:
git checkout main && git merge fix-the-thing && git push
git tag -a v0.9.0-rc.NN -m "..." && git push origin v0.9.0-rc.NN

# If it doesn't:
# … iterate on fix-the-thing branch, dispatch again, repeat
git push --delete origin fix-the-thing  # cleanup when done
```

This pattern is the difference between **rc.21 → rc.22 → rc.23 → rc.24**
(four tags in two days) and **one rc.NN tag, three workflow_dispatch
trial runs in between**.  Same cost, much less version churn.
