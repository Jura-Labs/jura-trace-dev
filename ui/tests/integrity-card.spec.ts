import { test, expect } from '@playwright/test';
import type { Page } from '@playwright/test';

/**
 * Integrity card regression suite.
 *
 * Authored as the Stage 1+2 baseline for the DetectorRow refactor
 * (2026-04-28).  The integrity card had zero direct e2e coverage
 * before this file landed — this exercises every detector row in
 * the "Is the content intact?" card on `/verify` so that subsequent
 * structural changes (e.g. extracting a `DetectorRow.svelte`
 * component) cannot regress the user-visible rendering without
 * breaking these tests.
 *
 * Tests inject a complete VerificationResult via the
 * `__juraSetVerifyResult` window hook (registered only in dev
 * builds — see ui/src/routes/verify/+page.svelte L678).  No live
 * sidecar is required.
 */

// ──────────────────────────────────────────────────────────────────
// Fixture builders.  `baseResult` carries the universal envelope;
// per-test overrides supply the detector results under test.
// ──────────────────────────────────────────────────────────────────

interface MockVerificationResult {
  mode: string;
  sourceType: string;
  contentType: string;
  overallTrust: number;
  metadataFlags: string[];
  // detector slots — null defaults
  elaResult: any;
  noiseResult: any;
  copyMoveResult: any;
  jpegGhostResult: any;
  segmentedElaResult: any;
  colourTemperatureResult: any;
  shadowConsistencyResult: any;
  spliceBoundaryResult: any;
  nprResult: any;
  dctAnalysisResult: any;
  fourierAnalysisResult: any;
  deepfakeResult: any;
  clipResult: any;
  exifAnalysis: any;
  c2paManifest: any;
  c2paValid: boolean | null;
  watermarkExtractResult: any;
  videoDeepfakeResult: any;
  videoFramesResult: any;
  transcriptionResult: any;
  claimCheckResult: any;
  ragClaimResult: any;
  aiGenerator: any;
  inputQuality: any;
  detectorsRun: string[];
  // catch-all so per-test overrides can add more
  [key: string]: any;
}

function baseResult(): MockVerificationResult {
  return {
    mode: 'standard',
    sourceType: 'file',
    contentType: 'image',
    overallTrust: 0.5,
    metadataFlags: [],
    elaResult: null,
    noiseResult: null,
    copyMoveResult: null,
    jpegGhostResult: null,
    segmentedElaResult: null,
    colourTemperatureResult: null,
    shadowConsistencyResult: null,
    spliceBoundaryResult: null,
    nprResult: null,
    dctAnalysisResult: null,
    fourierAnalysisResult: null,
    deepfakeResult: null,
    clipResult: null,
    exifAnalysis: null,
    c2paManifest: null,
    c2paValid: null,
    watermarkExtractResult: null,
    videoDeepfakeResult: null,
    videoFramesResult: null,
    transcriptionResult: null,
    claimCheckResult: null,
    ragClaimResult: null,
    aiGenerator: null,
    inputQuality: null,
    detectorsRun: [],
  };
}

// 1×1 PNG black pixel (smallest legal PNG). Used for any base64-image
// fixture field — the UI renders <img src="data:..."> regardless of
// content, so we just need something the browser will accept.
const TINY_PNG_BASE64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkAAIAAAoAAv/' +
  'lxKUAAAAASUVORK5CYII=';

