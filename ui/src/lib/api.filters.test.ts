// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Pins the file-picker contract for the Protect and Verify workflows.
 *
 * v1.0 narrowed (2026-05-22) the Protect dialog to the four formats
 * accepted by our C2PA Validator submission (JPEG / PNG / TIFF / WebP),
 * because c2pa-rs can sign more formats than our validator submission
 * covers, and the Generator-track pre-submission audit recommended we
 * not produce signed output we cannot claim is conformant.
 *
 * Verify keeps a wider image set (HEIC / HEIF / AVIF included) because
 * forensic analysis runs without signing. PDF + video dropped from both
 * surfaces for v1.0 (PDF deferred to v1.1; video + audio deferred to
 * v1.0.1 per project_v1_video_audio_drop).
 *
 * The 2026-04-28 format-support truth-grid (JTV-105) earlier excluded
 * formats with no detector path; that exclusion list is unchanged:
 *
 *   - webm / mkv / avi — no C2PA, no watermark, no fingerprint, no trust
 *   - docx / odt / epub / txt — 0.50 trust with zero analysis
 *   - gif — animated frames have no C2PA / watermark / fingerprint
 *   - wav / mp3 / flac / ogg / aac / m4a — overfitted audio deepfake model
 *   - stl / obj / gltf / glb — no detector path
 *
 * If a future PR re-adds any of these (or re-adds video / PDF before the
 * v1.0.1 / v1.1 unhide) this spec fails before pilots see the regression.
 */
import { describe, expect, it } from 'vitest';
import { PROTECT_FILE_FILTERS, VERIFY_FILE_FILTERS } from './api';

const PROHIBITED_FOR_V1_0 = [
  // No detector path
  'webm', 'mkv', 'avi', 'docx', 'gif',
  // Audio dropped to v1.1 (JTV-110 AASIST retrain)
  'wav', 'mp3', 'flac', 'ogg', 'aac', 'm4a',
  // 3D — no detector path
  'stl', 'obj', 'gltf', 'glb',
  // Dropped from BOTH surfaces in v1.0:
  // PDF deferred to v1.1 (Generator-track audit narrowed scope)
  'pdf',
  // Video deferred to v1.0.1 per project_v1_video_audio_drop
  'mp4', 'mov',
];

const REQUIRED_PROTECT_V1_0 = [
  // C2PA Validator-conformant scope only (Validator submission 2026-05-06)
  'jpg', 'jpeg', 'png', 'tiff', 'tif', 'webp',
];

const REQUIRED_VERIFY_V1_0 = [
  // Wider image set than Protect — forensic analysis doesn't require
  // C2PA validator conformance.
  'jpg', 'jpeg', 'png', 'tiff', 'tif', 'webp',
  'heic', 'heif', 'avif',
];

const PROHIBITED_FOR_PROTECT_V1_0 = [
  // Signable by c2pa-rs but outside our Validator-conformant scope.
  'heic', 'heif', 'avif', 'bmp',
];

const flatten = (
  filters: { name: string; extensions: string[] }[],
): string[] =>
  filters.flatMap((f) => f.extensions);

describe('PROTECT_FILE_FILTERS', () => {
  const all = flatten(PROTECT_FILE_FILTERS);

  it.each(PROHIBITED_FOR_V1_0)(
    'does not list %s (excluded by JTV-105 truth-grid)',
    (ext) => {
      expect(all).not.toContain(ext);
    },
  );

  it.each(PROHIBITED_FOR_PROTECT_V1_0)(
    'does not list %s (outside Validator-conformant scope)',
    (ext) => {
      expect(all).not.toContain(ext);
    },
  );

  it.each(REQUIRED_PROTECT_V1_0)(
    'lists %s (validator-conformant signing format)',
    (ext) => {
      expect(all).toContain(ext);
    },
  );

  it('has a single category labelled for C2PA scope', () => {
    const names = PROTECT_FILE_FILTERS.map((f) => f.name);
    expect(names).toEqual(['C2PA-signable images']);
  });
});

describe('VERIFY_FILE_FILTERS', () => {
  const all = flatten(VERIFY_FILE_FILTERS);

  it.each(PROHIBITED_FOR_V1_0)(
    'does not list %s (excluded by JTV-105 truth-grid)',
    (ext) => {
      expect(all).not.toContain(ext);
    },
  );

  it.each(REQUIRED_VERIFY_V1_0)(
    'lists %s (supported v1.0 verify format)',
    (ext) => {
      expect(all).toContain(ext);
    },
  );
});
