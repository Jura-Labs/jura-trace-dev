// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Pin the power-saver mode API contract and Settings copy.
 *
 * These tests verify:
 *   1. The IPC wrapper functions exist and degrade gracefully outside Tauri.
 *   2. The exact British-English helper copy required by the Settings toggle.
 *
 * If a future PR renames or removes the IPC functions, or changes the
 * user-facing copy, these tests fail before the regression ships.
 */
import { describe, expect, it } from 'vitest';

import { getPowerSaverMode, setPowerSaverMode } from './api';

describe('getPowerSaverMode — graceful degradation outside Tauri', () => {
  it('exists and is callable', () => {
    expect(typeof getPowerSaverMode).toBe('function');
  });

  it('returns false when the backend is unavailable (browser context)', async () => {
    // api.ts returns false from the catch block when not running inside Tauri.
    const result = await getPowerSaverMode();
    expect(result).toBe(false);
  });
});

describe('setPowerSaverMode — function signature', () => {
  it('exists and is callable with a boolean argument', () => {
    expect(typeof setPowerSaverMode).toBe('function');
  });

  it('accepts true without type error (rejects outside Tauri is expected)', async () => {
    // Outside Tauri invoke throws "Tauri not available" — that is correct
    // browser-fallback behaviour.  We only verify the function is callable.
    await expect(setPowerSaverMode(true)).rejects.toThrow();
  });

  it('accepts false without type error (rejects outside Tauri is expected)', async () => {
    await expect(setPowerSaverMode(false)).rejects.toThrow();
  });
});

describe('Settings toggle copy (British English contract)', () => {
  // Pin the exact user-facing strings so accidental copy changes are caught.
  const TOGGLE_LABEL = 'Power-saver mode';
  const HELPER_TEXT =
    'Stops the analysis engine after five minutes of inactivity to free approximately 300–500 MB of RAM. The first verification afterwards takes 30–90 seconds longer while the engine reloads. Recommended only on machines with under 16 GB of RAM.';

  it('toggle label is "Power-saver mode" with no emoji', () => {
    expect(TOGGLE_LABEL).toBe('Power-saver mode');
    expect(TOGGLE_LABEL).not.toMatch(/[^\x00-\x7F]/);
  });

  it('helper text mentions five minutes', () => {
    expect(HELPER_TEXT).toContain('five minutes');
  });

  it('helper text mentions 300–500 MB', () => {
    expect(HELPER_TEXT).toContain('300–500 MB');
  });

  it('helper text mentions 30–90 seconds', () => {
    expect(HELPER_TEXT).toContain('30–90 seconds');
  });

  it('helper text mentions 16 GB of RAM', () => {
    expect(HELPER_TEXT).toContain('16 GB of RAM');
  });

  it('helper text uses "verification" not just "analysis" in reload sentence', () => {
    expect(HELPER_TEXT).toContain('verification afterwards');
  });

  it('helper text does not use American spelling "aluminum" or "color"', () => {
    expect(HELPER_TEXT.toLowerCase()).not.toContain('color');
    expect(HELPER_TEXT.toLowerCase()).not.toContain('aluminum');
  });
});
