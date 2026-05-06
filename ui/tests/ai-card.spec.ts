// SPDX-License-Identifier: AGPL-3.0-or-later

import { test, expect } from '@playwright/test';
import type { Page } from '@playwright/test';

/**
 * AI card regression suite.
 *
 * Counterpart to integrity-card.spec.ts.  Exercises the
 * "Is this AI-generated?" card on /verify, covering:
 *   - panel-level state-label and Pass/Concern derivation across
 *     verdict combinations
 *   - the plain-English aiVerdictSentence (JTV-89) for each state
 *   - GBM Deepfake row chrome via AiDetectorRow + bespoke body
 *     content (signals accordion, bridging sentence, threshold
 *     display, raw-score gating)
 *   - CLIP/UnivFD row chrome + zero-shot bar toggle (JTV-86)
 *   - aiDetectionSuppressed branch (screenshot-class content)
 *   - visual regression baselines for each state
 *
 * Fixtures injected via the dev-only `__juraSetVerifyResult` window
 * hook; no live sidecar required.
 */

// ──────────────────────────────────────────────────────────────────
// Fixture builders
// ──────────────────────────────────────────────────────────────────

interface MockVerificationResult {
  mode: string;
  sourceType: string;
  contentType: string;
  overallTrust: number;
  metadataFlags: string[];
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
  contentTypeResult: any;
  detectorsRun: string[];
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
    contentTypeResult: null,
    detectorsRun: [],
  };
}

const TINY_PNG_BASE64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkAAIAAAoAAv/' +
  'lxKUAAAAASUVORK5CYII=';
const TINY_PNG_DATA_URL = `data:image/png;base64,${TINY_PNG_BASE64}`;

interface GbmOpts {
  score?: number;
  suspicious?: boolean;
  verdictLevel?: 'synthetic' | 'inconclusive' | 'authentic';
  confidence?: 'high' | 'medium' | 'low';
  triggeredCount?: number;
  totalSignals?: number;
}

function gbmFixture(opts: GbmOpts = {}): any {
  const total = opts.totalSignals ?? 19;
  const triggered = opts.triggeredCount ?? 0;
  const signals = Array.from({ length: total }, (_, i) => ({
    name: `signal_${i}`,
    description: `Signal ${i} description`,
    weight: 1.0,
    triggered: i < triggered,
  }));
  return {
    score: opts.score ?? 0.05,
    suspicious: opts.suspicious ?? false,
    confidence: opts.confidence ?? 'medium',
    verdictLevel: opts.verdictLevel ?? 'authentic',
    signals,
    heatmapUrl: TINY_PNG_DATA_URL,
    summary: 'GBM detector summary',
    classifierScore: opts.score ?? 0.05,
    classifierAvailable: true,
    verdictThresholds: {
      syntheticMin: 0.55,
      authenticMax: 0.25,
      modelVersion: 'gbm-v4',
      thresholdBasis: 'Option C calibration',
    },
  };
}

interface ClipOpts {
  score?: number;
  verdictLevel?: 'synthetic' | 'inconclusive' | 'authentic';
  confidence?: 'high' | 'medium' | 'low';
  univfdAvailable?: boolean;
}

function clipFixture(opts: ClipOpts = {}): any {
  return {
    score: opts.score ?? 0.05,
    verdictLevel: opts.verdictLevel ?? 'authentic',
    confidence: opts.confidence ?? 'medium',
    classProbs: {
      photograph: 0.21,
      real_scene: 0.21,
      manipulated: 0.20,
      ai_generated: 0.20,
      synthetic: 0.18,
    },
    summary: 'CLIP detector summary',
    univfdScore: opts.univfdAvailable === false ? null : opts.score ?? 0.05,
    univfdAvailable: opts.univfdAvailable !== false,
  };
}

// ──────────────────────────────────────────────────────────────────
// Page injection helper.
// ──────────────────────────────────────────────────────────────────

async function injectAndOpenAiCard(
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

  const aiCardButton = page.locator(
    'button[aria-controls="card-ai-body"]',
  );
  await aiCardButton.waitFor({ state: 'visible', timeout: 8000 });
  const expanded = await aiCardButton.getAttribute('aria-expanded');
  if (expanded !== 'true') {
    await aiCardButton.click();
  }
  await page.waitForSelector('#card-ai-body', { timeout: 5000 });
}

// ──────────────────────────────────────────────────────────────────
// Card header — state-label + Pass/Concern derivation
// ──────────────────────────────────────────────────────────────────

