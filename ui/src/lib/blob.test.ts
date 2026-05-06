// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Pin the contract for the shared CSV escape helper.
 *
 * The formula-injection neutralisation rule (prepend a tab to fields
 * starting with =, +, -, @) was added 2026-04-28 to close the MEDIUM
 * finding from the security audit (project_security_audit memory).
 * If a future change weakens this rule, this spec fails before pilots
 * see the regression.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { escapeCsvField, triggerDownload } from './blob';

describe('escapeCsvField', () => {
  it('returns empty string for null and undefined', () => {
    expect(escapeCsvField(null)).toBe('');
    expect(escapeCsvField(undefined)).toBe('');
  });

  it('returns plain values unmodified when no special chars', () => {
    expect(escapeCsvField('hello')).toBe('hello');
    expect(escapeCsvField(42)).toBe('42');
    expect(escapeCsvField(true)).toBe('true');
  });

  it('quote-wraps fields containing commas', () => {
    expect(escapeCsvField('a, b')).toBe('"a, b"');
  });

  it('quote-wraps and escapes embedded double-quotes', () => {
    expect(escapeCsvField('she said "hi"')).toBe('"she said ""hi"""');
  });

  it('quote-wraps fields containing newlines', () => {
    expect(escapeCsvField('line1\nline2')).toBe('"line1\nline2"');
  });

  it.each([
    ['+1+1', '\t+1+1'],
    ['-2', '\t-2'],
    ['@SUM(A1)', '\t@SUM(A1)'],
  ])(
    'prepends tab to formula-triggering field %s (CSV-injection guard)',
    (input, expected) => {
      // No commas/quotes/newlines so the result is not quote-wrapped —
      // it's just the tab-prefixed string.
      expect(escapeCsvField(input)).toBe(expected);
    },
  );

  it('quote-wraps AND tab-prefixes when the formula contains quotes', () => {
    // A classic CSV-injection payload like `=cmd|"/c calc"!A0` has both
    // an = prefix AND embedded double-quotes; the result must be both
    // tab-prefixed and quote-wrapped (with quotes doubled).
    expect(escapeCsvField('=cmd|"/c calc"!A0')).toBe(
      '"\t=cmd|""/c calc""!A0"',
    );
  });

  it('combines formula-prefix and quote-wrapping when both apply', () => {
    // A field that starts with = AND contains a comma must be both
    // tab-prefixed and quote-wrapped.
    expect(escapeCsvField('=A1,B1')).toBe('"\t=A1,B1"');
  });

  it('does not prefix fields that contain but do not start with formula chars', () => {
    expect(escapeCsvField('total = 5')).toBe('total = 5');
    expect(escapeCsvField('a-b')).toBe('a-b');
  });
});

/**
 * triggerDownload — browser-fallback path.
 *
 * The original Protect-page bug (JTV-129) was that the anchor was never
 * attached to the DOM and the blob URL was revoked synchronously,
 * causing the Tauri webview to ignore the click. These tests pin the
 * fixed behaviour so a future change cannot silently re-break the
 * pattern.
 *
 * The Tauri-runtime branch is not exercised here (it requires the
 * `__TAURI_INTERNALS__` global and the dynamic import of the Tauri
 * plugin modules); manual verification in the Tauri dev app covers it.
 */
describe('triggerDownload (browser-fallback path)', () => {
  let createObjectURL: ReturnType<typeof vi.fn>;
  let revokeObjectURL: ReturnType<typeof vi.fn>;
  let originalCreate: typeof URL.createObjectURL;
  let originalRevoke: typeof URL.revokeObjectURL;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let appendSpy: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let removeSpy: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let clickSpy: any;

  beforeEach(() => {
    vi.useFakeTimers();
    createObjectURL = vi.fn(() => 'blob:mock-url-123');
    revokeObjectURL = vi.fn();
    originalCreate = URL.createObjectURL;
    originalRevoke = URL.revokeObjectURL;
    URL.createObjectURL = createObjectURL as unknown as typeof URL.createObjectURL;
    URL.revokeObjectURL = revokeObjectURL as unknown as typeof URL.revokeObjectURL;

    appendSpy = vi.spyOn(document.body, 'appendChild');
    removeSpy = vi.spyOn(document.body, 'removeChild');
    clickSpy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.useRealTimers();
    URL.createObjectURL = originalCreate;
    URL.revokeObjectURL = originalRevoke;
    appendSpy.mockRestore();
    removeSpy.mockRestore();
    clickSpy.mockRestore();
  });

  it('attaches the anchor to document.body before clicking it', async () => {
    const blob = new Blob(['hello,world'], { type: 'text/csv' });

    await triggerDownload(blob, 'test.csv');

    // The fix: the anchor MUST be in the DOM when click() fires.
    expect(appendSpy).toHaveBeenCalledTimes(1);
    expect(clickSpy).toHaveBeenCalledTimes(1);
    expect(appendSpy.mock.invocationCallOrder[0]).toBeLessThan(
      clickSpy.mock.invocationCallOrder[0],
    );
  });

  it('sets the download attribute to the requested filename', async () => {
    const blob = new Blob(['x'], { type: 'text/csv' });

    await triggerDownload(blob, 'jura_assets_2026-04-28.csv');

    const anchor = appendSpy.mock.calls[0][0] as HTMLAnchorElement;
    expect(anchor.tagName).toBe('A');
    expect(anchor.download).toBe('jura_assets_2026-04-28.csv');
    expect(anchor.href).toBe('blob:mock-url-123');
  });

  it('defers URL.revokeObjectURL by ~1s so the download has time to start', async () => {
    const blob = new Blob(['x'], { type: 'text/csv' });

    await triggerDownload(blob, 't.csv');

    // The original bug: revoke was synchronous — blob URL was gone
    // before the browser/webview could fetch it. Now deferred via
    // setTimeout(..., 1000).
    expect(revokeObjectURL).not.toHaveBeenCalled();

    vi.advanceTimersByTime(999);
    expect(revokeObjectURL).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(revokeObjectURL).toHaveBeenCalledOnce();
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:mock-url-123');
    // Anchor cleaned up at the same point.
    expect(removeSpy).toHaveBeenCalledOnce();
  });
});