// All 11 detectors firing — used to verify every help icon exists,
// every conditional caption renders the suspicious variant, and every
// row sub-line is reachable.
function allSuspiciousFixture(): MockVerificationResult {
  const r = baseResult();
  r.overallTrust = 0.18;
  r.elaResult = {
    score: 0.74,
    suspicious: true,
    elaImageBase64: TINY_PNG_BASE64,
    summary: 'Elevated compression error detected',
  };
  r.noiseResult = {
    score: 0.62,
    suspicious: true,
    anomalousBlocks: 18,
    totalBlocks: 64,
    summary: 'Noise variance unusually heterogeneous',
  };
  r.copyMoveResult = {
    score: 0.55,
    suspicious: true,
    cloneRegions: [
      { x: 10, y: 10, width: 50, height: 50 },
      { x: 200, y: 100, width: 50, height: 50 },
    ],
    visualisationBase64: TINY_PNG_BASE64,
    summary: 'Cloned regions found',
  };
  r.jpegGhostResult = {
    score: 0.48,
    suspicious: true,
    ghostImageBase64: TINY_PNG_BASE64,
    summary: 'Mixed compression history',
  };
  r.segmentedElaResult = {
    score: 0.46,
    suspicious: true,
    anomalousRegions: 5,
    totalRegions: 64,
    visualizationBase64: TINY_PNG_BASE64,
    summary: 'Localised compression anomaly',
  };
  r.colourTemperatureResult = {
    score: 0.42,
    suspicious: true,
    anomalousRegions: 2,
    totalRegions: 16,
    globalMeanA: 5.2,
    globalMeanB: -3.4,
    heatmapBase64: TINY_PNG_BASE64,
    summary: 'Colour balance discontinuity',
  };
  r.shadowConsistencyResult = {
    score: 0.58,
    suspicious: true,
    inconsistentRegions: 3,
    totalRegions: 12,
    globalLightDirection: 142.5,
    heatmapBase64: TINY_PNG_BASE64,
    summary: 'Shadows inconsistent',
  };
  r.spliceBoundaryResult = {
    score: 0.51,
    suspicious: true,
    suspiciousBoundaries: 2,
    totalBoundariesChecked: 12,
    boundaries: [
      { x: 100, y: 100, width: 80, height: 60, jpegGridAligned: true, noiseAsymmetric: true, featheringDetected: false, confidence: 0.72 },
    ],
    heatmapBase64: TINY_PNG_BASE64,
    summary: 'Composite-edge candidates',
  };
  r.nprResult = {
    score: 0.49,
    suspicious: true,
    hvCorrelation: 0.42,
    diffVarianceRatio: 1.8,
    hfEnergyRatio: 0.91,
    heatmapBase64: TINY_PNG_BASE64,
    summary: 'Pixel correlations atypical',
  };
  r.dctAnalysisResult = {
    score: 0.44,
    suspicious: true,
    acCoefficientOfVariation: 0.65,
    dcStd: 12.3,
    acMean: 8.1,
    acStd: 5.8,
    heatmapBase64: TINY_PNG_BASE64,
    summary: 'Mixed compression energy across blocks',
  };
  r.fourierAnalysisResult = {
    score: 0.52,
    suspicious: true,
    peakCount: 28,
    spectrumBase64: TINY_PNG_BASE64,
    summary: 'Periodic spectral peaks',
  };
  return r;
}

// All clean — tests that conditional captions switch to the
// "no anomalies detected" copy and that clean-state hints render.
function allCleanFixture(): MockVerificationResult {
  const r = baseResult();
  r.overallTrust = 0.85;
  r.elaResult = {
    score: 0.05,
    suspicious: false,
    elaImageBase64: TINY_PNG_BASE64,
    summary: 'No compression anomalies',
  };
  r.noiseResult = {
    score: 0.31,
    suspicious: false,
    anomalousBlocks: 0,
    totalBlocks: 64,
    summary: 'Borderline noise distribution',
  };
  r.copyMoveResult = {
    score: 0,
    suspicious: false,
    cloneRegions: [],
    visualisationBase64: TINY_PNG_BASE64,
    summary: 'No cloned regions',
  };
  r.jpegGhostResult = {
    score: 0,
    suspicious: false,
    ghostImageBase64: TINY_PNG_BASE64,
    summary: 'Uniform compression history',
  };
  return r;
}