test.describe('AI card — header', () => {
  test.use({ viewport: { width: 1280, height: 900 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
      localStorage.setItem('jura-setup-complete', 'true');
    });
  });

  test('"Both detectors flagged AI" + Concern when both verdicts synthetic', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({
      score: 0.78,
      suspicious: true,
      verdictLevel: 'synthetic',
      confidence: 'high',
    });
    r.clipResult = clipFixture({
      score: 0.87,
      verdictLevel: 'synthetic',
      confidence: 'high',
    });
    await injectAndOpenAiCard(page, r);

    const header = page.locator('button[aria-controls="card-ai-body"]');
    await expect(header).toContainText('Both detectors flagged AI');
    await expect(header).toContainText('Concern');
  });

  test('"Both detectors clear" + Pass when both verdicts authentic', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({
      score: 0.05,
      suspicious: false,
      verdictLevel: 'authentic',
    });
    r.clipResult = clipFixture({
      score: 0.05,
      verdictLevel: 'authentic',
    });
    await injectAndOpenAiCard(page, r);

    const header = page.locator('button[aria-controls="card-ai-body"]');
    await expect(header).toContainText('Both detectors clear');
    await expect(header).toContainText('Pass');
  });

  test('"Detectors disagree" when GBM clear but CLIP synthetic', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({
      score: 0.05,
      suspicious: false,
      verdictLevel: 'authentic',
    });
    r.clipResult = clipFixture({
      score: 0.87,
      verdictLevel: 'synthetic',
    });
    await injectAndOpenAiCard(page, r);

    const header = page.locator('button[aria-controls="card-ai-body"]');
    await expect(header).toContainText('Detectors disagree');
  });

  test('"Suppressed" badge replaces state-label when content type unsuitable', async ({ page }) => {
    const r = baseResult();
    r.contentTypeResult = {
      contentType: 'screenshot',
      confidence: 0.95,
      aiDetectionSuitable: false,
      reasoning: 'Screenshot — AI detection unreliable',
    };
    r.deepfakeResult = gbmFixture({ score: 0.5, suspicious: false });
    r.clipResult = clipFixture({ score: 0.5, verdictLevel: 'inconclusive' });
    await injectAndOpenAiCard(page, r);

    const header = page.locator('button[aria-controls="card-ai-body"]');
    await expect(header).toContainText('Suppressed');
    // The state-label and Pass/Concern badges should NOT appear.
    await expect(header).not.toContainText('Both detectors');
    await expect(header).not.toContainText('Detectors disagree');
  });

  test('panel header exposes ContextualHelpLink to /help/how-it-works', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture();
    r.clipResult = clipFixture();
    await injectAndOpenAiCard(page, r);

    const helpLink = page.locator(
      'a[href="/help/how-it-works#two-ai-checks"]',
    ).first();
    await expect(helpLink).toBeVisible();
    await expect(helpLink).toHaveAttribute('aria-label', /Why two AI checks/);
  });
});

// ──────────────────────────────────────────────────────────────────
// Verdict sentence (JTV-89)
// ──────────────────────────────────────────────────────────────────

