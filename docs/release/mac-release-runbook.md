---
title: macOS Release Runbook
description: Operational procedure for cutting a signed, notarised macOS release of Jura Trace locally and uploading it to the public release repo.
last-updated: 11 June 2026
---

# macOS Release Runbook

macOS release builds are produced locally, not in CI. The CI macOS job fails by design at the per-file codesign step (stderr is silenced with `>/dev/null 2>&1` inside `set -euo pipefail`, causing the only visible error to be `sort: stdout: Broken pipe`). This failure is expected and creates the draft release as a side effect. Windows CI succeeds and uploads its assets automatically.

---

## 1. Pre-flight checklist

Work through every item before running the build script.

**Disk space**

```bash
df -h /
```

Free space must be above 5 GB. If it is not, run:

```bash
cargo clean --manifest-path src-tauri/Cargo.toml
```

A cold rebuild takes 10–15 minutes but avoids mid-build ENOSPC failures. Also delete `sidecar/build/` if it exists (PyInstaller output, 200–300 MB).

**Quit running processes**

Quit any running instance of Jura Trace and stop the sidecar before building. Stale processes cause port collisions and lock the sidecar binary on disk.

**Detach stale hdiutil mounts**

A leftover disk image mount from a previous failed build will cause `bundle_dmg.sh` to fail. Run:

```bash
for d in $(hdiutil info | awk '/^\/dev\/disk/ {print $1}'); do hdiutil detach "$d" -force; done
```

**Models on external USB**

The build script copies `models/deepfake_classifier.joblib` and `models/univfd_probe.joblib` into `src-tauri/models/`. Verify the external USB is mounted before starting.

**Notarisation credentials**

Run:

```bash
xcrun notarytool history --keychain-profile jura-trace-notary
```

