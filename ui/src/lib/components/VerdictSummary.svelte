<script lang="ts">
  import { getTrustLevel } from '$lib/types';
  import type { VerificationResult, VerdictLevel } from '$lib/types';
  // Note: SegmentedElaResult, ShadowConsistencyResult, ColourTemperatureResult,
  // SpliceBoundaryResult are accessed via result fields — no separate import needed.

  // ── Props ─────────────────────────────────────────────────────────────
  interface Props {
    result: VerificationResult;
    fileName: string;
  }

  const { result, fileName }: Props = $props();

  // ── Derived flags ─────────────────────────────────────────────────────

  /** Trust level bucket — forced to 'medium' when deepfake is inconclusive,
   *  ensuring the verdict card can never show green "High Trust" alongside
   *  an amber "Inconclusive" verdict. */
  const trustLevel = $derived(() => {
    const raw = getTrustLevel(result.overallTrust);
    const verdict = result.deepfakeResult?.verdictLevel;
    if ((verdict === 'inconclusive' || verdict === 'synthetic') && raw === 'high') {
      return 'medium';
    }
    return raw;
  });

  /** C2PA credentials present and cryptographically valid */
  const hasValidC2pa = $derived(
    result.c2paManifest != null && result.c2paManifest.isValid === true
  );

  /**
   * An invisible AI generator watermark was found in the image.
   * This is the strongest positive AI signal — takes priority in the verdict.
   */
  const hasAiWatermark = $derived(
    result.deepfakeResult?.watermarks != null &&
    result.deepfakeResult.watermarks.some((w) => w.detected)
  );

  /** Three-way verdict level from the sidecar (authentic / inconclusive / synthetic) */
  const verdictLevel = $derived<VerdictLevel | null>(
    (result.deepfakeResult?.verdictLevel as VerdictLevel) ?? null
  );

  /** Confidence level from the deepfake detector */
  const deepfakeConfidence = $derived(
    result.deepfakeResult?.confidence ?? 'low'
  );

  /**
   * AI generation detected via three-way verdict.
   * Uses verdict_level when available, falls back to suspicious boolean.
   */
  const isAiGenerated = $derived(
    hasAiWatermark ||
    (verdictLevel === 'synthetic' && deepfakeConfidence !== 'low') ||
    (verdictLevel === null && result.deepfakeResult?.suspicious === true)
  );

  /** Detection is inconclusive — signals present but not definitive */
  const isInconclusive = $derived(
    verdictLevel === 'inconclusive' ||
    (verdictLevel === 'synthetic' && deepfakeConfidence === 'low')
  );

  /**
   * Highest-confidence composite signal: both splice boundary AND segmented ELA
   * agree there is a cut edge. Two independent detectors corroborating each other
   * is the strongest manipulation signal available from region analysis.
   */
  const hasCompositeSpliceSignal = $derived(
    result.spliceBoundaryResult?.suspicious === true &&
    result.segmentedElaResult?.suspicious === true
  );

  /**
   * At least one forensic analysis layer found suspicious patterns
   * consistent with post-capture editing or compositing.
   * Includes region-based detectors: shadow consistency and colour temperature.
   */
  const hasManipulation = $derived(
    (result.elaResult?.suspicious === true) ||
    (result.noiseResult?.suspicious === true) ||
    (result.copyMoveResult?.suspicious === true) ||
    (result.shadowConsistencyResult?.suspicious === true) ||
    (result.colourTemperatureResult?.suspicious === true) ||
    hasCompositeSpliceSignal
  );

  /**
   * Forensic signals disagree — some clean, some suspicious.
   * This pattern often indicates codec artefacts or recompression,
   * not genuine manipulation.
   */
  const hasMixedSignals = $derived((): boolean => {
    const elaClean = result.elaResult != null && !result.elaResult.suspicious;
    const noiseFlag = result.noiseResult?.suspicious === true;
    const copyMoveFlag = result.copyMoveResult?.suspicious === true;
    const elaFlag = result.elaResult?.suspicious === true;
    const noiseClean = result.noiseResult != null && !result.noiseResult.suspicious;

    // ELA clean but noise/copy-move suspicious = likely codec artefact
    if (elaClean && (noiseFlag || copyMoveFlag)) return true;
    // Noise clean but ELA suspicious = possible recompression
    if (noiseClean && elaFlag) return true;
    return false;
  });

  /**
   * One or more high-severity or critical EXIF anomalies detected.
   * These suggest the provenance metadata has been altered or stripped.
   */
  const hasCriticalExif = $derived(
    result.exifAnalysis != null &&
    result.exifAnalysis.findings.some(
      (f) => f.severity === 'critical' || f.severity === 'high'
    )
  );

  // ── Confidence label ──────────────────────────────────────────────────

  const confidenceLabel = $derived((): string => {
    if (!result.deepfakeResult) return '';
    const c = deepfakeConfidence;
    if (c === 'high') return 'High confidence';
    if (c === 'medium') return 'Medium confidence';
    return 'Low confidence';
  });

  const confidenceBadgeClass = $derived((): string => {
    const c = deepfakeConfidence;
    if (c === 'high') return 'text-text-light dark:text-quartz bg-obsidian/10 dark:bg-obsidian/40 border-flint/20';
    if (c === 'medium') return 'text-flint-dark dark:text-flint-light bg-obsidian/5 dark:bg-obsidian/30 border-flint/15';
    return 'text-flint-dark dark:text-flint-light bg-obsidian/5 dark:bg-obsidian/20 border-flint/10';
  });

  // ── Verdict label ─────────────────────────────────────────────────────

  /**
   * Derives a short verdict label in priority order:
   *   AI watermark > AI generated (high conf) > inconclusive > mixed signals
   *   > manipulation > critical EXIF > C2PA valid > no C2PA > low trust
   */
  const verdictLabel = $derived((): string => {
    if (hasAiWatermark) return 'AI Watermark Detected';
    if (isAiGenerated && deepfakeConfidence === 'high') return 'Likely AI Generated';
    if (isAiGenerated) return 'Possible AI Generation';
    if (isInconclusive && hasManipulation) return 'Mixed Signals — Further Review Recommended';
    if (isInconclusive) return 'Inconclusive — Further Review Recommended';
    if (hasMixedSignals()) return 'Mixed Signals — Further Review Recommended';
    if (hasCompositeSpliceSignal) return 'Splice Detected — High Confidence';
    if (hasManipulation && trustLevel() === 'low') return 'Manipulation Detected';
    if (hasManipulation) return 'Possible Manipulation';
    if (hasCriticalExif && trustLevel() === 'low') return 'Provenance Anomalies Found';
    if (hasCriticalExif) return 'Provenance Concerns';
    if (hasValidC2pa && trustLevel() === 'high') return 'Likely Authentic';
    if (hasValidC2pa) return 'Credentials Present';
    if (trustLevel() === 'high') return 'No Concerns Found';
    if (trustLevel() === 'medium') return 'Unverified';
    return 'Low Trust';
  });

  // ── Explanation paragraph ─────────────────────────────────────────────

  /**
   * Generates a plain-English explanation paragraph (2–3 sentences).
   * No technical acronyms. British spelling throughout.
   */
  const explanation = $derived((): string => {
    const scorePercent = Math.round(result.overallTrust * 100);
    const name = fileName;
    const confText = result.deepfakeResult
      ? ` (${confidenceLabel().toLowerCase()})`
      : '';

    if (hasAiWatermark) {
      const wm = result.deepfakeResult?.watermarks?.find((w) => w.detected);
      const generatorName = wm
        ? wm.watermarkType.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())
        : 'an AI image generator';
      return (
        `An invisible watermark embedded by ${generatorName} was found in "${name}". ` +
        `This is a reliable indicator that the image was produced synthetically rather than captured by a camera. ` +
        `The overall trust score is ${scorePercent}%.`
      );
    }

    if (isAiGenerated) {
      return (
        `Statistical analysis of "${name}" found patterns consistent with AI-generated imagery${confText}. ` +
        `Multiple detection signals were triggered, suggesting the image was synthesised rather than photographed. ` +
        `The overall trust score is ${scorePercent}%.`
      );
    }

    if (isInconclusive) {
      return (
        `Analysis of "${name}" produced inconclusive results${confText}. ` +
        `Some signals suggest possible AI generation or editing, but the evidence is not strong enough for a definitive judgement. ` +
        `Review the individual analysis sections below and consider using reverse image search for additional context. ` +
        `The overall trust score is ${scorePercent}%.`
      );
    }

    if (hasMixedSignals()) {
      const layers: string[] = [];
      if (result.elaResult?.suspicious) layers.push('compression artefacts');
      if (result.noiseResult?.suspicious) layers.push('noise patterns');
      if (result.copyMoveResult?.suspicious) layers.push('duplicated regions');
      const clean: string[] = [];
      if (result.elaResult && !result.elaResult.suspicious) clean.push('compression analysis');
      if (result.noiseResult && !result.noiseResult.suspicious) clean.push('noise analysis');
      return (
        `Forensic analysis of "${name}" produced mixed results: ` +
        `${layers.join(' and ')} were flagged, but ${clean.join(' and ')} found no concerns. ` +
        `This pattern is common with recompressed images or modern image codecs (AVIF, WebP) and may not indicate manipulation. ` +
        `The overall trust score is ${scorePercent}%.`
      );
    }

    if (hasCompositeSpliceSignal) {
      return (
        `Region-level analysis of "${name}" found corroborating evidence of splicing: ` +
        `both the segmented compression map and splice boundary detector identified the same anomalous edges. ` +
        `This is the highest-confidence composite manipulation signal. ` +
        `The overall trust score is ${scorePercent}%.`
      );
    }

    if (hasManipulation && hasCriticalExif) {
      return (
        `Forensic analysis of "${name}" found evidence of pixel-level editing and anomalies in the embedded provenance data. ` +
        `These findings, taken together, suggest the image may have been altered after capture. ` +
        `The overall trust score is ${scorePercent}%.`
      );
    }

    if (hasManipulation) {
      const layers: string[] = [];
      if (result.elaResult?.suspicious) layers.push('compression artefact patterns');
      if (result.noiseResult?.suspicious) layers.push('inconsistent noise distribution');
      if (result.copyMoveResult?.suspicious) layers.push('duplicated regions');
      if (result.shadowConsistencyResult?.suspicious) layers.push('inconsistent shadow direction');
      if (result.colourTemperatureResult?.suspicious) layers.push('colour temperature anomalies');
      const detail = layers.length > 0
        ? `Specifically, ${layers.join(' and ')} were detected.`
        : 'One or more forensic layers returned suspicious results.';
      return (
        `Forensic analysis of "${name}" identified signals that may indicate post-capture editing. ` +
        `${detail} ` +
        `The overall trust score is ${scorePercent}%.`
      );
    }

    if (hasCriticalExif) {
      return (
        `The embedded provenance data in "${name}" contains significant anomalies, such as missing or inconsistent camera information. ` +
        `This may indicate the original metadata was altered or stripped. ` +
        `The overall trust score is ${scorePercent}%.`
      );
    }

    if (hasValidC2pa && trustLevel() === 'high') { // explanation: C2PA + high trust
      const generator = result.aiGenerator;
      if (generator) {
        return (
          `"${name}" carries a valid signed provenance record, but that record identifies it as created by ${generator}. ` +
          `The file's authenticity as a camera-captured image is therefore in question. ` +
          `The overall trust score is ${scorePercent}%.`
        );
      }
      return (
        `"${name}" carries a valid signed provenance record confirming its origin and history. ` +
        `No forensic anomalies were detected. ` +
        `The overall trust score is ${scorePercent}%.`
      );
    }

    if (hasValidC2pa) {
      return (
        `"${name}" carries a valid signed provenance record, though some other signals reduced the overall confidence. ` +
        `Review the individual analysis sections below for more detail. ` +
        `The overall trust score is ${scorePercent}%.`
      );
    }

    if (trustLevel() === 'high') {
      return (
        `No forensic concerns were found in "${name}". ` +
        `The file has no signed provenance record, so its origin cannot be independently confirmed, ` +
        `but no signs of manipulation or synthetic generation were detected. ` +
        `The overall trust score is ${scorePercent}%.`
      );
    }

    if (trustLevel() === 'medium') {
      return (
        `"${name}" could not be fully verified. ` +
        `It carries no signed provenance record, and some signals are ambiguous. ` +
        `The overall trust score is ${scorePercent}%.`
      );
    }

    // Low trust fallback
    return (
      `The analysis of "${name}" returned a low trust score of ${scorePercent}%. ` +
      `Multiple signals indicate concerns about this file's provenance or integrity. ` +
      `Review the individual analysis sections below for detail.`
    );
  });

  // ── Colour classes ────────────────────────────────────────────────────

  /** Is this verdict in the "danger" (red) category? */
  const isDanger = $derived(
    hasAiWatermark ||
    isAiGenerated ||
    hasCompositeSpliceSignal ||
    (hasManipulation && trustLevel() === 'low') ||
    trustLevel() === 'low'
  );

  /** Is this verdict in the "warning" (amber) category? */
  const isWarning = $derived(
    !isDanger && (isInconclusive || hasMixedSignals() || hasManipulation || hasCriticalExif || trustLevel() === 'medium')
  );

  /** Border and background tint for the verdict card */
  const cardClass = $derived((): string => {
    if (isDanger) return 'border-cinnabar/30 bg-cinnabar/5';
    if (isWarning) return 'border-amber/30 bg-amber/5';
    return 'border-malachite/30 bg-malachite/5';
  });

  /** Colour of the indicator dot */
  const dotClass = $derived((): string => {
    if (isDanger) return 'bg-cinnabar';
    if (isWarning) return 'bg-amber';
    return 'bg-malachite';
  });

  /** Colour of the verdict label text */
  const verdictTextClass = $derived((): string => {
    if (isDanger) return 'text-cinnabar-light';
    if (isWarning) return 'text-amber-light';
    return 'text-malachite-light';
  });
