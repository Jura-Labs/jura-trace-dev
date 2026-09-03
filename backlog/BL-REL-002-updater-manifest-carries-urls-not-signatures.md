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

**It has happened once already.** Per the brain, the rc.25 installers
carried a Tauri updater public key that was subsequently lost, and testers
had to install fresh for rc.26 onwards. This is the second time in one
release cycle that the update path has been broken in a way nobody noticed
until it was probed.

**The manifest being served is not the one the workflow produces.**
`.github/workflows/release.yml:1496-1580` builds the manifest with
`owner: 'juralabs'` and `baseUrl` of
`https://github.com/juralabs/jura-trace/...`, all lower case. The live
manifest uses `Jura-Labs`. Either it was written by hand, or by the
Cloudflare worker in front of `juralabs.org`, and in both cases the
workflow's own logic is not what is in front of users. That workflow step
is also `continue-on-error`, which is SR-26 in the security register.

## What would fix it

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

## What not to do

Do not ship v1.0.1 until this is fixed and tested. A release that existing
users cannot receive is not a release, and cutting one would put a second
stranded version in the field.

Do not fix this by removing the updater. The alternative to a working
updater is expecting people who verify other people's media to notice a
GitHub release by themselves.