This must return a JSON history payload (even an empty array is fine; it proves the credentials can reach Apple's servers). If the command errors with "could not retrieve the credentials", re-run the one-time setup:

```bash
xcrun notarytool store-credentials "jura-trace-notary" \
  --apple-id <your-apple-id-email> \
  --team-id Y82C4P9L7F \
  --password <16-char-app-specific-password>
```

Generate a new app-specific password at https://appleid.apple.com if needed.

Notarisation is a **release blocker**. A DMG with only Developer ID signing installs cleanly on the building Mac but triggers a Gatekeeper warning on every other Mac. Do not publish an un-notarised build.

**Updater signing key**

```bash
ls -la ~/.tauri/jura-trace-v10.key
```

The file must exist and be mode `600`. The build script auto-loads it. Without this key the `.app.tar.gz.sig` is empty and the auto-updater cannot apply the release.

---

## 2. The release flow

```
Push tag to jura-archive
  → CI creates draft release on jura-trace (Windows succeeds + uploads, Mac fails as expected)
  → Run bash scripts/build-local-mac.sh
  → Verify outputs
  → Upload Mac artefacts
  → Publish draft once both platforms have assets
```

**Tag and push:**

```bash
git tag v0.9.0-rc.X
git push origin v0.9.0-rc.X
```

**Run the build:**

```bash
bash scripts/build-local-mac.sh
```

The full build takes 25–40 minutes on Apple Silicon (sidecar PyInstaller: 5–10 min, Rust compile: 10–15 min, notarisation: 1–10 min).

---

## 3. Build phases and key log lines

| Phase | What it does | Log line that matters |
|---|---|---|
| Pre-flight | Checks OS, cert, key, notarisation creds | `Apple notarisation creds not set` = RELEASE BLOCKER. Stop, fix creds, restart. |
| Clean | Deletes `bundle/`, `sidecar-bundle/`, `sidecar/dist/`, `sidecar/build` | `Cleaned.` |
| Frontend | `npm install && npm run build` in `ui/` | `Frontend built to ui/build/` |
| Sidecar | PyInstaller `--onedir` build | `Sidecar built (NNNmb)` |
| Smoke test | Probes every `/forensics/*` endpoint on the frozen binary | Failure here means a PyInstaller missing-dep bug. Do not proceed. |
| Stage | Copies `sidecar/dist/jura-sidecar` into `src-tauri/sidecar-bundle/` | `Staged to src-tauri/sidecar-bundle/` |
| **Phase 0.5** | Per-file signs every Mach-O in `src-tauri/sidecar-bundle/` BEFORE Tauri bundling. This is the fix for the rc.29 notarisation failure (Phase 6 re-copies from source, overwriting .app signatures). | `Per-file signed NNN Mach-O binaries in src-tauri/sidecar-bundle/` |
| Models | Copies `.joblib` files from external USB into `src-tauri/models/` | Warning if files missing; check USB mount. |
| Macro-cache invalidation | Deletes `target/.../release/jura-trace` + `.fingerprint/jura-trace-*`, then runs `cargo build --release`. Prevents stale frontend being embedded. | `Macro cache invalidated and binary rebuilt.` |
| Phase 1 | `cargo tauri bundle --bundles app` signs the `.app` top level | `Built .app: ...` |
| Phase 5a | Re-signs nested `.so`/`.dylib` inside `.app` (belt-and-braces; Phase 0.5 is the primary fix) | `Per-file signed NNN nested .so/.dylib files.` |
| Phase 5b/5c | Deep-verify `.app` + spot-check 5 nested files for `TeamIdentifier=Y82C4P9L7F` | Any `MIS-SIGNED` line = abort. |
| Phase 6 | `cargo tauri bundle --bundles dmg,updater` from the re-signed `.app` | `DMG built: ...` |
| **Phase 6.5** | Walks every Mach-O in final `.app` and asserts all bear `TeamIdentifier=Y82C4P9L7F`. Fails fast here rather than wasting 25 min at notarisation. | `All NNN nested Mach-O binaries in final .app signed by Team=Y82C4P9L7F` |
| Phase 7 | Signs the DMG itself | `DMG signed.` |
| Phase 8 | Notarises via `xcrun notarytool --wait`, then staples ticket to DMG and `.app` | `Notarisation accepted.` then `Stapled + Gatekeeper-validated.` |

---

## 4. Post-build verification

**Gatekeeper assessment:**

```bash
spctl --assess --verbose=2 --type install "path/to/Jura Trace_VERSION_aarch64.dmg"
```

The output must contain `source=Notarized Developer ID`. Plain `source=Developer ID` (without "Notarized") means the DMG was signed but not notarised. Do not publish.

**Updater artefacts present:**

```bash
find src-tauri/target/aarch64-apple-darwin/release/bundle/macos -name "*.app.tar.gz" -o -name "*.app.tar.gz.sig"
```

Both files must exist and be non-empty. If the `.sig` is empty (0 bytes), the updater key was not loaded. Check `~/.tauri/jura-trace-v10.key` exists and re-run.

---

## 5. Upload and publish

```bash
gh release upload v0.9.0-rc.X --repo Jura-Labs/jura-trace \
  "src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/Jura Trace_VERSION_aarch64.dmg" \
  "src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Jura Trace.app.tar.gz" \
  "src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Jura Trace.app.tar.gz.sig"
```

Check the draft release on `Jura-Labs/jura-trace` and confirm both Windows (`.msi`, `.exe`) and macOS (`.dmg`, `.app.tar.gz`, `.app.tar.gz.sig`) assets are present. Then publish the draft.

---

## 6. Failure modes

| Symptom | Cause | Fix |
|---|---|---|
| `bundle_dmg.sh failed` or `hdiutil: resource busy` | Stale hdiutil mount from a previous failed build | Run the `for d in ...` detach loop from Pre-flight §1, then retry |
| `Apple notarisation creds not set` in pre-flight | Keychain profile absent or env vars not set | Run `xcrun notarytool store-credentials "jura-trace-notary"` and provide credentials |
| Notarisation rejected: "binary is not signed with a valid Developer ID certificate" | Phase 0.5 did not run or was skipped; Phase 6 re-copied unsigned source | Confirm you are on a build with Phase 0.5 present (commit `8aac1d8`+); check Phase 6.5 gate output |
| Stale frontend shipped (app launches but shows previous build's UI) | Cargo macro-cache not invalidated; `tauri::generate_context!()` re-used old `ui/build/` | Build script handles this since commit `162f338` (macro-cache wipe step). If running manually, delete `target/.../release/jura-trace` + `.fingerprint/jura-trace-*` before `cargo tauri bundle`. |
| `sort: stdout: Broken pipe` in CI macOS job | Expected. CI per-file-sign loop silences codesign stderr, causing a broken-pipe exit under `set -euo pipefail`. This is a known CI issue, not a build failure. | Ignore. Build locally as described in this runbook. |
| `.app.tar.gz.sig` is 0 bytes | `TAURI_SIGNING_PRIVATE_KEY` not set; key file absent at `~/.tauri/jura-trace-v10.key` | Verify the key file exists (mode 600). The build script auto-loads it if present. |
