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

**The manifest being served is not the one the workflow produces.**
`.github/workflows/release.yml:1496-1580` builds the manifest with
`owner: 'juralabs'` and `baseUrl` of
`https://github.com/juralabs/jura-trace/...`, all lower case. The live
manifest uses `Jura-Labs`. Either it was written by hand, or by the
Cloudflare worker in front of `juralabs.org`, and in both cases the
workflow's own logic is not what is in front of users. That workflow step
is also `continue-on-error`, which is SR-26 in the security register.

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

Three groups, and they are still not in the same position:

| Group | Downloads | Does the manifest fix reach them? |
|---|---|---|
| macOS dmg, Windows msi | 31 + 70 = 101 | **Expected yes.** Key verified, endpoint verified, artefact is what the manifest points at. What remains untested is the update flow end to end, which is step 3 |
| Windows setup exe (NSIS) | 23 | **Unknown.** The key is fine, but the manifest's `windows-x86_64` entry points at the `.msi`. Whether an NSIS install accepts an MSI as its update artefact needs checking before it is promised |
| Linux deb | 21 | **No, and no fix changes that.** There is no `linux-x86_64` entry because the AppImage is no longer built, and Tauri's updater does not update a deb in place. The `.deb.sig` exists and is validly signed, which is beside the point |

So the notice is needed for **21 people at minimum**, and for 44 if the NSIS
question resolves badly. It is no longer plausibly all 145. For the deb
users it is the only channel that exists.

Drafting the notice is Paul's call on wording and channel, and it touches
`claims.md`, since it says in public that a shipped version could not
update. It should not go out before the fix is live and tested, so that
"download this build" points at something that works.

## What would fix it

0. ~~**Confirm the signing key**~~ **Done, 3 September 2026.** The shipped
   binary embeds `1AED7E4A127C6230` and every shipped signature was made
   with it. This step is closed; the remaining reach question is the NSIS
   one, which step 3 answers.

1. **Find out what actually serves the endpoint.** Not the workflow. Check
   the Cloudflare worker or whatever writes the file on `juralabs.org`
   before changing anything, because a fix applied to `release.yml` may not
   reach the served artefact at all.

2. **Inline the signature.** The value must be the exact contents of the
   `.sig` file, trimmed. If `release.yml` is the path that is fixed, note
   that its asset fetch through octokit at line 1568 is the same pattern
   that failed for `SHA256SUMS` and was fixed in commit `bbfd3fc` by
   switching to the `gh` CLI. That precedent is the likely fix here too:
   octokit is returning the redirect URL rather than the asset body.

3. **Test the update before believing it.** Install v1.0.0 from the public
   release, publish a v1.0.1 to a smoke manifest, and observe an actual
   update completing. The manifest returning HTTP 200 is not evidence; it
   returned 200 all along.

4. **Attach `latest.json` to the release** so the declared fallback
   endpoint resolves.

5. **Decide what Linux users get.** Either restore an AppImage build for
   the updater target, or state plainly on the download page that the deb
   does not self-update. Silence is the worst of the three.

6. **Send the re-download notice**, once 1 to 5 are done and tested. Scope
   it to whatever the step 3 test shows cannot be reached automatically:
   21 people at minimum, 44 if the NSIS question resolves badly. Wording
   and channel are Paul's.

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
