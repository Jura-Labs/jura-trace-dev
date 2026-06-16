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
 *     latest release across all platforms. Polled by the desktop app on
 *     start-up and at the user-configured interval.
 *
 *   GET /api/updates/{platform}/{current-version}
 *     Per-platform delta check. Returns 204 No Content when the requesting
 *     client is already on the latest version, or the full Tauri-format
 *     JSON for the platform when an update is available. Reduces wire
 *     bytes for the common no-update case.
 *
 *     {platform} is one of: darwin-aarch64, darwin-x86_64, linux-x86_64,
 *                          windows-x86_64
 *     {current-version} is a semver string the client reports
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

    if (path === "/api/updates/latest.json") {
      return handleLatestManifest(env, ctx);
    }

    const perPlatformMatch = path.match(/^\/api\/updates\/([a-z0-9-]+_[a-z0-9-]+|[a-z]+-[a-z0-9_]+)\/(.+)$/);
    if (perPlatformMatch) {
      const platform = perPlatformMatch[1];
      const currentVersion = perPlatformMatch[2];
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

  const manifest = buildManifest(release);
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
  const resp = await fetch(url, {
    headers: {
      "User-Agent": USER_AGENT,
      Authorization: `Bearer ${env.GITHUB_TOKEN}`,
      Accept: "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
    },
  });
  if (!resp.ok) {
    console.error(`GitHub releases fetch failed: ${resp.status} ${await resp.text()}`);
    return null;
  }
  return (await resp.json()) as GitHubRelease;
}

/**
 * Build the full Tauri-format aggregated manifest from a GitHub release.
 * Platforms without a matching asset are omitted (so a release that
 * only built macOS does not appear in the Windows or Linux response).
 */
function buildManifest(release: GitHubRelease): TauriUpdateManifest {
  const platforms: Record<string, TauriUpdatePlatform> = {};
  for (const platform of Object.keys(PLATFORM_ASSET_SUFFIXES)) {
    const block = buildPlatformBlockSync(release, platform);
    if (block) {
      platforms[platform] = block;
    }
  }
  return {
    version: release.tag_name.replace(/^v/, ""),
    notes: release.body || release.name || `Release ${release.tag_name}`,
    pub_date: release.published_at,
    platforms,
  };
}

/**
 * Async wrapper around buildPlatformBlockSync that also fetches the
 * minisign signature contents inline (the .sig file is small, typically
 * <500 bytes). The Tauri auto-updater wants the signature inline in the
 * manifest, not just as a URL.
 */
async function buildPlatformBlock(release: GitHubRelease, platform: string): Promise<TauriUpdatePlatform | null> {
  const sync = buildPlatformBlockSync(release, platform);
  if (!sync) return null;

  // Replace the signature URL with the actual signature contents.
  const sigResp = await fetch(sync.signature, {
    headers: { "User-Agent": USER_AGENT },
  });
  if (!sigResp.ok) {
    console.error(`Signature fetch failed for ${platform}: ${sigResp.status}`);
    return null;
  }
  const sigText = (await sigResp.text()).trim();
  return { url: sync.url, signature: sigText };
}

/**
 * Synchronous variant returns the .sig URL rather than the contents,
 * used by buildManifest when assembling the aggregated response. The
 * caller is responsible for fetching the actual signature contents.
 *
 * For the aggregated /latest.json endpoint we accept that signatures
 * are passed as URLs rather than inline contents; Tauri's updater
 * supports both forms.
 *
 * Uses PLATFORM_ASSET_CONFIG rather than a single suffix because the
 * release workflow uploads the installer under a friendly name but the
 * .sig under its original Tauri-produced name — so the URL suffix and
 * the sig suffix differ for Linux and Windows.
 */
function buildPlatformBlockSync(release: GitHubRelease, platform: string): TauriUpdatePlatform | null {
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
  return { url: asset.browser_download_url, signature: sig.browser_download_url };
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
