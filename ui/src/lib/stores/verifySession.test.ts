// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, it, expect, beforeEach } from 'vitest';
import {
  saveBatchSession,
  restoreBatchSession,
  clearBatchSession,
  saveVerifySession,
  restoreVerifySession,
  clearVerifySession,
  type BatchListSession,
  type VerifySession,
} from './verifySession';
import type { VerificationResult } from '$lib/types';

const sampleBatch = (): BatchListSession => ({
  items: [
    {
      id: 'a-1',
      fileName: 'sunset.jpg',
      filePath: '/tmp/sunset.jpg',
      status: 'done',
      overallTrust: 0.82,
      error: null,
      startedAt: 1000,
      finishedAt: 2000,
    },
    {
      id: 'b-2',
      fileName: 'broken.png',
      filePath: '/tmp/broken.png',
      status: 'error',
      overallTrust: null,
      error: 'decode failed',
      startedAt: 2000,
      finishedAt: 2100,
    },
  ],
  mode: 'standard',
  savedAt: '2026-06-13T10:00:00.000Z',
});

// A minimal single-result session — the store only touches result.noiseResult
// when stripping, so an empty result object is sufficient for these tests.
const sampleSingle = (): VerifySession => ({
  result: {} as unknown as VerificationResult,
  fileName: 'single.jpg',
  filePath: '/tmp/single.jpg',
  previewDataUrl: null,
  mode: 'deep',
  verifiedAt: '2026-06-13T10:05:00.000Z',
});

describe('batch session store', () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  it('returns null when nothing has been saved', () => {
    expect(restoreBatchSession()).toBeNull();
  });

  it('round-trips the lightweight list through save and restore', () => {
    const session = sampleBatch();
    saveBatchSession(session);

    const restored = restoreBatchSession();
    expect(restored).not.toBeNull();
    expect(restored?.mode).toBe('standard');
    expect(restored?.savedAt).toBe('2026-06-13T10:00:00.000Z');
    expect(restored?.items).toHaveLength(2);

    const [done, errored] = restored!.items;
    expect(done.overallTrust).toBe(0.82);
    expect(done.status).toBe('done');
    expect(errored.status).toBe('error');
    expect(errored.overallTrust).toBeNull();
    expect(errored.error).toBe('decode failed');
  });

  it('clearBatchSession removes the cached list', () => {
    saveBatchSession(sampleBatch());
    expect(restoreBatchSession()).not.toBeNull();

    clearBatchSession();
    expect(restoreBatchSession()).toBeNull();
  });

  it('the latest save replaces the previous one', () => {
    saveBatchSession(sampleBatch());
    saveBatchSession({ items: [], mode: 'deep', savedAt: '2026-06-13T11:00:00.000Z' });

    const restored = restoreBatchSession();
    expect(restored?.items).toHaveLength(0);
    expect(restored?.mode).toBe('deep');
  });

  it('returns null when the stored payload is corrupt', () => {
    sessionStorage.setItem('jura-verify-batch-session', '{ not valid json');
    expect(restoreBatchSession()).toBeNull();
  });

  it('is independent of the single-result session', () => {
    saveBatchSession(sampleBatch());
    saveVerifySession(sampleSingle());

    // Clearing the single session must not touch the batch list.
    clearVerifySession();
    expect(restoreVerifySession()).toBeNull();
    expect(restoreBatchSession()?.items).toHaveLength(2);

    // ...and clearing the batch list must not touch the single session.
    saveVerifySession(sampleSingle());
    clearBatchSession();
    expect(restoreBatchSession()).toBeNull();
    expect(restoreVerifySession()?.fileName).toBe('single.jpg');
  });
});
