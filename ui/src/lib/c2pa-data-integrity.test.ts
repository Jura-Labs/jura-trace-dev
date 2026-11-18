// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Pin the data integrity line of the validation summary (BL-C2PA-004).
 *
 * The first case is the validation output Jura Trace v1.2.0 gave on
 * 12 November 2026 for David Buchanan's whole-file exclusion sample
 * ("How to Hack Time, With C2PA"). c2pa-rs reports a pass and a fail for
 * the data hash together; the summary used to show the pass.
 */
import { describe, expect, it } from 'vitest';
import { DATA_INTEGRITY_TEXT, dataIntegrityState } from './c2pa-labels';

const BUCHANAN_WHOLE_FILE_EXCLUSION = [
  { code: 'claimSignature.validated', outcome: 'pass', explanation: 'claim signature valid' },
  { code: 'assertion.hashedURI.match', outcome: 'pass', explanation: 'hashed uri matched: self#jumbf=c2pa.assertions/c2pa.hash.data' },
  { code: 'assertion.dataHash.match', outcome: 'pass', explanation: 'data hash valid' },
  { code: 'signingCredential.ocsp.skipped', outcome: 'info', explanation: 'OCSP fetching skipped' },
  {
    code: 'assertion.dataHash.mismatch',
    outcome: 'fail',
    explanation: 'data hash exclusion does not match the manifest location in the asset',
  },
];

describe('dataIntegrityState', () => {
  it('lets the exclusion fail outrank the pass reported beside it', () => {
    const state = dataIntegrityState(BUCHANAN_WHOLE_FILE_EXCLUSION);
    expect(state).toBe('not-covered');
    expect(DATA_INTEGRITY_TEXT[state]).not.toMatch(/not been modified/);
  });

  it('reports a modified file when the hash itself does not match', () => {
    expect(
      dataIntegrityState([
        { code: 'assertion.dataHash.mismatch', outcome: 'fail', explanation: 'asset hash error' },
      ]),
    ).toBe('failed');
  });

  it('confirms integrity only when nothing failed', () => {
    expect(
      dataIntegrityState([{ code: 'assertion.dataHash.match', outcome: 'pass' }]),
    ).toBe('confirmed');
  });

  it('says not checked when there is no data hash result', () => {
    expect(dataIntegrityState([{ code: 'claimSignature.validated', outcome: 'pass' }])).toBe(
      'not-checked',
    );
  });
});

describe('dataIntegrityState with unsigned bytes', () => {
  // c2pa-rs 0.91.3 passes an exact manifest exclusion plus a range over the
  // pixels: every check below is a pass or informational. Trace's own
  // measurement (unsignedBytes) is the only signal.
  const MANIFEST_PLUS_PIXELS = [
    { code: 'claimSignature.validated', outcome: 'pass' },
    { code: 'assertion.dataHash.match', outcome: 'pass', explanation: 'data hash valid' },
    { code: 'assertion.dataHash.additionalExclusionsPresent', outcome: 'info' },
  ];

  it('does not confirm integrity when part of the image is unsigned', () => {
    expect(dataIntegrityState(MANIFEST_PLUS_PIXELS, 6167)).toBe('partly-not-covered');
  });

  it('still confirms integrity when nothing is unsigned', () => {
    expect(dataIntegrityState(MANIFEST_PLUS_PIXELS)).toBe('confirmed');
  });
});
