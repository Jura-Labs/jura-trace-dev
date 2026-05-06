// SPDX-License-Identifier: AGPL-3.0-or-later

const isTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/**
 * Trigger a file download for the given blob. In Tauri, opens the native
 * save dialog and writes via `@tauri-apps/plugin-fs`. In a regular
 * browser, falls back to the anchor-click pattern with proper DOM
 * attachment and a deferred URL.revokeObjectURL.
 *
 * The Tauri webview blocks the bare `link.click()` pattern that worked
 * in Chrome, which is why this helper exists. See JTV-129 — the Protect
 * page's "Export Asset Database" button silently did nothing because it
 * used the bare pattern.
 */
export async function triggerDownload(
  blob: Blob,
  filename: string,
): Promise<void> {
  if (isTauri) {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const { writeFile } = await import('@tauri-apps/plugin-fs');
      const chosen = await save({ defaultPath: filename });
      if (chosen) {
        await writeFile(chosen, new Uint8Array(await blob.arrayBuffer()));
      }
      return;
    } catch (e) {
      console.warn(
        'Tauri save dialog failed, falling back to browser download:',
        e,
      );
    }
  }

  // Browser fallback. Append to body BEFORE click — some browsers and
  // Tauri webviews require the anchor be in the document. Defer revoke
  // so the download has time to start.
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.style.display = 'none';
  document.body.appendChild(a);
  a.click();
  setTimeout(() => {
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, 1000);
}

/**
 * Escape a single CSV field. Handles the standard quoting rules
 * (commas, quotes, newlines) AND neutralises CSV formula injection by
 * prepending a tab character to fields starting with `=`, `+`, `-`,
 * or `@` — these would otherwise execute as formulas in older Excel.
 *
 * OWASP CSV-injection guidance (and the 2026-04-28 security audit
 * MEDIUM finding) — tab prefix is preferred over single-quote because
 * the single-quote can render as a literal character in Numbers and
 * LibreOffice.
 */
export function escapeCsvField(
  value: string | number | boolean | undefined | null,
): string {
  if (value == null) return '';
  let str = String(value);

  // Formula-injection neutralisation — must come BEFORE quote-wrapping.
  if (/^[=+\-@]/.test(str)) {
    str = `\t${str}`;
  }

  if (str.includes(',') || str.includes('"') || str.includes('\n')) {
    return `"${str.replace(/"/g, '""')}"`;
  }
  return str;
}

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
