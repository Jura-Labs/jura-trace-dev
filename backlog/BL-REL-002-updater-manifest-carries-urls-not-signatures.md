# BL-REL-002: the updater manifest carries URLs where signatures belong, so no v1.0.0 install can update

**Status**: Open. Found 3 September 2026.
**Raised**: 3 September 2026
**Severity**: **Highest open item.** Every v1.0.0 installation in the field
has a non-functional update path. There is no channel through which any fix
in this backlog reaches an existing user.

## What is wrong

`https://juralabs.org/api/updates/latest.json`, fetched 3 September 2026,
returns HTTP 200 and this:

```json
"darwin-aarch64": {
  "url": "https://github.com/Jura-Labs/jura-trace/releases/download/v1.0.0/Jura.Trace.app.tar.gz",
  "signature": "https://github.com/Jura-Labs/jura-trace/releases/download/v1.0.0/Jura.Trace.app.tar.gz.sig"
}
```

The `signature` field holds **the URL of the signature file**. Tauri's
updater expects the *contents* of that file: the base64 minisign blob. The
asset at that URL holds what should have been inlined:

```
dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkK...
```

Tauri does not dereference a URL in that field. It will attempt to verify
the downloaded archive against the literal string, fail, and reject the
update. Both platforms in the manifest are affected.

Two further problems in the same file:

**There is no `linux-x86_64` entry.** Consistent with commit `aa6f562`,
which switched the Linux build to deb-only because "RPM hangs
xz-compressing the 607MB onnx". The updater's Linux target is the
AppImage, which is no longer built, so the manifest generator skips the
platform. Linux users have no update path by construction.

**The GitHub fallback endpoint is a 404.** `tauri.conf.json:37-45` lists
two endpoints. The second,
`https://github.com/Jura-Labs/jura-trace/releases/latest/download/latest.json`,
returns 404: no `latest.json` asset was ever attached to the release. There
is no fallback if the primary endpoint moves or breaks.

## How it showed up

Probing the endpoint while triaging JTV-210, which was raised on 26 May
2026 as "Updater /latest.json: verify inline-vs-URL signature, fix
Cloudflare worker if needed" and marked urgent. It was never done. The
release shipped three weeks later.

## Why it is worse than one broken feature

**It is the delivery mechanism for every other fix.** 145 installers were
downloaded from the v1.0.0 release (21 Linux deb, 31 macOS dmg, 23 Windows
exe, 70 Windows msi). Those users are on a build from 18 June. Nothing in
this backlog can reach them, including a security fix, without them
noticing a new release by hand and reinstalling.

**It has happened once already, differently.** Per the brain, the rc.25
installers carried a Tauri updater public key that was subsequently lost,
and testers had to install fresh for rc.26 onwards. That is not what
happened here: the v1.0.0 key checks out, as recorded below. This is a
second, independent break of the same path in one release cycle, and both
were found only by probing.

## Root cause, found 3 September 2026

It is not the release workflow. The live endpoint is served by the
Cloudflare worker in this repository,
`infrastructure/cloudflare-worker-updater/src/index.ts`. The proof is the
capitalisation: live URLs say `Jura-Labs`, which is GitHub API passthrough,
while `release.yml:1507` hardcodes lowercase `juralabs`.

The worker has two code paths. The per-platform one,
`buildPlatformBlock` (lines 317 to 331), fetches the signature file and
inlines its contents correctly. The aggregated one, which is what
`/latest.json` uses, does not. `buildPlatformBlockSync`, line 359:

```ts
  return { url: asset.browser_download_url, signature: sig.browser_download_url };
```

And immediately above it, at lines 337 to 340, the reason:

> For the aggregated /latest.json endpoint we accept that signatures
> are passed as URLs rather than inline contents; Tauri's updater
> supports both forms.

**That comment is false, and it is the bug.** Tauri has never supported
both forms, in v1 or v2. The plugin passes the string straight to
minisign-verify and never dereferences it. Somebody wrote down a belief
that was wrong, and the implementation followed the comment rather than the
contract.

A related falsehood sits in `release.yml` at around line 1644, which says
clients "will succeed via the fallback endpoint". The fallback 404s.

## Who the fix reaches, and who needs telling instead

**Paired dependency, recorded on Paul's instruction, 3 September 2026: this
item does not close without a re-download notice going out. Fixing the
manifest is half the work.** How much of the field the fix reaches is a
question to settle by testing, not by assuming, and the answer changes how
loud the notice has to be.

The manifest is fetched live at every update check, so a server-side
correction is at least capable of reaching an installed client.

**The signing key is verified, 3 September 2026.** This was the assumption
that could have made the whole field unreachable, and it holds:

- `JuraTrace-1.0.0-macOS-AppleSilicon.dmg` and
  `JuraTrace-1.0.0-Linux-x86_64.deb` present on this machine hash to
  `dbbc4836...` and `94aed07e...`, matching `SHA256SUMS.txt` on the public
  release byte for byte. They are the shipped artefacts, not local rebuilds.
- The executable inside the shipped `.app`,
  `Contents/MacOS/jura-trace` (44.8 MB, 20 June 2026), embeds exactly one
  minisign public key, and it is the expected one. It decodes to
  `untrusted comment: minisign public key: 1AED7E4A127C6230`. No stale
  second key is present.
- All four shipped `.sig` files (`.app.tar.gz`, `.msi`, `-setup.exe`,
  `.deb`) carry key ID `1AED7E4A127C6230` in their signature payloads,
  matching that public key.
