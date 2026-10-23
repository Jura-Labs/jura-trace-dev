/**
 * Jura Trace auto-updater endpoint (Cloudflare Worker)
 *
 * Serves the Tauri auto-updater manifest from juralabs.org so the desktop
 * app can check for updates without a direct dependency on GitHub Releases
 * being reachable from every user network.
 *
 * Routes:
 *
 *   GET /api/updates/latest.json
 *     Canonical aggregated manifest. Returns the Tauri-format JSON for the
 *     latest release across all platforms. Every install up to v1.2.0 has
 *     this URL baked into its binary and polls it on start-up and when the
 *     user presses Check for Updates, so this route stays for as long as
 *     any of those installs exist. It carries no platform or version.
 *
 *   GET /api/updates/{platform}/{current-version}
 *     Per-platform delta check. Returns 204 No Content when the requesting
 *     client is already on the latest version, or the Tauri-format JSON for
 *     the platform when an update is available. Reduces wire bytes for the
 *     common no-update case.
 *
 *     From v1.3.0 this is the route the app polls. tauri.conf.json sets the
 *     endpoint to
 *       /api/updates/{{target}}-{{arch}}-{{bundle_type}}/{{current_version}}
 *     and tauri-plugin-updater fills the variables in, so {platform} arrives
 *     as `{os}-{arch}-{bundle_type}`: darwin-aarch64-app, windows-x86_64-msi,
 *     windows-x86_64-nsis, linux-x86_64-appimage, linux-x86_64-deb, or
 *     `-unknown` for a binary that was never bundled. The bare keys
 *     (darwin-aarch64, linux-x86_64, windows-x86_64) and linux-x86_64-deb
 *     are still accepted as before. See resolvePlatform for the mapping.
 *
 *     The response is keyed by the manifest key the plugin searches for.
 *     It looks for `{os}-{arch}-{bundle_type}` first and `{os}-{arch}` second
 *     (tauri-plugin-updater 2.13.3, Updater::get_urls), so a .deb install
 *     asking as linux-x86_64-deb gets a block under that exact key, and an
 *     NSIS install asking as windows-x86_64-nsis gets the MSI under
 *     windows-x86_64, which its second lookup finds.
 *
 *     {current-version} is a semver string the client reports. Nothing else
 *     about the client is in the request: no identifier, no header the
 *     worker reads. The platform and version are counted by day (see
 *     countCheck) and that is the whole of what is kept.
 *
 * Backing store:
 *
 *   GitHub Releases on juralabs/jura-trace. The worker fetches the latest
 *   release tag + asset URLs + minisign signatures (.sig files) and
 *   re-formats them into the Tauri auto-updater schema.
 *
 *   This keeps GitHub as the single source of truth for binary distribution
 *   (no double-upload) while letting juralabs.org act as the published
 *   update endpoint. If GitHub Releases is unreachable for the worker
 *   itself, the desktop app falls back to its second configured endpoint
 *   (the GitHub Releases URL directly).
 *
 * Caching:
 *
 *   Latest-release lookup is cached at the edge for 5 minutes. The cache
 *   key includes the platform when serving the per-platform route. Cache
 *   is bypassed when the GitHub fetch fails so transient failures do not
 *   poison the cache.
 *
 * Public key for signature verification:
 *
 *   The minisign public key lives in src-tauri/tauri.conf.json
 *   (plugins.updater.pubkey). The worker does NOT need to verify
 *   signatures itself; it passes the .sig URLs through, and the desktop
 *   app verifies them against its bundled pubkey before applying any
 *   update.
 *
 * Deployment: see DEPLOYMENT.md in this directory.
 */

interface Env {
  /** GitHub PAT with read access to juralabs/jura-trace releases. */
  GITHUB_TOKEN: string;
  /** Optional override for the GitHub repo (defaults to juralabs/jura-trace). */
  GITHUB_REPO?: string;
  /**
   * Optional KV namespace for update-check counts (loop L4, approved
   * 2026-09-06).
   *
   * Optional on purpose. If the binding is absent the worker behaves exactly
   * as it did before: counting is skipped and nothing fails. The updater has
   * already failed 145 people once, and a counter is not worth a second
   * outage.
   */
  UPDATE_COUNTS?: KVNamespace;
}

