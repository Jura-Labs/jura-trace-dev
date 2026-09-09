# BL-REL-002: the updater manifest carries URLs where signatures belong, so no v1.0.0 install can update

**Status**: Open, largely resolved on the server side; the re-download notice and the live update on a real v1.0.0 install remain. The live manifest carries inline signatures (verified 8 September) and the fallback endpoint resolves (attached 6 September), so the sections below that describe URLs in the `signature` field and a 404 fallback are history, not the present. See "Update, 8 September 2026" at the foot of this file. Found 3 September 2026.
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

## Two more faults found 4 September. The manifest fix alone is not enough.

### Windows signatures are invalidated after they are verified

`release.yml` does this, in this order:

| Line | Step |
|---|---|
| 1118 | `tauri-action` builds the MSI **and minisigns it** |
| 1147 | "Verify updater signatures (Windows)" checks the `.sig` is present |
| 1208 | `azure/trusted-signing-action` **rewrites the MSI in place** |
| 1241 | Upload the signed artefacts |

The Azure step's own comment says it "countersigns each matched binary in
place — the original files are overwritten with their signed versions".

So the `.sig` is computed over the pre-Azure bytes and the uploaded MSI is
the post-Azure bytes. They cannot match. **Every Windows auto-update fails
minisign verification, and fixing the manifest does not change that.**

The macOS and Linux path does not have this problem: it verifies at line
965 and uploads at 1017 with no mutation in between. Only Windows inserts a
re-signing step between verification and upload. The repository already
knows this lesson elsewhere: the sidecar executable is deliberately signed
*before* bundling, with a comment at line 670 explaining exactly why order
matters.

Dates from the `createUpdaterArtifacts` switch on 22 May.

**Fix**: minisign the artefacts *after* Azure signing, or re-generate the
`.sig` from the final uploaded bytes, and make the verification step check
the bytes that actually ship. The gate must run last.

### The UI turns endpoint failure into "You're up to date"

`ui/src/lib/updater.ts:117`:

```ts
if (message.includes('No updates available') || message.includes('404')) {
    return { state: 'up-to-date' };
}
```

For a static-JSON updater a 404 is always an endpoint failure. Tauri
signals "no update available" through the version comparison, not through
a 404. So an unreachable or missing manifest renders to the user as a green
"up to date".

The docstring above it contradicts itself within four lines, first saying
404 means "the endpoint is reachable and just has no newer version", then
saying a 404 is "typically a temporary infrastructure issue".

This matters more than it looks, because **the declared fallback endpoint
returns 404 today** (see below). Any user who pressed Check for Updates
during the June to September outage would have been told they were up to
date. `ui/src/lib/updater.test.ts:90-99` asserts this behaviour, so the
test suite certifies it.

**Fix**: map 404 to an error state, not to up-to-date, and correct the
docstring. Add an end-to-end case against a 404ing endpoint asserting the
error state is shown.

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

### Step 3 has its first real client-side evidence, 7 September 2026

Until now every check on this item was made from outside the product:
`curl` the manifest, decode the base64, compare key IDs. All necessary, and
none of it proves a shipped binary does anything.

The updater end-to-end workflow ran on 7 September (run 34143290758). The
real Windows MSI was installed on a clean runner, `juralabs.org` was
pointed at the machine, and the installed binary asked, unprompted:

```
  request: /api/updates/latest.json
```

That is the first evidence that a shipped Jura Trace client reaches the
update endpoint at all. It also proves the TLS half incidentally: the CA
existed only in `LocalMachine\Root`, so `rustls-platform-verifier` is
genuinely consulting the OS trust store, which had been an assumption.

**What it does not prove, and step 3 is not yet closed.** The test stops at
the request. It does not show the client accepting the manifest, verifying
the minisign signature, downloading the MSI, or applying the update —
those need WebDriver to drive the UI past the check. Nor does it touch the
deb or the NSIS case, which step 5 says decide the size of the notice.

So: the endpoint is reached, and the two failure modes that would have been
invisible from outside (DNS and TLS) are eliminated. The remainder of step
3 stands.

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

## Update, 8 September 2026

