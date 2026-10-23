# Jura Trace auto-updater worker: deployment

This Cloudflare Worker answers requests to `juralabs.org/api/updates/*`
and serves the Tauri auto-updater manifest assembled from GitHub
Releases on `juralabs/jura-trace`. It also answers `juralabs.org/dl/*`,
the counted download links (see "Counted download links" below).

The Tauri desktop app already lists `https://juralabs.org/api/updates/latest.json`
as its primary updater endpoint (`src-tauri/tauri.conf.json` line 39),
with the direct GitHub URL as fallback. Until this worker is deployed
the primary endpoint returns 404 and the app falls through to GitHub.

## One-time setup

1. Install Wrangler if you do not already have it.

   ```sh
   npm install -g wrangler
   wrangler --version   # 3.85+ recommended
   ```

2. Authenticate against the juralabs.org Cloudflare account.

   ```sh
   wrangler login
   ```

   This opens a browser to the Cloudflare OAuth flow. Sign in with the
   account that owns the `juralabs.org` zone (per memory
   `project_jtv145_cloudflare_active` the account ID is recorded
   alongside the CAA / DoH rules).

3. Install local devDependencies.

   ```sh
   cd infrastructure/cloudflare-worker-updater
   npm install
   ```

4. Set the GitHub PAT as a secret. The PAT needs `public_repo` scope on
   `juralabs/jura-trace` (read access to releases is sufficient; the
   worker does not modify any GitHub state).

   ```sh
   wrangler secret put GITHUB_TOKEN
   ```

   When prompted, paste the PAT. Generate the PAT at
   https://github.com/settings/tokens/new with scope `public_repo`,
   expiry: 1 year, name: "juralabs.org updater worker".

   Rotate this PAT annually. Calendar reminder in the v1.0.1 release
   cycle covers the first rotation cycle.

## Deploy

```sh
cd infrastructure/cloudflare-worker-updater
npm run deploy
```

Wrangler bundles `src/index.ts`, uploads it to Cloudflare, and binds
the `juralabs.org/api/updates/*` and `juralabs.org/dl/*` routes. First
deploy emits the worker URL and the route binding confirmation.

Before deploying, run the unit tests and the type check:

```sh
npm test            # vitest, no network, under a second
npm run typecheck   # tsc --noEmit over src/ and test/
```

Nothing in CI runs these yet; they are local checks.

## Verify

After deployment, test against the live endpoint:

```sh
# Aggregated manifest (should return JSON with all platform blocks)
curl https://juralabs.org/api/updates/latest.json | jq .

# Per-platform check, current version equal to latest tag
# Expect 204 No Content
curl -i https://juralabs.org/api/updates/darwin-aarch64/0.9.0

# Per-platform check, stale version
# Expect 200 with single-platform JSON block
curl https://juralabs.org/api/updates/darwin-aarch64/0.8.0 | jq .

# The form a v1.3.0 or later install sends: {os}-{arch}-{bundle_type}.
# Expect 200 with the block keyed darwin-aarch64, linux-x86_64,
# linux-x86_64-deb or windows-x86_64 respectively (the key the plugin's
# fallback lookup finds), or 204 when the version is already the latest.
for p in darwin-aarch64-app linux-x86_64-appimage linux-x86_64-deb \
         windows-x86_64-msi windows-x86_64-nsis; do
  curl -s -o /dev/null -w "$p %{http_code}\n" "https://juralabs.org/api/updates/$p/0.0.0"
done

# Counted download link
# Expect 302, a Location on github.com/Jura-Labs/jura-trace/releases/download/,
# and Cache-Control: no-store. A HEAD is not counted; a curl GET would be,
# under downloads-ci: (curl is classed as automation).
curl -I https://juralabs.org/dl/1.2.0/macos

# Expect 404 JSON
curl -i https://juralabs.org/dl/1.2.0/not-a-slug
```

Every request to the update routes is counted in KV (see wrangler.toml). A
curl, or anything sending `X-Jura-Test`, is filed under `checks-ci:` rather
than `checks:` (see isAutomation in src/index.ts), so these hand checks do
not inflate the figure that says whether anyone outside is running Trace.

**Deploy before the v1.3.0 tag is cut.** v1.3.0 is the first release whose
installs ask on the `{os}-{arch}-{bundle_type}` form. Until this worker is
deployed those requests get 400 or 404 from juralabs.org, the plugin logs
it and moves on to the GitHub fallback (latest.json, which carries every
key including linux-x86_64-deb), so updates still arrive on every install
type, but silently and uncounted. Deploying early is safe: no shipped
install asks on the new form yet, and latest.json is unchanged.

The desktop app's updater check (automatic, or manual via Settings →
Service Status → Check for Updates) hits the
endpoint on launch.

## Counted download links (`/dl`)

`GET /dl/<version>/<slug>` answers a 302 to the matching installer on
GitHub Releases and counts the click on the way through. It exists
because the download page used to link straight at github.com, so the
website never saw a click and the only download figure was GitHub's,
which is cumulative since release day and includes every CI run that
fetched an installer.

The redirect is pure: no GitHub API call, no check that the asset
exists, no cache. `<version>` must match `MAJOR.MINOR.PATCH` with an
optional `-rc.N`, and `<slug>` must be one of the names below. Anything
else is a 404 and is not counted, so a crawler walking `/dl/` leaves
nothing behind.

