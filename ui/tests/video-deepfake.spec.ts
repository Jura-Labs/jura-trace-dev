// SPDX-License-Identifier: AGPL-3.0-or-later

import { test, expect } from '@playwright/test';

/**
 * Mock VerificationResult matching the actual TypeScript interface.
 * Field names must match ui/src/lib/types.ts VerificationResult exactly.
 */
const MOCK_VIDEO_RESULT = {
  mode: 'standard',
  sourceType: 'file',
  contentType: 'video',
  overallTrust: 0.42,
  metadataFlags: [],
  elaResult: null,
  noiseResult: null,
  copyMoveResult: null,
  deepfakeResult: null,
  nprResult: null,
  caResult: null,
  clipResult: null,
  jpegGhostResult: null,
  exifAnalysis: null,
  c2paManifest: null,
  claimVerdict: null,
  ragClaimResult: null,
  segmentedElaResult: null,
  shadowConsistencyResult: null,
  colourTemperatureResult: null,
  spliceBoundaryResult: null,
  watermarkExtractResult: null,
  transcriptionResult: null,
  claimCheckResult: null,
  aiGenerator: null,
  videoFramesResult: {
    frameCount: 4,
    timestamps: [0.0, 2.5, 5.0, 7.5],
    frames: [],
    success: true,
  },
  videoDeepfakeResult: {
    frameResults: [
      {
        frameIndex: 0, timestamp: 0.0, score: 0.22, suspicious: false, verdictLevel: 'authentic',
        signals: [
          { name: 'Spectral decay', description: 'Frequency spectrum analysis', weight: 0.15, triggered: false },
          { name: 'Noise pattern', description: 'Noise consistency check', weight: 0.2, triggered: false },
        ],
        classifierScore: 0.18, classifierAvailable: true,
      },
      {
        frameIndex: 1, timestamp: 2.5, score: 0.55, suspicious: true, verdictLevel: 'inconclusive',
        signals: [
          { name: 'Spectral decay', description: 'Frequency spectrum analysis', weight: 0.15, triggered: true },
          { name: 'Noise pattern', description: 'Noise consistency check', weight: 0.2, triggered: false },
        ],
        classifierScore: 0.61, classifierAvailable: true,
      },
      {
        frameIndex: 2, timestamp: 5.0, score: 0.72, suspicious: true, verdictLevel: 'synthetic',
        signals: [
          { name: 'Spectral decay', description: 'Frequency spectrum analysis', weight: 0.15, triggered: true },
          { name: 'Noise pattern', description: 'Noise consistency check', weight: 0.2, triggered: true },
        ],
        classifierScore: 0.78, classifierAvailable: true,
      },
      {
        frameIndex: 3, timestamp: 7.5, score: 0.31, suspicious: false, verdictLevel: 'authentic',
        signals: [
          { name: 'Spectral decay', description: 'Frequency spectrum analysis', weight: 0.15, triggered: false },
          { name: 'Noise pattern', description: 'Noise consistency check', weight: 0.2, triggered: false },
        ],
        classifierScore: 0.25, classifierAvailable: true,
      },
    ],
    aggregateScore: 0.45,
    aggregateVerdict: 'inconclusive',
    aggregateConfidence: 'moderate',
    framesAnalysed: 4,
    framesRequested: 6,
    temporalAvailable: true,
    temporalNoiseDrift: 0.12,
    temporalSpectralDrift: 0.52,
    temporalLbpDrift: 0.08,
    mode: 'standard',
    duration: 10.0,
    success: true,
    message: 'Video deepfake analysis complete',
  },
};

/**
 * Inject mock verification result via the test hook on the verify page.
 */
async function injectMockResult(page: import('@playwright/test').Page) {
  await page.waitForSelector('h1');
  // Wait for the test hook to be registered (DEV mode only)
  await page.waitForFunction(() => !!(window as any).__juraSetVerifyResult, { timeout: 5000 });

  await page.evaluate((mockResult) => {
    const fn = (window as any).__juraSetVerifyResult;
    if (fn) fn(mockResult);
  }, MOCK_VIDEO_RESULT);

  // Wait for the results to render (trust ring uses aria-label, not visible
  // text). The verify v2 default UI (since 2026-04-16) puts the video
  // analysis section inside the AI card body, which is collapsed by default
  // when the mock has no AI findings — expand it to make the timeline visible.
  await page.waitForSelector('section[aria-label="Verification result summary"]', { timeout: 8000 });
  const aiCardToggle = page.locator('button[aria-controls="card-ai-body"]');
  await aiCardToggle.waitFor({ state: 'visible', timeout: 5000 });
  if ((await aiCardToggle.getAttribute('aria-expanded')) !== 'true') {
    await aiCardToggle.click();
  }
  await page.waitForSelector('[aria-labelledby="video-analysis-heading"]', { timeout: 5000 });
}

// Video deepfake analysis is dropped from v1.0 (JTV-138, 2 May 2026) — see
// CHANGELOG.md and `project_v1_video_audio_drop.md`. The verify page now
// renders a "Planned — v1.0.x" banner on video files instead of the timeline,
// so these timeline / accordion / aggregate-verdict tests are skipped at the
// describe level. Re-enable as part of JTV-139 (v1.0.1 video re-add) once
// the calibration matrix and Global Majority device gates are met.
test.describe.skip('Video deepfake timeline (re-enable in JTV-139 v1.0.x re-add)', () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true'); localStorage.setItem('jura-setup-complete', 'true');
    });
    await page.goto('/verify');
  });

  test('frame timeline renders with score badges for each analysed frame', async ({ page }) => {
    await injectMockResult(page);

    const timeline = page.locator('[role="list"][aria-label="Video frame deepfake analysis timeline"]');
    await expect(timeline).toBeVisible();

    const frameItems = timeline.locator('[role="listitem"]');
    await expect(frameItems).toHaveCount(4);

    const buttons = timeline.locator('button[aria-expanded]');
    await expect(buttons).toHaveCount(4);

    for (let i = 0; i < 4; i++) {
      await expect(buttons.nth(i)).toHaveAttribute('aria-expanded', 'false');
    }
  });

  test('frame accordion expands to show signals on click', async ({ page }) => {
    await injectMockResult(page);

    const timeline = page.locator('[role="list"][aria-label="Video frame deepfake analysis timeline"]');
    await expect(timeline).toBeVisible();

    const thirdButton = timeline.locator('button[aria-expanded]').nth(2);
    await thirdButton.click();

    await expect(thirdButton).toHaveAttribute('aria-expanded', 'true');

    const detailPanel = page.locator('#v2-frame-detail-2');
    await expect(detailPanel).toBeVisible();

    await expect(detailPanel).toContainText('5.0s');
    await expect(detailPanel).toContainText('Synthetic');
    await expect(detailPanel).toContainText('Spectral decay');
    await expect(detailPanel).toContainText('Noise pattern');

    await thirdButton.click();
    await expect(thirdButton).toHaveAttribute('aria-expanded', 'false');
    await expect(detailPanel).not.toBeVisible();
  });

  test('aggregate verdict badge displays correct classification', async ({ page }) => {
    await injectMockResult(page);

    const videoSection = page.locator('section[aria-labelledby="video-analysis-heading"]');
    await expect(videoSection).toBeVisible();

    await expect(videoSection).toContainText('Inconclusive');
    await expect(videoSection).toContainText('45%');
    await expect(videoSection).toContainText('4 frames');
    await expect(videoSection).toContainText('drift detected');
  });
});
