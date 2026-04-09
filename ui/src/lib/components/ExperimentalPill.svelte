<!--
  ExperimentalPill.svelte
  ─────────────────────────────────────────────────────────────────────────
  Canonical "experimental feature" disclosure badge for Jura Trace.

  ADR: docs/decisions/experimental-ui-tag.md (DEC-2026-04-09-001)

  Purpose:
    Every feature whose output is not yet trustworthy enough for evidentiary
    use must carry this pill adjacent to its heading. The pill is a
    transparency disclosure, not a feature gate — the underlying pipeline
    continues to run exactly as before.

  Variant system:
    • informational   → "EXPERIMENTAL — informational only"
                        For features that run but whose output should not be
                        quoted in isolation (e.g. CLIP zero-shot classification).
    • not-in-scoring  → "EXPERIMENTAL — not in scoring"
                        For features visible in Expert View that do not
                        contribute to the numeric trust score.
    • layout          → "EXPERIMENTAL layout"
                        For whole-page or whole-section layout variants
                        (e.g. the verify v2 route when it ships).
    • uncalibrated    → "EXPERIMENTAL — weight uncalibrated"
                        For detectors wired into trust scoring at a
                        consensus weight pending empirical calibration
                        (e.g. JPEG Ghost at 0.5×, S28-FU9).

  Companion element:
    The ADR requires every pill to link to a plain-English explanation.
    Supply either `helpHref` (wraps the pill in an <a>) or `tooltip`
    (adds a title attribute). If neither is supplied the pill renders
    without a companion — a console warning fires in development.

  Visual tokens: bg-lapis/15 text-lapis dark:text-lapis-light border-lapis/30
  Why lapis: advisory / informational (not amber=caution, not cinnabar=error).
-->
<script lang="ts">
  import { onMount } from 'svelte';

  interface Props {
    /** The wording variant — see component doc comment for descriptions. */
    variant?: 'informational' | 'not-in-scoring' | 'layout' | 'uncalibrated';
    /**
     * Optional tooltip text (use this OR helpHref, not both).
     * Short explanations only — the ADR requires a help link for anything
     * longer than one sentence.
     */
    tooltip?: string;
    /**
     * Optional link to a /help/ anchor or calibration document.
     * When supplied, the pill is wrapped in an <a> element so the companion
     * is keyboard-accessible. Should point to a stable in-app anchor.
     */
    helpHref?: string;
    /**
     * Accessible label override.
     * Falls back to a generated label derived from the variant text.
     */
    ariaLabel?: string;
  }

  const {
    variant = 'informational',
    tooltip,
    helpHref,
    ariaLabel,
  }: Props = $props();

  const SUFFIX_MAP: Record<NonNullable<Props['variant']>, string> = {
    'informational':  '— informational only',
    'not-in-scoring': '— not in scoring',
    'layout':         'layout',
    'uncalibrated':   '— weight uncalibrated',
  };

  const ARIA_MAP: Record<NonNullable<Props['variant']>, string> = {
    'informational':  'Experimental feature: informational only, not validated for evidentiary use in isolation',
    'not-in-scoring': 'Experimental feature: not included in the numeric trust score',
    'layout':         'Experimental layout: parallel view under evaluation',
    'uncalibrated':   'Experimental feature: trust score weight is empirically uncalibrated',
  };

  const suffix = $derived(SUFFIX_MAP[variant ?? 'informational']);
  const resolvedAriaLabel = $derived(ariaLabel ?? ARIA_MAP[variant ?? 'informational']);
  const resolvedTitle = $derived(tooltip ?? resolvedAriaLabel);

  // Warn in development if no companion element is provided — ADR requirement.
  onMount(() => {
    if (import.meta.env.DEV && !tooltip && !helpHref) {
      console.warn(
        '[ExperimentalPill] ADR DEC-2026-04-09-001 requires every pill to have ' +
        'a companion element (tooltip or helpHref). Add one of these props.'
      );
    }
    if (import.meta.env.DEV && tooltip && helpHref) {
      console.warn(
        '[ExperimentalPill] Both tooltip and helpHref supplied — use one only. ' +
        'helpHref is preferred for anything longer than one sentence (ADR §companion).'
      );
    }
  });

  const pillClasses =
    'inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-semibold tracking-wider ' +
    'bg-lapis/15 text-lapis dark:text-lapis-light border border-lapis/30 ' +
    'whitespace-nowrap select-none';

  const linkClasses =
    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis ' +
    'focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian ' +
    'rounded-full';
</script>

{#if helpHref}
  <!--
    When a helpHref is provided, wrap the pill in a link so keyboard users
    can reach the explanation without a mouse hover.
  -->
  <a
    href={helpHref}
    class={linkClasses}
    aria-label={resolvedAriaLabel}
    title={resolvedTitle}
  >
    <span class={pillClasses} aria-hidden="true">
      <span class="uppercase">Experimental</span>&nbsp;<span class="normal-case font-medium">{suffix}</span>
    </span>
  </a>
{:else}
  <!--
    No link — render as a non-interactive span with a title tooltip.
    role="note" tells screen readers this is supplementary information.
  -->
  <span
    class={pillClasses}
    role="note"
    aria-label={resolvedAriaLabel}
    title={resolvedTitle}
  >
    <span class="uppercase">Experimental</span>&nbsp;<span class="normal-case font-medium">{suffix}</span>
  </span>
{/if}
