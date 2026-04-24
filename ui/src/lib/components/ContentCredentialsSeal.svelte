<!--
  ContentCredentialsSeal.svelte
  ─────────────────────────────────────────────────────────────────────────
  The official C2PA "cr" information pin per the C2PA UX Recommendations
  v1.4 specification (§4.1).

  The geometry is the canonical mark supplied by the CAI Content
  Credentials conformance tool — see `c2pa-cr-pin-official.svg` in this
  directory for the verbatim reference asset (and its source URL).
  We embed the same three paths here, substituting `currentColor` for
  `black` so the mark can be tinted via CSS for state colouring while the
  shape itself remains the official one.

  States (v1.4 removed the "Incomplete" state — two only):
    • valid   — malachite tint.  Icon alone = validity.
    • invalid — cinnabar tint with a small secondary warning badge at the
                squared corner.  The "cr" letters remain fully legible —
                §4.1 forbids drawing over the letters themselves.
    • none    — flint tint; no Content Credentials present.

  WCAG 2.2 AA:
    • SVG has role="img" and a <title> for tooltip / screen readers.
    • Contrast: malachite (#5B8A5F) and cinnabar (#C0392B) on obsidian
      background ≥ 4.5:1.
-->
<script lang="ts">
  interface Props {
    /** Content Credentials validation state. */
    state: 'valid' | 'invalid' | 'none';
    /** Icon size variant.  Default 'md'. */
    size?: 'sm' | 'md' | 'lg';
  }

  const { state, size = 'md' }: Props = $props();

  // Pixel sizes for each variant.  The official viewBox is 36×36; we scale
  // the rendered <svg> but retain the native coordinate system so the
  // geometry stays pixel-perfect.
  const PIXELS = { sm: 18, md: 24, lg: 32 } as const;
  const px = $derived(PIXELS[size]);

  const TOOLTIP = {
    valid:   'Content Credentials present and verified',
    invalid: 'Content Credential unavailable or invalid',
    none:    'No Content Credentials attached',
  } as const;

  const tooltip = $derived(TOOLTIP[state]);

  // Tailwind colour class for the wrapping span — drives `currentColor`
  // inside the SVG.  Using wrapper class (not inline fill) so dark-mode
  // variants apply consistently with the rest of the palette.
  const TINT = {
    valid:   'text-malachite-dark dark:text-malachite-light',
    invalid: 'text-cinnabar-dark dark:text-cinnabar-light',
    none:    'text-flint-dark dark:text-flint-light',
  } as const;

  const tintClass = $derived(TINT[state]);

  // Secondary warning badge coordinates — viewBox is 36×36, place a small
  // triangle at the squared lower-right corner.  The "cr" sits inside the
  // bounded area from ~(7,12) to ~(29,27) so the corner badge at ~(27,27)
  // does not overlap the letters.
  const WARNING_ORIGIN = 24;
  const WARNING_SIZE = 11;
</script>

<span class="inline-flex flex-shrink-0 {tintClass}">
  <svg
    width={px}
    height={px}
    viewBox="0 0 36 36"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    role="img"
    aria-label={tooltip}
  >
    <title>{tooltip}</title>

    <!-- ── Official C2PA "cr" pin — path data from the CAI conformance
         tool's content_credentials_icon.svg.  `currentColor` replaces
         the source asset's `black` so the colour follows the wrapper's
         `text-*` class. ── -->

    <!-- "c" -->
    <path
      d="M13.6652 26.4829C9.59549 26.4829 7.05518 23.2945 7.05518 19.51C7.05518 15.7255 9.59549 12.5371 13.6652 12.5371C16.9572 12.5371 19.1865 14.6886 19.7826 17.4881H16.4647C16.024 16.2439 14.9872 15.4922 13.6652 15.4922C11.6174 15.4922 10.2694 17.0993 10.2694 19.51C10.2694 21.9207 11.6174 23.5278 13.6652 23.5278C15.039 23.5278 16.1018 22.7243 16.5165 21.4023H19.8086C19.2642 24.2796 17.009 26.4829 13.6652 26.4829Z"
      fill="currentColor"
    />
    <!-- "r" -->
    <path
      d="M21.1194 26.12V12.9H24.23V14.3257C24.9558 13.3666 26.0964 12.7445 27.8072 12.7445H28.6107V15.8032H27.7813C26.6148 15.8032 25.889 16.0624 25.3446 16.5549C24.7225 17.0734 24.3596 17.9288 24.3596 19.2249V26.12H21.1194Z"
      fill="currentColor"
    />
    <!-- Outline pin with squared lower-right corner. -->
    <path
      d="M1.56009 18C1.56009 8.92061 8.92143 1.56009 18.0011 1.56009C27.0819 1.56009 34.4441 8.92156 34.4441 18.0021V34.4399H18C8.9205 34.4399 1.56009 27.0795 1.56009 18Z"
      stroke="currentColor"
      stroke-width="3.12018"
    />

    {#if state === 'invalid'}
      <!-- Secondary warning triangle at the squared corner.  Kept clear
           of the "cr" letters (which occupy the left/centre) per
           spec §4.1 — "do not obscure the letters". -->
      <g transform="translate({WARNING_ORIGIN} {WARNING_ORIGIN})">
        <path
          d="M {WARNING_SIZE / 2} 0 L {WARNING_SIZE} {WARNING_SIZE} L 0 {WARNING_SIZE} Z"
          fill="#C0392B"
          stroke="#EDEAE4"
          stroke-width="1"
          stroke-linejoin="round"
        />
        <line
          x1={WARNING_SIZE / 2}
          y1={WARNING_SIZE * 0.3}
          x2={WARNING_SIZE / 2}
          y2={WARNING_SIZE * 0.65}
          stroke="#EDEAE4"
          stroke-width="1.2"
          stroke-linecap="round"
        />
        <circle
          cx={WARNING_SIZE / 2}
          cy={WARNING_SIZE * 0.82}
          r="0.9"
          fill="#EDEAE4"
        />
      </g>
    {/if}
  </svg>
</span>
