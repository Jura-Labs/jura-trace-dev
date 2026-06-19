# Auto-update Hosting Review

**Date:** 2026-06-18
**Author:** DevOps agent (Paul Griffin review)
**Status:** Decision required before Mon 22 June launch

---

## Context

Jura Trace desktop uses the Tauri v2 auto-updater configured in
`src-tauri/tauri.conf.json` lines 38-41. Primary endpoint:

```
https://juralabs.org/api/updates/latest.json
```

Fallback endpoint:

```
https://github.com/Jura-Labs/jura-trace/releases/latest/download/latest.json
```

The primary is served by the Cloudflare Worker at
`infrastructure/cloudflare-worker-updater/src/index.ts`. The worker
fetches `https://api.github.com/repos/juralabs/jura-trace/releases/latest`
and re-formats the result into the Tauri manifest schema.

The proposed distribution change: present installers from
`juralabs.org/download` as PRIMARY and move the GitHub release on
`Jura-Labs/jura-trace` BACK TO DRAFT after launch.

---

## Q1 — What happens to the worker if the GitHub release is drafted?

**Answer: the entire auto-update pipeline breaks. Completely.**

Three separate failure modes, all triggered by the same root cause:

**A. `fetchLatestRelease()` returns nothing.**

`index.ts` line 272-287 calls
`https://api.github.com/repos/juralabs/jura-trace/releases/latest`.
GitHub's REST API documents that this endpoint returns "the latest
published full release" — draft releases are explicitly excluded.
If the only release on `jura-trace` is a draft, the API returns 404
(or the most recent non-draft release, which may be an older version).
The worker logs the error and returns HTTP 502 to the desktop app.

**B. Even with a full PAT, `browser_download_url` for draft assets is not publicly fetchable.**

`buildPlatformBlockSync()` (`index.ts` line 359) sets the `url` field
in the Tauri manifest to `asset.browser_download_url`. For a published
release this resolves to a public CDN URL. For a draft release the same
field value is a private GitHub URL requiring authentication. End-user
desktop app machines have no GitHub credentials, so the download step
fails even if the worker somehow obtained the draft asset metadata.

**C. Inline-signature fetch also fails.**

`buildPlatformBlock()` (`index.ts` lines 317-331) fetches the `.sig`
asset body over HTTP to inline the minisign text. If the release is a
draft, the `.sig` `browser_download_url` is also a private URL.
The `sigResp.ok` check on line 326 returns false; the function returns
`null`; the per-platform route returns 404 to the desktop app.

**D. Fallback endpoint also fails.**

`tauri.conf.json` line 40 lists the fallback:
`https://github.com/Jura-Labs/jura-trace/releases/latest/download/latest.json`
This URL is an alias for the latest PUBLISHED release's asset named
`latest.json`. Tauri v2 no longer generates a `latest.json` asset by
default (only with `v1Compatible` mode). If the release is a draft, this
URL also resolves to nothing. Both endpoints return errors; the desktop
app shows "Update check failed" silently.

**Bottom line:** Drafting the release makes the auto-updater completely
inoperative at the worker level AND the fallback level.

---

## Q2 — How does `juralabs.org/download` work today?

The downloads page is a static HTML file at
`infrastructure/static-pages/downloads/index.html` served from
Cloudflare Pages (or an equivalent static host — see the README at
`infrastructure/static-pages/README.md`).

**It proxies GitHub entirely.** Every table row in `downloads/index.html`
(lines 250-268) points directly to:

```
https://github.com/Jura-Labs/jura-trace/releases/latest
```

This is the GitHub Releases PAGE for human visitors, not a direct asset
download URL. The README (line 42-45) explains the reason: release
assets are version-stamped (`JuraTrace-<ver>-...`), so a fixed
`/latest/download/<name>` URL cannot resolve without per-release
templating. There is no R2 bucket, no Cloudflare Stream, no self-hosted
binary store. The "recommended download card" JS also reads the
`href` from the table row, which is the same GitHub Releases page URL.