| Slug | Redirects to (`<v>` is the version) |
|---|---|
| `macos` | `JuraTrace-<v>-macOS-AppleSilicon.dmg` |
| `windows-exe` | `JuraTrace-<v>-Windows-x64-setup.exe` |
| `windows-msi` | `JuraTrace-<v>-Windows-x64.msi` |
| `linux-appimage` | `JuraTrace-<v>-Linux-x86_64.AppImage` |
| `linux-deb` | `JuraTrace-<v>-Linux-x86_64.deb` |
| `linux-server` | `JuraTrace-<v>-Linux-x86_64-server.tar.gz` |
| `sha256sums` | `SHA256SUMS.txt` |

All under `https://github.com/Jura-Labs/jura-trace/releases/download/v<v>/`.
The names are the friendly names `release.yml` uploads; if the release
workflow renames an asset, `DOWNLOAD_SLUGS` in `src/index.ts` and this
table change with it.

The website's download buttons have to be pointed at these URLs for the
count to mean anything. That is a change to the juralabs.org site, made
separately from the worker.

## Request counts in KV

The `UPDATE_COUNTS` KV namespace holds one integer per key, incremented
by the worker after the response has gone (`ctx.waitUntil`). Counting
never blocks and never fails a request. Keys expire after 100 days.

| Key | Meaning |
|---|---|
| `checks:<YYYY-MM-DD>:latest` | Update checks against `/api/updates/latest.json` (GET and HEAD) |
| `checks:<YYYY-MM-DD>:<platform>` | Update checks against the per-platform route |
| `checks:<YYYY-MM-DD>:<platform>:<version>` | The same, split by the version the install reports |
| `downloads:<YYYY-MM-DD>:<slug>:<version>` | GETs of `/dl/<version>/<slug>` that redirected (HEAD is not counted) |
| `checks-ci:...`, `downloads-ci:...` | The same keys, for requests the worker takes to be automation |

Days are UTC. Read them with Wrangler (`--remote` reads production;
without it Wrangler reads the local dev store):

```sh
wrangler kv key list --binding UPDATE_COUNTS --remote --prefix downloads:
wrangler kv key list --binding UPDATE_COUNTS --remote --prefix downloads-ci:
wrangler kv key list --binding UPDATE_COUNTS --remote --prefix checks:
wrangler kv key get  --binding UPDATE_COUNTS --remote "downloads:2026-10-23:macos:1.2.0"
```

`key list` returns names only; the value of each is one `key get`. For a
week's figure, list with the prefix and get each key. KV is eventually
consistent and the increment is a read-modify-write, so two requests in
the same instant can lose a count. The figures are directional. If they
ever need to be exact, that is Durable Objects or Analytics Engine, not
a lock here.

### The automation heuristic

Our own release workflow, live legs, installer tests and hand checks
fetch installers and poll the update endpoint, and they were about a
fifth of GitHub's download total by October 2026. The worker files a
request under the `-ci` keys when any of these holds (`isAutomation` in
`src/index.ts`):

1. The request carries an `X-Jura-Test` header, any value. This is the
   explicit opt-in for workflows that touch the live worker.
2. The request arrives from AS8075 (Microsoft), where GitHub-hosted
   Actions runners live. Self-hosted runners are not covered by this
   rule and rely on the other two.
3. The `User-Agent` names a command-line or library client: `GitHub CLI`
   or `gh/`, `curl/`, `Wget`, `python-requests`, `Go-http-client`.

It is a heuristic. A developer who curls a link from home is filed as
automation; a script that sends a browser user agent from a residential
line is not. Either way the cost is one count in the wrong bucket.

The Tauri updater is deliberately not caught. `tauri-plugin-updater`
sends `User-Agent: tauri-plugin-updater/<crate version>` (2.10.0 in
v1.0.0 and v1.1.0, 2.13.3 in v1.2.0), and the tests pin that it
classifies as a real check. An update check counted as CI would quietly
empty the one field signal this worker provides.

### Privacy position

Counts only. The worker stores a number per day per route. It does not
store IP addresses, user agents, cookies, request IDs or anything else
that could identify a person, a device or an install; it sets no cookie
and the redirect carries no tracking parameter. The user agent and the
network ASN are read to choose between the plain and the `-ci` bucket
and are then discarded with the request. Two people downloading are
indistinguishable from one person downloading twice, and that is the
intended precision. "Trace collects nothing" remains true of the app;
this counting happens on our website and update service, and the
privacy page should say so.

## Local development

```sh
cd infrastructure/cloudflare-worker-updater
wrangler dev
```

Hits `http://localhost:8787/api/updates/latest.json` locally without
touching production routes. Set the GITHUB_TOKEN in a local `.dev.vars`
file (gitignored; see `.dev.vars.example`).

## Observability

```sh
wrangler tail
```

Streams worker logs in real time. Useful for diagnosing the rare case
when GitHub Releases returns an unexpected shape.

Free-tier Workers include 100,000 requests/day; the Jura Trace pilot
volume (~50 active installs × 1 update check per hour) is ~1,200
requests/day. The endpoint will not approach the free-tier ceiling
before a 5-digit install base.

## Failure modes + fallback

When the worker is unreachable OR GitHub Releases is unreachable to
the worker, the desktop app falls back to its second configured
updater endpoint (the direct GitHub Releases URL in
`tauri.conf.json` line 40). The fallback is a published GitHub
Releases assets URL (`/releases/latest/download/latest.json`) which
GitHub will serve from its own CDN.

This means the worker is a performance + redirection-control
optimisation rather than a hard dependency. Updates still flow if
the worker is down.

## Removing the worker

If juralabs.org is ever migrated away from Cloudflare or the auto-
updater architecture changes:

```sh
wrangler delete
```

The desktop app continues to update via its fallback GitHub Releases
endpoint until the next desktop release ships a config change.
