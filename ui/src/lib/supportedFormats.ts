// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Single source of truth for the file formats Jura Trace v1.0 will verify.
 *
 * Read by the Verify page's file picker AND by both drag-and-drop paths.
 * It lives here, outside the component, so those three cannot drift apart
 * and so the rule can be unit tested.
 *
 * They did drift. The picker filtered to these extensions while neither
 * drop handler checked anything, so a dropped PDF ran the full pipeline
 * while help/format-support told the user PDFs were excluded. Two gestures,
 * two different products, one of them undocumented.
 *
 * That mattered beyond tidiness: lopdf 0.34, used by pdf_provenance.rs, has
 * a known stack overflow on deeply nested PDFs (RUSTSEC-2026-0187), and the
 * ungated drop handler was a route for a hostile document to reach it.
 *
 * v1.0 is images only. Video and audio were dropped under JTV-138; PDF was
 * dropped on 11 May 2026 with document forensics deferred. When any of
 * those return, add them here and the picker and both drop paths follow.
 */
export const SUPPORTED_EXTENSIONS = [
  'jpg',
  'jpeg',
  'png',
  'tiff',
  'tif',
  'webp',
  'avif',
  'heic',
  'heif',
] as const;

/** Human-readable list for user-facing copy, kept next to the rule it describes. */
export const SUPPORTED_FORMATS_LABEL = 'JPEG, PNG, TIFF, WebP, HEIC and AVIF';

/**
 * True when a filename or path carries an extension v1.0 can verify.
 *
 * Extension-based, deliberately. This is a routing gate, not a security
 * boundary: it decides which pipeline a file enters, and the Rust format
 * router sniffs actual content afterwards. Its job is to stop the app
 * accepting a file it has told the user it does not accept.
 */
export function isSupportedFile(nameOrPath: string): boolean {
  const base = nameOrPath.split(/[\\/]/).pop() ?? '';
  // A name with no dot, or a dotfile with no extension, has no extension.
  const lastDot = base.lastIndexOf('.');
  if (lastDot <= 0) return false;
  const ext = base.slice(lastDot + 1).toLowerCase();
  return (SUPPORTED_EXTENSIONS as readonly string[]).includes(ext);
}
