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
