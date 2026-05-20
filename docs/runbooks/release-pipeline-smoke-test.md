# Release pipeline smoke test: new infrastructure

**Ticket:** JTV-155
**Phase:** Pre-launch validation
**When to run:** After all four prerequisite tickets are deployed
**Wall-clock:** 2 to 4 hours including the overnight Mac Mini window
**Target:** Complete before rc.25 tag-cut (week of 15 June 2026)
**Author:** Generated 2026-05-20

## What this verifies

End-to-end the full release pipeline produces a usable signed
installer that the existing desktop app can detect, download, and
install via the auto-updater.

The pipeline crosses **four independent systems** that have each been
stood up by separate tickets:

1. **Codeberg** repository + Actions queue (JTV-153)
2. **Mac Mini M4** Forgejo runner for macOS builds (JTV-152)
3. **GitHub Actions** Windows runner for Windows builds + Azure
   Trusted Signing
4. **Cloudflare Worker** at juralabs.org for the auto-updater endpoint
   (JTV-146)

Plus the `juralabs.org/downloads/` static page (JTV-148) as the
human-facing download surface.

This smoke test exercises every junction.

## Prerequisites: must all be green before starting

- [ ] Codeberg `juralabs/jura-trace` repo populated with the current
      `main` branch + all tags (JTV-153 Phase 1 complete)
- [ ] Codeberg secrets configured (JTV-153 Phase 1.3)
- [ ] Mac Mini M4 Forgejo runner registered + verified picking up
      jobs (JTV-152 SETUP.md Step 5 confirmed)
- [ ] `org.juralabs.forgejo-runner.plist` loaded; runner active
      window 02:00 to 08:00 Copenhagen (JTV-152 SETUP.md Step 6)
- [ ] Cloudflare Worker deployed; `curl -i https://juralabs.org/api/updates/latest.json`
      returns HTTP 200 or 502 (both prove the Worker is reachable;
      pre-launch a 502 is expected because no stable release exists yet).
      See JTV-146 DEPLOYMENT.md verify section.
- [ ] juralabs.org/downloads/ page resolves; install links work
      (JTV-148 deployed)
- [ ] GitHub `juralabs/jura-trace` installer-host repo's
      `RELEASE_PAT` secret valid + has `repo` scope (JTV-153
      Phase 2 Option A complete)
- [ ] One known-good currently-installed Jura Trace build available
      for the auto-updater leg (cleanest if it's the v1.0 launch
      candidate that will receive the test update)

## Test sequence

### 1. Cut a smoke-test pre-release tag

From a clean local clone of `codeberg.org/juralabs/jura-trace` (or
push to both remotes if Phase 1 of JTV-153 left the GitHub remote
active as a transitional mirror):

```sh
cd ~/Downloads/ecoadvisor/juralabs

# Confirm clean working tree on main, latest commit pulled
git checkout main
git pull codeberg main
git status

# Use a clearly-test version that will not clash with a real release
# Suggested format: vX.Y.Z-rc.NN-smoke-YYYYMMDD
TEST_TAG="v0.9.0-rc.25-smoke-$(date +%Y%m%d)"

# Tag + push to Codeberg (NOT to GitHub source; that's archived
# after JTV-153 Phase 3, and the runner workflow lives on Codeberg)
git tag -a "$TEST_TAG" -m "Smoke test of the new release pipeline"
git push codeberg "$TEST_TAG"

echo "Pushed $TEST_TAG to Codeberg. Expected workflow trigger: tag push."
```

### 2. Confirm Codeberg picks up the tag and queues the workflow

Within ~30 seconds of the push, the Codeberg Actions UI should show
a queued run. Open:

```
https://codeberg.org/juralabs/jura-trace/actions
```

Expect: one queued workflow run referencing the new tag.

If no run appears, check `.forgejo/workflows/release.yml` has the
tag-push trigger:

```yaml
on:
  push:
    tags: ['v*']
```