</script>

<!--
  VerdictSummary — plain-English verdict card for a VerificationResult.

  Layout:
    ┌──────────────────────────────────────────────────────┐
    │  [dot]  Verdict label  [confidence]       score %   │
    │  Explanation paragraph (2–3 sentences)               │
    └──────────────────────────────────────────────────────┘
-->
<section
  class="rounded-lg border px-5 py-4 {cardClass()}"
  aria-label="Verdict summary"
>
  <!-- Header row: dot + label + confidence badge + score -->
  <div class="flex items-center justify-between gap-4 mb-3">
    <div class="flex items-center gap-2.5 min-w-0">
      <!-- Indicator dot -->
      <span
        class="flex-shrink-0 w-2.5 h-2.5 rounded-full {dotClass()}"
        aria-hidden="true"
      ></span>

      <!-- Verdict label -->
      <p class="text-sm font-medium {verdictTextClass()} leading-snug">
        {verdictLabel()}
      </p>

      <!-- Confidence badge (shown when deepfake analysis ran) -->
      {#if result.deepfakeResult && confidenceLabel()}
        <span
          class="flex-shrink-0 text-xs font-medium px-1.5 py-0.5 rounded border
                 {confidenceBadgeClass()}"
          aria-label="Detection confidence: {confidenceLabel()}"
        >
          {confidenceLabel()}
        </span>
      {/if}
    </div>

    <!-- Trust score badge -->
    <span
      class="flex-shrink-0 text-xs tabular-nums font-medium px-2 py-0.5 rounded
             {verdictTextClass()} bg-obsidian/10 dark:bg-obsidian/40 border {cardClass()}"
      aria-label="Trust score: {Math.round(result.overallTrust * 100)} per cent"
    >
      {Math.round(result.overallTrust * 100)}%
    </span>
  </div>

  <!-- Explanation -->
  <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
    {explanation()}
  </p>
</section>
