<script lang="ts">
  import type { VerificationResult, SidecarHealth } from '$lib/types';
  import { getTrustLevel } from '$lib/types';
  import LogoMark from '$lib/components/LogoMark.svelte';

  // ── Props ──────────────────────────────────────────────────────────────
  interface Props {
    result: VerificationResult;
    fileName: string;
    sidecarHealth: SidecarHealth | null;
    onViewExpert: () => void;
  }

  const { result, fileName, sidecarHealth, onViewExpert }: Props = $props();

  // ── Detector counts ────────────────────────────────────────────────────

  /**
   * Total number of forensic detectors Jura Trace can run when fully
   * equipped (sidecar online + all optional capabilities enabled).
   * Update this constant when new detectors are added.
   */
  const TOTAL_DETECTORS = 19;

  /**
   * Count how many detectors actually produced a result for this
   * verification. Each detector that returned a result object counts as one.
   * Regional detectors count individually (4 sub-detectors).
   */
  const detectorsRan = $derived((): number => {
    let count = 0;
    // Core always-available detectors
    if (result.exifAnalysis) count++;
    if (result.c2paManifest !== undefined || result.c2paValid !== undefined) count++;
    // Sidecar ML detectors
    if (result.elaResult) count++;
    if (result.noiseResult) count++;
    if (result.copyMoveResult) count++;
    if (result.deepfakeResult) count++;
    // Extended detectors (deep/archival mode)
    if (result.nprResult) count++;
    if (result.jpegGhostResult) count++;
    if (result.caResult) count++;
    if (result.clipResult) count++;
    // Region-based detectors (4 sub-detectors)
    if (result.segmentedElaResult) count++;
    if (result.shadowConsistencyResult) count++;
    if (result.colourTemperatureResult) count++;
    if (result.spliceBoundaryResult) count++;
    // Additional signals
    if (result.watermarkExtractResult) count++;
    if (result.videoDeepfakeResult) count++;
    if (result.transcriptionResult) count++;
    if (result.claimCheckResult || result.ragClaimResult) count++;
    if (result.thumbnailCheck) count++;
    return count;
  });

  /** Whether the analysis engine (sidecar) was online for this verification. */
  const sidecarWasOnline = $derived(
    result.elaResult != null ||
    result.noiseResult != null ||
    result.deepfakeResult != null
  );

  // ── Verdict logic (mirrors VerdictSummary derivations) ────────────────

  const hasValidC2pa = $derived(
    result.c2paManifest != null && result.c2paManifest.isValid === true
  );

  const hasAiWatermark = $derived(
    result.deepfakeResult?.watermarks != null &&
    result.deepfakeResult.watermarks.some((w) => w.detected)
  );

  const verdictLevel = $derived(result.deepfakeResult?.verdictLevel ?? null);
  const deepfakeConfidence = $derived(result.deepfakeResult?.confidence ?? 'low');

  const isAiGenerated = $derived(
    hasAiWatermark ||
    (verdictLevel === 'synthetic' && deepfakeConfidence !== 'low') ||
    (verdictLevel === null && result.deepfakeResult?.suspicious === true)
  );

  const isInconclusive = $derived(
    verdictLevel === 'inconclusive' ||
    (verdictLevel === 'synthetic' && deepfakeConfidence === 'low')
  );

  const hasManipulation = $derived(
    result.elaResult?.suspicious === true ||
    result.noiseResult?.suspicious === true ||
    result.copyMoveResult?.suspicious === true ||
    result.shadowConsistencyResult?.suspicious === true ||
    result.colourTemperatureResult?.suspicious === true ||
    (result.spliceBoundaryResult?.suspicious === true && result.segmentedElaResult?.suspicious === true)
  );

  const hasCriticalExif = $derived(
    result.exifAnalysis != null &&
    result.exifAnalysis.findings.some(
      (f) => f.severity === 'critical' || f.severity === 'high'
    )
  );

  const rawTrustLevel = $derived(getTrustLevel(result.overallTrust));
  const hasUncertainDeepfake = $derived(
    verdictLevel === 'inconclusive' || verdictLevel === 'synthetic'
  );
  const trustLevel = $derived((): 'high' | 'medium' | 'low' => {
    if (hasUncertainDeepfake && rawTrustLevel === 'high') return 'medium';
    return rawTrustLevel;
  });

  // ── Verdict category ───────────────────────────────────────────────────

  type VerdictCategory = 'authentic' | 'inconclusive' | 'ai-generated';

  const verdictCategory = $derived((): VerdictCategory => {
    if (hasAiWatermark || isAiGenerated) return 'ai-generated';
    if (isInconclusive || hasManipulation || hasCriticalExif || trustLevel() === 'medium' || trustLevel() === 'low') return 'inconclusive';
    return 'authentic';
  });

  // ── Plain-English verdict headline ────────────────────────────────────

  const verdictHeadline = $derived((): string => {
    const cat = verdictCategory();
    if (cat === 'ai-generated') {
      if (hasAiWatermark) return 'AI-Generated Content Detected';
      if (deepfakeConfidence === 'high') return 'Likely AI-Generated';
      return 'Possible AI Generation';
    }
    if (cat === 'inconclusive') {
      if (hasManipulation && (result.spliceBoundaryResult?.suspicious && result.segmentedElaResult?.suspicious)) {
        return 'Possible Composite Image';
      }
      if (hasManipulation) return 'Signs of Editing Detected';
      if (hasCriticalExif) return 'Provenance Anomalies Found';
      return 'Inconclusive — Further Review Advised';
    }
    // Authentic
    if (hasValidC2pa) return 'Likely Authentic — Credentials Verified';
    return 'No Concerns Found';
  });

  // ── Colour scheme ──────────────────────────────────────────────────────

  const colorScheme = $derived((): {
    border: string; bg: string; headline: string; dot: string; bullet: string; cta: string
  } => {
    const cat = verdictCategory();
    if (cat === 'ai-generated') return {
      border: 'border-cinnabar/40',
      bg: 'bg-cinnabar/5 dark:bg-cinnabar/8',
      headline: 'text-cinnabar dark:text-cinnabar-light',
      dot: 'bg-cinnabar',
      bullet: 'border-cinnabar/50 text-cinnabar dark:text-cinnabar-light',
      cta: 'border-cinnabar/40 text-cinnabar dark:text-cinnabar-light hover:bg-cinnabar/10',
    };
    if (cat === 'inconclusive') return {
      border: 'border-amber/40',
      bg: 'bg-amber/5 dark:bg-amber/8',
      headline: 'text-amber dark:text-amber-light',
      dot: 'bg-amber',
      bullet: 'border-amber/50 text-amber dark:text-amber-light',
      cta: 'border-amber/40 text-amber dark:text-amber-light hover:bg-amber/10',
    };
    return {
      border: 'border-malachite/40',
      bg: 'bg-malachite/5 dark:bg-malachite/8',
      headline: 'text-malachite dark:text-malachite-light',
      dot: 'bg-malachite',
      bullet: 'border-malachite/50 text-malachite dark:text-malachite-light',
      cta: 'border-malachite/40 text-malachite dark:text-malachite-light hover:bg-malachite/10',
    };
  });

  // ── Summary bullets ────────────────────────────────────────────────────

  /**
   * Derives 3-4 plain-English explanation bullets from the result.
   * Each bullet answers one key question about the content.
   * British spelling throughout. No technical acronyms in the primary text.
   */
  const summaryBullets = $derived((): { text: string; flagged: boolean }[] => {
    const bullets: { text: string; weight: number; flagged: boolean }[] = [];

    // 1. AI generation signal (highest priority)
    if (hasAiWatermark) {
      const wm = result.deepfakeResult?.watermarks?.find((w) => w.detected);
      const generatorName = wm
        ? wm.watermarkType.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())
        : 'an AI image generator';
      bullets.push({
        text: `An invisible AI watermark from ${generatorName} was found — strong indicator of synthetic content`,
        weight: 10,
        flagged: true,
      });
    } else if (verdictLevel === 'synthetic' && deepfakeConfidence === 'high') {
      bullets.push({
        text: 'Multiple AI generation indicators found — statistical patterns are consistent with synthetic imagery',
        weight: 9,
        flagged: true,
      });
    } else if (verdictLevel === 'synthetic') {
      bullets.push({
        text: 'AI generation signals detected, though confidence is not high enough for a definitive conclusion',
        weight: 8,
        flagged: true,
      });
    } else if (verdictLevel === 'inconclusive') {
      bullets.push({
        text: 'AI generation analysis returned an inconclusive result — some signals present, but not conclusive',
        weight: 7,
        flagged: true,
      });
    } else if (result.deepfakeResult && verdictLevel === 'authentic') {
      bullets.push({
        text: 'No AI generation signatures detected — pixel patterns are consistent with camera-captured content',
        weight: 0,
        flagged: false,
      });
    }

    // 2. Camera / EXIF metadata
    if (result.exifAnalysis) {
      const exif = result.exifAnalysis;
      const cameraModel = result.exifAnalysis?.findings?.find(
        (f) => f.checkId === 'camera_model' || f.category === 'camera'
      );
      if (hasCriticalExif) {
        const highFindings = exif.findings.filter(f => f.severity === 'critical' || f.severity === 'high');
        bullets.push({
          text: `Camera metadata contains ${highFindings.length} significant anomal${highFindings.length === 1 ? 'y' : 'ies'} — provenance data may have been altered`,
          weight: 6,
          flagged: true,
        });
      } else if (!exif.hasExif) {
        bullets.push({
          text: 'No camera metadata present — the file does not contain EXIF data to confirm its origin',
          weight: 3,
          flagged: true,
        });
      } else {
        // Try to surface the camera make/model from parsed metadata if available
        const modelFinding = exif.findings.find(f => f.checkId?.includes('model'));
        bullets.push({
          text: 'Camera metadata is present and consistent — no provenance anomalies detected',
          weight: 0,
          flagged: false,
        });
        void modelFinding; // consumed by message above
      }
    }

    // 3. C2PA provenance credentials
    if (result.c2paManifest) {
      if (hasValidC2pa) {
        const generator = result.c2paManifest.claimGenerator;
        if (result.aiGenerator) {
          bullets.push({
            text: `C2PA provenance record is valid and declares AI generation — signed by ${result.aiGenerator}`,
            weight: 8,
            flagged: true,
          });
        } else {
          const signerText = generator ? ` — signed by ${generator}` : '';
          bullets.push({
            text: `C2PA provenance record verified${signerText}`,
            weight: 0,
            flagged: false,
          });
        }
      } else {
        bullets.push({
          text: 'C2PA provenance record is present but failed cryptographic validation — credential may be tampered',
          weight: 7,
          flagged: true,
        });
      }
    } else if (result.c2paValid === false) {
      bullets.push({
        text: 'C2PA provenance credential is present but invalid — the signing chain could not be verified',
        weight: 7,
        flagged: true,
      });
    }

    // 4. Editing / manipulation signals
    if (result.spliceBoundaryResult?.suspicious && result.segmentedElaResult?.suspicious) {
      bullets.push({
        text: 'Two independent detectors identified evidence of compositing — high-confidence splice signal',
        weight: 8,
        flagged: true,
      });
    } else if (result.elaResult?.suspicious) {
      bullets.push({
        text: 'Compression analysis detected inconsistencies consistent with localised editing or compositing',
        weight: 5,
        flagged: true,
      });
    } else if (result.copyMoveResult?.suspicious) {
      bullets.push({
        text: 'Duplicated regions detected — content may have been cloned within the image',
        weight: 5,
        flagged: true,
      });
    } else if (result.noiseResult?.suspicious) {
      bullets.push({
        text: 'Noise pattern anomalies detected — may indicate heavy post-processing or compositing',
        weight: 4,
        flagged: true,
      });
    } else if (result.elaResult && !result.elaResult.suspicious && result.noiseResult && !result.noiseResult.suspicious) {
      bullets.push({
        text: 'Pixel-level forensic analysis found no signs of editing or compositing',
        weight: 0,
        flagged: false,
      });
    }

    // 5. Jura Trace watermark — provenance credential
    if (result.watermarkExtractResult?.hasWatermark && result.watermarkExtractResult.extractedPayload) {
      bullets.push({
        text: `Jura Trace provenance watermark detected — institution: ${result.watermarkExtractResult.extractedPayload}`,
        weight: 0,
        flagged: false,
      });
    }

    // Sort: most flagged and highest weight first
    bullets.sort((a, b) => {
      if (a.flagged !== b.flagged) return a.flagged ? -1 : 1;
      return b.weight - a.weight;
    });

    // Return top 4, converting to the simpler output type
    return bullets.slice(0, 4).map(b => ({ text: b.text, flagged: b.flagged }));
  });

  // ── Actionable next step ───────────────────────────────────────────────

  const nextStep = $derived((): string => {
    const cat = verdictCategory();
    if (cat === 'ai-generated') {
      return 'Treat as AI-generated content. Do not present as a camera photograph.';
    }
    if (cat === 'inconclusive') {
      return 'Consider seeking additional verification before sharing or publishing.';
    }
    return 'No immediate action required based on automated analysis.';
  });

  // ── Analysis completeness text ─────────────────────────────────────────

  const completenessText = $derived((): string => {
    const ran = detectorsRan();
    if (!sidecarWasOnline) {
      return `Limited analysis — ${ran} detector${ran === 1 ? '' : 's'} (Analysis Engine offline)`;
    }
    return `Analysed with ${ran} of ${TOTAL_DETECTORS} detectors`;
  });

  const completenessIsLimited = $derived(!sidecarWasOnline || detectorsRan() < 8);
