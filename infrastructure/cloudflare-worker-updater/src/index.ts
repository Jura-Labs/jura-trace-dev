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
 *     {platform} is one of: darwin-aarch64, linux-x86_64, linux-x86_64-deb,
 *                          windows-x86_64
 *     {current-version} is a semver string the client reports
 *
 *   GET|HEAD /dl/{version}/{slug}
 *     Counted download link. Answers 302 to the matching installer on
 *     GitHub Releases, so the website's download buttons can point here
 *     instead of straight at github.com and the click gets counted on the
 *     way through. {version} is a strict semver string (1.2.0, 1.3.0-rc.1)
 *     and {slug} is one of the fixed names in DOWNLOAD_SLUGS; anything else
 *     is a 404. The worker does not check that the asset exists.
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

// ── Request counting (loop L4, extended to downloads W43-13) ───────────────
//
// Trace collects nothing from the device and this does not change that. What
// is counted here is a request the worker already receives, reduced to a
// number: how many update checks arrived, on what day, against which route;
// and, since October 2026, how many download links were followed, on what
// day, for which installer.
//
// Explicitly NOT recorded: IP address, user agent, any header, any identifier
// of any kind. Two installs are indistinguishable, and one install checking
// twice is indistinguishable from two installs checking once. That is the
// intended precision. The number answers "is anyone running this" and nothing
// finer. The user agent and the network ASN are looked at, once, to decide
// which bucket the count goes in (see isAutomation), and then dropped.
//
// Why the worker and not analytics: Cloudflare's free plan retains
// request-level analytics for one day, a limit hit by hand on 5 September
// while trying to answer exactly this question. A durable count has to be
// written by something that outlives the request.
//
// Why the worker and not GitHub's download counts: those are cumulative from
// the day of release and every CI run that fetches an installer adds to them,
// so a release week's tests cannot be separated out afterwards (about a fifth
// of the 473 counted by 23 October 2026 were our own). A per-day count with
// automation in its own bucket is what the GitHub figure cannot give.

/** UTC day key, so the boundary does not move with British summer time. */
function dayKey(now: Date): string {
  return now.toISOString().slice(0, 10);
}

/** What is being counted. The KV key starts with this, "-ci" appended for automation. */
type CountKind = "checks" | "downloads";

/**
 * Decide whether a request comes from automation rather than a person or an
 * installed copy of Trace. Automation is counted under "checks-ci:" and
 * "downloads-ci:" instead of "checks:" and "downloads:", so our own release
 * workflows, live legs and hand checks stop inflating the figures that are
 * meant to say whether anyone outside is using this.
 *
 * This is a heuristic, not a security control. It will misfile a developer
 * who curls a download link from home, and it will miss a script with a
 * browser user agent on a residential connection. Both are rare enough, and
 * the cost of either is one count in the wrong bucket. Nothing it reads is
 * stored. Any of the three rules is enough:
 *
 *   1. The X-Jura-Test header is present. The explicit opt-in: a workflow
 *      that touches the live worker sends it and is filed as a test whatever
 *      its network or client looks like. Value ignored, presence is enough.
 *
 *   2. The request arrives from AS8075 (Microsoft), which is where
 *      GitHub-hosted Actions runners live. Catches every runner request
 *      regardless of client, including the Windows live leg's precheck, which
 *      uses a custom user agent. Self-hosted runners are not on AS8075 and
 *      rely on the other two rules. request.cf is absent in local dev and in
 *      unit tests, hence the optional chain.
 *
 *   3. The user agent names a command-line or library HTTP client: the GitHub
 *      CLI ("GitHub CLI 2.x", or "gh/" in older builds), curl, Wget,
 *      python-requests, Go-http-client. These are what release.yml's smoke
 *      step, the installer test and a person checking a link by hand use.
 *      Browsers never send any of them.
 *
 * Deliberately NOT caught: the Tauri updater. tauri-plugin-updater builds its
 * reqwest client with user_agent("tauri-plugin-updater/<crate version>")
 * (src/updater.rs, const UPDATER_USER_AGENT, same in 2.10.0 which v1.0.0 and
 * v1.1.0 ship and 2.13.3 which v1.2.0 ships). None of the patterns above
 * match it, and the test suite pins that, because an updater request counted
 * as CI would silently empty the one field signal this worker exists to give.
 */