// The exact pilot scenario reported 2026-04-28: AI fish image with
// clean Copy-Move (score 0% with visual present), Segmented ELA at
// 46% (5/64 regions), Noise Pattern at 31% (no visual).
function pilotFishFixture(): MockVerificationResult {
  const r = baseResult();
  r.overallTrust = 0.32;
  r.elaResult = {
    score: 0.18,
    suspicious: false,
    elaImageBase64: TINY_PNG_BASE64,
    summary: 'No compression anomalies',
  };
  r.noiseResult = {
    score: 0.31,
    suspicious: false,
    anomalousBlocks: 4,
    totalBlocks: 64,
    summary: 'Slightly irregular noise distribution',
  };
  r.copyMoveResult = {
    score: 0,
    suspicious: false,
    cloneRegions: [],
    visualisationBase64: TINY_PNG_BASE64,
    summary: 'No cloned regions',
  };
  r.jpegGhostResult = {
    score: 0,
    suspicious: false,
    ghostImageBase64: TINY_PNG_BASE64,
    summary: 'Uniform compression history',
  };
  r.segmentedElaResult = {
    score: 0.46,
    suspicious: true,
    anomalousRegions: 5,
    totalRegions: 64,
    visualizationBase64: TINY_PNG_BASE64,
    summary: 'Localised compression anomaly',
  };
  r.colourTemperatureResult = {
    score: 0,
    suspicious: false,
    anomalousRegions: 0,
    totalRegions: 16,
    globalMeanA: 0.5,
    globalMeanB: 0.3,
    heatmapBase64: TINY_PNG_BASE64,
    summary: 'Colour balance consistent',
  };
  return r;
}

// ──────────────────────────────────────────────────────────────────
// Page injection helper.
// ──────────────────────────────────────────────────────────────────

async function injectAndOpenIntegrityCard(
  page: Page,
  result: MockVerificationResult,
): Promise<void> {
  await page.goto('/verify');
  await page.waitForSelector('h1');
  await page.waitForFunction(() => !!(window as any).__juraSetVerifyResult, {
    timeout: 5000,
  });

  await page.evaluate((r) => {
    (window as any).__juraSetVerifyResult(r);
  }, result);

  // The integrity card is collapsed by default — click to expand.
  const integrityCardButton = page.locator(
    'button[aria-controls="card-integrity-body"]',
  );
  await integrityCardButton.waitFor({ state: 'visible', timeout: 8000 });
  // Only click if not already expanded (idempotent).
  const expanded = await integrityCardButton.getAttribute('aria-expanded');
  if (expanded !== 'true') {
    await integrityCardButton.click();
  }
  // Body must be visible
  await page.waitForSelector('#card-integrity-body', { timeout: 5000 });
}

// ──────────────────────────────────────────────────────────────────
// Test setup
// ──────────────────────────────────────────────────────────────────

