<script lang="ts">
  import type { VerificationResult, SidecarHealth, DeepfakeSignal } from '$lib/types';

  // ── Props ────────────────────────────────────────────────────────────
  interface Props {
    result: VerificationResult;
    sidecarHealth: SidecarHealth | null;
    /**
     * Jura Trace desktop app version to display in the Version section.
     * Sourced from the Tauri `getVersion()` API in the calling page. Falls
     * back to "unknown" when not provided (e.g. browser dev mode).
     */
    appVersion?: string | null;
  }

  let { result, sidecarHealth, appVersion = null }: Props = $props();

  // ── Derived ──────────────────────────────────────────────────────────

  interface StageCitation {
    authors: string;
    year: number;
    paper: string;
  }

  /** Which pipeline stages ran, derived from presence of result sub-objects */
  const stages = $derived([
    {
      id: 'exif',
      label: 'EXIF Analysis',
      description: 'Reads embedded metadata fields and checks for anomalies, inconsistencies, and signs of editing.',
      ran: true, // Always runs
      citation: { authors: 'Kee, E. & Farid, H.', year: 2011, paper: 'Digital Forensics of EXIF Metadata Inconsistencies in Digital Photographs' } as StageCitation,
    },
    {
      id: 'c2pa',
      label: 'C2PA Verification',
      description: 'Reads and validates C2PA provenance manifests embedded in the file to establish provenance.',
      ran: true, // Always runs
      citation: { authors: 'Coalition for Content Provenance and Authenticity', year: 2024, paper: 'C2PA Content Credentials Technical Specification v2.3' } as StageCitation,
    },
    {
      id: 'ela',
      label: 'Error Level Analysis',
      description: 'Re-compresses the image at a known quality and measures pixel-level differences to detect regions that have been edited at a different compression history.',
      ran: result.elaResult !== undefined,
      citation: { authors: 'Krawetz, N.', year: 2007, paper: "A Picture's Worth: Digital Image Analysis and Forensics" } as StageCitation,
    },
    {
      id: 'noise',
      label: 'Noise Analysis',
      description: 'Analyses block-wise noise variance across the image. Inconsistent noise patterns between regions can indicate splicing, inpainting, or compositing.',
      ran: result.noiseResult !== undefined,
      citation: { authors: 'Mahdian, B. & Saic, S.', year: 2009, paper: 'Noise Inconsistencies in Digital Photographs' } as StageCitation,
    },
    {
      id: 'copymove',
      label: 'Copy-Move Detection',
      description: 'Searches for duplicated regions within the image using feature matching. A high number of matched pairs may indicate content has been cloned from one area to another.',
      ran: result.copyMoveResult !== undefined,
      citation: { authors: 'Lowe, D.G.', year: 2004, paper: 'Distinctive Image Features from Scale-Invariant Keypoints' } as StageCitation,
    },
    {
      id: 'deepfake',
      label: 'AI Generation Detection',
      description: 'Applies a weighted ensemble of statistical signals — frequency spectrum analysis, colour distribution, texture regularity — to assess whether the image is likely AI-generated.',
      ran: result.deepfakeResult !== undefined,
      citation: { authors: 'Ojha, U. et al.', year: 2023, paper: 'Towards Universal Fake Image Detectors that Generalise Across Generative Models' } as StageCitation,
    },
  ]);

  const triggeredSignalCount = $derived(
    result.deepfakeResult
      ? result.deepfakeResult.signals.filter((s: DeepfakeSignal) => s.triggered).length
      : 0
  );

  const sidecarVersion = $derived(
    sidecarHealth?.version ?? null
  );
</script>

<!--
  MethodologyPanel — "How was this analysed?"
  A collapsible disclosure panel providing transparency about which pipeline
  stages ran, what each signal measures, version provenance, and a disclaimer.
