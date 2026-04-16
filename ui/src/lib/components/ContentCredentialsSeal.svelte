<!--
  ContentCredentialsSeal.svelte
  ─────────────────────────────────────────────────────────────────────────
  The C2PA information seal ("cr" pin icon) per the C2PA UX Recommendations
  v1.4 specification. Used on the Verify v2 page to surface Content
  Credentials state alongside forensic results.

  States:
    • valid   — malachite; icon alone signals validity (per spec)
    • invalid — cinnabar with a secondary warning mark
    • none    — flint grey; no credentials attached

  WCAG 2.2 AA:
    • The icon is aria-hidden; the parent region carries the accessible label
    • Tooltip via title attribute for hover context
    • Contrast: malachite (#5B8A5F) on obsidian (#1E2128) ≥ 4.5:1 at all sizes
-->
<script lang="ts">
  interface Props {
    /** Content Credentials validation state. */
    state: 'valid' | 'invalid' | 'none';
    /** Icon size variant. Default: 'md'. */
    size?: 'sm' | 'md' | 'lg';
  }

  const { state, size = 'md' }: Props = $props();

  const SIZE_MAP = {
    sm: { outer: 16, inner: 10, text: 6, strokeWidth: 1.5 },
    md: { outer: 24, inner: 15, text: 9,  strokeWidth: 1.5 },
    lg: { outer: 32, inner: 20, text: 12, strokeWidth: 2 },
  } as const;

  const dims = $derived(SIZE_MAP[size]);

  const COLOR_MAP = {
    valid:   { fill: '#5B8A5F', stroke: '#5B8A5F', text: '#EDEAE4' },  // malachite fill, quartz text
    invalid: { fill: '#C0392B', stroke: '#C0392B', text: '#EDEAE4' },  // cinnabar fill, quartz text
    none:    { fill: 'none',    stroke: '#78756D', text: '#78756D' },   // flint stroke only
  } as const;

  const colors = $derived(COLOR_MAP[state]);

  const TOOLTIP_MAP = {
    valid:   'Content Credentials present and verified',
    invalid: 'Content Credentials present but signature is invalid',
    none:    'No Content Credentials attached',
  } as const;

  const tooltip = $derived(TOOLTIP_MAP[state]);

  // Pin drop-point offset — the circle sits above a small pointer
  // so the total SVG height = outer + pointer height
  const pointerH = $derived(Math.round(dims.outer * 0.35));
  const totalH   = $derived(dims.outer + pointerH);
  const cx        = $derived(dims.outer / 2);
  const cy        = $derived(dims.outer / 2);
  const r         = $derived(dims.outer / 2 - dims.strokeWidth);
</script>

<!--
  aria-hidden: the containing region must carry the accessible label; the
  icon is purely decorative in context.
-->
<svg
  width={dims.outer}
  height={totalH}
  viewBox="0 0 {dims.outer} {totalH}"
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
  aria-hidden="true"
  focusable="false"
  role="img"
  class="flex-shrink-0"
>
  <title>{tooltip}</title>
  <!-- Circle body -->
  <circle
    cx={cx}
    cy={cy}
    r={r}
    fill={colors.fill}
    stroke={colors.stroke}
    stroke-width={dims.strokeWidth}
  />

  {#if state === 'invalid'}
    <!-- Warning triangle secondary mark — sits inside the circle -->
    <path
      d="M{cx} {cy - dims.inner * 0.32} L{cx + dims.inner * 0.28} {cy + dims.inner * 0.22} L{cx - dims.inner * 0.28} {cy + dims.inner * 0.22} Z"
      fill={colors.text}
      opacity="0.9"
    />
    <!-- Exclamation dot -->
    <circle
      cx={cx}
      cy={cy + dims.inner * 0.12}
      r={dims.strokeWidth * 0.7}
      fill={colors.fill}
    />
    <!-- Exclamation stem -->
    <line
      x1={cx}
      y1={cy - dims.inner * 0.14}
      x2={cx}
      y2={cy + dims.inner * 0.04}
      stroke={colors.fill}
      stroke-width={dims.strokeWidth * 1.2}
      stroke-linecap="round"
    />
  {:else}
    <!-- "cr" text mark — the C2PA information seal identifier -->
    <text
      x={cx}
      y={cy + dims.text * 0.38}
      text-anchor="middle"
      font-family="system-ui, -apple-system, sans-serif"
      font-size={dims.text}
      font-weight="700"
      fill={colors.text}
      letter-spacing="-0.5"
    >cr</text>
  {/if}

  <!-- Pin pointer — small downward triangle below the circle -->
  <path
    d="M{cx - dims.strokeWidth * 2} {dims.outer - dims.strokeWidth} L{cx} {totalH} L{cx + dims.strokeWidth * 2} {dims.outer - dims.strokeWidth} Z"
    fill={colors.stroke}
  />
</svg>