### 3. macOS leg: Mac Mini M4 picks up the job

The macOS build only runs during the 02:00 to 08:00 Copenhagen
window. If the tag is pushed outside that window, the job sits in
the queue until 02:00.

To shortcut the wait for this smoke test, manually start the runner:

```sh
# On the Mac Mini, in a Terminal session
launchctl load ~/Library/LaunchAgents/org.juralabs.forgejo-runner.plist
# Wait ~10 seconds for the runner to handshake with Codeberg
tail -f ~/Library/Logs/forgejo-runner/stdout.log
```

Expect within 30 seconds:

- `[INFO] runner ready`
- `[INFO] task picked up: macos-build` (or whatever the job name is in
  the workflow)
- `[INFO] cloning repository ...`

If the runner picks up the job but stalls on clone, check the Forgejo
runner's network reachability to `codeberg.org`.

### 4. macOS build artefacts

The macOS build job should produce:

- `Jura.Trace_<version>_aarch64.app.tar.gz` (Tauri updater payload)
- `Jura.Trace_<version>_aarch64.app.tar.gz.sig` (minisign signature)
- `Jura.Trace_<version>_aarch64.dmg` (installer)

All three should land at:

```
~/forgejo-jobs/<job_id>/src-tauri/target/aarch64-apple-darwin/release/bundle/
```

The workflow then uploads them to GitHub Releases on
`juralabs/jura-trace` via the `RELEASE_PAT` secret.

#### Verify the artefacts are signed correctly

```sh
# Sample one nested .dylib from the sidecar bundle inside the .app
APP=~/forgejo-jobs/<job_id>/src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Jura\ Trace.app

# Top-level .app signing
codesign --verify --deep --strict --verbose=2 "$APP" 2>&1 | tail -5
# Expect: "valid on disk" + "satisfies its Designated Requirement"

# Notarisation ticket stapled (only if APPLE_ID is set as a secret)
xcrun stapler validate "$APP"
# Expect: "The validate action worked!"

# JTV-184 Phase 5 verification: nested dylibs signed by Y82C4P9L7F
SIDECAR_BUNDLE="$APP/Contents/Resources/sidecar-bundle"
sample_dylib=$(find "$SIDECAR_BUNDLE" -name "*.dylib" | head -1)
codesign --display --verbose=2 "$sample_dylib" 2>&1 | grep TeamIdentifier
# Expect: "TeamIdentifier=Y82C4P9L7F"
```

If the JTV-184 Phase 5 per-file-sign step fired correctly, all 610
nested dylibs should carry the Y82C4P9L7F team identifier. If they
don't, the per-file sign loop in `.forgejo/workflows/release.yml`
needs debugging before any further release goes out.

### 5. Windows leg: GitHub Actions

In parallel with the macOS build, the Windows job runs on
`windows-latest` GitHub Actions. Open:

```
https://github.com/Jura-Labs/jura-archive/actions
```

(Or the equivalent if you've moved the Windows workflow to Codeberg.
The current architecture keeps the Windows half on GitHub Actions
because Azure Trusted Signing requires a Windows runner that
GitHub-hosted actions provide for free.)

Expect within ~5 minutes:

- Windows job picked up by `windows-latest` runner
- MSI built + Azure Trusted Signing applied
- `Jura.Trace_<version>_x64_en-US.msi.zip` uploaded to
  `juralabs/jura-trace` GitHub Releases

#### Verify the MSI is signed

Download the MSI from GitHub Releases. On a Windows machine:

```powershell
Get-AuthenticodeSignature .\Jura.Trace_<version>_x64_en-US.msi
# Expect:
#   SignerCertificate: a7e35def-628b-4980-8785-2e535f709418
#   Status: Valid
#   StatusMessage: Signature verified.
```

The certificate ID a7e35def is the Azure Trusted Signing certificate
documented in CLAUDE.md.

