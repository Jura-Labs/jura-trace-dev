# Mac Mini M4 Forgejo runner: setup runbook

This runbook stands up the Forgejo runner on the Mac Mini M4 that
serves the Codeberg-hosted `juralabs/jura-trace` repository. The
runner is the macOS half of the CI pipeline; Windows builds run on
GitHub Actions (Azure Trusted Signing requires a Windows runner).

The runner is scheduled to run **02:00 to 08:00 Copenhagen time**
daily, matching the hosting plan agreed in
`project_hosting_migration_plan` memory. Outside that window the Mac
Mini is idle and the daemon is unloaded.

## Prerequisites

1. Mac Mini M4 powered on and signed in to a user account that holds
   the Apple Developer ID Application certificate
   (`Y82C4P9L7F`) in its login keychain. Verify with:

   ```sh
   security find-identity -v -p codesigning
   ```

   The Developer ID Application certificate should appear in the list.

2. macOS timezone set to Europe/Copenhagen. Verify with:

   ```sh
   systemsetup -gettimezone
   ```

   If not set, run `sudo systemsetup -settimezone Europe/Copenhagen`.

3. Homebrew installed (`brew --version` succeeds).

4. Xcode Command Line Tools installed (`xcode-select -p` returns a path).

5. Rust toolchain installed (`rustc --version` returns 1.80+).

6. Codeberg account with admin access to `codeberg.org/juralabs/jura-trace`
   for registering the runner token.

## Step 1: Install the runner binary

```sh
brew install forgejo-runner
forgejo-runner --version
# Expect: 5.0+ as of 2026-05-20
```

## Step 2: Generate a runner registration token on Codeberg

1. Open https://codeberg.org/juralabs/jura-trace/settings/actions/runners
2. Click **Create new runner**
3. Set name: `mac-mini-m4-prod`
4. Copy the registration token (single-use, expires in 24 hours)

## Step 3: Place the runner config

```sh
mkdir -p ~/.config/forgejo-runner
cp infrastructure/forgejo-runner/config.yml ~/.config/forgejo-runner/config.yml
```

Edit `~/.config/forgejo-runner/config.yml` and replace every
`<runner-user>` placeholder with the macOS account the runner will run
under (check the `HOME` env var + `workdir_parent`). The two launchd
plists in this directory use the same placeholder.

## Step 4: Register the runner

```sh
forgejo-runner register \
  --no-interactive \
  --instance https://codeberg.org \
  --token <PASTE_TOKEN_FROM_STEP_2> \
  --name mac-mini-m4-prod \
  --labels macos-arm64,self-hosted \
  --config ~/.config/forgejo-runner/config.yml
```

This writes `runner-state.json` to
`~/Library/Application Support/forgejo-runner/`. Keep this file out
of backups that leave the machine. It contains the long-lived runner
credential that authorises this Mac Mini to claim jobs from Codeberg.

## Step 5: Test the runner manually

Before scheduling via launchd, verify the runner connects and can
claim jobs:

```sh
forgejo-runner daemon --config ~/.config/forgejo-runner/config.yml
```

In a separate terminal, push a test workflow change to a feature
branch on `codeberg.org/juralabs/jura-trace`. The runner output
should show:

- `[INFO] runner ready`
- `[INFO] task picked up: <task_id>`

After confirming, Ctrl-C the manual daemon. The launchd plists in
Step 6 take over from here.

## Step 6: Install the launchd plists for scheduled run

```sh
mkdir -p ~/Library/Logs/forgejo-runner
mkdir -p ~/forgejo-jobs

cp infrastructure/forgejo-runner/org.juralabs.forgejo-runner.plist \
   ~/Library/LaunchAgents/

cp infrastructure/forgejo-runner/org.juralabs.forgejo-runner-stop.plist \
   ~/Library/LaunchAgents/

launchctl load ~/Library/LaunchAgents/org.juralabs.forgejo-runner.plist
launchctl load ~/Library/LaunchAgents/org.juralabs.forgejo-runner-stop.plist
```