</script>

<!--
  SimpleVerdict — Plain-English two-tier Simple View card.

  Shows the verdict headline, analysis completeness, 3-4 human-readable
  explanation bullets, the recommended next step, and a "See Detailed
  Analysis" toggle to expand into Expert View.
-->
<section
  class="rounded-xl border {colorScheme().border} {colorScheme().bg} overflow-hidden"
  aria-label="Verification result — simple view"
  aria-live="polite"
>

  <!-- ── Top bar: logo mark + headline ──────────────────────────── -->
  <div class="px-6 py-5 flex items-start gap-4">
    <!-- Logo mark scaled to give visual weight proportional to the verdict -->
    <div class="flex-shrink-0 mt-0.5" aria-hidden="true">
      <LogoMark size={36} />
    </div>

    <div class="flex-1 min-w-0">

      <!-- Verdict headline -->
      <h2 class="text-xl font-heading font-semibold leading-tight {colorScheme().headline}">
        {verdictHeadline()}
      </h2>

      <!-- Filename + mode badge -->
      <p class="text-sm text-text-light dark:text-quartz mt-1 truncate" title={fileName}>
        {fileName}
        {#if result.mode && result.mode !== 'standard'}
          <span class="ml-1.5 text-xs text-flint dark:text-flint-light">({result.mode} analysis)</span>
        {/if}
      </p>

      <!-- Analysis completeness indicator -->
      <p
        class="text-xs mt-2 font-medium {completenessIsLimited
          ? 'text-amber dark:text-amber-light'
          : 'text-flint dark:text-flint-light'}"
        aria-label="{completenessText()}"
      >
        {#if completenessIsLimited}
          <span class="inline-block w-2 h-2 rounded-full bg-amber mr-1.5 align-middle" aria-hidden="true"></span>
        {:else}
          <span class="inline-block w-2 h-2 rounded-full bg-malachite mr-1.5 align-middle" aria-hidden="true"></span>
        {/if}
        {completenessText()}
      </p>
    </div>
  </div>

  <!-- ── Summary bullets ────────────────────────────────────────── -->
  {#if summaryBullets().length > 0}
    <div class="px-6 pb-4" aria-label="Key findings">
      <ul class="space-y-2.5" role="list">
        {#each summaryBullets() as bullet}
          <li class="flex items-start gap-3">
            <!-- Colour-coded dot: flagged = semantic warning colour, clean = malachite -->
            <span
              class="flex-shrink-0 mt-1.5 w-2 h-2 rounded-full {bullet.flagged ? colorScheme().dot : 'bg-malachite'}"
              aria-hidden="true"
            ></span>
            <span class="text-sm text-text-light dark:text-quartz leading-relaxed">
              {bullet.text}
            </span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  <!-- ── Recommended next step ─────────────────────────────────── -->
  <div class="px-6 py-3 border-t {colorScheme().border} bg-black/5 dark:bg-black/10">
    <p class="text-xs text-flint dark:text-flint-light">
      <span class="font-medium text-text-light dark:text-quartz">Recommended action: </span>
      {nextStep()}
    </p>
  </div>

  <!-- ── Footer: see detailed analysis ─────────────────────────── -->
  <div class="px-6 py-4 flex items-center justify-between gap-4 border-t {colorScheme().border}">
    <p class="text-xs text-flint dark:text-flint-light">
      All analysis ran locally on your device.
    </p>
    <button
      type="button"
      onclick={onViewExpert}
      class="flex-shrink-0 text-sm px-4 py-2.5 min-h-[44px] rounded-lg border font-medium
             transition-colors duration-150
             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
             focus-visible:ring-offset-2 focus-visible:ring-offset-white
             dark:focus-visible:ring-offset-obsidian
             {colorScheme().cta}"
      aria-label="See detailed forensic analysis — expert view"
    >
      See Detailed Analysis
      <svg
        class="inline-block w-4 h-4 ml-1.5 -mt-0.5 align-middle"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        aria-hidden="true"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
      </svg>
    </button>
  </div>
</section>
