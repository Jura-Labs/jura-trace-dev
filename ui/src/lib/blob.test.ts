/**
 * Pin the contract for the shared CSV escape helper.
 *
 * The formula-injection neutralisation rule (prepend a tab to fields
 * starting with =, +, -, @) was added 2026-04-28 to close the MEDIUM
 * finding from the security audit (project_security_audit memory).
 * If a future change weakens this rule, this spec fails before pilots
 * see the regression.
 */
import { describe, expect, it } from 'vitest';
import { escapeCsvField } from './blob';

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
