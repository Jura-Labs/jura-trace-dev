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
  await page.waitForTimeout(500);

  await page.evaluate((mockResult) => {
    const fn = (window as any).__juraSetVerifyResult;
    if (fn) fn(mockResult);
  }, MOCK_VIDEO_RESULT);

  // Wait for the results to render (Trust Score section appears)
  await page.waitForSelector('text=Trust Score', { timeout: 8000 });

  // The video analysis is inside "Technical Details" which is collapsed.
  // Click the toggle to expand it.
  const detailsToggle = page.locator('button', { hasText: 'Technical Details' });
  await detailsToggle.click();

  // Now wait for the video analysis section to render
  await page.waitForSelector('[aria-labelledby="video-analysis-heading"]', { timeout: 5000 });
}

test.describe('Video deepfake timeline', () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
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

    const detailPanel = page.locator('#frame-detail-2');
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