// ── Update-check counting (loop L4) ────────────────────────────────────────
//
// Trace collects nothing from the device and this does not change that. What
// is counted here is a request the worker already receives, reduced to a
// number: how many update checks arrived, on what day, against which route.
//
// Explicitly NOT recorded: IP address, user agent, any header, any identifier
// of any kind. Two installs are indistinguishable, and one install checking
// twice is indistinguishable from two installs checking once. That is the
// intended precision. The number answers "is anyone running this" and nothing
// finer.
//
// Why the worker and not analytics: Cloudflare's free plan retains
// request-level analytics for one day, a limit hit by hand on 5 September
// while trying to answer exactly this question. A durable count has to be
// written by something that outlives the request.

/** UTC day key, so the boundary does not move with British summer time. */
function dayKey(now: Date): string {
  return now.toISOString().slice(0, 10);
}

/**
 * Record one update check.
 *
 * Never awaited by the request path and never allowed to throw: a failure to
 * count must not become a failure to update. Runs through ctx.waitUntil so it
 * settles after the response has gone.
 *
 * Read the counts back with:
 *   wrangler kv key list  --binding UPDATE_COUNTS
 *   wrangler kv key get   --binding UPDATE_COUNTS "checks:2026-09-06:latest"
 *
 * KV is eventually consistent and this is a read-modify-write, so simultaneous
 * checks can lose an increment. At a few hundred a day that is a rounding
 * error, and the figure is meant to be directional rather than forensic. If it
 * ever needs to be exact, move to Durable Objects or Analytics Engine; do not
 * add locking here.
 */
function countCheck(env: Env, ctx: ExecutionContext, bucket: string): void {
  const kv = env.UPDATE_COUNTS;
  if (!kv) return;

  ctx.waitUntil(
    (async () => {
      try {
        const key = `checks:${dayKey(new Date())}:${bucket}`;
        const current = await kv.get(key);
        const next = (current === null ? 0 : parseInt(current, 10) || 0) + 1;
        // 100-day retention: long enough to see a release land and a trend
        // form, short enough that the namespace does not grow without bound.
        await kv.put(key, String(next), { expirationTtl: 100 * 24 * 60 * 60 });
      } catch (err) {
        console.error(`update-check counting failed (non-fatal): ${err}`);
      }
    })(),
  );
}

interface GitHubAsset {
  name: string;
  browser_download_url: string;
  size: number;
}

interface GitHubRelease {
  tag_name: string;
  name: string;
  body: string;
  published_at: string;
  prerelease: boolean;
  draft: boolean;
  assets: GitHubAsset[];
}

interface TauriUpdatePlatform {
  signature: string;
  url: string;
}

interface TauriUpdateManifest {
  version: string;
  notes: string;
  pub_date: string;
  platforms: Record<string, TauriUpdatePlatform>;
}

const DEFAULT_REPO = "juralabs/jura-trace";
const CACHE_TTL_SECONDS = 300;
const USER_AGENT = "JuraTrace-Updater-Worker/1.0";

/**
 * Mapping from Tauri platform identifier to the asset-name suffixes the
 * Jura Trace release workflow uses.
 *
 * Tauri v2 with createUpdaterArtifacts: true (NOT "v1Compatible") produces:
 *   - macOS   : *_aarch64.app.tar.gz + *_aarch64.app.tar.gz.sig
 *   - Linux   : *_amd64.AppImage (friendly: JuraTrace-<ver>-Linux-x86_64.AppImage)
 *               + *_amd64.AppImage.sig (raw Tauri name, never renamed)
 *   - Windows : *_x64_en-US.msi (friendly: JuraTrace-<ver>-Windows-x64.msi)
 *               + *_x64_en-US.msi.sig (raw Tauri name, never renamed)
 *
 * NOTE: The v1Compatible format produced .AppImage.tar.gz and .nsis.zip
 * archives. v2 signs the installer directly — no separate archive.
 *
 * Because the release workflow renames the installer assets to friendly names
 * (JuraTrace-<ver>-...) but uploads .sig files raw (Tauri's own name), the
 * URL suffix and the sig suffix are different for Linux and Windows.
 * We carry both in a structured config rather than a plain string.
 */
interface PlatformAssetConfig {
  /** Suffix to find the downloadable installer on the release. */
  urlSuffix: string;
  /** Suffix to find the detached .sig file on the release (raw Tauri name). */
  sigSuffix: string;
}