test.describe('AI card — verdict sentence', () => {
  test.use({ viewport: { width: 1280, height: 900 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
      localStorage.setItem('jura-setup-complete', 'true');
    });
  });

  test('renders "consistent with AI-generated" when both detectors synthetic', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({ score: 0.78, suspicious: true, verdictLevel: 'synthetic' });
    r.clipResult = clipFixture({ score: 0.87, verdictLevel: 'synthetic' });
    await injectAndOpenAiCard(page, r);

    const sentence = page.locator('[data-testid="ai-verdict-sentence"]');
    await expect(sentence).toBeVisible();
    await expect(sentence).toContainText(
      /Both AI checks identified characteristics consistent with AI-generated content/,
    );
  });

  test('renders "consistent with a real photograph" when both authentic', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({ score: 0.05, suspicious: false, verdictLevel: 'authentic' });
    r.clipResult = clipFixture({ score: 0.05, verdictLevel: 'authentic' });
    await injectAndOpenAiCard(page, r);

    const sentence = page.locator('[data-testid="ai-verdict-sentence"]');
    await expect(sentence).toContainText(
      /Both AI checks indicate this is consistent with a real photograph/,
    );
  });

  test('renders "AI checks disagree" sentence when verdicts diverge', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({ score: 0.05, suspicious: false, verdictLevel: 'authentic' });
    r.clipResult = clipFixture({ score: 0.87, verdictLevel: 'synthetic' });
    await injectAndOpenAiCard(page, r);

    const sentence = page.locator('[data-testid="ai-verdict-sentence"]');
    await expect(sentence).toContainText(/AI checks disagree/);
    await expect(sentence).toContainText(/Treat the result as inconclusive/);
  });

  test('verdict sentence is the FIRST element in the card body', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({ score: 0.78, suspicious: true, verdictLevel: 'synthetic' });
    r.clipResult = clipFixture({ score: 0.87, verdictLevel: 'synthetic' });
    await injectAndOpenAiCard(page, r);

    // First child of #card-ai-body should be the verdict sentence (when
    // not suppressed).  This is the JTV-89 placement contract.
    const firstP = page.locator('#card-ai-body > p').first();
    await expect(firstP).toHaveAttribute('data-testid', 'ai-verdict-sentence');
  });

  test('NO verdict sentence when aiDetectionSuppressed', async ({ page }) => {
    const r = baseResult();
    r.contentTypeResult = {
      contentType: 'screenshot',
      confidence: 0.95,
      aiDetectionSuitable: false,
      reasoning: 'Screenshot',
    };
    r.deepfakeResult = gbmFixture();
    r.clipResult = clipFixture();
    await injectAndOpenAiCard(page, r);

    await expect(
      page.locator('[data-testid="ai-verdict-sentence"]'),
    ).toHaveCount(0);
    // Instead the suppression message should be shown.
    await expect(
      page.locator('#card-ai-body').getByText(
        /AI detection signals have been suppressed/,
      ),
    ).toBeVisible();
  });
});

// ──────────────────────────────────────────────────────────────────
// GBM Deepfake row
// ──────────────────────────────────────────────────────────────────

test.describe('AI card — GBM Deepfake row', () => {
  test.use({ viewport: { width: 1280, height: 900 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
      localStorage.setItem('jura-setup-complete', 'true');
    });
  });

  test('confidence chip and threshold display on GBM row', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({ score: 0.73, suspicious: true, verdictLevel: 'synthetic', confidence: 'high' });
    await injectAndOpenAiCard(page, r);

    const cardBody = page.locator('#card-ai-body');
    await expect(cardBody.getByText('AI Generation (GBM Deepfake)')).toBeVisible();
    await expect(cardBody.getByText('high confidence')).toBeVisible();
    // Threshold from verdictThresholds.syntheticMin (0.55 → 55%).
    await expect(cardBody.getByText('threshold 55%')).toBeVisible();
  });

  test('bridging sentence appears when score>threshold but no signals triggered', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({
      score: 0.73,
      suspicious: true,
      verdictLevel: 'synthetic',
      triggeredCount: 0,
      totalSignals: 19,
    });
    await injectAndOpenAiCard(page, r);

    const cardBody = page.locator('#card-ai-body');
    await expect(
      cardBody.getByText(
        /Score reflects statistical patterns across the model's 84-feature vector/,
      ),
    ).toBeVisible();
    await expect(
      cardBody.getByText(/None of the 19 named indicators triggered individually/),
    ).toBeVisible();
  });

  test('NO bridging sentence when at least one signal triggered', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({
      score: 0.73,
      suspicious: true,
      verdictLevel: 'synthetic',
      triggeredCount: 3,
      totalSignals: 19,
    });
    await injectAndOpenAiCard(page, r);

    const cardBody = page.locator('#card-ai-body');
    await expect(
      cardBody.getByText(/Score reflects statistical patterns/),
    ).toHaveCount(0);
  });

  test('signals accordion label adapts to triggered count', async ({ page }) => {
    // 0 triggered → "19 named indicators (none triggered)"
    const r0 = baseResult();
    r0.deepfakeResult = gbmFixture({
      score: 0.73,
      suspicious: true,
      verdictLevel: 'synthetic',
      triggeredCount: 0,
      totalSignals: 19,
    });
    await injectAndOpenAiCard(page, r0);
    await expect(
      page.locator('#card-ai-body').getByText(
        /19 named indicators \(none triggered\)/,
      ),
    ).toBeVisible();
  });

  test('signals accordion label "X of N triggered" when N>0 triggered', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({
      score: 0.73,
      suspicious: true,
      verdictLevel: 'synthetic',
      triggeredCount: 3,
      totalSignals: 19,
    });
    await injectAndOpenAiCard(page, r);

    await expect(
      page.locator('#card-ai-body').getByText(/3 of 19 indicators triggered/),
    ).toBeVisible();
  });

  test('verdict text gated behind raw scores by default', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({
      score: 0.73,
      suspicious: true,
      verdictLevel: 'synthetic',
    });
    await injectAndOpenAiCard(page, r);

    // "Verdict: Synthetic" cinnabar text should NOT appear in default
    // view (gated behind showRawScores per JTV-91).
    const cardBody = page.locator('#card-ai-body');
    await expect(cardBody.getByText(/^Verdict: Synthetic$/)).toHaveCount(0);
  });

  test('verdict text appears when raw-scores localStorage is set', async ({ page, context }) => {
    await context.addInitScript(() => {
      localStorage.setItem('jura-raw-scores-default', 'true');
    });
    const r = baseResult();
    r.deepfakeResult = gbmFixture({
      score: 0.73,
      suspicious: true,
      verdictLevel: 'synthetic',
    });
    await injectAndOpenAiCard(page, r);

    await expect(
      page.locator('#card-ai-body').getByText(/Verdict: Synthetic/),
    ).toBeVisible();
  });
});

