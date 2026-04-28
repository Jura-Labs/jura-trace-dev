/**
 * Pins the file-picker contract for the Protect and Verify workflows.
 *
 * The 2026-04-28 format-support truth-grid (JTV-105) excluded a set of
 * formats from v1.0 because they have no working detector path:
 *
 *   - webm / mkv / avi — no C2PA, no watermark, no fingerprint, no trust
 *   - docx / odt / epub / txt — 0.50 trust with zero analysis
 *   - gif — animated frames have no C2PA / watermark / fingerprint
 *   - wav / mp3 / flac / ogg / aac / m4a — overfitted audio deepfake model
 *     (AUC 1.0 on 2 speakers + 1 TTS engine; defer to v1.1 AASIST)
 *   - stl / obj / gltf / glb — no detector path
 *
 * If a future PR re-adds any of these to PROTECT_FILE_FILTERS or
 * VERIFY_FILE_FILTERS, this spec fails before pilots see the regression.
 */
import { describe, expect, it } from 'vitest';
import { PROTECT_FILE_FILTERS, VERIFY_FILE_FILTERS } from './api';

const PROHIBITED_FOR_V1_0 = [
  // Item 3 — no detector path
  'webm', 'mkv', 'avi', 'docx', 'gif',
  // Item 5 — audio dropped, deferred to v1.1 (JTV-110)
  'wav', 'mp3', 'flac', 'ogg', 'aac', 'm4a',
  // 3D — no detector path
  'stl', 'obj', 'gltf', 'glb',
];

const REQUIRED_FOR_V1_0 = [
  'jpg', 'jpeg', 'png', 'heic', 'pdf', 'mp4', 'mov',
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

  it.each(REQUIRED_FOR_V1_0)(
    'lists %s (supported v1.0 format)',
    (ext) => {
      expect(all).toContain(ext);
    },
  );

  it('exposes the four expected categories', () => {
    const names = PROTECT_FILE_FILTERS.map((f) => f.name);
    expect(names).toEqual(['All Supported', 'Images', 'Documents', 'Video']);
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

  it.each(REQUIRED_FOR_V1_0)(
    'lists %s (supported v1.0 format)',
    (ext) => {
      expect(all).toContain(ext);
    },
  );
});
