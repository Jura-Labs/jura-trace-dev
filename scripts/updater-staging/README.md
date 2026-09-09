# The macOS staging leg, as a command

`stage.sh` runs the release gate's staging leg: a pristine, publicly
downloaded Jura Trace v1.0.0 receives the local v1.1.0 build through the
real updater path, on this Mac, without the live endpoint ever being
touched. It is the rehearsal for the live leg, which is done by hand on
pristine installs after the real assets go live.

## Why it works

`tauri-plugin-updater` does not share the application's HTTP stack. It uses
`reqwest` with `rustls-platform-verifier`, so it validates TLS against the
system trust store. A throwaway certificate authority trusted in your login
keychain, a leaf certificate for `juralabs.org`, one line in `/etc/hosts`
sending `juralabs.org` to `127.0.0.1`, and a local HTTPS server on port 443
are therefore enough to make the installed v1.0.0 fetch our manifest instead
of the live one. `.github/workflows/updater-e2e.yml` uses the same trick on
Windows in CI. The Mac leg goes further, because a person can then press
Check for Updates and watch the install and relaunch, which the CI leg
cannot observe.

The public v1.0.0 already carries the current updater public key (rotated
22 May 2026, before v1.0.0 shipped), so a manifest and archive signed with
`~/.tauri/jura-trace-v10.key` verify on it. That is what makes this a real
rehearsal rather than a simulation.

## Run it

Build first, so there is a v1.1.0 archive and signature to serve:

```bash
scripts/build-local-mac.sh
```

Then, in order:

```bash
scripts/updater-staging/stage.sh prepare      # no sudo; downloads v1.0.0 once, ~850 MB
scripts/updater-staging/stage.sh trust        # no sudo; macOS asks for your password once
sudo scripts/updater-staging/stage.sh serve   # hosts entry + port 443; Ctrl-C to stop
```

With `serve` running, in another terminal:

```bash
open "/Volumes/MAC SSD/dev/cargo-target/updater-staging/v1.0.0/Jura Trace.app"
```

Watch the `serve` log. Within about a minute of launch the startup check
requests `/api/updates/latest.json`; the app reports that 1.1.0 is available
but installs nothing, by design. Then Settings, Check for Updates. The log
shows `/staging/Jura.Trace.app.tar.gz` being fetched, the app verifies the
minisign signature against its built-in key, installs, and relaunches as
1.1.0. Confirm with:

```bash
/usr/libexec/PlistBuddy -c "Print CFBundleShortVersionString" \
  "/Volumes/MAC SSD/dev/cargo-target/updater-staging/v1.0.0/Jura Trace.app/Contents/Info.plist"
curl -s http://127.0.0.1:8300/api/v1/health
```

The first should print `1.1.0`; the second should report `sidecarAvailable`
true, which is the compiled launcher doing its job inside an app delivered
by the updater. Record the result in `docs/release/macos-smoke-checklist.md`
section 5.

Finish with Ctrl-C on `serve` (it restores `/etc/hosts` itself) and:

```bash
scripts/updater-staging/stage.sh untrust
scripts/updater-staging/stage.sh status       # everything should read absent / free
```

## What it does not touch

Nothing is written to `/Applications`. The throwaway v1.0.0 lives under the
stage root on the SSD, and the updater replaces that copy in place. The only
two things changed outside the stage root are the hosts entry and the
keychain trust, both undone by the commands above and both visible in
`status`. Confirm the hosts entry is gone before any real update check, or
you will be testing against yourself indefinitely.

## Overrides, for testing the helper itself

`STAGE_PORT` (default 443), `HOSTS_FILE` (default `/etc/hosts`) and
`STAGE_ROOT` (default `$CARGO_TARGET_DIR/updater-staging`). Running
`STAGE_PORT=8443 HOSTS_FILE=/tmp/hosts.copy stage.sh serve` exercises
everything except the two privileged steps, and is how the helper was
checked before its first real run on 9 September 2026.

## What this proves, and what it does not

Proves: the shipped v1.0.0 binary resolves the endpoint, trusts the TLS,
accepts the manifest, verifies the signature, downloads, installs and
relaunches, and the installed 1.1.0 starts its sidecar. That is release gate
item 1 in full.

Does not prove: anything about production DNS, the Cloudflare worker, or the
GitHub fallback, all of which are bypassed by design. Those are the live
leg's job. And it is macOS only; Windows is covered by `updater-e2e.yml` for
the request path and by hand in the UTM VM for acceptance, and Linux cannot
be tested until two AppImage-bearing releases exist.