### 6. Manifest pipeline: verify `latest-smoke.json` on GitHub Releases

Smoke tags matching the pattern `vX.Y.Z-rc.NN-smoke-YYYYMMDD` get a
`latest-smoke.json` manifest (NOT `latest.json`, which is reserved for
stable releases). This is the workflow's smoke-test bypass: it exercises
the full manifest-generation path without polluting the stable update
channel for users on rc.x builds.

Confirm the manifest landed on GitHub Releases:

```sh
gh release view "$TEST_TAG" --repo juralabs/jura-trace \
  --json assets --jq '.assets[].name' | grep latest-smoke.json
# Expect: latest-smoke.json
```

Fetch and inspect the manifest contents directly from GitHub:

```sh
curl -sL "https://github.com/juralabs/jura-trace/releases/download/${TEST_TAG}/latest-smoke.json" | jq .
# Expect a JSON object with:
#   - version: matches $TEST_TAG (e.g. "v0.9.0-rc.25-smoke-20260601")
#   - platforms.darwin-aarch64.url + .signature (non-empty)
#   - platforms.windows-x86_64.url + .signature (non-empty)
# Linux is paused per docs/backlog.md, no linux-x86_64 block expected.
```

Validate the signatures are non-empty (zero-byte sigs are the classic
TAURI_SIGNING_PRIVATE_KEY misconfiguration failure mode):

```sh
curl -sL "https://github.com/juralabs/jura-trace/releases/download/${TEST_TAG}/latest-smoke.json" \
  | jq -r '.platforms | to_entries[] | "\(.key): \(.value.signature | length) bytes"'
# Expect: each platform reports >300 bytes of signature
```

### 6a. Cloudflare Worker liveness (independent of smoke manifest)

The Worker only serves `/api/updates/latest.json` (stable channel), not
`/latest-smoke.json`. Pre-launch (before v1.0.0) the Worker has nothing
to serve and will return either 502 (no stable release found) or the
last stable manifest if one exists. The smoke test confirms the Worker
is reachable and responding:

```sh
curl -i https://juralabs.org/api/updates/latest.json
# Expect HTTP 200 OR 502 (both prove the Worker is alive). A connection
# error or DNS failure means the Worker is not deployed; fix JTV-146.
```

Full Worker-to-manifest validation happens automatically at v1.0.0
launch when the workflow writes `latest.json` for the first time. That
is the moment to verify:

```sh
# After v1.0.0 ships
curl https://juralabs.org/api/updates/latest.json | jq '.version'
# Expect: "v1.0.0"
```

### 7. Desktop app: auto-updater detects the new version

On a Mac with the previous-version Jura Trace installed:

1. Quit Jura Trace fully (Cmd-Q, ensure no menu-bar process remains)
2. Re-launch Jura Trace
3. Within 30 seconds of startup, the auto-updater should detect the
   new version and display the update dialog

Inside the app, the update dialog should show:

- The new version string (matching step 6)
- The release notes (drawn from the GitHub Release body)
- An "Install Update" button

Click the button. Expected sequence:

- Download progress bar (downloading the .app.tar.gz from GitHub
  Releases; Cloudflare Worker only served the manifest, not the
  binary)
- Verification (minisign signature check against the bundled pubkey
  in `tauri.conf.json:43`)
- Install (the app quits, replaces itself, and relaunches)

Verify the new version is running:

- macOS menu bar → Jura Trace → About
- Or `defaults read "/Applications/Jura Trace.app/Contents/Info.plist" CFBundleShortVersionString`

### 8. End-user download path: juralabs.org/downloads/

In a private browsing window:

1. Visit https://juralabs.org/downloads/
2. The recommended download card should auto-detect your platform
3. Click the download link; the file should download from
   `github.com/juralabs/jura-trace/releases/latest/download/...`
4. Verify the downloaded file's SHA-256 against the release notes
5. Install + launch; the app should start cleanly with no Gatekeeper
   warnings (macOS) or SmartScreen warnings (Windows)

