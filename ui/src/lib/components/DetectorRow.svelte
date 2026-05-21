<!--
  DetectorRow.svelte
  ─────────────────────────────────────────────────────────────────────
  Row chrome for the "Is the content intact?" card on the verify page.

  Consolidates the title-line pattern that was duplicated 11 times in
  `routes/verify/+page.svelte` before this component existed:

    [status icon]  Detector name  [optional badges]  [?]   [score%]
    [optional always-visible explanation line]
    {body content as children}

  What this component owns:
    • The <li> wrapper and its suspicious-state amber tint
    • The status icon (cinnabar/amber tick or warning per `suspicious`)
    • The detector title with state-coloured text
    • Inline badges (ExperimentalPill, On-demand chip, Deep chip) via
      the `badges` snippet — rendered between the title and help link
    • The mandatory ContextualHelpLink at /help/forensic-detectors#anchor
    • The `flex-1` spacer that pushes the score to the row's right edge
    • The optional raw-score line via the `rawScore` snippet
    • The score percentage in the appropriate forensicScoreClass colour
    • The optional always-visible explanation line beneath the row title
      (for detectors with no heatmap to look at — Noise Pattern, DCT)

  What this component does NOT own:
    • Sub-line copy (each detector has unique wording, kept as parent-
      side conditional siblings) — pass via the default `children` slot.
    • Heatmap visualisations (use ImageZoom directly in `children`).
    • Raw-score grids and detail accordions (also `children`).

  This separation keeps the component small enough to reason about
  while still removing the per-row chrome boilerplate that motivated
  the refactor.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import ContextualHelpLink from './ContextualHelpLink.svelte';
  import { forensicScoreClass } from '$lib/scoring';

  interface Props {
    /** Detector display name (e.g. "Error Level Analysis"). */
    name: string;
    /** Continuous score in 0..1; rendered as a percentage and used for
        the colour class on the score text. */
    score: number;
    /** Whether the detector flagged this image.  Drives the row tint
        (amber background), the status icon (warning vs check), and the
        title-text colour. */
    suspicious: boolean;
    /** Anchor fragment on /help/forensic-detectors (e.g. `ela`,
        `noise-pattern`).  Concatenated to form the help link target. */
    helpAnchor: string;
    /** Accessible label for the help link.  Should answer the question
        "what does this help link explain?".  Defaults to a generic
        "Learn about {name}" if not supplied. */
    helpLabel?: string;
    /** Optional always-visible explanation line, shown directly under
        the row title.  For detectors with no heatmap (Noise Pattern,
        DCT) where the row would otherwise be a bare percentage. */
    alwaysVisibleHint?: string;
    /** Optional snippet for inline badges between the title and the
        help link.  Use for ExperimentalPill, On-demand chips, and
        Deep-mode chips. */
    badges?: Snippet;
    /** Optional snippet for the raw-score display rendered between
        the spacer and the score percentage (e.g. "score: 0.74 ·
        threshold: 0.49").  Caller is responsible for gating on
        showRawScores if relevant. */
    rawScore?: Snippet;
    /** Body content rendered after the row title — sub-lines,
        heatmap visualisations, raw-score grids, detail accordions. */
    children?: Snippet;
  }

  const {
    name,
    score,
    suspicious,
    helpAnchor,
    helpLabel,
    alwaysVisibleHint,
    badges,
    rawScore,
    children,
  }: Props = $props();

  const helpHref = $derived(`/help/forensic-detectors#${helpAnchor}`);
  const resolvedHelpLabel = $derived(helpLabel ?? `What does ${name} check?`);
</script>

<li class="px-5 py-3 {suspicious ? 'bg-amber/[0.04]' : ''}">
  <div class="flex items-center gap-3">
    <svg
      class="w-3.5 h-3.5 flex-shrink-0 {suspicious
        ? 'text-amber-dark dark:text-amber-light'
        : 'text-malachite-dark dark:text-malachite-light'}"
      viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"
      aria-hidden="true"
    >
      {#if suspicious}
        <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
      {:else}
        <polyline points="20 6 9 17 4 12" />
      {/if}
    </svg>
    <span class="text-sm {suspicious ? 'text-amber-dark dark:text-amber-light font-medium' : 'text-obsidian dark:text-quartz'}">{name}</span>
    {@render badges?.()}
    <ContextualHelpLink size="sm" href={helpHref} label={resolvedHelpLabel} />
    <span class="flex-1"></span>
    {@render rawScore?.()}
    <span class="text-xs tabular-nums {forensicScoreClass(score)}">{Math.round(score * 100)}%</span>
  </div>

  {#if alwaysVisibleHint}
    <p class="text-xs text-flint-dark dark:text-flint-light mt-1 ml-6 leading-snug">{alwaysVisibleHint}</p>
  {/if}

  {@render children?.()}
</li>