// ──────────────────────────────────────────────────────────────────
// CLIP / UnivFD row
// ──────────────────────────────────────────────────────────────────

test.describe('AI card — CLIP / UnivFD row', () => {
  test.use({ viewport: { width: 1280, height: 900 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
      localStorage.setItem('jura-setup-complete', 'true');
    });
  });

  test('experimental pill renders on CLIP row with correct help link', async ({ page }) => {
    const r = baseResult();
    r.clipResult = clipFixture({ score: 0.87, verdictLevel: 'synthetic' });
    await injectAndOpenAiCard(page, r);

    const cardBody = page.locator('#card-ai-body');
    // The pill text contains "EXPERIMENTAL" + "informational only".
    await expect(
      cardBody.getByText(/informational only/),
    ).toBeVisible();
    // Pill wraps a link to the AI checks help section.
    const pillLink = cardBody
      .locator('a[href="/help/how-it-works#two-ai-checks"]')
      .first();
    await expect(pillLink).toBeVisible();
  });

  test('zero-shot bars hidden by default when UnivFD probe is available', async ({ page }) => {
    const r = baseResult();
    r.clipResult = clipFixture({
      score: 0.87,
      verdictLevel: 'synthetic',
      univfdAvailable: true,
    });
    await injectAndOpenAiCard(page, r);

    const cardBody = page.locator('#card-ai-body');
    // The 5 zero-shot bar labels should NOT be visible by default.
    await expect(
      cardBody.locator('[aria-label="CLIP class probability distribution"]'),
    ).toHaveCount(0);
    // The toggle link should be present.
    await expect(
      cardBody.getByRole('button', {
        name: /Show zero-shot label distribution/,
      }),
    ).toBeVisible();
  });

  test('clicking the zero-shot toggle reveals the bars', async ({ page }) => {
    const r = baseResult();
    r.clipResult = clipFixture({
      score: 0.87,
      verdictLevel: 'synthetic',
      univfdAvailable: true,
    });
    await injectAndOpenAiCard(page, r);

    const cardBody = page.locator('#card-ai-body');
    await cardBody
      .getByRole('button', { name: /Show zero-shot label distribution/ })
      .click();

    // After click, the bars container appears.
    await expect(
      cardBody.locator('[aria-label="CLIP class probability distribution"]'),
    ).toBeVisible();
    // The Hide button replaces the Show toggle.
    await expect(
      cardBody.getByRole('button', { name: /Hide zero-shot label distribution/ }),
    ).toBeVisible();
  });

  test('zero-shot bars visible by default when UnivFD probe is NOT available', async ({ page }) => {
    const r = baseResult();
    r.clipResult = clipFixture({
      score: 0.4,
      verdictLevel: 'inconclusive',
      univfdAvailable: false,
    });
    await injectAndOpenAiCard(page, r);

    const cardBody = page.locator('#card-ai-body');
    // No probe → zero-shot is the headline signal → bars visible by default.
    await expect(
      cardBody.locator('[aria-label="CLIP class probability distribution"]'),
    ).toBeVisible();
    // The "Show zero-shot" toggle should NOT appear in this case.
    await expect(
      cardBody.getByRole('button', {
        name: /Show zero-shot label distribution/,
      }),
    ).toHaveCount(0);
  });

  test('zero-shot bars header copy adapts to UnivFD availability', async ({ page }) => {
    // With UnivFD: bars labelled "auxiliary, not used for score"
    const r = baseResult();
    r.clipResult = clipFixture({
      score: 0.87,
      verdictLevel: 'synthetic',
      univfdAvailable: true,
    });
    await injectAndOpenAiCard(page, r);

    const cardBody = page.locator('#card-ai-body');
    await cardBody
      .getByRole('button', { name: /Show zero-shot label distribution/ })
      .click();
    await expect(
      cardBody.getByText(/Zero-shot CLIP labels \(auxiliary, not used for score\)/),
    ).toBeVisible();
  });
});