**There is no infrastructure today for hosting installer binaries on
juralabs.org independently of GitHub.** The download page is a
branded front door that opens to GitHub Releases.

---

## Q3 — Correct architecture given juralabs.org-primary + GitHub-draft intent

Three options evaluated:

### Option A — Keep the GitHub release PUBLISHED; use juralabs.org as presentation layer only (RECOMMENDED)

The "GitHub draft after launch" intent should be interpreted as
"do not use GitHub as the consumer-facing download page" rather than
literally drafting the release record. The GitHub release on
`Jura-Labs/jura-trace` can remain published (so the API and CDN URLs
work) while:

- `juralabs.org/downloads/` is the canonical download page linked from
  all marketing materials.
- The GitHub Releases page is not promoted but is not hidden.

What changes:
- Nothing in `tauri.conf.json` — endpoint stays as-is.
- Nothing in `infrastructure/cloudflare-worker-updater/` except the
  known `.sig` inlining fix (see below).
- `infrastructure/static-pages/downloads/index.html` links already point
  to `github.com/Jura-Labs/jura-trace/releases/latest`; they continue to
  work.

Where `.sig` inlining happens: the worker's `buildManifest()` function
(`index.ts` line 295-309) calls `buildPlatformBlockSync` which returns
a `.sig` URL rather than inlined text. The per-platform route correctly
inlines via `buildPlatformBlock` (`index.ts` lines 317-331), but
`/api/updates/latest.json` (the primary endpoint polled by the desktop
app) still passes a URL. The known bug must be fixed: replace the
`buildPlatformBlockSync` calls inside `buildManifest()` with async
`buildPlatformBlock` calls. The function signature must become async.
This is a worker-only change; no desktop rebuild needed.

Desktop rebuild required: No. The endpoint is already `juralabs.org`.

Risk: GitHub remains a dependency for the worker's metadata fetch and the
asset CDN. If `Jura-Labs/jura-trace` is deleted or made private, the
updater breaks. Acceptable given the intended public-repo model.

### Option B — Host payloads and signatures on Cloudflare R2

Upload each installer and `.sig` to an R2 bucket at release time.
Update the worker to read from R2 rather than calling the GitHub API.

What changes:
- Add R2 bucket binding to `wrangler.toml`.
- Add a release-step to `release.yml` or `scripts/build-local-mac.sh`
  that uploads binaries + `.sig` files to R2 via `wrangler r2 object put`
  or the R2 S3-compatible API.
- Rewrite worker to read an `index.json` from R2 instead of calling the
  GitHub API.