const PLATFORM_ASSET_CONFIG: Record<string, PlatformAssetConfig> = {
  // macOS: Tauri names the updater archive "<productName>.app.tar.gz"
  // (e.g. "Jura Trace.app.tar.gz"), no version/arch in the name, uploaded raw.
  "darwin-aarch64": {
    urlSuffix: ".app.tar.gz",
    sigSuffix: ".app.tar.gz.sig",
  },
  // darwin-x86_64 omitted — Intel Mac users run the ARM build under Rosetta 2.
  // Linux: installer is friendly-renamed; .sig is uploaded raw.
  "linux-x86_64": {
    urlSuffix: ".AppImage",
    sigSuffix: "_amd64.AppImage.sig",
  },
  // Linux, installed from the .deb (BL-REL-008). tauri-plugin-updater looks
  // for "{os}-{arch}-{bundle_type}" before "{os}-{arch}", so a .deb install
  // takes this entry and an AppImage install never sees it. Without it a
  // .deb install fell back to linux-x86_64, downloaded the AppImage and
  // refused to install it. The plugin (2.10.0 in v1.1.0, and later) installs
  // a .deb with `pkexec dpkg -i`, so the user is asked for their password.
  // The .deb.sig is an updater signature like the others, uploaded raw.
  "linux-x86_64-deb": {
    urlSuffix: ".deb",
    sigSuffix: "_amd64.deb.sig",
  },
  // Windows: MSI is friendly-renamed; .sig is uploaded raw.
  // Must match the suffix release.yml's manifest job writes.
  "windows-x86_64": {
    urlSuffix: ".msi",
    sigSuffix: ".msi.sig",
  },
};

// Convenience lookup used by the per-platform route guard.
const PLATFORM_ASSET_SUFFIXES: Record<string, string> = Object.fromEntries(
  Object.entries(PLATFORM_ASSET_CONFIG).map(([k, v]) => [k, v.urlSuffix])
);

/**
 * Bundle types tauri-plugin-updater can put in `{{bundle_type}}` that do NOT
 * get an asset of their own, and so resolve to the bare `{os}-{arch}` entry.
 *
 *   app       macOS, from the .dmg or a bare .app. Always "app" on macOS.
 *   msi       Windows WiX installer. The entry already carries the MSI.
 *   nsis      Windows setup .exe. There is no NSIS updater asset, so an NSIS
 *             install is offered the MSI, exactly as it was from latest.json
 *             (the parallel-install consequence is BL-TEST-001, unchanged).
 *   appimage  Linux AppImage. The bare linux-x86_64 entry is the AppImage.
 *   unknown   The plugin's word for a binary that was never bundled, such
 *             as `cargo run`. Resolved rather than rejected so a dev build
 *             still gets a manifest, as it did from latest.json.
 *
 * `deb` is deliberately absent: linux-x86_64-deb is its own entry in
 * PLATFORM_ASSET_CONFIG and must stay so (BL-REL-008). `rpm` is absent
 * because no RPM is built; an rpm request gets 400, which the plugin treats
 * as "try the next endpoint".
 */
const BUNDLE_TYPES_WITHOUT_OWN_ASSET = ["app", "msi", "nsis", "appimage", "unknown"];

/**
 * Map the platform string a client asked for onto the PLATFORM_ASSET_CONFIG
 * key that serves it, or null when nothing does.
 *
 * Accepts the bare keys and linux-x86_64-deb as they are, and
 * `{os}-{arch}-{bundle_type}` for the bundle types listed above. Returns the
 * key the response block is published under, which is also the key the
 * plugin's fallback lookup finds.
 */