### 9. Clean up the smoke-test tag

After the smoke test passes, delete the test tag from Codeberg + the
test release from GitHub:

```sh
# Delete from Codeberg
git push codeberg --delete "$TEST_TAG"

# Delete from GitHub (the installer-host repo's Releases UI, or via gh)
gh release delete "$TEST_TAG" --repo juralabs/jura-trace --yes
```

This keeps the public release timeline clean for the actual v1.0
launch.

## Pass / fail criteria

The smoke test **passes** if all of the following are true:

- [ ] Codeberg picks up the tag push and queues a workflow run
- [ ] Mac Mini M4 runner claims the macOS job within 30 seconds of
      its window opening
- [ ] macOS .app builds, all 610 nested dylibs signed by Y82C4P9L7F,
      .app + DMG both notarised and stapled
- [ ] Windows MSI builds + Azure Trusted Signing applied
- [ ] Both platforms' artefacts uploaded to `juralabs/jura-trace`
      GitHub Releases via `RELEASE_PAT`
- [ ] Cloudflare Worker serves the updated manifest within 5 minutes
- [ ] Installed desktop app detects the update, downloads, verifies
      signature, installs, relaunches as the new version
- [ ] juralabs.org/downloads/ resolves and links work for a fresh-
      install path

The smoke test **fails** if any step blocks. Stop, diagnose the
specific failure, fix at the responsible system level, then re-run
from step 1 with a new test tag.

## Common failure modes + fixes

### Forgejo runner does not pick up the job

- Verify the runner is in its active window (02:00 to 08:00 Copenhagen)
- Check `~/Library/Logs/forgejo-runner/stdout.log` for handshake errors
- Restart the runner: `launchctl unload && launchctl load`

### macOS codesign fails

- Verify the user's login keychain holds the Developer ID Application
  certificate: `security find-identity -v -p codesigning`
- Unlock the keychain if it timed out: `security unlock-keychain`
- See `docs/install-guides/macos-signing-setup.md`

### macOS notarisation fails

- Verify `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` are set as
  Codeberg secrets and exposed to the workflow as env vars
- Check the notarytool submission log on Apple's side via:
  `xcrun notarytool history --apple-id ... --password ... --team-id ...`

### Windows MSI signing fails

- Verify Azure Trusted Signing secrets are set in GitHub Actions
  (the Windows workflow stays on GitHub Actions, not Codeberg)
- Confirm `AZURE_SIGNING_ENDPOINT` matches the region configured for
  the certificate

### Cloudflare Worker returns stale manifest

- Worker caches at the edge for 5 minutes. Wait + retry, or purge
  via `curl -X PURGE`
- Verify the worker's `GITHUB_TOKEN` secret has `public_repo` scope
  on `juralabs/jura-trace`
- Tail worker logs: `wrangler tail`

### Auto-updater verifies signature but install hangs

- Macros: ensure the previous-version .app is not running (background
  process can prevent the file-swap)
- Check `~/Library/Logs/Jura Trace/` for the updater log
- Fall back: manually download from the GitHub Release + re-install

### Auto-updater never sees the new version

- Hard-restart the desktop app (Cmd-Q, relaunch)
- Verify `tauri.conf.json:38-41` lists both endpoints (juralabs.org
  primary, GitHub fallback)
- Manually hit `juralabs.org/api/updates/latest.json` in a browser to
  confirm the manifest version

## After a successful smoke test

Document the result in `CHANGELOG.md` under the v1.0 pre-launch
section. Update `docs/backlog.md` to mark JTV-155 as Done.

In Plane, close JTV-146, JTV-148, JTV-152, JTV-153, JTV-154, JTV-155
with a comment pointing at this runbook + the smoke-test result.

Next gate: rc.25 tag-cut on week of 15 June 2026 for the v1.0 launch
on 22 June 2026.
