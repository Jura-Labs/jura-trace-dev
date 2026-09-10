# BL-TEST-001: nothing tests the installed product

**Status**: Open. **Approved by Paul at the terminal, 4 September 2026**,
as part of the v1.1.0 floor rather than as extra work.
**Raised**: 4 September 2026, from the assessment at
`~/jura-brain/inbox/archive/2026-09-04-trace-installer-testing.md`.
**Severity**: High. **This workflow IS the BL-REL-002 release gate.** The
floor already requires proving an update applies end to end; this is how
that proof gets made, repeatably, rather than by hand once.

## What is wrong

Every test in this repository runs before the product exists.

- 596 Rust tests and 20 API integration tests run against the crate.
- 435 sidecar tests run against Python modules.
- 107 vitest blocks run against components.
- ~148 Playwright specs run against **the SvelteKit dev server in a
  browser**, not against a Tauri build.

Nothing installs an installer. Nothing launches the installed binary.
Nothing checks that what a user downloads works.

The sharpest illustration is `ui/tests/check-for-updates.spec.ts`, whose
own header says it:

> ...run against the SvelteKit dev server (browser mode, no Tauri
> instance). The Tauri updater plugin is therefore unreachable; the tests
> assert browser-mode behaviour only.

So there is a test suite named after the updater which, by design, cannot
tell you the updater is broken. It has been passing throughout the period
in which no user could receive an update.

## How it showed up

Three defects shipped in v1.0.0 and survived for months. A suite that
installed the product and drove it would have caught all three on the day:

1. **The updater manifest carried signature-file URLs** where the signature
   belongs, so every update was rejected. Undetected from 18 June to 4
   September (BL-REL-002).
2. **NSIS installs are offered an MSI**, which silently creates a parallel
   per-machine install while the user's shortcuts stay on the old version.
   Still unproven either way, because nobody has ever run it.
3. **Linux vanished from the manifest** when the AppImage build was
   dropped, and the pipeline failed open, so the platform was told it was
   up to date indefinitely.

Two more from this week are the same class. The sidecar reported version
`0.9.0` while the app declared `1.0.0`, which a "version string matches the
tag" assertion catches immediately. And the C2PA panel renders
`Tool: [object Object]`, which a smoke flow that opens a signed image and
looks at the verdict panel catches immediately.

## The platform matrix, honestly

| Platform | Installer test | UI test | How |
|---|---|---|---|
| **Linux** | yes | yes | GitHub Actions ubuntu runner: install the deb, launch under `xvfb`, drive with `tauri-driver` and WebDriver, which is Tauri's supported e2e route |
| **Windows** | yes | yes | GitHub Actions windows runner: silent install (`msiexec /qn`, NSIS `/S`), verify binary and shortcuts, drive with `tauri-driver` via Edge WebDriver |
| **macOS** | partial | no | `tauri-driver` does not support macOS. An agent on Paul's Mac can mount the DMG, launch, screenshot and probe the sidecar, but click-through automation is fragile AppleScript. **macOS stays Paul's five-minute manual smoke**, which already happens because macOS ships from the local tree |

There is no iOS target. Tauri 2 could produce one, but that is a new
product, not a test gap.

Note what does not exist yet: there is no `tauri-driver` or WebDriver setup
anywhere in the repository. This is new tooling, not a configuration
change.

## What the suite must test

Roughly ten minutes of CI per run.

1. **Installer.** The asset downloads, installs silently, the binary
   launches, and **the version string matches the tag**.
2. **Smoke flow.** Open the app, load a sample signed image, confirm the
   verdict panel renders, open Settings. This is the layer that catches the
   `[object Object]` class of fault.
3. **The updater, end to end.** Install the **previous** version, point it
   at a staging manifest for the new one, press check-for-updates, and
   assert the update applies. `release.yml:1474-1490` already produces
   `latest-smoke.json` for exactly this purpose, so the staging manifest
   half exists.
4. **Artefacts.** Screenshots and logs uploaded per platform for human
   review.

## The NSIS assertion starts red, and must stay red

The Windows test asserts that **an MSI-offered update does not create a
parallel install**. On the current evidence it will fail, because the
updater plugin detects the downloaded artefact's format and runs the
matching installer without checking how the app was installed.

**Do not soften this assertion to make the workflow green.** Paul was
explicit about it on 4 September. A red test naming a real defect is the
correct state, and it is more useful than a green suite that has been
adjusted until it agrees with the bug. When the underlying problem is
fixed, whether by dropping NSIS or by serving per-installer manifests, the
test turns green on its own and means something.

## Cost

Two to three days, inside the v1.1.0 window, overlapping work the floor
demands anyway.

On runner minutes: the assessment put this within the free tier at this
cadence, and the Linux leg is cheap. Windows bills at 2× on a private
repository and a full release build there already costs 144 charged
minutes, so keep this workflow to the installer and smoke path rather than
rebuilding anything, and trigger it on releases and on demand rather than
on every push.

## What not to do

Do not extend the existing Playwright suite to cover this. It drives a dev
server in a browser and cannot reach the Tauri layer; adding installed-app
assertions there would produce another suite that passes while the product
is broken, which is the exact failure being fixed.

Do not skip the previous-version leg of the updater test. Installing the
new version and checking it runs proves nothing about updating. The bug
that stranded 145 users was only visible when a client on the old version
asked the live manifest for the new one.

Do not let macOS silently fall out of scope because it is manual. Write the
five-minute smoke down as a checklist in `docs/release/`, so it is a step
somebody performs rather than a habit somebody remembers.