- Update `infrastructure/static-pages/downloads/index.html` to link
  directly to R2 public URLs (or the worker's proxied URLs).

Where `.sig` inlining happens: the R2 `index.json` is generated at
release time with signatures already inlined. Worker reads it directly;
no secondary fetch.

Desktop rebuild required: No.

Risk: Double-upload complexity at every release. R2 storage is free-tier
for small volumes but adds an operational step. Decouples from GitHub
but requires maintaining the R2 upload script and the index.json schema.
Worth considering for v1.0.1 but too much new infrastructure for the
22 June launch.

### Option C — Static signed `latest.json` uploaded to juralabs.org

Generate a `latest.json` at release time (by hand or by script) with
signatures inlined, upload it to Cloudflare Pages under
`/api/updates/latest.json`, and retire the worker.

What changes:
- Every release: manually or via script generate and upload the manifest.
- Remove or redeploy the worker with a stub that serves the static file.
- Add per-platform files if the desktop ever uses the per-platform route.

Where `.sig` inlining happens: at release time in the generation script.

Desktop rebuild required: No.

Risk: Per-release manual step. On a solo developer schedule with local
Mac builds already being manual, this is feasible but adds friction.
Removes edge caching benefit. Viable fallback if GitHub API proves
unreliable for the worker, but not needed before launch.

---

## Q4 — Does any option require rebuilding, re-tagging, or re-notarising installer artifacts?

No. The minisign signature in the `.sig` file is computed over the binary
content of the installer at bundle time (via `cargo tauri bundle`). The
signature is host-independent: it does not encode any URL, CDN path, or
GitHub reference. Moving the payload from a GitHub CDN URL to an R2 URL
or any other host changes the `url` field in the Tauri manifest, but the
desktop app verifies the CONTENT of the downloaded file against the
signature, not the URL itself. A re-sign would be needed only if the
binary content changed.

One exception: the Tauri updater pubkey is baked into `tauri.conf.json`
at build time. If the minisign keypair is rotated after an installer is
published, that installer can no longer verify future update manifests
signed with the new key. This is the rc.25 situation documented in memory
`project_tauri_updater_key_rotation.md`. It is not triggered by hosting
changes.

---

## Q5 — Sequencing for 22 June launch vs v1.0.1 (~Aug)

### Must be true at launch (22 June):

1. **GitHub release on `Jura-Labs/jura-trace` is PUBLISHED, not drafted.**
   The worker and the fallback endpoint both depend on this. The release
   workflow (`release.yml` line 1641) flips `draft: false` at Phase 3
   publish; confirm it ran to completion before the launch announcement.

2. **Worker deployed to Cloudflare.** Currently the primary endpoint
   returns 404 and the desktop falls through to GitHub. For v1.0 to show
   the "Update available" dialog correctly from v1.0.1 onwards, the
   worker must be live before v1.0.1 ships. Ideal: deploy before launch
   so the endpoint is exercised by real installs from day one.
   Steps: `cd infrastructure/cloudflare-worker-updater && npm run deploy`
   after `wrangler secret put GITHUB_TOKEN` (PAT with `public_repo` scope
   on `Jura-Labs/jura-trace`). See `DEPLOYMENT.md`.

3. **Known `.sig` inlining bug fixed in the worker BEFORE v1.0.1.**
   The aggregated `/api/updates/latest.json` route (the primary endpoint)
   currently passes `.sig` URLs rather than inline text via
   `buildPlatformBlockSync` (`index.ts` line 297-300). The comment on
   line 338-339 acknowledges this and claims "Tauri's updater supports both
   forms" — this claim is incorrect for Tauri v2; the updater requires
   inline text. The fix is to make `buildManifest()` async and call
   `buildPlatformBlock()` (the async version that fetches and inlines)
   for each platform. This is a worker-only code change with no desktop
   rebuild. It MUST land before v1.0.1 or the primary update endpoint
   will silently fail to deliver updates to v1.0 installs.

### Can land before v1.0.1 (target ~4 Aug):

4. Update `infrastructure/static-pages/downloads/index.html` to link
   directly to the per-platform asset URLs rather than the releases page,
   once a stable naming convention is confirmed and the download counts
   are worth tracking separately from the GitHub Releases page view.
   Not a blocker for launch.

5. R2 binary hosting (Option B) as the path to true GitHub independence.
   Worth revisiting after v1.0.1 if the "GitHub as backing store" model
   causes friction with the Codeberg migration plan.

---

## Recommended approach

**Option A, with the `.sig` inlining fix applied to the worker before v1.0.1.**

Rationale:
- Zero infrastructure additions before a 4-day launch window.
- GitHub release stays published; the worker and both endpoints function.
- The download page at `juralabs.org/downloads/` is the promoted URL;
  GitHub Releases is not suppressed but is not the primary marketing
  touchpoint.
- The "GitHub back to draft" intent is better served by not linking to
  the GitHub Releases page in marketing copy rather than by actually
  drafting the release record.
- The `.sig` inline fix is a small async refactor in the worker with no
  surface area change.

---

## Risk list

| Risk | Severity | Mitigation |
|------|----------|------------|
| Release accidentally published as draft on 22 June | Critical | Confirm `gh release view --repo Jura-Labs/jura-trace` shows `draft: false` as part of pre-launch checklist |
| Worker not deployed before v1.0.1 | High | Desktop falls through to GitHub fallback (functional but unmonitored). Add worker deploy to v1.0.1 pre-requisite list |
| `.sig` URL vs inline bug in worker (`/latest.json` route) | High | Primary update check silently returns a manifest the Tauri updater rejects. Fix before v1.0.1 in the worker (no desktop rebuild) |
| GITHUB_TOKEN PAT expiry (1-year rotation) | Medium | Calendar reminder for June 2027; documented in `DEPLOYMENT.md` |
| Worker free-tier ceiling (100K req/day) | Low | ~1,200 req/day at launch (50 installs x 1/hr). Ceiling not reached below ~4,000 installs |
| `releases/latest/download/latest.json` fallback never resolves | Low | Tauri v2 does not emit `latest.json`; fallback is dead. Acceptable because primary endpoint is the designed path. Remove or replace the fallback URL in a future release |

---

## Files referenced

- `src-tauri/tauri.conf.json` lines 38-41 — updater endpoints + pubkey
- `infrastructure/cloudflare-worker-updater/src/index.ts` — full worker source
  - `fetchLatestRelease()` line 272: calls `releases/latest` (excludes drafts)
  - `buildManifest()` line 295: calls sync variant, passes `.sig` URLs
  - `buildPlatformBlockSync()` line 347: sets `signature` to `browser_download_url`
  - `buildPlatformBlock()` line 317: async, fetches and inlines `.sig` text (correct)
- `infrastructure/cloudflare-worker-updater/wrangler.toml` — route binding
- `infrastructure/cloudflare-worker-updater/DEPLOYMENT.md` — deploy steps + failure modes
- `infrastructure/static-pages/downloads/index.html` — download page (links to GitHub Releases page, not direct asset URLs)
- `infrastructure/static-pages/README.md` — explains the GitHub-link-through design
- `.github/workflows/release.yml` line 1641 — `draft: false` publish step

---

## Codeberg-primary / reduce-GitHub-dependency

**Date appended:** 2026-06-18
**Trigger:** User requirement to make Codeberg the public home for source and downloads, reduce GitHub dependency, and address source-code exposure via GitHub release auto-attached archives.

---

### Q1 — Source exposure: what does `Jura-Labs/jura-trace` actually contain?

Inspected via the GitHub MCP (`get_file_contents`) on 2026-06-18. The repo root on `main` contains:

```
.github/FUNDING.yml
.github/ISSUE_TEMPLATE/
.github/PULL_REQUEST_TEMPLATE.md
CODE_OF_CONDUCT.md
COMMERCIAL.md
CONTRIBUTING.md
GENAI_USE_POLICY.md
GETTING_STARTED.md
LICENSE
NOTICE
README.md
SECURITY.md
TRAINING.md
docs/
```

**No application source code is present.** There is no `src-tauri/`, `ui/`, `sidecar/`, or `Cargo.toml`. The repo contains only documentation, licensing text, and community governance files. This is the intended installer-only design from the two-repo model.

**However: GitHub release "Source code" auto-archives expose this repo's git tree, not the private `jura-archive` tree.** When GitHub auto-attaches `Source code (zip)` and `Source code (tar.gz)` to a release on `jura-trace`, those archives contain exactly what is in `jura-trace` at the tagged commit — the docs and governance files above. They do not contain application source from `jura-archive`. A user downloading those archives gets a doc-only tree, not the Rust/Python source.

**AGPL source-availability status:**
- The `jura-trace` repo on GitHub contains the full AGPL-licensed text in `LICENSE` and the AGPL source is available at `github.com/Jura-Labs/jura-archive` — but `jura-archive` is **private**.
- AGPL-3.0 §13 requires that the "Corresponding Source" is available to all users who interact with the software over a network, or that source is offered alongside binaries. For a desktop app distributed as an installer (not a network service), the standard GPL-family requirement is that the source must be available to anyone who receives a binary — typically by pointing to a public URL.
- **Currently, no public URL for the application source exists.** `jura-archive` is private. The Codeberg repo at `codeberg.org/jura-labs/jura-trace` is confirmed not yet live as of 2026-06-18 (the public source mirror planned for 22 June launch). The GitHub `jura-trace` release's "Source code" archives contain only docs, not application source.
- **AGPL compliance is therefore not yet satisfied.** It will be satisfied on 22 June when the Codeberg public repo goes live, provided that repo contains the full application source. This is a launch-day prerequisite, not just a nice-to-have.

---

### Q2 — Codeberg as download/update backing store: the size-limit linchpin

This is the decisive constraint.

**Hard upload ceiling: 100 MB per release asset.**

Codeberg sits behind Cloudflare, and Cloudflare's free plan enforces a 100 MB maximum request body for uploads. This is a Cloudflare infrastructure constraint that Codeberg cannot override without moving off Cloudflare or paying for an enterprise plan. It is not a Forgejo configuration value that can be raised via `app.ini`. Multiple Codeberg Community issues confirm this limit has been in place since at least 2023 and remains unchanged:
- `codeberg.org/Codeberg-e.V./requests/issues/129` — 100 MB limit confirmed, no exception process described for individual files
- `codeberg.org/Codeberg/Community/issues/499` — request to raise limit; no resolution
- `codeberg.org/Codeberg/build-deploy-gitea/pulls/82` — infrastructure PR to raise the Forgejo `ATTACHMENT_MAX_SIZE` setting; blocked by the Cloudflare layer regardless of the Forgejo setting
- `codeberg.org/forgejo/forgejo/issues/10083` — chunked upload proposal (2025); not yet merged or deployed on Codeberg

**v1.0.0 actual asset sizes** (confirmed via `gh release view v1.0.0 --repo Jura-Labs/jura-trace`):
- macOS DMG: 808 MB
- macOS `.app.tar.gz`: 815 MB
- Linux AppImage: 248 MB
- Linux `.deb`: 176 MB
- Linux `.rpm`: 176 MB
- Windows NSIS: 139 MB
- Windows MSI: 143 MB

Every single installer is above the 100 MB Codeberg ceiling. The macOS payloads are 8x the limit. **Codeberg cannot host any of these files as release assets.** This is an absolute hard block with no current workaround on Codeberg's hosted service.

**Storage quota context:** Codeberg rolled out per-user storage quotas from May 2025. The default combined limit for LFS, packages and attachments is 1.5 GB per user. Even if the per-file 100 MB ceiling were lifted by chunked upload support landing, the combined quota for all releases would be exhausted by two macOS release assets. Exceptions can be requested but are not automatic and are not guaranteed for binary-heavy projects.

**Conclusion: Codeberg cannot be the download or auto-update backing store for Jura Trace at current binary sizes.** This is not a policy question — it is an infrastructure ceiling imposed by Cloudflare's free plan that Codeberg has no path to remove in the near term.

---

### Q3 — Forgejo/Codeberg API compatibility with the current worker

The Forgejo REST API is substantially compatible with Gitea's, which was modelled on GitHub's v3 API. Key points relevant to the worker at `infrastructure/cloudflare-worker-updater/src/index.ts`:

**Endpoint equivalence:**
- GitHub: `GET https://api.github.com/repos/{owner}/{repo}/releases/latest`
- Forgejo/Codeberg: `GET https://codeberg.org/api/v1/repos/{owner}/{repo}/releases/latest`

The Forgejo SDK's `Attachment` struct maps the JSON field `browser_download_url` to the download URL — the same field name as GitHub's API. A Forgejo release asset response includes `browser_download_url` pointing to `https://codeberg.org/{owner}/{repo}/releases/download/{tag}/{filename}`.

**Worker changes required to source from Codeberg (if asset size were not a blocker):**
1. Replace `https://api.github.com/repos/juralabs/jura-trace/releases/latest` with `https://codeberg.org/api/v1/repos/jura-labs/jura-trace/releases/latest` in `fetchLatestRelease()`.
2. Replace the `Authorization: Bearer ${GITHUB_TOKEN}` header with `Authorization: token ${CODEBERG_TOKEN}` (Forgejo uses `token` not `Bearer` for API keys, though `Bearer` is also accepted in recent versions).
3. No structural change to `buildPlatformBlock()` or `buildManifest()` — the `browser_download_url` field name is identical.
4. Update `wrangler.toml` secret name from `GITHUB_TOKEN` to `CODEBERG_TOKEN`.

The API surface change is therefore a 3-line substitution in the worker. The worker itself is host-agnostic by design.

**Hybrid architecture viability (source on Codeberg, binaries on R2, worker unchanged):**
This is the clean answer to the constraint. The worker is already a manifest translation layer. It can be pointed at any JSON source that exposes asset URLs and a version string. R2 is the correct binary host. Codeberg is the correct source host. The worker bridges them by reading an `index.json` from R2 (generated at release time) rather than calling any git host API at all. This decouples the update pipeline entirely from both GitHub and Codeberg.

---

### Q4 — Recommended target architecture

The binding constraint from Q2 rules out Codeberg as a binary host. The recommended end-state:

```
Source (AGPL home)          Codeberg  codeberg.org/jura-labs/jura-trace
                                         — full application source
                                         — issue tracker (primary)
                                         — pull requests

Large binaries              Cloudflare R2  (jura-trace-releases bucket)
(installers + .sig files)      — macOS DMG + .app.tar.gz
                               — Windows MSI + NSIS
                               — Linux AppImage + deb + rpm
                               — .sig files co-located
                               — index.json (version manifest, pre-built at release time)

Update manifest / worker    Cloudflare Worker (unchanged route, repointed)
                               — reads index.json from R2 via bucket binding
                               — no longer calls GitHub API at all
                               — worker caching, per-platform routes unchanged

Download page               juralabs.org/downloads/
                               — links directly to R2 public URLs (or worker-proxied)
                               — GitHub Releases page not promoted but not removed

GitHub (residual)           Jura-Labs/jura-trace  (public, read-only archive)
                               — doc-only repo stays published
                               — release record stays published (AGPL "source offer" link)
                               — GitHub Actions removed or kept as optional mirror
                               — no longer the primary binary store
                               — no longer polled by worker
```

**GitHub dependency that cannot be cleanly removed:** GitHub will continue to auto-attach "Source code (zip/tar.gz)" to any release on `jura-trace`. Since `jura-trace` contains only docs (not application source), these archives are harmless but may confuse users. The practical solution is to note in the release body that the application source is at `codeberg.org/jura-labs/jura-trace`.

**Migration sequence (post-launch):**
1. Create R2 bucket `jura-trace-releases`. Bind to worker as KV or R2 binding.
2. Write `scripts/publish-release-r2.sh`: for each release asset, `wrangler r2 object put` the binary and its `.sig`, then generate and upload `index.json` with version, per-platform URLs (R2 public endpoint), and inlined `.sig` text. This replaces the GitHub API call entirely.
3. Rewrite worker `fetchLatestRelease()` to read `index.json` from R2 (single `env.RELEASE_BUCKET.get('index.json')`) instead of calling GitHub. Remove `GITHUB_TOKEN` secret and the GitHub API fetch.
4. Update `infrastructure/static-pages/downloads/index.html` to link directly to R2 asset URLs from the `index.json`.
5. Push v1.0.0 assets to R2 retroactively so the worker serves correct URLs from day one.
6. Deploy updated worker. Verify the primary endpoint `https://juralabs.org/api/updates/latest.json` returns valid manifest.
7. The GitHub `Jura-Labs/jura-trace` release remains published (it does no harm and provides an AGPL source-offer notice). It is no longer the active binary CDN.

---

### Q5 — Launch sequencing (Mon 22 June 2026)

**What must be true before 22 June for AGPL compliance:**
- The Codeberg public repo `codeberg.org/jura-labs/jura-trace` must be live with the full application source (or an equivalent public source URL). The doc-only `jura-trace` GitHub repo does not satisfy AGPL source availability. This is a legal prerequisite for distributing AGPL binaries.

**What changes for the binary/update hosting before 22 June:** Nothing. The prior analysis stands. The GitHub release on `Jura-Labs/jura-trace` must remain published. The Codeberg goal does not alter any launch-day binary hosting decision. The R2 migration is entirely post-launch work.

**What the Codeberg goal adds to launch-day checklist:**
- Verify `codeberg.org/jura-labs/jura-trace` (the fresh snapshot repo) is public and contains full application source before the 22 June announcement.
- Add a note to the GitHub release body pointing to `codeberg.org/jura-labs/jura-trace` as the canonical source home.
- Do not promote the GitHub `jura-trace` releases page as the source home in marketing copy.

**Nothing else changes before Monday.** The R2 migration, worker repoint, and Codeberg API compatibility work are all post-launch.

---

### Recommendation + risk list

**Recommendation:** Keep GitHub release published and worker unchanged for launch. Immediately after launch, execute the R2 migration (Q4 steps 1-7) to remove the GitHub-as-CDN dependency. Point Codeberg as the source home from launch day. Do not attempt to use Codeberg as a binary host at any point.

| Risk | Severity | Notes |
|------|----------|-------|
| AGPL non-compliance if Codeberg source repo is not live on 22 June | Critical | Must be verified before the announcement goes out |
| Codeberg 100 MB ceiling blocks any large binary upload attempt | Hard blocker | Do not attempt; use R2 |
| GitHub "Source code" archives on release confuse users expecting app source | Low | Mitigate by clear release body text pointing to Codeberg |
| R2 migration not done before v1.0.1: worker still calls GitHub API | Medium | v1.0 installs get updates via fallback if worker is broken; acceptable temporarily |
| R2 free tier 10 GB exhausted if many release versions are retained | Low | ~2.5 GB per release (macOS dominant); retain last 4 releases max; $0.015/GB/month beyond free tier |
| Codeberg storage quota exhausted by source repo history | Low | Source-only repo; no large binaries; 750 MB git storage default is ample for a text codebase |
| Worker repoint to R2 introduces `index.json` generation as manual release step | Medium | Mitigate by scripting `publish-release-r2.sh` before v1.0.1 release |

---

### Proposed Plane ticket (post-launch)

**Title:** JTV-XXX: Migrate binary hosting and auto-updater backing to Cloudflare R2, reduce GitHub dependency

**Description:**
Currently all installer binaries and the auto-updater manifest source from `Jura-Labs/jura-trace` GitHub Releases. This creates a dependency on GitHub for every user update check and every download. The goal is to make Codeberg the canonical source home (AGPL compliance anchor) and Cloudflare R2 the binary/update host, with the Cloudflare Worker updated to read from R2 rather than the GitHub API.

Deliverables:
1. Create R2 bucket `jura-trace-releases` with public read access via custom domain or worker proxy.
2. Write `scripts/publish-release-r2.sh` — uploads all platform assets + `.sig` files to R2 and generates `index.json` with inlined signatures and R2 URLs.
3. Rewrite `infrastructure/cloudflare-worker-updater/src/index.ts` `fetchLatestRelease()` to read `index.json` from R2 binding instead of calling the GitHub REST API. Remove `GITHUB_TOKEN` secret.
4. Update `infrastructure/static-pages/downloads/index.html` to link directly to R2 asset URLs.
5. Backfill v1.0.0 assets to R2 so the worker serves correct URLs retroactively.
6. Deploy updated worker and verify `https://juralabs.org/api/updates/latest.json` returns a valid Tauri manifest.
7. Update `DEPLOYMENT.md` and `CLAUDE.md` to reflect the new release process.

Acceptance criteria: the auto-updater primary endpoint returns a correct manifest with no GitHub API call in the worker logs. The GitHub `jura-trace` release can be archived/demoted to a secondary mirror without breaking updates.

Dependencies: R2 bucket creation requires a Cloudflare paid account OR staying within the 10 GB free tier (feasible for 3-4 release versions). Must complete before v1.0.1 ships (~4 Aug 2026) so v1.0 installs update cleanly via the primary endpoint.

Labels: infrastructure, DevOps, post-launch
Priority: Medium (blocks v1.0.1 update chain reliability)