export function resolvePlatform(requested: string): string | null {
  if (requested in PLATFORM_ASSET_CONFIG) return requested;
  const cut = requested.lastIndexOf("-");
  if (cut <= 0) return null;
  const base = requested.slice(0, cut);
  const bundle = requested.slice(cut + 1);
  if (BUNDLE_TYPES_WITHOUT_OWN_ASSET.includes(bundle) && base in PLATFORM_ASSET_CONFIG) {
    return base;
  }
  return null;
}

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);

    // CORS preflight for the rare case an HTTP-only client probes the endpoint.
    if (request.method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: corsHeaders(),
      });
    }

    // HEAD requests get the same handling as GET; Cloudflare strips the
    // body automatically. Supports curl -I health-checks + load-balancer
    // liveness probes without 405-ing.
    if (request.method !== "GET" && request.method !== "HEAD") {
      return jsonError(405, "Method not allowed", "Only GET and HEAD are supported on the updater endpoint.");
    }

    const path = url.pathname;

    // Counting happens here, before dispatch, and deliberately not inside the
    // handlers. handleLatestManifest returns early from the edge cache, so a
    // counter placed inside it would miss every cache hit and report a number
    // silently lower than the truth — the same shape of quiet wrongness this
    // worker was fixed for in BL-REL-002.
    //
    // HEAD is counted alongside GET. A HEAD is still an install asking whether
    // there is an update.

    if (path === "/api/updates/latest.json") {
      // Every install up to v1.2.0 has this URL baked into its binary, so
      // this bucket is the closest thing to a count of those installs that
      // exists. From v1.3.0 the app polls the per-platform route below, so
      // this bucket shrinks as people update and never gains a new install.
      countCheck(env, ctx, "latest");
      return handleLatestManifest(env, ctx);
    }

    // One path segment for the platform, one for the version. The platform
    // segment is `{os}-{arch}` or `{os}-{arch}-{bundle_type}`; what the
    // worker accepts is decided by resolvePlatform, not by this pattern.
    const perPlatformMatch = path.match(/^\/api\/updates\/([a-z][a-z0-9_-]*)\/([^/]+)$/);
    if (perPlatformMatch) {
      const requested = perPlatformMatch[1];
      const currentVersion = perPlatformMatch[2];
      const platform = resolvePlatform(requested);
      if (platform === null) {
        // Not counted. Counting an unknown platform would let anyone grow
        // the namespace with made-up keys, and a request the worker cannot
        // answer is not an install asking for an update.
        return jsonError(
          400,
          "Unknown platform",
          `Platform "${requested}" is not recognised. Valid: ${Object.keys(PLATFORM_ASSET_SUFFIXES).join(", ")}, ` +
            `or one of those followed by -${BUNDLE_TYPES_WITHOUT_OWN_ASSET.join(", -")}.`,
        );
      }
      // Counted as the client asked, so the buckets keep the bundle type
      // (windows-x86_64-msi and windows-x86_64-nsis are separate), with the
      // version as a second key. The platform is taken from the path the
      // client asked for, not inferred from anything about the client. The
      // version is what the install says it is running, which is what makes
      // adoption measurable: installs still reporting 1.2.0 have not moved.
      countCheck(env, ctx, requested);
      countCheck(env, ctx, `${requested}:${currentVersion}`);
      return handlePerPlatform(env, ctx, platform, currentVersion);
    }

    return jsonError(404, "Not found", `No route matches ${path}.`);
  },
};

/**
 * Build and serve the aggregated latest.json manifest for the Tauri
 * auto-updater. Cached at the edge for CACHE_TTL_SECONDS.
 */
async function handleLatestManifest(env: Env, ctx: ExecutionContext): Promise<Response> {
  const cacheKey = new Request("https://juralabs.org/api/updates/latest.json", { method: "GET" });
  const cache = caches.default;

  const cached = await cache.match(cacheKey);
  if (cached) {
    return cached;
  }

  const release = await fetchLatestRelease(env);
  if (!release) {
    return jsonError(502, "Upstream unavailable", "Could not fetch the latest release from GitHub.");
  }

  let manifest: TauriUpdateManifest;
  try {
    manifest = await buildManifest(release);
  } catch (err) {
    // A manifest we cannot assemble correctly must not be served or cached.
    // An error here is visible; a silently incomplete manifest is not.
    console.error(`Manifest assembly failed for ${release.tag_name}: ${err}`);
    return jsonError(
      503,
      "Manifest unavailable",
      "The update manifest could not be assembled. This has been logged.",
    );
  }

  const response = jsonResponse(200, manifest, { "Cache-Control": `public, max-age=${CACHE_TTL_SECONDS}` });

  // Write to edge cache (fire-and-forget).
  ctx.waitUntil(cache.put(cacheKey, response.clone()));
  return response;
}

/**
 * Per-platform endpoint. Returns 204 when the client's current version
 * matches the latest release tag; otherwise returns the per-platform
 * fragment of the Tauri manifest.
 */