## Caveat on the parallel-install pass, 7 September

The NSIS assertion has been passing: both installers landed in
`LOCALAPPDATA`, so no split appeared. That result is now qualified.

In the updater E2E run of 7 September the MSI was installed **on its own**,
with no NSIS install present, and it went to `C:\Program Files\Jura Trace`
— **per-machine**. In the installer test NSIS runs first, and there both
end up per-user.

The likeliest reading is that the MSI detects the existing per-user install
and upgrades it in place, rather than the MSI being per-user by nature. If
so, the assertion is passing on a path the stranded users did not
necessarily take, and a machine where the MSI lands first, or where the
NSIS install is not detected, could still produce the split this test is
meant to catch.

The pass was honest and is not being withdrawn. But it is narrower than it
reads, and the fix should not be considered proved by it. Establish which
of the two readings is true before treating the parallel-install risk as
closed — installing the MSI first on a clean machine and then NSIS would
settle it.

## Update, 10 September 2026: the updater leg ran green on jura-trace-dev

Run 34455331642, dispatched against `v1.1.0-rc.1-smoke-20260907`: the real
smoke MSI installed on `windows-latest`, launched, and thirteen seconds later
requested `/api/updates/latest.json` through the throwaway CA and the hosts
redirect. First run of this workflow on the repository that now matters;
its only earlier green was on jura-archive, 7 September.

Two dispatches before it failed at the download step with "release not
found". `RELEASE_PAT` is an organisation secret shared with this repo, and
its token could not see the draft smoke release on `Jura-Labs/jura-trace`,
which needs write access there. Paul replaced the token. The same secret is
what `release.yml` publishes with, so this was worth finding in September
rather than on release day.

Still true, and stated by the workflow itself: this proves the request
path, not the client's verdict. The macOS staging leg proved the verdict
and the install by hand on 9 and 10 September (BL-REL-004). The Windows
verdict needs WebDriver driving Settings, and Linux joins when two
AppImage-bearing releases exist.

## Update, 10 September 2026: the Windows leg now reads the client's verdict

The workflow above proved the request and stopped there. It now proves that
the client accepts what it is sent. Run 34486482125 on
`test/updater-e2e-windows-verdict`, against `v1.1.0-rc.1-smoke-20260907`:

```
14:03:50  installed: C:\Program Files\Jura Trace\jura-trace.exe
14:04:03  request: /api/updates/latest.json
14:04:05  OK: the client accepted the manifest and rendered its verdict:
          "Version 9.9.9 is available"
```

Two seconds between the request and the verdict. The artefact records the
page it was read from (`http://tauri.localhost/`), the document state
(`complete`), the startup check timestamp the app wrote, and the app's own
text, which reads "Version 9.9.9 is available You are running version
1.1.0."

### How the verdict is observed

Not WebDriver, which is what this file and the workflow both previously
named as the next increment. A machine-wide WebView2 policy adds
`--remote-debugging-port=9222` to the browser arguments of the app about to
be launched, and the step then speaks the DevTools protocol to the running
WebView2 and evaluates `document.body.textContent` until the startup
check's banner appears. The banner is rendered by
`ui/src/routes/+layout.svelte` when `runStartupUpdateCheck` returns
`available`, and nothing else in the product can put the advertised version
into that sentence.

Three reasons, all also written into the workflow header. It observes the
startup check, which reports and never installs, so no 800 MB download and
no install happens on a runner to prove a version comparison. The app is
still launched by the same step, so the listener and the observation stay in
one process, which the 7 September failure established as the only reliable
arrangement. And it adds no toolchain: no `cargo install tauri-driver`, and
no msedgedriver whose version must match the runtime.

### What the first attempt found, which is worth keeping

Run 34485684324 asked for the manifest and could not be observed: the
debugging port never answered. The cause is elevation. Steps on a GitHub
Actions Windows runner run as a High Integrity process, and WebView2 ignores
every `WEBVIEW2_*` environment override and every HKCU policy override for
an elevated host. HKLM policy overrides are honoured and are appended to the
arguments wry sets in code, so
`HKLM\SOFTWARE\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments`
is what opens the port. Any future attempt to drive this app on a runner,
by WebDriver or otherwise, meets the same wall.

That run is also the first evidence the new failure taxonomy works. It did
not report a missing verdict. It reported that the test could not look,
said so in those words, and failed.

### The assertion has been shown to fail

Run 34486719261, dispatched with `advertised_version` 1.0.0 against an
installed 1.1.0. The client asked for the manifest, the document was read in
full, no banner appeared because there was nothing to announce, and the
workflow failed on the verdict branch with the app's own text printed. The
startup check timestamp was present, which distinguishes a check that
completed and found nothing from a check that threw.

That dispatch is the mutation check for this workflow, and it is named in
the header so it can be rerun after any change to the observation code. It
answers the objection this backlog exists to raise: a green test nobody has
watched go red proves nothing about itself.

### What the Windows leg still does not prove

Nothing here downloads the advertised build, verifies its minisign
signature, runs the installer, or checks the version afterwards. The Windows
signing-order fault in BL-REL-002 lives entirely in that untested half, and
so does the NSIS parallel-install question of the caveat above. The Settings
button is not exercised either; it takes a different path through
`ui/src/lib/updater.ts` and installs what it finds, which is why the startup
check was the observation point.

The next increment is the install itself: serve an advertised build the
runner can actually fetch, press the button, and assert on the version the
installed binary reports afterwards. That is a heavier test than this one
and it needs the signing-order fix to have shipped in a real release first,
so it is the release after next, not this one.
