# macOS release smoke test

**Five minutes, performed by hand, on every release.**

macOS is the one platform this project cannot test automatically.
`tauri-driver` has no macOS support, so the click-through automation that
covers Linux and Windows does not exist here, and AppleScript equivalents are
fragile enough to produce failures that mean nothing.

That is a reason to write the manual test down, not a reason to skip it.
BL-TEST-001 is explicit: *do not let macOS silently fall out of scope because
it is manual*. A habit somebody remembers is not a step somebody performs.

macOS also has the least margin for error of the three platforms, because it
is the only one that ships from a local build rather than from CI.

---

## Before you start

Test the artefact that will be published, from where a user would get it.
Downloading the DMG from the release page rather than using the local build
output is the point: it is the only way to catch a signing or upload fault.

- [ ] Download the DMG from the release page, not from `target/`
- [ ] **On a Mac that is not the build machine**, if one is available. Gatekeeper
      behaves differently where the signing identity lives, which is exactly
      how an un-notarised build passes locally and fails for everyone else

---

## 1. Gatekeeper and notarisation

```
spctl --assess --verbose=2 --type install <path-to-dmg>
```

- [ ] Reports **`source=Notarized Developer ID`**

If it says `source=Developer ID` without the `Notarized` prefix, the DMG was
signed but not notarised. **Stop. Do not publish.** It will install cleanly on
the building Mac and throw a Gatekeeper warning on every other one, which
silently costs testers' confidence rather than producing a visible error.

---

## 2. Install

- [ ] DMG mounts without warning
- [ ] Drag to Applications succeeds
- [ ] App launches from Applications on first double-click, with no
      right-click-open workaround needed

---

## 3. The app is the version you think it is

With the app running:

```
curl -s http://127.0.0.1:8300/api/v1/health
```

- [ ] `status` is `ok`
- [ ] `version` matches the tag, without the leading `v`
- [ ] `sidecarAvailable` — record the value; see the note below

This is the same probe the Linux job makes, and it catches the same class of
fault: a build that runs but disagrees with its own tag.

`sidecarAvailable` may be `false` immediately after launch, because the
sidecar starts asynchronously. Wait ten seconds and re-run before treating a
`false` as a finding. If it stays `false`, the Analysis Engine did not start,
which is a release blocker on macOS specifically — this is the platform where
the `--onedir` sidecar bundle has broken before (JTV-184).

---

## 4. Smoke flow

The part automation cannot reach. Look at the screen.

- [ ] **Verify** a known-signed image. The verdict panel renders with a trust
      score and detector rows
- [ ] No `[object Object]`, no `undefined`, no `NaN` anywhere on the panel.
      This exact fault shipped in v1.0.0's C2PA panel and no test caught it,
      because no test looks at rendered output
- [ ] The Content Credentials section shows an issuer and a date, not blanks
- [ ] **Protect** a test image: signing completes and the output opens
- [ ] **Settings** opens. Network mode and Analysis Engine status both render
- [ ] **Help** opens

---

## 5. Update path

- [ ] Settings → Check for Updates responds, with a visible status rather than
      silence
- [ ] If an update is genuinely available, it downloads and applies
- [ ] From v1.1.0 onwards, the app restarts itself once the install finishes.
      Updating **from v1.0.0** it will not, and that is expected rather than a
      fault: the restart is v1.1.0's code and v1.0.0 does not have it. The
      button stays on "Installing, restarting shortly..." with the new version
      already on disk. Quit and reopen, then confirm the version.

**Recorded, v1.1.0 on 10 September 2026.** Live update from a pristine public
v1.0.0 against production, no hosts entry and no test certificate: bundle
1.1.0 on disk one minute after the click, then health 1.1.0 with the sidecar
available after quitting and reopening, "Notarized Developer ID", no updater
temporary directories left. See gate item 2 in `v1.1.0-plan.md`.

On a release where this Mac is already on the newest version, the honest
result is "up to date", and that is worth confirming rather than skipping: a
404 reported as "up to date" was a real defect until September 2026, so the
message being correct is itself the assertion.

---

## Record it

Write the result into the release's entry in `docs/release/`, or as a comment
on the release. One line is enough:

```
macOS smoke: v1.1.0, notarised OK, health 1.1.0 sidecar true, verify + protect
+ settings + help all render, update check reports up to date. PG, 13 Nov 2026.
```

An unrecorded pass is indistinguishable from a skipped test, which is the
whole reason this file exists.
