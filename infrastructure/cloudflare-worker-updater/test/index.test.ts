/**
 * Unit tests for the updater worker's routing and counting.
 *
 * Plain vitest on Node, no workerd. Request, Response and URL are the same
 * WHATWG classes in both runtimes. The Worker-only pieces are faked: a Map
 * stands in for the KV namespace, an object that collects promises stands
 * in for ExecutionContext, and the edge cache and global fetch are stubbed
 * so nothing here touches GitHub or the network.
 *
 * What is covered: the /dl redirect for every slug, the 404s for anything
 * that is not a slug or a version, the GET-counted / HEAD-not rule, the
 * automation classifier, and that the existing update-check counting still
 * writes the same keys it always did. Manifest assembly is not covered: it
 * needs real release assets and is exercised by the live legs in CI.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import worker, { DOWNLOAD_SLUGS, downloadUrl, isAutomation } from "../src/index";

// ── Fakes ──────────────────────────────────────────────────────────────────

/** Just enough of KVNamespace for the counter: get and put, in memory. */
class FakeKV {
  readonly store = new Map<string, string>();
  async get(key: string): Promise<string | null> {
    return this.store.get(key) ?? null;
  }
  async put(key: string, value: string, _opts?: unknown): Promise<void> {
    this.store.set(key, value);
  }
  asNamespace(): KVNamespace {
    return this as unknown as KVNamespace;
  }
}

/** Collects waitUntil promises so a test can wait for the counter to settle. */
function fakeContext(): { ctx: ExecutionContext; settle: () => Promise<unknown> } {
  const pending: Promise<unknown>[] = [];
  const ctx = {
    waitUntil(p: Promise<unknown>) {
      pending.push(p);
    },
    passThroughOnException() {},
    props: {},
  } as unknown as ExecutionContext;
  return { ctx, settle: () => Promise.all(pending) };
}

type Env = Parameters<typeof worker.fetch>[1];

function makeEnv(kv?: FakeKV): Env {
  return { GITHUB_TOKEN: "test-token", UPDATE_COUNTS: kv?.asNamespace() } as Env;
}

/**
 * A request as the worker would see it. request.cf is read-only on a real
 * Request and absent in Node, so it is attached with defineProperty, which
 * is also how the Workers runtime exposes it (an own property, not a header).
 */
function makeRequest(
  path: string,
  opts: { method?: string; headers?: Record<string, string>; asn?: number } = {},
): Request {
  const req = new Request(`https://juralabs.org${path}`, {
    method: opts.method ?? "GET",
    headers: opts.headers ?? {},
  });
  if (opts.asn !== undefined) {
    Object.defineProperty(req, "cf", { value: { asn: opts.asn } });
  }
  return req as unknown as Request;
}

async function run(path: string, opts: Parameters<typeof makeRequest>[1] = {}, kv?: FakeKV) {
  const { ctx, settle } = fakeContext();
  const response = await worker.fetch(makeRequest(path, opts), makeEnv(kv), ctx);
  await settle();
  return response;
}

