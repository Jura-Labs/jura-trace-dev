# Jura Trace auto-updater worker: deployment

This Cloudflare Worker answers requests to `juralabs.org/api/updates/*`
and serves the Tauri auto-updater manifest assembled from GitHub
Releases on `juralabs/jura-trace`.

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
the `juralabs.org/api/updates/*` route. First deploy emits the worker
URL and the route binding confirmation.

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
```

The desktop app's smoke test (`infrastructure/forgejo-runner/...` or
manual via Settings → Service Status → Check for Updates) hits the
endpoint on launch.

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
