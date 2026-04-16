<!--
  ContentCredentialsSeal.svelte
  ─────────────────────────────────────────────────────────────────────────
  The C2PA "cr" information pin per the C2PA UX Recommendations v1.4
  specification (§4.1). Rendered at L1 (content badge) and at L2 (adjacent
  to the Content Credentials heading) on the Verify page.

  Pin geometry:
    • Outlined circle with the lower-right quadrant squared off at 90°
      (the official C2PA "cr" pin shape — the squared corner is the
      anti-spoof marker, not a drop pointer).
    • Lowercase "cr" centred inside. The reference typeface is Store
      Norske Ja Medium; when unavailable we fall back to the system
      serif so the letters still read as a mark, not as body copy.

  States (v1.4 removed the "Incomplete" state — two only):
    • valid   — standard pin, malachite stroke/fill. Icon alone = validity.
    • invalid — cinnabar stroke with a secondary warning mark appended;
      the "cr" remains legible so the pin stays identifiable.
    • none    — flint outline only; no Content Credentials present.

  WCAG 2.2 AA:
    • The SVG is aria-hidden; parent region carries the accessible label.
    • Tooltip via <title> provides hover context.
    • Contrast: malachite (#5B8A5F) on obsidian (#1E2128) ≥ 4.5:1.
-->
<script lang="ts">
  interface Props {
    /** Content Credentials validation state. */
    state: 'valid' | 'invalid' | 'none';
    /** Icon size variant. Default: 'md'. */
    size?: 'sm' | 'md' | 'lg';
  }

  const { state, size = 'md' }: Props = $props();

  // Nominal box size for each variant — the pin fills the full square
  // (no drop pointer below). The squared lower-right corner replaces the
  // circle arc in that quadrant; see the SVG path below.
  const SIZE_MAP = {
    sm: { box: 18, stroke: 1.5, text: 8  },
    md: { box: 24, stroke: 1.75, text: 11 },
    lg: { box: 32, stroke: 2,   text: 14 },
  } as const;

  const dims = $derived(SIZE_MAP[size]);

  const COLOR_MAP = {
    valid:   { fill: '#5B8A5F', stroke: '#5B8A5F', text: '#EDEAE4' },
    invalid: { fill: '#C0392B', stroke: '#C0392B', text: '#EDEAE4' },
    none:    { fill: 'none',    stroke: '#78756D', text: '#78756D' },
  } as const;

  const colors = $derived(COLOR_MAP[state]);

  const TOOLTIP_MAP = {
    valid:   'Content Credentials present and verified',
    invalid: 'Content Credential unavailable or invalid',
    none:    'No Content Credentials attached',
  } as const;

  const tooltip = $derived(TOOLTIP_MAP[state]);

  // Pin path: a circle of radius r with the lower-right quadrant replaced
  // by two straight edges meeting at a right-angled corner. Drawn
  // counter-clockwise starting at the top of the circle:
  //   1. Arc from (cx, cy − r) to (cx + r, cy)        — top-right quadrant
  //   2. Line from (cx + r, cy) to (cx + r, cy + r)   — right edge down
  //   3. Line from (cx + r, cy + r) to (cx, cy + r)   — bottom edge left
  //   4. Arc from (cx, cy + r) to (cx, cy − r)        — left half of circle
  const cx     = $derived(dims.box / 2);
  const cy     = $derived(dims.box / 2);
  const r      = $derived(dims.box / 2 - dims.stroke);
  const pinPath = $derived(
    `M ${cx} ${cy - r} ` +
    `A ${r} ${r} 0 0 1 ${cx + r} ${cy} ` +
    `L ${cx + r} ${cy + r} ` +
    `L ${cx} ${cy + r} ` +
    `A ${r} ${r} 0 1 1 ${cx} ${cy - r} Z`
  );

  // Secondary warning badge for invalid state — sits at the squared corner.
  // A small triangle with an exclamation mark, attached to the bottom-right
  // tip so the "cr" inside the pin remains fully legible (§4.1 forbids
  // drawing over the "cr" characters).
  const warningSize = $derived(dims.box * 0.42);
  const warningOffset = $derived(dims.box - warningSize * 0.9);
</script>

<svg
  width={dims.box}
  height={dims.box}
  viewBox="0 0 {dims.box} {dims.box}"
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
  aria-hidden="true"
  focusable="false"
  role="img"
  class="flex-shrink-0"
>
  <title>{tooltip}</title>

  <!-- Pin body: circle with the lower-right quadrant squared off. -->
  <path
    d={pinPath}
    fill={colors.fill}
    stroke={colors.stroke}
    stroke-width={dims.stroke}
    stroke-linejoin="miter"
  />

  <!-- "cr" text mark — centred, slightly lifted so the mark sits visually
       centred with the squared corner rather than the geometric centre. -->
  <text
    x={cx - dims.stroke * 0.4}
    y={cy + dims.text * 0.36}
    text-anchor="middle"
    font-family="Georgia, 'Times New Roman', Cambria, serif"
    font-size={dims.text}
    font-weight="600"
    fill={colors.text}
    letter-spacing="-0.5"
  >cr</text>

  {#if state === 'invalid'}
    <!-- Secondary warning mark — a small triangle at the squared corner.
         Placed flush with the corner so the "cr" remains uncovered. -->
    <g transform="translate({warningOffset} {warningOffset})">
      <path
        d="M {warningSize / 2} 0 L {warningSize} {warningSize} L 0 {warningSize} Z"
        fill="#C0392B"
        stroke="#EDEAE4"
        stroke-width="0.75"
        stroke-linejoin="round"
      />
      <line
        x1={warningSize / 2}
        y1={warningSize * 0.3}
        x2={warningSize / 2}
        y2={warningSize * 0.65}
        stroke="#EDEAE4"
        stroke-width={dims.stroke * 0.8}
        stroke-linecap="round"
      />
      <circle
        cx={warningSize / 2}
        cy={warningSize * 0.82}
        r={dims.stroke * 0.45}
        fill="#EDEAE4"
      />
    </g>
  {/if}
</svg>
