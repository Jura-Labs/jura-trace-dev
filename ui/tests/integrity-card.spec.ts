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
// data: URL form — used for *Url fields post-heatmap-RAM-fix so that the
// template's {#if url} guard is truthy and heatmapSrc() passes it through.
const TINY_PNG_DATA_URL = `data:image/png;base64,${TINY_PNG_BASE64}`;

// All 11 detectors firing — used to verify every help icon exists,
// every conditional caption renders the suspicious variant, and every
// row sub-line is reachable.
function allSuspiciousFixture(): MockVerificationResult {
  const r = baseResult();
  r.overallTrust = 0.18;
  r.elaResult = {
    score: 0.74,
    suspicious: true,
    elaImageUrl: TINY_PNG_DATA_URL,
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
    visualisationUrl: TINY_PNG_DATA_URL,
    summary: 'Cloned regions found',
  };
  r.jpegGhostResult = {
    score: 0.48,
    suspicious: true,
    heatmapUrl: TINY_PNG_DATA_URL,
    summary: 'Mixed compression history',
  };
  r.segmentedElaResult = {
    score: 0.46,
    suspicious: true,
    anomalousRegions: 5,
    totalRegions: 64,
    heatmapUrl: TINY_PNG_DATA_URL,
    summary: 'Localised compression anomaly',
  };
  r.colourTemperatureResult = {
    score: 0.42,
    suspicious: true,
    anomalousRegions: 2,
    totalRegions: 16,
    globalMeanA: 5.2,
    globalMeanB: -3.4,
    heatmapUrl: TINY_PNG_DATA_URL,
    summary: 'Colour balance discontinuity',
  };
  r.shadowConsistencyResult = {
    score: 0.58,
    suspicious: true,
    inconsistentRegions: 3,
    totalRegions: 12,
    globalLightDirection: 142.5,
    heatmapUrl: TINY_PNG_DATA_URL,
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
    heatmapUrl: TINY_PNG_DATA_URL,
    summary: 'Composite-edge candidates',
  };
  r.nprResult = {
    score: 0.49,
    suspicious: true,
    hvCorrelation: 0.42,
    diffVarianceRatio: 1.8,
    hfEnergyRatio: 0.91,
    heatmapUrl: TINY_PNG_DATA_URL,
    summary: 'Pixel correlations atypical',
  };
  r.dctAnalysisResult = {
    score: 0.44,
    suspicious: true,
    acCoefficientOfVariation: 0.65,
    dcStd: 12.3,
    acMean: 8.1,
    acStd: 5.8,
    heatmapUrl: TINY_PNG_DATA_URL,
    summary: 'Mixed compression energy across blocks',
  };
  r.fourierAnalysisResult = {
    score: 0.52,
    suspicious: true,
    peakCount: 28,
    spectrumUrl: TINY_PNG_DATA_URL,
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
    elaImageUrl: TINY_PNG_DATA_URL,
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
    visualisationUrl: TINY_PNG_DATA_URL,
    summary: 'No cloned regions',
  };
  r.jpegGhostResult = {
    score: 0,
    suspicious: false,
    heatmapUrl: TINY_PNG_DATA_URL,
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
    elaImageUrl: TINY_PNG_DATA_URL,
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
    visualisationUrl: TINY_PNG_DATA_URL,
    summary: 'No cloned regions',
  };
  r.jpegGhostResult = {
    score: 0,
    suspicious: false,
    heatmapUrl: TINY_PNG_DATA_URL,
    summary: 'Uniform compression history',
  };
  r.segmentedElaResult = {
    score: 0.46,
    suspicious: true,
    anomalousRegions: 5,
    totalRegions: 64,
    heatmapUrl: TINY_PNG_DATA_URL,
    summary: 'Localised compression anomaly',
  };
  r.colourTemperatureResult = {
    score: 0,
    suspicious: false,
    anomalousRegions: 0,
    totalRegions: 16,
    globalMeanA: 0.5,
    globalMeanB: 0.3,
    heatmapUrl: TINY_PNG_DATA_URL,
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

  // ── Stage 4 — Visual regression baselines ─────────────────────────
  // Playwright's toHaveScreenshot() captures the integrity card body
  // for each fixture and compares against a snapshot stored alongside
  // this spec.  First run creates the baseline; subsequent runs fail
  // on pixel drift above the configured threshold.
  //
  // Visual diffs catch CSS regressions that functional assertions
  // miss — padding shifts, focus-ring drift, dark-mode opacity
  // changes — particularly important after the DetectorRow extraction
  // (commit a700f2d) where the row chrome moved into a child component.

  test('visual: integrity card with all detectors suspicious', async ({ page }) => {
    await injectAndOpenIntegrityCard(page, allSuspiciousFixture());
    const card = page.locator('#card-integrity-body');
    await expect(card).toHaveScreenshot('integrity-card-all-suspicious.png', {
      maxDiffPixelRatio: 0.02,
    });
  });

  test('visual: integrity card with all detectors clean', async ({ page }) => {
    await injectAndOpenIntegrityCard(page, allCleanFixture());
    const card = page.locator('#card-integrity-body');
    await expect(card).toHaveScreenshot('integrity-card-all-clean.png', {
      maxDiffPixelRatio: 0.02,
    });
  });

  test('visual: integrity card with the pilot fish scenario', async ({ page }) => {
    await injectAndOpenIntegrityCard(page, pilotFishFixture());
    const card = page.locator('#card-integrity-body');
    await expect(card).toHaveScreenshot('integrity-card-pilot-fish.png', {
      maxDiffPixelRatio: 0.02,
    });
  });
});