Reconciled against the live endpoints and `origin/main` at `d0ca411d`. Pull
request numbers in the body of this file above are `jura-archive` numbers
(#31, #65, #67, #78); the `jura-trace-dev` repository created on
8 September numbers from 1 again.

### The live manifest, fetched 8 September 2026 at 22:22 UTC

`curl -sS -D - https://juralabs.org/api/updates/latest.json`:

```
HTTP/2 200
date: Tue, 08 Sep 2026 22:22:08 GMT
content-type: application/json; charset=utf-8
content-length: 25703
cache-control: public, max-age=14400
cf-cache-status: HIT
age: 169
last-modified: Tue, 08 Sep 2026 22:19:19 GMT
server: cloudflare
```

Body: `version` 1.0.0, `pub_date` 2026-06-18T07:32:56Z, two platforms.

| Platform | `url` | `signature` |
|---|---|---|
| `darwin-aarch64` | `.../v1.0.0/Jura.Trace.app.tar.gz` | 408 characters, base64, begins `dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBm`, decodes to `untrusted comment: signature from ...`. Not a URL |
| `windows-x86_64` | `.../v1.0.0/JuraTrace-1.0.0-Windows-x64.msi` | 420 characters, same shape. Not a URL |

No `linux-x86_64` entry.

So the defect in the title is fixed in production. The worker source that
does it is `infrastructure/cloudflare-worker-updater/src/index.ts`
(`3cedd6ce`, 4 September, merged in jura-archive PR #31 on 5 September):
`buildPlatformBlockSync` no longer exists, `locatePlatformAssets`
(`index.ts:520-532`) returns URLs only and cannot produce a manifest block,
`buildPlatformBlock` (`index.ts:472-495`) fetches the `.sig` contents, and
`looksLikeSignature` (`index.ts:459-464`) refuses anything that starts with
`http`, contains whitespace, or is not base64 of more than 64 characters.
The false comment quoted in "Root cause" above is gone. The live output
proves the deployed worker is that code or equivalent; the repository
holds no deployment record, so the deploy date itself is not verifiable
from here.

**One discrepancy, now measured rather than inferred.** "Who the fix
reaches" above says the worker edge cache is 5 minutes, and `index.ts:160`
does set `CACHE_TTL_SECONDS = 300`. The live response carries
`max-age=14400`, `cf-cache-status: HIT`, and `last-modified` and
`accept-ranges` headers the worker never sets. An earlier version of this
paragraph, written on 8 September, concluded from those headers that
propagation could take up to four hours. That was wrong, and it was
withdrawn on 9 September after the edge TTL was measured.

The measurement: the URL was fetched once a minute for nine minutes. `age`
climbed 27, 87, 147, 207, 267 on one cached copy, the edge then refetched
from the worker at about 330 seconds after the fill, and a fresh copy
started at age 60 with a new `last-modified`. **The edge honours the
worker's `max-age=300`.** Two probes settle where the headers come from:
the worker-only per-platform route and a cache-busted `latest.json` both
answer with the worker's own `max-age=300` and no `cf-cache-status`, so the
worker is bound and running and the annotations are added by the cache
layer in front of it. The zone was read on 9 September: it has no Cache
Rules at all (the `http_request_cache_settings` phase has no entrypoint
ruleset), `cache_level` is the default `aggressive`, one Page Rule exists
and it is the `/downloads*` redirect, and **`browser_cache_ttl` is 14400**.
That single zone setting is what rewrites the header on the way out.

So the propagation figure of five minutes stands. The rewritten header is
cosmetic for the updater, because `tauri-plugin-updater` ignores
`Cache-Control` and fetches per check; it affects only a person fetching
the URL in a browser, who would be shown a copy up to four hours old. Two
things follow. Set the zone's Browser Cache TTL to "Respect Existing
Headers" (API value `0`), so the header says what the edge does. And on
release day, purge the exact URL after the v1.1.0 manifest goes live, which
removes even the five minutes; that step is now item 0 of the plan's manual
gate. Neither changes the worker, which is correct as written.

**Done, 9 September 2026.** Paul set Browser Cache TTL to Respect Existing
Headers in the dashboard; the API read shows `browser_cache_ttl` `0`,
modified 07:36:26 UTC. Verified at 07:39 UTC: a GET returned the worker's
own `cache-control: public, max-age=300`, and the next request was
`cf-cache-status: HIT`, `age: 0`, `max-age=300`. The header now says what
the edge does. The discrepancy this section describes is closed; the
purge step in the release plan stands.

### The fallback endpoint

"There is no fallback" above is no longer true.
`https://github.com/Jura-Labs/jura-trace/releases/latest/download/latest.json`
now returns 302 to `.../download/v1.0.0/latest.json` and then 200 with a
25,703-byte body identical in shape to the worker's: version 1.0.0, both
signatures inline (408 and 420 characters), no Linux entry. The asset
`latest.json` was attached to the v1.0.0 release at 2026-09-06T21:21:45Z by
`83dfab73` (jura-archive PR #67, "attach the update manifest, so the
declared fallback exists"). Step 4 is done and was not recorded here until
now.

### Which steps this proves, and which it does not

| Step | State on 8 September |
|---|---|
| 0, 1 | Done, as already recorded |
| 2, inline the signature | **Done and live.** Proven by the curl above |
| 2a, completeness assertion | Partly. The signature-shape check is in (`index.ts:459-464`, `:487-493`) and a platform that fails it is dropped with an error log rather than served. The manifest still fails open on a missing platform: `PLATFORM_ASSET_CONFIG` (`index.ts:189-200`) matches `.AppImage` for Linux, no AppImage exists, so Linux is silently absent, exactly as before |
| 3, test before believing | Partly. The offline half (curl, decode, key ID) is done. jura-archive run 34143290758 on 7 September showed the real Windows MSI reaching `/api/updates/latest.json`. **The live leg, a pristine public v1.0.0 install on macOS and on Windows updating unprompted, has not been run** and cannot be until v1.1.0 assets exist. It is the manual gate's item 2 in `docs/release/v1.1.0-plan.md`, and this item stays open on it |
| 4, attach `latest.json` | **Done**, 6 September, above |
| 5, Linux and NSIS | Open. No Linux entry in either manifest; the deb path is untested; the NSIS duplicate-install question is unanswered |
| 6, the notice | Open. Paul's wording and channel |

The Windows signing-order fault ("Two more faults found 4 September") is
fixed in tree by `2e4fe4f5` (jura-archive PR #31). No release has been
built with it, so every Windows `.sig` currently on the v1.0.0 release is
still the pre-Azure one, and the live manifest's `windows-x86_64` block,
correct in form, still cannot verify against the MSI it points at. The
macOS block has no such problem. That is the strongest reason the live leg
is the gate and not the curl.

The UI's 404-as-up-to-date mapping is fixed by `5a36da72` and a startup
update check added by `13907406` (both jura-archive PR #31 and #32,
5 September); neither reaches a user until v1.1.0 ships.

### What this means for the status

Every server-side fix this item asked for is live. Nothing a user has
installed has changed, no v1.0.0 client has been observed completing an
update, and the notice has not gone out. Per Paul's instruction of
3 September, recorded above, the item does not close on the code fix.

## Update, 9 September 2026: the updater archive now ships a signed launcher

A defect this item did not know about, found by opening the local v1.1.0
build's artefacts by hand. `Contents/MacOS/jura-sidecar` was a shell script
whose code signature lived in extended attributes. The DMG carries them, so
the DMG passed every check. The updater archive is a tar, Tauri's tar
writer drops extended attributes, and `tauri-plugin-updater` extracts with
the same library, so the launcher reached the extracted app "not signed at
all" and Gatekeeper assessed it as `rejected, source=no usable signature`
where the DMG's copy assessed as `Unnotarized Developer ID`. Re-signing that
one file made the archived app pass deep verification; it was the only
defect. Separately, Phase 6 of the build script had never produced an
updater archive at all (`updater` is not a macOS bundle target), so the
archive on disk was Phase 1's, made before the re-signing.

Fixed in PR #49. The launcher is now a compiled Mach-O whose signature is
inside the file, Phase 6 builds the `app` target so the archive is written
from the signed state, and Phase 6.5 extracts the archive on every build and
runs `codesign --verify --deep --strict` on it. The proving rebuild's archive
reports 617 nested binaries signed, deep verification valid, and Gatekeeper
`Unnotarized Developer ID`.

Two consequences for this item. First, the v1.0.0 archive published in June
was built by the same script and very likely has the same defect; that does
not matter for updating *from* v1.0.0, because the client extracts the new
archive, and nobody has ever updated from it. Second, the live leg is still
the gate. Nothing above has been observed on a v1.0.0 client; it has been
observed on the artefact such a client would receive, which is necessary
and not sufficient. The status line stands.