test.describe('Integrity card — detector rows', () => {
  test.use({ viewport: { width: 1280, height: 900 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
      localStorage.setItem('jura-setup-complete', 'true');
    });
  });

  // ── Help-icon presence (regression net for the row-chrome refactor) ──

  test('every detector row has a help icon linking to /help/forensic-detectors', async ({ page }) => {
    await injectAndOpenIntegrityCard(page, allSuspiciousFixture());

    const cardBody = page.locator('#card-integrity-body');
    // Each of the 11 detectors should expose a `?` help link with the
    // expected anchor.  Absence of any link is a regression.
    const expectedAnchors = [
      'ela',
      'noise-pattern',
      'copy-move',
      'jpeg-ghost',
      'segmented-ela',
      'colour-temperature',
      'shadow-consistency',
      'splice-boundary',
      'npr',
      'dct-analysis',
      'fourier-analysis',
    ];

    for (const anchor of expectedAnchors) {
      const link = cardBody.locator(`a[href="/help/forensic-detectors#${anchor}"]`);
      await expect(link, `help icon for ${anchor} should exist`).toHaveCount(1);
      await expect(link).toBeVisible();
      // ARIA: must have an aria-label so screen readers announce purpose.
      await expect(link).toHaveAttribute('aria-label', /.+/);
    }
  });

  // ── Copy-Move clean-state hint (the active misreading we just fixed) ──

  test('Copy-Move at 0% renders "No cloned regions detected." when visualisation present', async ({ page }) => {
    await injectAndOpenIntegrityCard(page, pilotFishFixture());

    const cardBody = page.locator('#card-integrity-body');
    await expect(cardBody.getByText('Copy-Move Detection')).toBeVisible();
    // Clean-state hint must appear (added by Scope A 2026-04-28).
    await expect(cardBody.getByText('No cloned regions detected.')).toBeVisible();
    // The misleading caption must NOT appear when score is 0.
    await expect(
      cardBody.getByText(/matched coloured pairs join the cloned regions/),
    ).toHaveCount(0);
  });

  // ── Caption conditionality ──

  test('ImageZoom caption switches to "no anomalies" copy when not suspicious', async ({ page }) => {
    await injectAndOpenIntegrityCard(page, allCleanFixture());

    const cardBody = page.locator('#card-integrity-body');
    // ELA clean caption
    await expect(cardBody.getByText(/no significant compression anomalies detected/)).toBeVisible();
    // Copy-Move clean caption (under the visualisation, not the row hint)
    await expect(cardBody.getByText(/no cloned regions detected, image shown unmarked/)).toBeVisible();
    // JPEG Ghost clean caption
    await expect(cardBody.getByText(/no compression-history anomalies detected/)).toBeVisible();
  });

  test('ImageZoom caption uses suspicious wording when flagged', async ({ page }) => {
    await injectAndOpenIntegrityCard(page, allSuspiciousFixture());

    const cardBody = page.locator('#card-integrity-body');
    // ELA suspicious caption
    await expect(cardBody.getByText(/bright regions indicate higher compression-error mismatch/)).toBeVisible();
    // Copy-Move suspicious caption
    await expect(cardBody.getByText(/matched coloured pairs join the cloned regions/)).toBeVisible();
    // Segmented ELA suspicious caption
    await expect(cardBody.getByText(/flagged blocks show locally anomalous compression error/)).toBeVisible();
  });

  // ── Always-visible explanation lines (visualisation-less detectors) ──

  test('Noise Pattern row always shows its plain-English explanation', async ({ page }) => {
    await injectAndOpenIntegrityCard(page, pilotFishFixture());

    const cardBody = page.locator('#card-integrity-body');
    // The always-visible explanation line must be present even when
    // the row is not suspicious — addresses the "31% with no
    // visualisation, no context" pilot complaint.
    await expect(
      cardBody.getByText(
        /Measures whether noise distribution is uniform across the photo/,
      ),
    ).toBeVisible();
  });

  // ── Segmented ELA copy update ──

  test('Segmented ELA flagged copy uses the new "image blocks" wording', async ({ page }) => {
    await injectAndOpenIntegrityCard(page, pilotFishFixture());

    const cardBody = page.locator('#card-integrity-body');
    await expect(
      cardBody.getByText(/5 of 64 image blocks show unusual compression/),
    ).toBeVisible();
  });

  // ── Score rendering invariant ──

  test('every active detector row shows a percentage score', async ({ page }) => {
    await injectAndOpenIntegrityCard(page, allSuspiciousFixture());

    const cardBody = page.locator('#card-integrity-body');
    // 11 detector rows × 1 percentage element each.  Use the
    // tabular-nums + % suffix selector to count score elements.
    const scoreCells = cardBody.locator('span.tabular-nums', { hasText: '%' });
    // Allow some slack — there may be additional %ages in raw-scores
    // mode (currently off by default).
    const count = await scoreCells.count();
    expect(count).toBeGreaterThanOrEqual(11);
  });

  // ── Help-page integration: the anchor target must exist ──

  test('forensic-detectors help page renders with all expected anchors', async ({ page }) => {
    await page.goto('/help/forensic-detectors');
    await page.waitForSelector('h1');

    const expectedAnchors = [
      'ela',
      'noise-pattern',
      'copy-move',
      'jpeg-ghost',
      'segmented-ela',
      'colour-temperature',
      'shadow-consistency',
      'splice-boundary',
      'npr',
      'dct-analysis',
      'fourier-analysis',
    ];

    for (const anchor of expectedAnchors) {
      const section = page.locator(`section#${anchor}`);
      await expect(
        section,
        `help section ${anchor} should exist`,
      ).toHaveCount(1);
    }
  });

  // ── Keyboard reachability of help icons (a11y baseline) ──

  test('help icons are focusable via keyboard', async ({ page }) => {
    await injectAndOpenIntegrityCard(page, allSuspiciousFixture());

    const cardBody = page.locator('#card-integrity-body');
    const firstHelpLink = cardBody
      .locator('a[href="/help/forensic-detectors#ela"]')
      .first();
    await firstHelpLink.focus();
    await expect(firstHelpLink).toBeFocused();
  });
});
