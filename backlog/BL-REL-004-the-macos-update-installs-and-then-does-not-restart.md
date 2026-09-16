# BL-REL-004: the macOS update installs, says "restarting shortly", and never restarts

**Status**: Fault 1 fixed in tree (PR #53, option (a), Paul's decision
9 September 2026) and proven on the staging leg 10 September: a running
1.1.0 installed a relabelled 1.1.1 and relaunched itself unattended. The
release notes still owe v1.0.0 users the quit-and-reopen sentence, because
their update runs v1.0.0's code. Fault 2 remains open for a later release.
**Severity**: Fault 1 High: it is what every v1.0.0 user on macOS will see
when they take the v1.1.0 update. Fault 2 Low: it needs the app to be
installed on a volume other than the boot volume.
**Raised**: 9 September 2026, from `scripts/updater-staging/stage.sh serve`
against the public v1.0.0 and the local, notarised v1.1.0 build.

## Verified, not inferred

The throwaway v1.0.0 (`~/Applications/jura-trace-updater-staging/`) fetched
the staging manifest, downloaded and verified the archive, and replaced its
own bundle on disk:

```
21:42:26  v1.0.0 launched (pid 3269)
21:42:46  bundle on disk is now 1.1.0, mtime of Jura Trace.app
21:43:18  health: {"version":"1.0.0","sidecarAvailable":true,"uptimeSeconds":50}
21:45:35  still pid 3269; its executable is
          $TMPDIR/tauri_current_appZoZt9W/current_app/Contents/MacOS/jura-trace
          (the moved-aside old bundle; the directory is already deleted)
21:45:50  quit by hand, reopened from the same path
21:46:05  health: {"version":"1.1.0","sidecarAvailable":true,"uptimeSeconds":13}
```

The button read "Installing, restarting shortly..." from 21:42 until the
app was quit by hand. `spctl -a -t exec` on the installed 1.1.0: "Notarized
Developer ID".

## Fault 1: nothing restarts the app

`tauri-plugin-updater` 2.10.0 on macOS extracts the archive, renames the
running bundle into a temp directory, renames the new bundle into place,
touches it, and returns. It does not restart anything; that is the caller's
job. The caller is `ui/src/lib/updater.ts`, whose `runUpdate` ends:

```ts
    await update.downloadAndInstall((event) => { ... });
    onStatus({ state: 'installing' });
```

and `settings/+page.svelte` renders `installing` as "Installing, restarting
shortly...". No call to relaunch follows, `tauri-plugin-process` is not a
dependency, and `src-tauri/capabilities/default.json` grants no
`process:` permission. The spinner runs until the user quits. Meanwhile
the running process is the old binary, executing from a directory the
updater has already deleted, and the health endpoint keeps answering
`1.0.0`.

This is a BL-SILENT-001 instance: the UI reports a step that no code
performs. Nobody noticed because, per BL-REL-002, no update has ever
reached a user.

### What it means for v1.1.0

- v1.0.0 users run v1.0.0's code. Whatever we ship, their update will
  install and then sit on "restarting shortly" until they quit and reopen.
  The v1.1.0 release notes and the update-available copy on the website
  have to say so, plainly: "When the spinner stops, quit Jura Trace and
  open it again."
- v1.1.0 itself decides what v1.1.0 users see on the *next* update. Two
  options:
  - (a) Do what the copy promises: add `tauri-plugin-process`, grant
    `process:allow-restart`, call `relaunch()` after
    `downloadAndInstall` resolves, and cover it in `updater.test.ts`.
    Rebuild and re-notarise (about 25 minutes with
    `scripts/build-local-mac.sh`). Recommended: the copy is right, the
    code is wrong, and the change is small and local.
  - (b) Change the copy to "Installed. Quit and reopen Jura Trace to
    finish." No rebuild of Rust, but still a rebuild and re-notarise
    because the frontend ships inside the bundle. Same cost, worse
    product.
- Either way the staging leg passed for what it can test: manifest,
  TLS, signature, install, and the installed bundle launching as 1.1.0
  with a healthy sidecar. "Relaunch" in gate item 1 of the plan was
  never true and is corrected there.

## Fault 2: an app on another volume cannot update at all

The first attempt used the throwaway on the external SSD, where
`stage.sh prepare` had put it. Settings, Check for Updates, ended with:

```
Cross-device link (os error 18)
```

The plugin's backup step is `std::fs::rename(app_bundle, $TMPDIR/tauri_current_app*/current_app)`.
`rename(2)` cannot cross filesystems, `$TMPDIR` is on the boot volume, and
the plugin treats any error other than `PermissionDenied` as fatal. So any
user who keeps Jura Trace on an external disk, a second APFS volume, or a
network share gets this error verbatim, with no hint. The stage helper now
keeps the throwaway on the boot volume, which is also what a real
`/Applications` install looks like.

Fix belongs upstream (copy instead of rename when `rename` returns
`EXDEV`), or locally by catching the error string in `classifyError` and
saying "Jura Trace can only update itself when it is in the Applications
folder on your startup disk. Move it there and try again." Low priority;
file upstream when there is time.

## What to do, in order

1. Paul decides (a) or (b) for Fault 1. Either needs one more build.
2. Whichever is chosen, add the quit-and-reopen sentence to the v1.1.0
   release notes and to the website's update copy, because v1.0.0 users
   will hit it regardless.
3. After the rebuild, re-run `stage.sh serve` against the new archive.
   The pass condition is not "1.0.0 updates with no manual quit": the
   relaunch code runs in the *updating* app, and 1.0.0 has none. It is:
   install the new build as the throwaway, relabel the staged manifest one
   patch higher over the same signed archive, and the running 1.1.0 must
   come back by itself with a new pid and its executable inside the
   bundle. **Done, 10 September 2026, 08:28:15: pid 28116 became 28254,
   health 1.1.0, sidecar true, notarised, no temp directories left.**
4. Fault 2: add the friendly error to `classifyError` in a later release;
   open an upstream issue against tauri-plugin-updater.

## Why this belongs beside BL-SILENT-001

Every earlier instance in that file was a check that passed while testing
nothing. This one is a message that describes an action nobody coded. The
common thread is unchanged: the mechanism's report of itself was trusted
instead of its effect, and the first time the effect was observed
(9 September 2026, the first update ever applied to a real Jura Trace
install) the report was wrong.
