// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { trustLevelLabel, verdictTrustLevel, type Verdict } from './types';

const verdict = (band: Verdict['band'], score: number): Verdict => ({
  band,
  score,
  ceilingApplied: null,
  bandBoundaries: { trusted: 0.7, uncertain: 0.4 },
});

describe('verdictTrustLevel', () => {
  it('reads the band the backend computed', () => {
    expect(verdictTrustLevel({ verdict: verdict('trusted', 0.9) })).toBe('high');
    expect(verdictTrustLevel({ verdict: verdict('uncertain', 0.9) })).toBe('medium');
    expect(verdictTrustLevel({ verdict: verdict('untrusted', 0.1) })).toBe('low');
    expect(verdictTrustLevel({ verdict: verdict('inconclusive', 0.9) })).toBe('inconclusive');
  });

  it('returns null for a result saved before the verdict block', () => {
    expect(verdictTrustLevel({})).toBeNull();
    expect(verdictTrustLevel({ verdict: null })).toBeNull();
  });
});

describe('trustLevelLabel', () => {
  it('follows the band, not the score, when they differ', () => {
    // A capped result: the score alone would read High Trust.
    expect(trustLevelLabel({ overallTrust: 0.9, verdict: verdict('uncertain', 0.9) })).toBe(
      'Moderate Trust'
    );
    expect(trustLevelLabel({ overallTrust: 0.9, verdict: verdict('inconclusive', 0.9) })).toBe(
      'Inconclusive'
    );
  });

  it('bands the score for a result with no verdict block', () => {
    expect(trustLevelLabel({ overallTrust: 0.7 })).toBe('High Trust');
    expect(trustLevelLabel({ overallTrust: 0.4 })).toBe('Moderate Trust');
    expect(trustLevelLabel({ overallTrust: 0.39 })).toBe('Low Trust');
  });
});