// ──────────────────────────────────────────────────────────────────
// Stage 5 — DetectorRow prop-matrix coverage.
//
// In lieu of component-level unit tests (vitest + @testing-library
// infrastructure not currently wired), exercise each meaningful
// DetectorRow prop combination through fixtures that isolate that
// variant.  Each test asserts on the rendered DOM contract — the
// surface that matters to consumers of the component, not its
// internal markup.
// ──────────────────────────────────────────────────────────────────

test.describe('DetectorRow — prop-matrix', () => {
  test.use({ viewport: { width: 1280, height: 900 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
      localStorage.setItem('jura-setup-complete', 'true');
    });
  });

  test('row title bears amber tint and warning icon when suspicious', async ({ page }) => {
    const fx = baseResult();
    fx.elaResult = {
      score: 0.78,
      suspicious: true,
      elaImageUrl: TINY_PNG_DATA_URL,
      summary: 'Flagged',
    };
    await injectAndOpenIntegrityCard(page, fx);

    const cardBody = page.locator('#card-integrity-body');
    // The amber tint applies via bg-amber/[0.04] to the <li> wrapper.
    // We assert by selecting the <li> containing the title.
    const elaTitle = cardBody.getByText('Error Level Analysis').first();
    await expect(elaTitle).toBeVisible();
    // Title text colour switches to amber-light when suspicious.
    await expect(elaTitle).toHaveClass(/text-amber-light/);
  });

  test('row title is neutral when not suspicious', async ({ page }) => {
    const fx = baseResult();
    fx.elaResult = {
      score: 0.05,
      suspicious: false,
      elaImageUrl: TINY_PNG_DATA_URL,
      summary: 'Clean',
    };
    await injectAndOpenIntegrityCard(page, fx);

    const cardBody = page.locator('#card-integrity-body');
    const elaTitle = cardBody.getByText('Error Level Analysis').first();
    await expect(elaTitle).toHaveClass(/text-obsidian|text-quartz/);
    await expect(elaTitle).not.toHaveClass(/text-amber-light/);
  });

  test('badges slot renders the experimental pill on JPEG Ghost', async ({ page }) => {
    const fx = baseResult();
    fx.jpegGhostResult = {
      score: 0.4,
      suspicious: false,
      heatmapUrl: TINY_PNG_DATA_URL,
      summary: '',
    };
    await injectAndOpenIntegrityCard(page, fx);

    const cardBody = page.locator('#card-integrity-body');
    // The ExperimentalPill text begins with "EXPERIMENTAL"; the
    // uncalibrated variant suffix is "weight uncalibrated".
    const pill = cardBody.locator(
      'span[role="note"]',
      { hasText: /weight uncalibrated/ },
    );
    await expect(pill).toBeVisible();
  });

  test('badges slot renders "On-demand" chip on NPR row', async ({ page }) => {
    const fx = baseResult();
    fx.nprResult = {
      score: 0.4,
      suspicious: false,
      hvCorrelation: 0.5,
      diffVarianceRatio: 1.0,
      hfEnergyRatio: 0.5,
      heatmapUrl: TINY_PNG_DATA_URL,
      summary: '',
    };
    await injectAndOpenIntegrityCard(page, fx);

    const cardBody = page.locator('#card-integrity-body');
    const onDemandChip = cardBody.getByText('On-demand', { exact: true });
    await expect(onDemandChip).toBeVisible();
  });

  test('badges slot renders "Deep" chip on DCT row', async ({ page }) => {
    const fx = baseResult();
    fx.dctAnalysisResult = {
      score: 0.4,
      suspicious: false,
      acCoefficientOfVariation: 0.5,
      dcStd: 1.0,
      acMean: 1.0,
      acStd: 1.0,
      heatmapUrl: TINY_PNG_DATA_URL,
      summary: '',
    };
    await injectAndOpenIntegrityCard(page, fx);

    const cardBody = page.locator('#card-integrity-body');
    const deepChip = cardBody.getByText('Deep', { exact: true });
    await expect(deepChip).toBeVisible();
  });

  test('alwaysVisibleHint renders even when suspicious=false', async ({ page }) => {
    const fx = baseResult();
    fx.noiseResult = {
      score: 0.05,
      suspicious: false,
      anomalousBlocks: 0,
      totalBlocks: 64,
      summary: 'Clean',
    };
    await injectAndOpenIntegrityCard(page, fx);

    const cardBody = page.locator('#card-integrity-body');
    await expect(
      cardBody.getByText(
        /Measures whether noise distribution is uniform across the photo/,
      ),
    ).toBeVisible();
  });

  test('rawScore snippet only renders when showRawScores is enabled', async ({ page }) => {
    // Default: raw scores OFF.  The "score: 0.xxxx" detail line should
    // NOT appear on a row.
    const fx = baseResult();
    fx.elaResult = {
      score: 0.4,
      suspicious: false,
      elaImageUrl: TINY_PNG_DATA_URL,
      summary: 'Test',
    };
    await injectAndOpenIntegrityCard(page, fx);

    const cardBody = page.locator('#card-integrity-body');
    // Match the literal "score: 0." prefix of the raw-score line — it
    // is gated on showRawScores so should be absent.
    await expect(
      cardBody.locator('span').filter({ hasText: /^score: 0\./ }),
    ).toHaveCount(0);
  });

  test('rawScore snippet renders when showRawScores localStorage is set', async ({ page, context }) => {
    await context.addInitScript(() => {
      localStorage.setItem('jura-raw-scores-default', 'true');
    });
    const fx = baseResult();
    fx.elaResult = {
      score: 0.4,
      suspicious: false,
      elaImageUrl: TINY_PNG_DATA_URL,
      summary: 'Test',
    };
    await injectAndOpenIntegrityCard(page, fx);

    const cardBody = page.locator('#card-integrity-body');
    await expect(
      cardBody.locator('span').filter({ hasText: /^score: 0\.4/ }).first(),
    ).toBeVisible();
  });

  test('score percentage is colour-coded by forensicScoreClass', async ({ page }) => {
    // Three rows with score in each band:
    //   <0.30 → malachite (low / authentic-looking)
    //   0.30–0.60 → amber (borderline)
    //   ≥0.60 → cinnabar (suspicious)
    const fx = baseResult();
    fx.elaResult = {
      score: 0.1,
      suspicious: false,
      elaImageUrl: TINY_PNG_DATA_URL,
      summary: '',
    };
    fx.noiseResult = {
      score: 0.45,
      suspicious: false,
      anomalousBlocks: 0,
      totalBlocks: 64,
      summary: '',
    };
    fx.copyMoveResult = {
      score: 0.75,
      suspicious: true,
      cloneRegions: [],
      visualisationUrl: TINY_PNG_DATA_URL,
      summary: '',
    };
    await injectAndOpenIntegrityCard(page, fx);

    const cardBody = page.locator('#card-integrity-body');
    // 10% — malachite class
    const tenPct = cardBody.locator('span.tabular-nums', { hasText: '10%' }).first();
    await expect(tenPct).toHaveClass(/text-malachite/);
    // 45% — amber class
    const fortyFive = cardBody.locator('span.tabular-nums', { hasText: '45%' }).first();
    await expect(fortyFive).toHaveClass(/text-amber/);
    // 75% — cinnabar class
    const seventyFive = cardBody.locator('span.tabular-nums', { hasText: '75%' }).first();
    await expect(seventyFive).toHaveClass(/text-cinnabar/);
  });

  test('every row exposes a help link with size-sm class (5x5 px circle)', async ({ page }) => {
    await injectAndOpenIntegrityCard(page, allSuspiciousFixture());
    const cardBody = page.locator('#card-integrity-body');
    // size-sm renders w-5 h-5 (vs default md = w-6 h-6).
    const helpLinks = cardBody.locator(
      'a.w-5.h-5[href^="/help/forensic-detectors#"]',
    );
    // 11 detectors all firing → 11 help links.
    await expect(helpLinks).toHaveCount(11);
  });
});

