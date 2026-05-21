<!--
  AiDetectorRow.svelte
  ─────────────────────────────────────────────────────────────────────
  Row chrome for the "Is this AI-generated?" card on the verify page.

  Sibling of `DetectorRow.svelte` for the integrity card — separated
  rather than parameterised because the AI card's row layout is
  structurally different:

    Integrity card (DetectorRow):
      [icon] [name] [badges] [?]      [raw]  [score%]
       py-3, items-center, single-line title

    AI card (AiDetectorRow):
      [icon] [name + badges + ?]                [score%]
             [summary, accordion, etc.]
       py-4, items-start, multi-line content column

  Visual state still keys off a binary `suspicious` flag (matching the
  current GBM/CLIP rendering convention — the 3-state verdict_level is
  surfaced via gated raw-score text, not via the row chrome itself).
  Help affordance is intentionally omitted: the existing
  ContextualHelpLink at the AI card panel header covers both detectors
  collectively, and the per-row ExperimentalPill on the CLIP/UnivFD
  row is itself a link to the help page.

  What this component owns:
    • <li> wrapper and suspicious-state amber tint
    • Status icon (warning if suspicious, tick otherwise)
    • Title text + state-coloured emphasis
    • Confidence chip (e.g. "high confidence") via prop
    • Inline-badge slot for ExperimentalPill, threshold display, etc.
    • Score percentage in the appropriate forensicScoreClass colour
      (rendered to the right of the content column)

  What this component does NOT own:
    • Summary text (each row has unique copy)
    • Verdict text, signals accordion, zero-shot bars, raw-score
      blocks — all `children`.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { forensicScoreClass } from '$lib/scoring';

  interface Props {
    /** Detector display name (e.g. "AI Generation (GBM Deepfake)"). */
    name: string;
    /** Continuous score in 0..1; rendered as a percentage and used
        for the colour class on the score text. */
    score: number;
    /** Whether the detector flagged this image.  Drives row tint,
        status icon, and title text colour.  When the underlying
        detector exposes a 3-state verdict_level, callers should
        derive `suspicious` from it (synthetic OR inconclusive →
        true; authentic → false) so the visual state matches the
        narrative state. */
    suspicious: boolean;
    /** Optional confidence label rendered as a chip after the title.
        Caller is responsible for the wording (e.g. "high confidence"
        vs the bare confidence value). */
    confidence?: string;
    /** Optional snippet for inline badges between the title /
        confidence chip and the score percentage.  Use for
        ExperimentalPill, threshold display, etc. */
    badges?: Snippet;
    /** Body content — summary text, verdict line, signals accordion,
        zero-shot bars, raw-score blocks. */
    children?: Snippet;
  }

  const { name, score, suspicious, confidence, badges, children }: Props = $props();
</script>

<li class="px-5 py-4 {suspicious ? 'bg-amber/[0.04]' : ''}">
  <div class="flex items-start gap-3">
    <svg
      class="w-4 h-4 mt-0.5 flex-shrink-0 {suspicious
        ? 'text-amber-dark dark:text-amber-light'
        : 'text-malachite-dark dark:text-malachite-light'}"
      viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
      aria-hidden="true"
    >
      {#if suspicious}
        <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
      {:else}
        <polyline points="20 6 9 17 4 12" />
      {/if}
    </svg>

    <div class="flex-1 min-w-0">
      <div class="flex items-center gap-2 mb-1 flex-wrap">
        <span class="text-sm font-medium {suspicious ? 'text-amber-dark dark:text-amber-light' : 'text-obsidian dark:text-quartz'}">{name}</span>
        {#if confidence}
          <span class="text-xs px-1.5 py-0.5 rounded bg-gray-100 dark:bg-graphite-light border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light">{confidence}</span>
        {/if}
        {@render badges?.()}
      </div>
      {@render children?.()}
    </div>

    <span class="text-sm font-medium tabular-nums flex-shrink-0 {forensicScoreClass(score)}">{Math.round(score * 100)}%</span>
  </div>
</li>
