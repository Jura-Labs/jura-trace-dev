/**
 * Convert a base64-encoded string to a blob: URL suitable for CSP-safe
 * image rendering. Callers must revoke the returned URL when it is no
 * longer needed (e.g. on component destroy) to avoid memory leaks.
 */
export function base64ToBlobUrl(base64: string, mime: string): string {
  const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
  const blob = new Blob([bytes], { type: mime });
  return URL.createObjectURL(blob);
}

/**
 * Tracks blob URLs created during a component's lifetime and revokes
 * them all when `revokeAll()` is called. Usage:
 *
 *   const blobs = createBlobTracker();
 *   // In template: src={blobs.url(base64, 'image/png')}
 *   // On destroy:  blobs.revokeAll();
 */
export function createBlobTracker() {
  const cache = new Map<string, string>();

  return {
    /** Return a blob: URL for the given base64 data, caching by identity. */
    url(base64: string, mime: string): string {
      const key = base64.slice(0, 64) + base64.length;
      let existing = cache.get(key);
      if (existing) return existing;
      const blobUrl = base64ToBlobUrl(base64, mime);
      cache.set(key, blobUrl);
      return blobUrl;
    },

    /** Revoke all tracked blob URLs to free memory. */
    revokeAll() {
      for (const blobUrl of cache.values()) {
        URL.revokeObjectURL(blobUrl);
      }
      cache.clear();
    }
  };
}
