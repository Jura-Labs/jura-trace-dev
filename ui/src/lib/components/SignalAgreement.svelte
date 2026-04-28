<script lang="ts">
  // ── SignalAgreement ─────────────────────────────────────────────────────
  // Per-detector summary table showing the result of each analysis layer
  // at a glance. Highlights disagreements between detectors.
  // Designed to give analysts a quick cross-signal overview.

  import type { VerificationResult } from '$lib/types';

  // ── Props ───────────────────────────────────────────────────────────────

  interface Props {
    result: VerificationResult;
  }

  const { result }: Props = $props();

  // ── Types ───────────────────────────────────────────────────────────────

  type SignalOutcome = 'clean' | 'concerns' | 'suspicious' | 'not_run' | 'valid' | 'not_present' | 'invalid' | 'authentic' | 'inconclusive' | 'synthetic';

  interface SignalRow {
    id: string;
    detector: string;
    outcome: SignalOutcome;
    /** 0–1, or null when not run */
    confidence: number | null;
  }

  // ── Derived rows ────────────────────────────────────────────────────────

  const rows = $derived<SignalRow[]>([
    // C2PA Provenance
    {
      id: 'c2pa',
      detector: 'C2PA Provenance',
      outcome: result.c2paManifest == null
        ? 'not_present'
        : result.c2paManifest.isValid
          ? 'valid'
          : 'invalid',
      confidence: result.c2paManifest != null ? 1.0 : null,
    },

    // EXIF Metadata — map trustScore to three buckets
    {
      id: 'exif',
      detector: 'EXIF Metadata',
      outcome: result.exifAnalysis == null
        ? 'not_run'
        : result.exifAnalysis.trustScore > 0.8
          ? 'clean'
          : result.exifAnalysis.trustScore >= 0.4
            ? 'concerns'
            : 'suspicious',
      confidence: result.exifAnalysis?.trustScore ?? null,
    },

    // ELA
    {
      id: 'ela',
      detector: 'Error Level Analysis',
      outcome: result.elaResult == null
        ? 'not_run'
        : result.elaResult.suspicious
          ? 'suspicious'
          : 'clean',
      confidence: result.elaResult != null ? 1 - result.elaResult.score : null,
    },

    // Noise Analysis
    {
      id: 'noise',
      detector: 'Noise Analysis',
      outcome: result.noiseResult == null
        ? 'not_run'
        : result.noiseResult.suspicious
          ? 'suspicious'
          : 'clean',
      confidence: result.noiseResult != null ? 1 - result.noiseResult.score : null,
    },

    // Copy-Move Detection
    {
      id: 'copymove',
      detector: 'Copy-Move Detection',
      outcome: result.copyMoveResult == null
        ? 'not_run'
        : result.copyMoveResult.suspicious
          ? 'suspicious'
          : 'clean',
      confidence: result.copyMoveResult != null ? 1 - result.copyMoveResult.score : null,
    },

    // AI Generation Detection — use verdictLevel when present
    {
      id: 'deepfake',
      detector: 'AI Generation Detection',
      outcome: result.deepfakeResult == null
        ? 'not_run'
        : result.deepfakeResult.verdictLevel === 'authentic'
          ? 'authentic'
          : result.deepfakeResult.verdictLevel === 'inconclusive'
            ? 'inconclusive'
            : result.deepfakeResult.verdictLevel === 'synthetic'
              ? 'synthetic'
              : result.deepfakeResult.suspicious
                ? 'suspicious'
                : 'clean',
      confidence: result.deepfakeResult != null
        ? result.deepfakeResult.confidence === 'high' ? 0.9
          : result.deepfakeResult.confidence === 'medium' ? 0.6
          : 0.3
        : null,
    },

    // NPR (Neighbouring Pixel Relationships) — deep mode only
    {
      id: 'npr',
      detector: 'Pixel Relationships (NPR)',
      outcome: result.nprResult == null
        ? 'not_run'
        : result.nprResult.suspicious
          ? 'suspicious'
          : 'clean',
      confidence: result.nprResult != null ? 1 - result.nprResult.score : null,
    },

    // JPEG Ghost Detection — deep mode only
    {
      id: 'jpegGhost',
      detector: 'JPEG Ghost',
      outcome: result.jpegGhostResult == null
        ? 'not_run'
        : result.jpegGhostResult.suspicious
          ? 'suspicious'
          : 'clean',
      confidence: result.jpegGhostResult != null ? 1 - result.jpegGhostResult.score : null,
    },
  ]);

  // ── Disagreement detection ───────────────────────────────────────────────
  //
  // A "disagreement" exists when the subset of detectors that *did* run
  // contains at least one clean/authentic/valid signal AND at least one
  // suspicious/synthetic/invalid signal. Concerns and inconclusive alone
  // do not constitute a hard disagreement but do contribute to "mixed".

  const runRows = $derived(rows.filter((r) => r.outcome !== 'not_run'));

  const hasCleanSignal = $derived(
    runRows.some((r) => r.outcome === 'clean' || r.outcome === 'authentic' || r.outcome === 'valid')
  );

  const hasSuspiciousSignal = $derived(
    runRows.some((r) => r.outcome === 'suspicious' || r.outcome === 'synthetic' || r.outcome === 'invalid')
  );

  const hasConcernSignal = $derived(
    runRows.some((r) => r.outcome === 'concerns' || r.outcome === 'inconclusive')
  );

  /**
   * Hard disagreement: at least one clean/authentic detector contradicts
   * at least one suspicious/synthetic/invalid detector.
   */
  const hasDisagreement = $derived(hasCleanSignal && hasSuspiciousSignal);

  /**
   * Mixed: concerns/inconclusive alongside clean, without hard disagreement.
   */
  const hasMixed = $derived(!hasDisagreement && hasConcernSignal && hasCleanSignal);

  // ── Colour helpers ───────────────────────────────────────────────────────

  function outcomeTextClass(outcome: SignalOutcome): string {
    switch (outcome) {
      case 'clean':
      case 'authentic':
      case 'valid':
        return 'text-malachite-dark dark:text-malachite-light';
      case 'concerns':
      case 'inconclusive':
        return 'text-amber-dark dark:text-amber-light';
      case 'suspicious':
      case 'synthetic':
      case 'invalid':
        return 'text-cinnabar-dark dark:text-cinnabar-light';
      case 'not_run':
      case 'not_present':
      default:
        return 'text-flint-dark dark:text-flint-light';
    }
  }

  function outcomeBgClass(outcome: SignalOutcome): string {
    switch (outcome) {
      case 'clean':
      case 'authentic':
      case 'valid':
        return 'bg-malachite/10 border-malachite/20';
      case 'concerns':
      case 'inconclusive':
        return 'bg-amber/10 border-amber/20';
      case 'suspicious':
      case 'synthetic':
      case 'invalid':
        return 'bg-cinnabar/10 border-cinnabar/20';
      case 'not_run':
      case 'not_present':
      default:
        return 'bg-gray-100 dark:bg-graphite border-gray-300 dark:border-graphite-light';
    }
  }

  function outcomeLabel(outcome: SignalOutcome): string {
    switch (outcome) {
      case 'clean':       return 'Clean';
      case 'authentic':   return 'Authentic';
      case 'valid':       return 'Valid';
      case 'concerns':    return 'Concerns';
      case 'inconclusive': return 'Inconclusive';
      case 'suspicious':  return 'Suspicious';
      case 'synthetic':   return 'Synthetic';
      case 'invalid':     return 'Invalid';
      case 'not_present': return 'Not present';
      case 'not_run':
      default:            return 'Not run';
    }
  }

  /** Row border class — highlight disagreeing rows with amber */
  function rowBorderClass(row: SignalRow): string {
    if (!hasDisagreement) return 'border-border-light dark:border-graphite/60';
    const isSuspicious = row.outcome === 'suspicious' || row.outcome === 'synthetic' || row.outcome === 'invalid';
    const isClean = row.outcome === 'clean' || row.outcome === 'authentic' || row.outcome === 'valid';
    if (isSuspicious || isClean) return 'border-amber/30';
    return 'border-border-light dark:border-graphite/60';
  }

  /** Confidence bar width — clamped to 0–100% */
  function confidencePercent(value: number): number {
    return Math.round(Math.min(Math.max(value, 0), 1) * 100);
  }
