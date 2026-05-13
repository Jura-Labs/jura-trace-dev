// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  buildFpPayload,
  buildFpClipboardText,
  buildFpMailtoUri,
  resolvePlatform,
  copyFpReport,
  openFpMailto,
  FP_FEEDBACK_EMAIL,
  type FpReportPayload,
} from './fp-report';

describe('fp-report helpers', () => {
  const fixedNow = '2026-05-13T09:00:00.000Z';

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(fixedNow));
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  describe('buildFpPayload', () => {
    it('produces the locked Tier 1 surface (5 fields, deterministic timestamp)', () => {
      const payload = buildFpPayload('modern_codec', 'image/png', '0.9.0');
      expect(payload).toMatchObject({
        reasonCode: 'modern_codec',
        mimeType: 'image/png',
        appVersion: '0.9.0',
        timestamp: fixedNow,
      });
      expect(typeof payload.platform).toBe('string');
    });

    it('falls back to "unknown" mime when undefined', () => {
      const payload = buildFpPayload('other', undefined, '0.9.0');
      expect(payload.mimeType).toBe('unknown');
    });

    it('never includes reason_note, deepfake_score, signalScoresJson, or file_hash', () => {
      const payload = buildFpPayload('other', 'image/jpeg', '0.9.0');
      // Tier 1 surface is exactly these 5 keys — nothing else.
      expect(Object.keys(payload).sort()).toEqual(
        ['appVersion', 'mimeType', 'platform', 'reasonCode', 'timestamp'].sort()
      );
    });
  });

  describe('resolvePlatform', () => {
    it('prefers navigator.userAgentData.platform when available', () => {
      const nav = navigator as Navigator & {
        userAgentData?: { platform?: string };
      };
      const restore = nav.userAgentData;
      try {
        Object.defineProperty(nav, 'userAgentData', {
          value: { platform: 'macOS' },
          configurable: true,
        });
        expect(resolvePlatform()).toBe('macOS');
      } finally {
        Object.defineProperty(nav, 'userAgentData', {
          value: restore,
          configurable: true,
        });
      }
    });

    it('falls back to navigator.platform when userAgentData absent', () => {
      const nav = navigator as Navigator & {
        userAgentData?: { platform?: string };
      };
      const prevUad = nav.userAgentData;
      try {
        Object.defineProperty(nav, 'userAgentData', {
          value: undefined,
          configurable: true,
        });
        // navigator.platform is provided by happy-dom — non-empty string.
        expect(typeof resolvePlatform()).toBe('string');
        expect(resolvePlatform()).not.toBe('');
      } finally {
        Object.defineProperty(nav, 'userAgentData', {
          value: prevUad,
          configurable: true,
        });
      }
    });
  });

  describe('buildFpClipboardText', () => {
    const payload: FpReportPayload = {
      reasonCode: 'modern_codec',
      mimeType: 'image/png',
      appVersion: '0.9.0',
      platform: 'macOS',
      timestamp: fixedNow,
    };

    it('renders all 5 fields on labelled lines', () => {
      const text = buildFpClipboardText(payload);
      expect(text).toContain('Reason:    modern_codec');
      expect(text).toContain('Type:      image/png');
      expect(text).toContain('Version:   0.9.0');
      expect(text).toContain('Platform:  macOS');
      expect(text).toContain(`Timestamp: ${fixedNow}`);
    });

    it('includes the "no personal data" prompt at the end', () => {
      const text = buildFpClipboardText(payload);
      expect(text).toMatch(/avoid personal data/i);
    });

    it('never leaks Tier 2 fields (deepfake score, hash, notes)', () => {
      const text = buildFpClipboardText(payload);
      expect(text.toLowerCase()).not.toMatch(/deepfake_score|file_hash|reason_note|signal_scores/);
    });
  });

  describe('buildFpMailtoUri', () => {
    const payload: FpReportPayload = {
      reasonCode: 'modern_codec',
      mimeType: 'image/png',
      appVersion: '0.9.0',
      platform: 'macOS',
      timestamp: fixedNow,
    };

    it('targets feedback@juralabs.org', () => {
      const uri = buildFpMailtoUri(payload);
      expect(uri.startsWith(`mailto:${FP_FEEDBACK_EMAIL}?`)).toBe(true);
    });

    it('includes the version in the subject', () => {
      const uri = buildFpMailtoUri(payload);
      const subjectMatch = uri.match(/subject=([^&]+)/);
      expect(subjectMatch).not.toBeNull();
      expect(decodeURIComponent(subjectMatch![1])).toContain('0.9.0');
      expect(decodeURIComponent(subjectMatch![1])).toMatch(/False Positive Report/);
    });

    it('keeps URI length under the 500-byte threshold for mail-client compatibility', () => {
      const uri = buildFpMailtoUri(payload);
      // Locked design (project_fp_report_v1_locked.md): under 500 bytes so
      // older Outlook builds don't truncate the URI silently.
      expect(uri.length).toBeLessThan(500);
    });

    it('encodes special characters in the body', () => {
      const uri = buildFpMailtoUri(payload);
      // Body uses URL encoding — colons and newlines must be encoded.
      expect(uri).toMatch(/body=/);
      const bodyMatch = uri.match(/body=(.+)$/);
      expect(bodyMatch).not.toBeNull();
      // Decoded body should match the clipboard text exactly.
      expect(decodeURIComponent(bodyMatch![1])).toBe(buildFpClipboardText(payload));
    });
  });

  describe('copyFpReport', () => {
    const payload: FpReportPayload = {
      reasonCode: 'high_iso',
      mimeType: 'image/jpeg',
      appVersion: '0.9.0',
      platform: 'Linux',
      timestamp: fixedNow,
    };

    it('writes the clipboard text to navigator.clipboard', async () => {
      const writeText = vi.fn().mockResolvedValue(undefined);
      Object.defineProperty(navigator, 'clipboard', {
        value: { writeText },
        configurable: true,
      });

      const ok = await copyFpReport(payload);
      expect(ok).toBe(true);
      expect(writeText).toHaveBeenCalledWith(buildFpClipboardText(payload));
    });

    it('returns false when clipboard write rejects', async () => {
      const writeText = vi.fn().mockRejectedValue(new Error('denied'));
      Object.defineProperty(navigator, 'clipboard', {
        value: { writeText },
        configurable: true,
      });

      const ok = await copyFpReport(payload);
      expect(ok).toBe(false);
    });
  });

  describe('openFpMailto', () => {
    const payload: FpReportPayload = {
      reasonCode: 'other',
      mimeType: 'image/webp',
      appVersion: '0.9.0',
      platform: 'Windows',
      timestamp: fixedNow,
    };

    it('sets window.location.href to the mailto: URI', () => {
      // happy-dom's window.location is writable for this kind of test.
      const prevHref = window.location.href;
      try {
        const ok = openFpMailto(payload);
        expect(ok).toBe(true);
        expect(window.location.href).toContain(`mailto:${FP_FEEDBACK_EMAIL}`);
        expect(window.location.href).toContain('subject=');
      } finally {
        // Best-effort restore — some happy-dom builds need this to be set
        // to an explicit URL rather than restored to the previous value.
        try { window.location.href = prevHref; } catch { /* ignore */ }
      }
    });
  });
});