async function handlePerPlatform(
  env: Env,
  ctx: ExecutionContext,
  platform: string,
  currentVersion: string,
): Promise<Response> {
  if (!(platform in PLATFORM_ASSET_SUFFIXES)) {
    return jsonError(
      400,
      "Unknown platform",
      `Platform "${platform}" is not recognised. Valid: ${Object.keys(PLATFORM_ASSET_SUFFIXES).join(", ")}.`,
    );
  }

  const release = await fetchLatestRelease(env);
  if (!release) {
    return jsonError(502, "Upstream unavailable", "Could not fetch the latest release from GitHub.");
  }

  // Tauri version strings in the release tag may or may not have a "v" prefix;
  // normalise both sides.
  const latestVersion = release.tag_name.replace(/^v/, "");
  const clientVersion = currentVersion.replace(/^v/, "");
  if (latestVersion === clientVersion) {
    return new Response(null, {
      status: 204,
      headers: {
        ...corsHeaders(),
        "Cache-Control": `public, max-age=${CACHE_TTL_SECONDS}`,
      },
    });
  }

  const platformBlock = await buildPlatformBlock(release, platform);
  if (!platformBlock) {
    return jsonError(
      404,
      "Asset not found",
      `No build for platform "${platform}" found in release ${release.tag_name}.`,
    );
  }

  // Per-platform response is the SAME shape as the aggregated manifest
  // (Tauri accepts either). Single-platform map keeps wire bytes small.
  const manifest: TauriUpdateManifest = {
    version: latestVersion,
    notes: release.body || release.name || `Release ${release.tag_name}`,
    pub_date: release.published_at,
    platforms: { [platform]: platformBlock },
  };

  return jsonResponse(200, manifest, { "Cache-Control": `public, max-age=${CACHE_TTL_SECONDS}` });
}

/**
 * Fetch the latest non-draft, non-prerelease release from GitHub.
 * Returns null on transient failure (the caller surfaces a 502 to the
 * client, and the desktop app's second configured endpoint takes over).
 */
async function fetchLatestRelease(env: Env): Promise<GitHubRelease | null> {
  const repo = env.GITHUB_REPO || DEFAULT_REPO;
  const url = `https://api.github.com/repos/${repo}/releases/latest`;
  // The releases of a public repo can be read without a token, at GitHub's
  // unauthenticated rate limit. That is enough for `wrangler dev` against the
  // real release and means no token has to be written to disk to try a
  // worker change locally. Production sets GITHUB_TOKEN as a secret.
  const headers: Record<string, string> = {
    "User-Agent": USER_AGENT,
    Accept: "application/vnd.github+json",
    "X-GitHub-Api-Version": "2022-11-28",
  };
  if (env.GITHUB_TOKEN) {
    headers.Authorization = `Bearer ${env.GITHUB_TOKEN}`;
  }
  const resp = await fetch(url, { headers });
  if (!resp.ok) {
    console.error(`GitHub releases fetch failed: ${resp.status} ${await resp.text()}`);
    return null;
  }
  return (await resp.json()) as GitHubRelease;
}

/**
 * Build the full Tauri-format aggregated manifest from a GitHub release,
 * with each platform's minisign signature fetched and inlined.
 *
 * Platforms without a matching asset are omitted, so a release that only
 * built macOS does not appear in the Windows or Linux response. Every
 * omission is logged, and a manifest with no platforms at all throws
 * rather than being served: see the note on failing closed below.
 */
async function buildManifest(release: GitHubRelease): Promise<TauriUpdateManifest> {
  const names = Object.keys(PLATFORM_ASSET_CONFIG);

  // Fetch every platform's signature concurrently. Four small files.
  const blocks = await Promise.all(
    names.map((platform) => buildPlatformBlock(release, platform)),
  );

  const platforms: Record<string, TauriUpdatePlatform> = {};
  const dropped: string[] = [];

  names.forEach((platform, i) => {
    const block = blocks[i];
    if (block) {
      platforms[platform] = block;
    } else {
      dropped.push(platform);
    }
  });

  // Fail closed, not open. Dropping a platform silently is what stranded
  // Linux: the manifest looked healthy and told a whole platform it was up
  // to date, indefinitely, with no error anywhere. A platform legitimately
  // absent from a release (a macOS-only build, say) still leaves the others
  // serviceable, so this warns rather than throws — but it must be visible
  // in the logs, and an empty platform map is always a fault.
  if (dropped.length > 0) {
    console.warn(
      `Manifest for ${release.tag_name} omits ${dropped.join(", ")}: ` +
        `no matching installer asset, or its .sig could not be read. ` +
        `Clients on those platforms will see no update.`,
    );
  }
  if (Object.keys(platforms).length === 0) {
    throw new Error(
      `Refusing to serve an empty manifest for ${release.tag_name}. ` +
        `No platform yielded both an installer and a readable signature.`,
    );
  }

  return {
    version: release.tag_name.replace(/^v/, ""),
    notes: release.body || release.name || `Release ${release.tag_name}`,
    pub_date: release.published_at,
    platforms,
  };
}