export function isAutomation(request: Request): boolean {
  if (request.headers.has("X-Jura-Test")) return true;
  if (request.cf?.asn === 8075) return true;
  return AUTOMATION_USER_AGENTS.test(request.headers.get("User-Agent") ?? "");
}

// "gh/" needs the word boundary so it cannot match the tail of some other
// product token ending in "gh"; "tauri-plugin-updater/" ends in "r/" and is
// safe either way, but the boundary makes the rule say what it means.
const AUTOMATION_USER_AGENTS = /GitHub CLI|\bgh\/|curl\/|Wget|python-requests|Go-http-client/i;

/**
 * Record one request against a daily bucket.
 *
 * Never awaited by the request path and never allowed to throw: a failure to
 * count must not become a failure to update or to download. Runs through
 * ctx.waitUntil so it settles after the response has gone.
 *
 * Keys are <kind>[-ci]:<YYYY-MM-DD>:<bucket>, for example
 *   checks:2026-09-06:latest
 *   checks:2026-09-06:linux-x86_64:1.1.0
 *   checks-ci:2026-10-20:latest
 *   downloads:2026-10-23:macos:1.2.0
 *   downloads-ci:2026-10-23:linux-server:1.2.0
 *
 * Read the counts back with:
 *   wrangler kv key list  --binding UPDATE_COUNTS --remote --prefix downloads:
 *   wrangler kv key get   --binding UPDATE_COUNTS --remote "checks:2026-09-06:latest"
 *
 * KV is eventually consistent and this is a read-modify-write, so simultaneous
 * requests can lose an increment. At a few hundred a day that is a rounding
 * error, and the figure is meant to be directional rather than forensic. If it
 * ever needs to be exact, move to Durable Objects or Analytics Engine; do not
 * add locking here.
 */
function count(env: Env, ctx: ExecutionContext, kind: CountKind, automation: boolean, bucket: string): void {
  const kv = env.UPDATE_COUNTS;
  if (!kv) return;

  ctx.waitUntil(
    (async () => {
      try {
        const key = `${kind}${automation ? "-ci" : ""}:${dayKey(new Date())}:${bucket}`;
        const current = await kv.get(key);
        const next = (current === null ? 0 : parseInt(current, 10) || 0) + 1;
        // 100-day retention: long enough to see a release land and a trend
        // form, short enough that the namespace does not grow without bound.
        await kv.put(key, String(next), { expirationTtl: 100 * 24 * 60 * 60 });
      } catch (err) {
        console.error(`${kind} counting failed (non-fatal): ${err}`);
      }
    })(),
  );
}

// ── Counted download links (/dl) ───────────────────────────────────────────
//
// juralabs.org/download used to link straight at github.com, so the website
// never saw the click and the only figure was GitHub's cumulative one. These
// routes put the worker in the middle for exactly one redirect. No asset
// lookup, no GitHub API call, no cache: a 302 built from two validated path
// segments and a fixed table, with Cache-Control: no-store so every click
// reaches the worker rather than a browser or edge cache.

/**
 * Where the redirect points. Fixed on purpose: env.GITHUB_REPO exists so the
 * API fetch can be tested against a fork, and a download link must never
 * follow it anywhere else.
 */
const DOWNLOAD_BASE = "https://github.com/Jura-Labs/jura-trace/releases/download";

/**
 * The only versions a download link accepts: MAJOR.MINOR.PATCH, with an
 * optional -rc.N. Anchored, digits only, no leading "v" (the redirect adds
 * it). Anything looser is an open redirect waiting to happen, since the
 * value goes into a URL.
 */
const DOWNLOAD_VERSION = /^\d+\.\d+\.\d+(?:-rc\.\d+)?$/;

/**
 * Slug to release asset name, with "<v>" standing for the version. These
 * are the friendly names release.yml uploads, checked against the v1.2.0
 * release on 23 October 2026. A slug missing here is a 404, never a
 * passthrough; the path is not a file name and is not treated as one.
 */
