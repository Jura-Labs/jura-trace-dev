# BL-REL-003: the release page advertises a Linux AppImage that was never built

**Status**: Faults 1 and 2's recurrence closed 6 September 2026. Linux
auto-update remains blocked on building the AppImage.

- Fault 1 (the live 404) — **fixed by Paul, 6 September.** The v1.0.0 row
  now reads "Planned for a future release. Use the DEB package below."
  Verified: every file the published body links is an uploaded asset.
- Recurrence — **fixed**, branch
  `fix/release-notes-advertise-only-what-exists`. The template no longer
  composes a row for a bundle that is not built, and a gate before Publish
  fails the release if any linked file was not uploaded.
- Fault 2 (Linux cannot auto-update) — **still open.** Blocked on the
  approved AppImage build; see "What to do, in order" step 3.

**Originally raised**: 5 September 2026, while scoping F3.
**Severity**: High for Linux users, and it is live right now on the public
download page. Two distinct faults share one cause.
**Raised**: 5 September 2026

## Verified, not inferred

```
GET .../v1.0.0/JuraTrace-1.0.0-Linux-x86_64.AppImage   ->  404
GET .../v1.0.0/JuraTrace-1.0.0-Linux-x86_64.deb        ->  200
```

The published v1.0.0 release body contains this row:

> **Linux** (x86_64) — *AppImage* | **[JuraTrace-1.0.0-Linux-x86_64.AppImage]**(…) | Self-signed (AGPL source build)

`gh release view v1.0.0` lists ten assets. There is no AppImage among them,
and there never was.

## The cause

`release.yml:88` composes the AppImage filename and `:109` writes it into
the release body unconditionally. `release.yml:790` builds Linux with
`--bundles deb`, and the comment above it explains why: since the ~607 MB
CLIP onnx was bundled, RPM hangs xz-compressing it for over 88 minutes on a
2-core runner and AppImage squashfs is also slow, so both were deferred.

The build target changed. The release notes template did not. Nothing
connects them, so the page kept advertising a file the pipeline had stopped
producing.

## Fault 1: a 404 on the public download page

Every Linux visitor since 18 June has been offered a download that does not
exist, listed **first**, above the .deb that does. The .deb has 21
downloads; there is no way to know how many people clicked AppImage,
got a 404, and left.

This is the cheapest fix in the backlog and it does not need a release: the
release body is editable text on an already-published release.

## Fault 2: Linux can never auto-update, and this is why

`infrastructure/cloudflare-worker-updater/src/index.ts:129-133` configures
`linux-x86_64` with `urlSuffix: ".AppImage"` and `sigSuffix:
"_amd64.AppImage.sig"`. Neither asset is ever produced, so the platform is
silently dropped from every manifest.

**This is the concrete mechanism behind "this is how Linux vanished" in
BL-REL-002 F2.** That item identified the fail-open behaviour; this
identifies what it was failing open *about*.

It also cannot be fixed by pointing the manifest at the .deb. Tauri v2's
Linux updater installs AppImages; a .deb is not a substitute. So Linux
auto-update is blocked on actually building the AppImage, which is the
1.5-to-2-day item already approved on 3 September.

## What to do, in order

1. **Edit the v1.0.0 release body now** to remove the AppImage row, or
   replace it with a line saying the AppImage is planned. Minutes, no
   release required, stops the bleeding. Needs Paul, since it edits a
   published artefact.
2. **Make the release body describe what was built**, rather than a fixed
   template. The generator should list assets that exist, or assert that
   every file it links was uploaded. A release note that can advertise a
   missing file is the same defect class as BL-SILENT-001: it reports a
   state nobody checked.
3. **Build the AppImage** (approved, 1.5 to 2 days), which unblocks Linux
   auto-update and makes the original row honest.
4. **Until 3 lands, make the dropped platform loud.** BL-REL-002's
   completeness assertion already fails the manifest when a platform is
   missing. Confirm it treats a *deliberately* absent Linux as a
   configuration choice rather than an error, so the gate does not have to
   be softened to get a green release. The clean way is to remove
   `linux-x86_64` from `PLATFORM_ASSET_CONFIG` until the asset exists, so
   absence is declared rather than discovered.

## Why this belongs beside BL-CLAIM-001 and BL-CLAIM-002

Those items cover claims in the product and the README. This is a claim on
the download page, which is the first thing a new user sees and the only
page most of the 145 will ever have read. It is also the most concrete kind
of false claim: not a nuance about what a detector does, but a link that
does not resolve.

## Update, 10 September 2026: the AppImage builds, on this repository

Release workflow run 34470287893 on `jura-trace-dev`, dispatched Linux-only
with tag `v1.1.0-rc.2-smoke-20260910` and no tag pushed. The version gate,
the draft creation and the Linux build all passed. The draft on
`Jura-Labs/jura-trace` carries `JuraTrace-1.1.0-rc.2-smoke-20260910-Linux-x86_64.AppImage`
(853 MB), the `.deb` (777 MB), both minisign `.sig` files, `SHA256SUMS.txt`
and `latest-smoke.json`, whose `linux-x86_64` entry has the friendly AppImage
name as its URL and a minisign signature, not a URL, in the signature field.
First AppImage ever built for Jura Trace, with the replaced `RELEASE_PAT`,
on the repository releases will come from. The workflow had been disabled
since 8 September as the guard for the tag migration; it is enabled again,
so a `v*` tag push now fires a full release.

The Publish job then failed at the manifest completeness assertion,
"missing darwin-aarch64 windows-x86_64". That is BL-REL-002's gate F2 doing
its job on a one-platform run; the workflow's own comment says partial
dispatches are for diagnostics and leave a draft. Not a defect.

Fault 2 therefore moves from "blocked on the AppImage" to "waiting for
v1.1.0". Linux auto-update needs an AppImage-bearing release to update
from, so the first Linux update a user can take is v1.1.0 to v1.1.1, and
the updater end-to-end workflow gains its Linux job at that point.