// Pin the day so the expected key is a constant rather than a race with
// midnight UTC. setSystemTime without fake timers mocks Date only, which is
// all the counter uses.
const TODAY = "2026-10-23";
beforeEach(() => {
  vi.setSystemTime(new Date(`${TODAY}T12:00:00Z`));
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

// ── /dl redirect ───────────────────────────────────────────────────────────

describe("downloadUrl", () => {
  it("maps every slug to its friendly asset name under the v-prefixed tag", () => {
    const expected: Record<string, string> = {
      macos: "JuraTrace-1.2.0-macOS-AppleSilicon.dmg",
      "windows-exe": "JuraTrace-1.2.0-Windows-x64-setup.exe",
      "windows-msi": "JuraTrace-1.2.0-Windows-x64.msi",
      "linux-appimage": "JuraTrace-1.2.0-Linux-x86_64.AppImage",
      "linux-deb": "JuraTrace-1.2.0-Linux-x86_64.deb",
      "linux-server": "JuraTrace-1.2.0-Linux-x86_64-server.tar.gz",
      sha256sums: "SHA256SUMS.txt",
    };
    // The table in the test and the table in the worker must agree on the
    // set of slugs, or a slug could be added without a redirect test.
    expect(Object.keys(expected).sort()).toEqual(Object.keys(DOWNLOAD_SLUGS).sort());
    for (const [slug, file] of Object.entries(expected)) {
      expect(downloadUrl("1.2.0", slug)).toBe(
        `https://github.com/Jura-Labs/jura-trace/releases/download/v1.2.0/${file}`,
      );
    }
  });

  it("accepts a release-candidate version", () => {
    expect(downloadUrl("1.3.0-rc.2", "macos")).toBe(
      "https://github.com/Jura-Labs/jura-trace/releases/download/v1.3.0-rc.2/JuraTrace-1.3.0-rc.2-macOS-AppleSilicon.dmg",
    );
  });

  it.each([
    ["v1.2.0", "leading v"],
    ["1.2", "two components"],
    ["1.2.0.1", "four components"],
    ["1.2.0-beta.1", "non-rc pre-release"],
    ["1.2.0-rc", "rc without a number"],
    ["latest", "a word"],
    ["1.2.0/", "trailing slash"],
    ["1.2.0%0a", "encoded control character"],
    ["..", "dot segment"],
    ["", "empty"],
  ])("rejects version %j (%s)", (version) => {
    expect(downloadUrl(version, "macos")).toBeNull();
  });

  it.each([
    "MACOS",
    "macos.dmg",
    "JuraTrace-1.2.0-macOS-AppleSilicon.dmg",
    "latest.json",
    "..",
    "constructor",
    "__proto__",
    "toString",
    "",
  ])("rejects slug %j", (slug) => {
    expect(downloadUrl("1.2.0", slug)).toBeNull();
  });
});

describe("GET /dl/{version}/{slug}", () => {
  it.each(Object.keys(DOWNLOAD_SLUGS))("302s %s to GitHub with no-store", async (slug) => {
    const res = await run(`/dl/1.2.0/${slug}`);
    expect(res.status).toBe(302);
    expect(res.headers.get("Location")).toBe(downloadUrl("1.2.0", slug));
    expect(res.headers.get("Cache-Control")).toBe("no-store");
  });

  it("ignores a query string", async () => {
    const res = await run("/dl/1.2.0/macos?utm_source=x&redirect=https://evil.example");
    expect(res.status).toBe(302);
    expect(res.headers.get("Location")).toBe(downloadUrl("1.2.0", "macos"));
  });

  it("404s a bad version in the worker's JSON error shape", async () => {
    const res = await run("/dl/v1.2.0/macos");
    expect(res.status).toBe(404);
    expect(res.headers.get("Content-Type")).toMatch(/application\/json/);
    const body = (await res.json()) as { error: string; detail: string };
    expect(body.error).toBe("Not found");
    expect(body.detail).toContain("/dl/v1.2.0/macos");
  });

  it("404s an unknown slug", async () => {
    const res = await run("/dl/1.2.0/windows");
    expect(res.status).toBe(404);
    const body = (await res.json()) as { error: string };
    expect(body.error).toBe("Not found");
  });

  it.each([
    // new URL() collapses a ".." segment, plain or as %2e%2e, before the
    // worker sees the path: these arrive as /x and /dl/macos and miss the
    // /dl route entirely.
    "/dl/1.2.0/../../x",
    "/dl/1.2.0/%2e%2e/macos",
    // An encoded slash is not a separator to the URL parser, so these
    // survive as single segments and must fail the version or slug check.
    "/dl/1.2.0%2f..%2f../macos",
    "/dl/1.2.0/macos%2f..%2fetc",
    "/dl/..%2f1.2.0/macos",
    // Extra or missing segments.
    "/dl/1.2.0/macos/",
    "/dl/1.2.0/macos/extra",
    "/dl/1.2.0",
    "/dl/1.2.0/",
    "/dl/",
    "/dl",
    "/dl//macos",
  ])("404s %s", async (path) => {
    const res = await run(path);
    expect(res.status).toBe(404);
  });

  it("never redirects anywhere but the release repo", async () => {
    for (const path of [
      "/dl/1.2.0/macos",
      "/dl/1.2.0/sha256sums",
      "/dl/0.9.0-rc.25/linux-deb",
    ]) {
      const res = await run(path);
      expect(res.headers.get("Location")).toMatch(
        /^https:\/\/github\.com\/Jura-Labs\/jura-trace\/releases\/download\/v/,
      );
    }
  });
});

// ── Counting ───────────────────────────────────────────────────────────────

describe("download counting", () => {
  it("counts a GET under downloads:<day>:<slug>:<version>", async () => {
    const kv = new FakeKV();
    await run("/dl/1.2.0/macos", {}, kv);
    await run("/dl/1.2.0/macos", {}, kv);
    await run("/dl/1.2.0/linux-deb", {}, kv);
    expect(Object.fromEntries(kv.store)).toEqual({
      [`downloads:${TODAY}:macos:1.2.0`]: "2",
      [`downloads:${TODAY}:linux-deb:1.2.0`]: "1",
    });
  });

  it("does not count a HEAD, but still redirects it", async () => {
    const kv = new FakeKV();
    const res = await run("/dl/1.2.0/macos", { method: "HEAD" }, kv);
    expect(res.status).toBe(302);
    expect(res.headers.get("Location")).toBe(downloadUrl("1.2.0", "macos"));
    expect(kv.store.size).toBe(0);
  });

  it("does not count a 404", async () => {
    const kv = new FakeKV();
    await run("/dl/1.2.0/nope", {}, kv);
    await run("/dl/nope/macos", {}, kv);
    expect(kv.store.size).toBe(0);
  });

  it("files automation under downloads-ci:", async () => {
    const kv = new FakeKV();
    await run("/dl/1.2.0/macos", { headers: { "User-Agent": "curl/8.7.1" } }, kv);
    await run("/dl/1.2.0/macos", { asn: 8075 }, kv);
    await run("/dl/1.2.0/macos", { headers: { "X-Jura-Test": "1" } }, kv);
    expect(Object.fromEntries(kv.store)).toEqual({
      [`downloads-ci:${TODAY}:macos:1.2.0`]: "3",
    });
  });

  it("still redirects when there is no KV binding", async () => {
    const res = await run("/dl/1.2.0/macos");
    expect(res.status).toBe(302);
  });

  it("still redirects when KV throws", async () => {
    const kv = new FakeKV();
    kv.get = async () => {
      throw new Error("KV is down");
    };
    const errors = vi.spyOn(console, "error").mockImplementation(() => {});
    const res = await run("/dl/1.2.0/macos", {}, kv);
    expect(res.status).toBe(302);
    expect(errors).toHaveBeenCalledOnce();
    errors.mockRestore();
  });
});

describe("update-check counting (unchanged keys)", () => {
  beforeEach(() => {
    // The aggregated route returns from the edge cache when it can, which is
    // the path a counter inside the handler would have missed. Serve a hit
    // so the test proves counting happens before the cache is consulted.
    vi.stubGlobal("caches", {
      default: {
        match: async () => new Response("{}", { headers: { "Content-Type": "application/json" } }),
        put: async () => {},
      },
    });
    // The per-platform route has no cache and goes to GitHub. Fail that
    // fetch: the worker answers 502 and the count must have happened anyway.
    vi.stubGlobal("fetch", async () => new Response("unavailable", { status: 500 }));
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  it("counts latest.json under checks:<day>:latest, GET and HEAD alike", async () => {
    const kv = new FakeKV();
    const ua = { "User-Agent": "tauri-plugin-updater/2.13.3" };
    await run("/api/updates/latest.json", { headers: ua }, kv);
    await run("/api/updates/latest.json", { headers: ua, method: "HEAD" }, kv);
    expect(Object.fromEntries(kv.store)).toEqual({ [`checks:${TODAY}:latest`]: "2" });
  });

  it("counts the per-platform route under both its platform and platform:version keys", async () => {
    const kv = new FakeKV();
    const res = await run("/api/updates/linux-x86_64-deb/1.1.0", {}, kv);
    expect(res.status).toBe(502);
    expect(Object.fromEntries(kv.store)).toEqual({
      [`checks:${TODAY}:linux-x86_64-deb`]: "1",
      [`checks:${TODAY}:linux-x86_64-deb:1.1.0`]: "1",
    });
  });

  it("files automation under checks-ci:", async () => {
    const kv = new FakeKV();
    await run("/api/updates/latest.json", { headers: { "User-Agent": "curl/8.7.1" } }, kv);
    await run("/api/updates/latest.json", { asn: 8075 }, kv);
    expect(Object.fromEntries(kv.store)).toEqual({ [`checks-ci:${TODAY}:latest`]: "2" });
  });

  it("does not count unmatched routes", async () => {
    const kv = new FakeKV();
    const res = await run("/api/updates/", {}, kv);
    expect(res.status).toBe(404);
    expect(kv.store.size).toBe(0);
  });
});

// ── Classifier ─────────────────────────────────────────────────────────────

describe("isAutomation", () => {
  const ua = (value: string) => makeRequest("/dl/1.2.0/macos", { headers: { "User-Agent": value } });

  it.each([
    // The Tauri updater, every version Trace has shipped. Must stay false.
    "tauri-plugin-updater/2.10.0",
    "tauri-plugin-updater/2.13.3",
    // Browsers.
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64; rv:131.0) Gecko/20100101 Firefox/131.0",
    "Mozilla/5.0 (Linux; Android 14; SM-A546B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Mobile Safari/537.36",
    // A product token that merely ends in "gh" is not the GitHub CLI.
    "Hugh/1.0",
    // The worker's own outbound agent, should it ever loop back.
    "JuraTrace-Updater-Worker/1.0",
    "",
  ])("is false for user agent %j", (value) => {
    expect(isAutomation(ua(value))).toBe(false);
  });

  it.each([
    "GitHub CLI 2.58.0",
    "gh/2.40.1",
    "curl/8.7.1",
    "Wget/1.21.4",
    "python-requests/2.32.3",
    "Go-http-client/1.1",
    "Go-http-client/2.0",
  ])("is true for user agent %j", (value) => {
    expect(isAutomation(ua(value))).toBe(true);
  });

  it("is true for a request from AS8075 whatever the user agent", () => {
    const req = makeRequest("/dl/1.2.0/macos", {
      headers: { "User-Agent": "jura-trace-live-leg-precheck" },
      asn: 8075,
    });
    expect(isAutomation(req)).toBe(true);
  });

  it("is false for the same custom user agent from another network", () => {
    const req = makeRequest("/dl/1.2.0/macos", {
      headers: { "User-Agent": "jura-trace-live-leg-precheck" },
      asn: 13335,
    });
    expect(isAutomation(req)).toBe(false);
  });

  it("is true when X-Jura-Test is present, whatever its value", () => {
    expect(isAutomation(makeRequest("/dl/1.2.0/macos", { headers: { "X-Jura-Test": "" } }))).toBe(true);
    expect(isAutomation(makeRequest("/dl/1.2.0/macos", { headers: { "x-jura-test": "release" } }))).toBe(true);
  });

  it("is false with no header, no cf and a missing user agent", () => {
    expect(isAutomation(makeRequest("/dl/1.2.0/macos"))).toBe(false);
  });
});