// ──────────────────────────────────────────────────────────────────
// AiDetectorRow chrome (icon, tint, score colouring)
// ──────────────────────────────────────────────────────────────────

test.describe('AI card — AiDetectorRow chrome', () => {
  test.use({ viewport: { width: 1280, height: 900 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
      localStorage.setItem('jura-setup-complete', 'true');
    });
  });

  test('GBM row gets amber-light title when suspicious', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({
      score: 0.73,
      suspicious: true,
      verdictLevel: 'synthetic',
    });
    await injectAndOpenAiCard(page, r);

    const title = page
      .locator('#card-ai-body')
      .getByText('AI Generation (GBM Deepfake)')
      .first();
    await expect(title).toHaveClass(/text-amber-light/);
  });

  test('CLIP row gets amber-light title when verdictLevel synthetic', async ({ page }) => {
    const r = baseResult();
    r.clipResult = clipFixture({ score: 0.87, verdictLevel: 'synthetic' });
    await injectAndOpenAiCard(page, r);

    const title = page
      .locator('#card-ai-body')
      .getByText('CLIP / UnivFD Probe')
      .first();
    await expect(title).toHaveClass(/text-amber-light/);
  });

  test('CLIP row gets neutral title when verdictLevel inconclusive', async ({ page }) => {
    const r = baseResult();
    r.clipResult = clipFixture({ score: 0.45, verdictLevel: 'inconclusive' });
    await injectAndOpenAiCard(page, r);

    const title = page
      .locator('#card-ai-body')
      .getByText('CLIP / UnivFD Probe')
      .first();
    // Inconclusive maps to suspicious=false in the AI card's
    // current convention (the verdict text is the differentiator).
    await expect(title).not.toHaveClass(/text-amber-light/);
  });
});

// ──────────────────────────────────────────────────────────────────
// Visual regression baselines
// ──────────────────────────────────────────────────────────────────

test.describe('AI card — visual regression', () => {
  test.use({ viewport: { width: 1280, height: 900 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
      localStorage.setItem('jura-setup-complete', 'true');
    });
  });

  test('visual: AI card with both detectors synthetic', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({
      score: 0.73,
      suspicious: true,
      verdictLevel: 'synthetic',
      confidence: 'high',
      triggeredCount: 0,
    });
    r.clipResult = clipFixture({
      score: 0.87,
      verdictLevel: 'synthetic',
      confidence: 'high',
      univfdAvailable: true,
    });
    await injectAndOpenAiCard(page, r);

    const card = page.locator('#card-ai-body');
    await expect(card).toHaveScreenshot('ai-card-both-synthetic.png', {
      maxDiffPixelRatio: 0.02,
    });
  });

  test('visual: AI card with both detectors authentic', async ({ page }) => {
    const r = baseResult();
    r.deepfakeResult = gbmFixture({
      score: 0.05,
      suspicious: false,
      verdictLevel: 'authentic',
    });
    r.clipResult = clipFixture({
      score: 0.05,
      verdictLevel: 'authentic',
      univfdAvailable: true,
    });
    await injectAndOpenAiCard(page, r);

    const card = page.locator('#card-ai-body');
    await expect(card).toHaveScreenshot('ai-card-both-authentic.png', {
      maxDiffPixelRatio: 0.02,
    });
  });

  test('visual: AI card when suppressed', async ({ page }) => {
    const r = baseResult();
    r.contentTypeResult = {
      contentType: 'screenshot',
      confidence: 0.95,
      aiDetectionSuitable: false,
      reasoning: 'Screenshot — AI detection unreliable',
    };
    r.deepfakeResult = gbmFixture();
    r.clipResult = clipFixture();
    await injectAndOpenAiCard(page, r);

    const card = page.locator('#card-ai-body');
    await expect(card).toHaveScreenshot('ai-card-suppressed.png', {
      maxDiffPixelRatio: 0.02,
    });
  });
});