-->
<details class="group bg-white dark:bg-obsidian/50 border border-border-light dark:border-graphite rounded-lg overflow-hidden">

  <!-- ── Toggle header ──────────────────────────────────────────────── -->
  <summary
    class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer select-none
           list-none
           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
           focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
           hover:bg-gray-50 dark:hover:bg-graphite/40 transition-colors duration-150"
    aria-label="How was this analysed? — toggle methodology details"
  >
    <span class="text-sm font-medium text-text-light dark:text-flint-light">How was this analysed?</span>

    <!-- Chevron rotates when open — group-open is set by <details> -->
    <svg
      class="w-4 h-4 text-text-light dark:text-flint-light flex-shrink-0 transition-transform duration-200 motion-safe:group-open:rotate-180"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </summary>

  <!-- ── Panel body ──────────────────────────────────────────────────── -->
  <div class="border-t border-border-light dark:border-graphite divide-y divide-border-light dark:divide-graphite/60">

    <!-- ── Section: Pipeline Stages ─────────────────────────────────── -->
    <section class="px-4 py-4" aria-labelledby="methodology-stages-heading">
      <h3 id="methodology-stages-heading" class="text-xs section-label uppercase tracking-wide mb-3">
        Pipeline Stages
      </h3>

      <div class="space-y-2">
        {#each stages as stage (stage.id)}
          <div class="flex items-start gap-3">
            <!-- Status indicator -->
            <div
              class="flex-shrink-0 mt-0.5 w-5 h-5 rounded-full flex items-center justify-center
                     {stage.ran
                       ? 'bg-malachite/15 border border-malachite/30'
                       : 'bg-gray-100 dark:bg-graphite-light/50 border border-gray-200 dark:border-graphite-light'}"
              aria-label="{stage.label}: {stage.ran ? 'ran' : 'skipped'}"
            >
              {#if stage.ran}
                <!-- Tick mark -->
                <svg class="w-3 h-3 text-malachite-dark dark:text-malachite-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
                </svg>
              {:else}
                <!-- Dash / minus for skipped -->
                <svg class="w-3 h-3 text-flint-dark dark:text-flint-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M20 12H4" />
                </svg>
              {/if}
            </div>

            <!-- Stage name + description -->
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="text-xs font-medium {stage.ran ? 'text-text-light dark:text-quartz' : 'text-flint-dark dark:text-flint-light'}">
                  {stage.label}
                </span>
                <span
                  class="text-xs px-1.5 py-px rounded
                         {stage.ran
                           ? 'bg-malachite/10 text-malachite-dark dark:text-malachite-light'
                           : 'bg-gray-100 dark:bg-graphite-light text-flint-dark dark:text-flint-light'}"
                >
                  {stage.ran ? 'Ran' : 'Skipped'}
                </span>
              </div>
              <p class="text-xs muted-help mt-0.5 leading-relaxed">{stage.description}</p>
              {#if stage.citation}
                <p class="text-xs muted-help mt-0.5 leading-snug italic">
                  {stage.citation.authors} ({stage.citation.year}). {stage.citation.paper}.
                </p>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </section>

    <!-- ── Section: Deepfake Signals (conditional) ───────────────────── -->
    {#if result.deepfakeResult && result.deepfakeResult.signals.length > 0}
      <section class="px-4 py-4" aria-labelledby="methodology-signals-heading">
        <h3 id="methodology-signals-heading" class="text-xs section-label uppercase tracking-wide mb-1">
          AI Generation Signals
        </h3>
        <p class="text-xs muted-help mb-3">
          {triggeredSignalCount} of {result.deepfakeResult.signals.length} signals triggered in this analysis.
          Each signal contributes a weighted score to the overall AI Generation Detection result.
        </p>

        <div class="space-y-1.5">
          {#each result.deepfakeResult.signals as signal (signal.name)}
            <div
              class="rounded-md px-3 py-2 text-xs
                     {signal.triggered
                       ? 'bg-amber/10 border border-amber/20'
                       : 'bg-gray-50 dark:bg-graphite-light/40 border border-gray-200 dark:border-graphite-light/60'}"
            >
              <div class="flex items-center justify-between gap-3 mb-0.5">
                <div class="flex items-center gap-2 min-w-0">
                  <!-- Triggered indicator -->
                  <span
                    class="flex-shrink-0 w-1.5 h-1.5 rounded-full {signal.triggered ? 'bg-amber' : 'bg-flint/40 dark:bg-flint/30'}"
                    aria-label="{signal.triggered ? 'Triggered' : 'Not triggered'}"
                  ></span>
                  <span class="font-mono {signal.triggered ? 'text-amber-dark dark:text-amber-light' : 'text-flint-dark dark:text-flint-light'} truncate">
                    {signal.name}
                  </span>
                </div>
                <div class="flex items-center gap-3 flex-shrink-0">
                  <span class="muted-help tabular-nums">
                    weight: {signal.weight.toFixed(1)}
                  </span>
                  <span
                    class="text-xs px-1.5 py-px rounded
                           {signal.triggered
                             ? 'bg-amber/15 text-amber-dark dark:text-amber-light'
                             : 'bg-gray-100 dark:bg-graphite-light text-flint-dark dark:text-flint-light'}"
                  >
                    {signal.triggered ? 'Triggered' : 'Clear'}
                  </span>
                </div>
              </div>
              <p class="muted-help leading-relaxed pl-3.5">{signal.description}</p>
            </div>
          {/each}
        </div>
      </section>
    {/if}

    <!-- ── Section: Version ──────────────────────────────────────────── -->
    <section class="px-4 py-4" aria-labelledby="methodology-version-heading">
      <h3 id="methodology-version-heading" class="text-xs section-label uppercase tracking-wide mb-2">
        Version
      </h3>

      <div class="space-y-1 text-xs">
        <div class="flex items-center justify-between gap-4">
          <span class="muted-help">Jura Trace</span>
          <span class="text-text-light dark:text-quartz font-mono tabular-nums">v{appVersion ?? 'unknown'}</span>
        </div>
        {#if sidecarVersion}
          <div class="flex items-center justify-between gap-4">
            <span class="muted-help">Analysis Engine</span>
            <span class="text-text-light dark:text-quartz font-mono tabular-nums">v{sidecarVersion}</span>
          </div>
        {:else}
          <div class="flex items-center justify-between gap-4">
            <span class="muted-help">Analysis Engine</span>
            <span class="muted-help font-mono">offline</span>
          </div>
        {/if}
      </div>
    </section>

    <!-- ── Section: Disclaimer ───────────────────────────────────────── -->
    <section class="px-4 py-4" aria-labelledby="methodology-disclaimer-heading">
      <h3 id="methodology-disclaimer-heading" class="text-xs section-label uppercase tracking-wide mb-2">
        What This Does and Does Not Prove
      </h3>
      <p class="text-xs muted-help leading-relaxed">
        This analysis uses statistical and structural methods to assess content integrity. It does not
        constitute definitive proof of authenticity or manipulation. Individual signals may produce
        false positives or false negatives depending on the content type, compression history, and
        processing applied to the file. Results should be considered alongside other evidence and
        professional judgement.
      </p>
    </section>

  </div>
</details>