Verify both are loaded:

```sh
launchctl list | grep forgejo
```

The first launch fires at the next 02:00 in your local timezone. The
runner daemon will exit at the next 08:00 when the stop plist fires.

## Step 7: Stop macOS sleep during the active window

macOS Energy Saver may put the Mac Mini to sleep during the 02:00 to
08:00 runner window. Configure a Power Schedule that wakes the
machine just before the window:

```sh
sudo pmset repeat wakeorpoweron MTWRFSU 01:55:00
```

Disable display sleep to avoid lid-close behaviour interfering (the
Mac Mini has no display, but external monitors can cause edge cases):

```sh
sudo pmset -a displaysleep 0
```

The Mac Mini draws ~6 W at idle during the active window; total
nightly energy cost is negligible.

## Step 8: Smoke test the full pipeline

Push a tag to `codeberg.org/juralabs/jura-trace` matching the release
workflow trigger (e.g. `v0.9.0-rc.30`). Within a few seconds of the
02:00 window opening, the runner should:

1. Pick up the workflow
2. Clone the repository to `~/forgejo-jobs/<job_id>/`
3. Run the macOS build steps
4. Upload artefacts to the configured destination (GitHub Releases on
   `juralabs/jura-trace` per the existing release.yml)

Verify by:

- Watching `~/Library/Logs/forgejo-runner/stdout.log` in real time
- Checking the Codeberg Actions UI for the run progress
- Confirming the artefacts land on GitHub Releases at the expected URL

See `docs/runbooks/release-pipeline-smoke-test.md` for the full
end-to-end smoke test covering both Mac Mini macOS builds and the
Windows + Linux halves of the pipeline.

## Troubleshooting

### Runner fires but no jobs are picked up

- Confirm the runner is registered against the correct Codeberg repo:
  ```sh
  cat ~/Library/Application\ Support/forgejo-runner/runner-state.json | jq .
  ```
  The `address` field should be `https://codeberg.org`.
- Check the workflow's `runs-on:` field matches a label this runner
  advertises (`macos-arm64` or `self-hosted`).

### Codesign fails inside the build

The runner must run as the user whose login keychain holds the Apple
Developer ID Application certificate. If you registered the runner
under a different user, re-register under the correct account. The
launchd plist explicitly loads as a LaunchAgent (user scope) rather
than a LaunchDaemon (system scope) for this reason.

If the keychain is locked, codesign will block with a dialog. Pre-
unlock for the active window via:

```sh
security unlock-keychain -p '<keychain_password>' ~/Library/Keychains/login.keychain-db
```

Add this to the runner start sequence if locked-keychain becomes a
recurring issue. See `docs/install-guides/macos-signing-setup.md` for
the signing setup specifics.

### Notarisation fails

The runner needs `APPLE_ID`, `APPLE_PASSWORD` (app-specific password),
and `APPLE_TEAM_ID` available as Codeberg secrets, exposed to the
workflow as env vars. Confirm these are set in the Codeberg repo
settings → Actions → Secrets.

### Disk fills up

The runner's job working directories accumulate at `~/forgejo-jobs/`.
The runner cleans up successful jobs but leaves failed jobs for
debugging. Clean stale ones with:

```sh
find ~/forgejo-jobs -mindepth 1 -maxdepth 1 -mtime +30 -exec rm -rf {} +
```

Add to a weekly cron if disk pressure becomes a concern.

## Out of scope here

- **Windows runner.** Windows builds use GitHub Actions
  (`windows-latest`) because Azure Trusted Signing requires Windows.
  This Mac Mini runner only serves the macOS half of the pipeline.
- **Linux runner.** Linux builds are currently paused per
  `docs/backlog.md`. When they resume, a separate Linux runner (likely
  a Hetzner VPS) handles them; this Mac Mini does not.
- **Multi-job parallelism.** Capacity is set to 1 concurrent job. The
  pilot release cadence (1 to 2 releases per week) does not need
  parallelism. Raise `runner.capacity` in config.yml later if the
  release queue grows.