- The shipped binary's updater endpoint is
  `https://juralabs.org/api/updates/latest.json`, the one that is currently
  serving the broken manifest.

So the rc.25 key-loss failure has **not** recurred. The only thing wrong is
the manifest, which is server-side and fixable without touching anybody's
installation.

Three groups, revised after technical review on 3 September:

| Group | Downloads | Does the manifest fix reach them? |
|---|---|---|
| macOS dmg, Windows msi | 31 + 70 = 101 | **Yes, but only when they press the button.** No client-side manifest cache; plugin-updater 2.10.0 fetches fresh per check and compares semver, so 1.0.1 > 1.0.0 passes. The worker edge cache is 5 minutes. **But there is no startup check**: `ui/src/lib/updater.ts` runs only from the Settings button and the menu item. A user who never opens that menu never receives anything |
| Windows setup exe (NSIS) | 23 | **No, and worse than no.** The plugin detects the downloaded artefact's format and runs the matching installer without checking how the app was installed. The MSI's signature is valid, so the update "succeeds": msiexec lays down a parallel per-machine copy while the NSIS install and its shortcuts stay at 1.0.0. The user ends up with two installs and probably keeps using the old one |
| Linux deb | 21 | **Possibly rescuable, contrary to what this file said earlier.** Tauri's plugin-updater added `.deb` and `.rpm` support around v2.6 in 2025, and the lock file pins 2.10.0, which postdates it. That is consistent with the build having emitted a `.deb.sig` at all. A `linux-x86_64` entry pointing at the deb, with sig suffix `_amd64.deb.sig`, may rescue all 21 without restoring the AppImage. Untested, and worth testing early because it is the difference between a notice to 21 people and a notice to none |

**The reach question is therefore not settled and three of the answers
changed on review.** The honest summary is that a manifest fix alone
guarantees delivery to nobody, because nothing prompts a user to check.
That argues for the re-download notice going to all 145 regardless of how
the deb and NSIS questions resolve, and for treating a startup or periodic
update check as part of this item rather than a separate improvement.

Drafting the notice is Paul's call on wording and channel, and it touches
`claims.md`, since it says in public that a shipped version could not
update. It should not go out before the fix is live and tested, so that
"download this build" points at something that works.

## What would fix it

0. ~~**Confirm the signing key**~~ **Done, 3 September 2026.** The shipped
   binary embeds `1AED7E4A127C6230` and every shipped signature was made
   with it. This step is closed; the remaining reach question is the NSIS
   one, which step 3 answers.

1. ~~**Find out what actually serves the endpoint.**~~ **Done.** It is
   `infrastructure/cloudflare-worker-updater/src/index.ts`, above.

2. **Make the aggregated path inline the signature**, as the per-platform
   path at lines 317 to 331 already does, and delete the false comment at
   337 to 340 so nobody restores the behaviour it justifies. Add the
   `linux-x86_64` entry while in the file: `PLATFORM_ASSET_CONFIG` at lines
   129 to 131 currently matches only `.AppImage`, which is why Linux
   vanished silently when the build went deb-only in `aa6f562`.

2a. **Add a completeness assertion before the manifest goes live.** This is
   the risk that produced the Linux gap and it is unaddressed. Both the
   worker and the workflow fail *open*: when an asset or a `.sig` is
   missing or misnamed they warn and drop that platform, and the result is
   a manifest that looks healthy and silently tells a whole platform it is
   up to date, forever, with no error anywhere. Given that macOS is built
   locally and Windows in CI, and the assets are assembled by hand, one
   misnamed `.sig` is all it takes. Assert that every expected platform key
   is present and that every signature base64-decodes before serving.

3. **Test the update before believing it.** Most of it can be checked
   before any release: fix the worker, `curl` the manifest, and
   minisign-verify the inline signature offline. Then use the existing
   smoke path (`latest-smoke.json`, `release.yml:1490`) with a dev build
   whose endpoint is overridden. No throwaway public version is needed.
   Shipped clients cannot be repointed, so the first real proof is the live
   manifest itself: stage it by publishing the assets and manifest, then
   verify an update on a machine running the public 1.0.0, **before**
   announcing anything. Test the deb and the NSIS cases explicitly; they
   are the two that decide the size of the notice.

4. **Attach `latest.json` to the release** so the declared fallback
   endpoint resolves.

5. **Decide what Linux and NSIS users get.** For Linux, test the deb
   updater path first; if it works, add the entry and nobody needs telling.
   If it does not, say plainly on the download page that the deb does not
   self-update. For NSIS, either drop that installer or serve per-installer
   manifests, because one static `windows-x86_64` key cannot serve both
   formats and the current arrangement produces a silent duplicate install.

6. **Send the re-download notice**, once 1 to 5 are done and tested.
   Because nothing prompts a user to check for updates, the working
   assumption should be that it goes to all 145 and not only to whichever
   groups the technical fix misses. Wording and channel are Paul's.

**This item does not close at step 5.** A fixed manifest with nobody told
is a repair that only future downloads benefit from.

## What not to do

Do not ship v1.0.1 until this is fixed, tested, and scheduled. A release
that existing users cannot receive is not a release, and cutting one would
put a second stranded version in the field. Paul's instruction of 3
September 2026 is explicit: v1.0.1 is not cut before this has a scheduled
fix.

Do not fix this by removing the updater. The alternative to a working
updater is expecting people who verify other people's media to notice a
GitHub release by themselves.