export const DOWNLOAD_SLUGS: Readonly<Record<string, string>> = {
  macos: "JuraTrace-<v>-macOS-AppleSilicon.dmg",
  "windows-exe": "JuraTrace-<v>-Windows-x64-setup.exe",
  "windows-msi": "JuraTrace-<v>-Windows-x64.msi",
  "linux-appimage": "JuraTrace-<v>-Linux-x86_64.AppImage",
  "linux-deb": "JuraTrace-<v>-Linux-x86_64.deb",
  "linux-server": "JuraTrace-<v>-Linux-x86_64-server.tar.gz",
  sha256sums: "SHA256SUMS.txt",
};

/**
 * Resolve a /dl/{version}/{slug} pair to the GitHub Releases URL it stands
 * for, or null when either part fails validation. Pure, so the tests can
 * exercise every slug without a request.
 */
export function downloadUrl(version: string, slug: string): string | null {
  if (!DOWNLOAD_VERSION.test(version)) return null;
  const template = Object.prototype.hasOwnProperty.call(DOWNLOAD_SLUGS, slug) ? DOWNLOAD_SLUGS[slug] : undefined;
  if (!template) return null;
  return `${DOWNLOAD_BASE}/v${version}/${template.replace("<v>", version)}`;
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

    // Decided once per request and threaded into every count below, so the
    // update-check and download buckets split the same way.
    const automation = isAutomation(request);

    // Counting happens here, before dispatch, and deliberately not inside the
    // handlers. handleLatestManifest returns early from the edge cache, so a
    // counter placed inside it would miss every cache hit and report a number
    // silently lower than the truth — the same shape of quiet wrongness this
    // worker was fixed for in BL-REL-002.
    //
    // HEAD is counted alongside GET for update checks. A HEAD is still an
    // install asking whether there is an update. It is NOT counted for
    // downloads, where a HEAD is a link checker or a browser preflight and
    // a GET is the click.

    // /dl/{version}/{slug}: validated and counted here, redirect built here.
    // Only a request that resolves is counted, so a scanner walking /dl/ for
    // paths that do not exist leaves nothing in KV.
    const downloadMatch = path.match(/^\/dl\/([^/]+)\/([^/]+)$/);
    if (downloadMatch) {
      const [, version, slug] = downloadMatch;
      const target = downloadUrl(version, slug);
      if (!target) {
        return jsonError(
          404,
          "Not found",
          `No download matches ${path}. Expected /dl/<version>/<slug> with slug one of: ${Object.keys(DOWNLOAD_SLUGS).join(", ")}.`,
        );
      }
      if (request.method === "GET") {
        count(env, ctx, "downloads", automation, `${slug}:${version}`);
      }
      // Built by hand rather than Response.redirect(), whose headers are
      // immutable and would not take the no-store.
      return new Response(null, {
        status: 302,
        headers: {
          Location: target,
          "Cache-Control": "no-store",
        },
      });
    }

    if (path === "/api/updates/latest.json") {
      // Every v1.0.0 install has this URL baked into its binary, so this
      // bucket is the closest thing to a count of the original 145 that
      // exists. New installs will appear here too until the endpoint format
      // changes, which is a separate decision.
      count(env, ctx, "checks", automation, "latest");
      return handleLatestManifest(env, ctx);
    }

    const perPlatformMatch = path.match(/^\/api\/updates\/([a-z0-9-]+_[a-z0-9-]+|[a-z]+-[a-z0-9_]+)\/(.+)$/);
    if (perPlatformMatch) {
      const platform = perPlatformMatch[1];
      const currentVersion = perPlatformMatch[2];
      // Platform is taken from the path the client asked for, not inferred
      // from anything about the client. The version is what the install says
      // it is running, which is what makes the re-download notice measurable:
      // installs still reporting 1.0.0 have not taken it.
      count(env, ctx, "checks", automation, platform);
      count(env, ctx, "checks", automation, `${platform}:${currentVersion}`);
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