/**
 * A minisign signature file is a two-line document: an "untrusted comment"
 * line, then a base64 payload. Tauri base64-encodes the whole file, so what
 * we inline is a single base64 blob.
 *
 * This exists because the field it guards held a URL in production from the
 * v1.0.0 release on 18 June 2026 until 4 September, which made every update
 * fail signature verification. Cheap to check, and it encodes the bug so it
 * cannot come back quietly.
 */
function looksLikeSignature(value: string): boolean {
  if (!value) return false;
  if (/^https?:\/\//i.test(value)) return false;
  if (/\s/.test(value)) return false;
  return /^[A-Za-z0-9+/=]+$/.test(value) && value.length > 64;
}

/**
 * Async wrapper around locatePlatformAssets that also fetches the
 * minisign signature contents inline (the .sig file is small, typically
 * <500 bytes). The Tauri auto-updater wants the signature inline in the
 * manifest, not just as a URL.
 */
async function buildPlatformBlock(release: GitHubRelease, platform: string): Promise<TauriUpdatePlatform | null> {
  const assets = locatePlatformAssets(release, platform);
  if (!assets) return null;

  const sigResp = await fetch(assets.signatureUrl, {
    headers: { "User-Agent": USER_AGENT },
  });
  if (!sigResp.ok) {
    console.error(
      `Signature fetch failed for ${platform}: ${sigResp.status} from ${assets.signatureUrl}`,
    );
    return null;
  }

  const signature = (await sigResp.text()).trim();
  if (!looksLikeSignature(signature)) {
    console.error(
      `Signature for ${platform} does not look like a minisign blob ` +
        `(${signature.length} chars, starts "${signature.slice(0, 24)}"). Refusing to serve it.`,
    );
    return null;
  }

  return { url: assets.installerUrl, signature };
}

/** Two URLs on a release: the installer, and its detached signature file. */
interface PlatformAssetUrls {
  installerUrl: string;
  signatureUrl: string;
}

/**
 * Find the installer and the .sig file for a platform on a GitHub release.
 *
 * This returns URLs only, and deliberately does NOT return a
 * TauriUpdatePlatform. It used to, under the name buildPlatformBlockSync,
 * with the .sig URL sitting in the `signature` field — which is a manifest
 * block that looks correct, type-checks, and is wrong. That shape is how
 * the URL reached production and stayed there from 18 June to 4 September
 * 2026, failing every update. Callers must fetch the signature contents;
 * buildPlatformBlock does.
 *
 * Uses PLATFORM_ASSET_CONFIG rather than a single suffix because the
 * release workflow uploads the installer under a friendly name but the
 * .sig under its original Tauri-produced name, so the two suffixes differ
 * for Linux and Windows.
 */
function locatePlatformAssets(release: GitHubRelease, platform: string): PlatformAssetUrls | null {
  const cfg = PLATFORM_ASSET_CONFIG[platform];
  if (!cfg) return null;
  const asset = release.assets.find((a) => a.name.toLowerCase().endsWith(cfg.urlSuffix.toLowerCase()));
  if (!asset) return null;
  // Locate the .sig by its own suffix — it may have a different base name than
  // the installer asset when the installer was friendly-renamed on upload.
  const sig = release.assets.find((a) => a.name.toLowerCase().endsWith(cfg.sigSuffix.toLowerCase()));
  if (!sig) {
    console.warn(`No .sig found for ${platform} (expected suffix: ${cfg.sigSuffix})`);
    return null;
  }
  return { installerUrl: asset.browser_download_url, signatureUrl: sig.browser_download_url };
}

function jsonResponse(status: number, body: unknown, extraHeaders: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "Content-Type": "application/json; charset=utf-8",
      ...corsHeaders(),
      ...extraHeaders,
    },
  });
}

function jsonError(status: number, title: string, detail: string): Response {
  return jsonResponse(status, { error: title, detail });
}

function corsHeaders(): Record<string, string> {
  return {
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Methods": "GET, HEAD, OPTIONS",
    "Access-Control-Max-Age": "86400",
  };
}