</script>

<!--
  SignalAgreement — per-detector result summary table.

  Columns: Detector | Result | Confidence
  Rows: C2PA, EXIF, ELA, Noise, Copy-Move, AI Detection

  Agreement / disagreement summary banner shown at top.
-->
<div
  class="bg-white dark:bg-obsidian/50 border border-border-light dark:border-graphite rounded-lg overflow-hidden"
  aria-label="Signal agreement summary"
>

  <!-- ── Section header ─────────────────────────────────────────────── -->
  <div class="flex items-center justify-between gap-3 px-4 py-3 border-b border-border-light dark:border-graphite">
    <h3 class="text-sm font-medium text-flint-dark dark:text-flint-light">Signal Agreement</h3>

    <!-- Agreement indicator badge -->
    {#if hasDisagreement}
      <span
        class="text-xs px-2 py-0.5 rounded border bg-amber/10 text-amber-dark dark:text-amber-light border-amber/30 font-medium"
        role="status"
        aria-label="Detectors disagree — mixed signals present"
      >
        Mixed signals
      </span>
    {:else if hasMixed}
      <span
        class="text-xs px-2 py-0.5 rounded border bg-amber/10 text-amber-dark dark:text-amber-light border-amber/20"
        role="status"
        aria-label="Some inconclusive signals present"
      >
        Some inconclusive
      </span>
    {:else if runRows.length === 0}
      <span class="text-xs text-flint-dark dark:text-flint-light">No detectors ran</span>
    {:else}
      <span
        class="text-xs px-2 py-0.5 rounded border bg-malachite/10 text-malachite-dark dark:text-malachite-light border-malachite/20"
        role="status"
        aria-label="Detectors agree"
      >
        Consistent
      </span>
    {/if}
  </div>

  <!-- ── Disagreement callout ────────────────────────────────────────── -->
  {#if hasDisagreement}
    <div
      class="px-4 py-2.5 border-b border-border-light dark:border-graphite bg-amber/5 text-xs text-amber-dark dark:text-amber-light leading-relaxed"
      role="alert"
      aria-live="polite"
    >
      <span class="font-medium text-amber-dark dark:text-amber-light">Detectors disagree.</span>
      Some analysis layers returned clean results while others flagged concerns.
      This pattern can occur with recompressed images, modern codecs, or partial edits.
      Review individual sections for detail.
    </div>
  {/if}

  <!-- ── Table header ────────────────────────────────────────────────── -->
  <div
    class="grid grid-cols-[1fr_120px_80px] gap-3 px-4 py-2 border-b border-border-light dark:border-graphite text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide"
    aria-hidden="true"
  >
    <span>Detector</span>
    <span>Result</span>
    <span class="text-right">Confidence</span>
  </div>

  <!-- ── Rows ────────────────────────────────────────────────────────── -->
  <ul
    class="divide-y divide-border-light dark:divide-graphite/60"
    role="list"
    aria-label="Detection results by method"
  >
    {#each rows as row (row.id)}
      <li
        class="grid grid-cols-[1fr_120px_80px] gap-3 items-center px-4 py-2.5
               border-l-2 transition-colors duration-150
               {rowBorderClass(row)}"
        role="listitem"
        aria-label="{row.detector.replace(' *', '')}: {outcomeLabel(row.outcome)}{row.confidence != null ? ', confidence ' + confidencePercent(row.confidence) + ' per cent' : ''}"
      >

        <!-- Detector name -->
        <span class="text-xs text-text-light dark:text-quartz leading-snug">{row.detector}</span>

        <!-- Result badge -->
        <span>
          <span
            class="inline-block text-xs px-2 py-0.5 rounded border font-medium
                   {outcomeTextClass(row.outcome)} {outcomeBgClass(row.outcome)}"
            aria-hidden="true"
          >
            {outcomeLabel(row.outcome)}
          </span>
        </span>

        <!-- Confidence indicator -->
        <div class="flex flex-col items-end gap-1" aria-hidden="true">
          {#if row.confidence != null}
            <span class="text-xs tabular-nums {outcomeTextClass(row.outcome)}">
              {confidencePercent(row.confidence)}%
            </span>
            <!-- Mini progress bar -->
            <div
              class="w-12 h-1 rounded-full bg-gray-200 dark:bg-graphite-light overflow-hidden"
              role="presentation"
            >
              <div
                class="h-full rounded-full transition-all duration-300 ease-out
                       {row.outcome === 'clean' || row.outcome === 'authentic' || row.outcome === 'valid'
                         ? 'bg-malachite'
                         : row.outcome === 'concerns' || row.outcome === 'inconclusive'
                           ? 'bg-amber'
                           : row.outcome === 'not_run' || row.outcome === 'not_present'
                             ? 'bg-flint/20'
                             : 'bg-cinnabar'}"
                style="width: {confidencePercent(row.confidence)}%"
              ></div>
            </div>
          {:else}
            <span class="text-xs text-flint-dark dark:text-flint-light">—</span>
          {/if}
        </div>

      </li>
    {/each}
  </ul>

</div>
