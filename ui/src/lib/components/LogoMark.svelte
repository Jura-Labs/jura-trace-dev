<script lang="ts">
  let { size = 24, className = '' }: { size?: number; className?: string } = $props();
</script>

<!--
  Jura Trace eye-mark (almond/lens form).
  Mirrors docs/branding/logo-eye-mark.svg exactly so the in-app mark and
  the brand assets cannot drift apart.

  At sub-32 px the quartz catchlight becomes sub-pixel and effectively
  drops out of the render — by design.  The almond + iris + pupil remain
  legible all the way down to 14 px.
-->
<svg
  viewBox="0 0 256 256"
  xmlns="http://www.w3.org/2000/svg"
  width={size}
  height={size}
  class={className}
  aria-hidden="true"
  role="img"
>
  <!-- Almond outline (lapis) -->
  <path
    d="M14,127c76.67-98.67,153.33-98.67,230,0-76.67,98.67-153.33,98.67-230,0Z"
    fill="none"
    stroke="#5A85B5"
    stroke-width="16"
    stroke-linejoin="round"
  />
  <!-- Iris (malachite fill, lapis ring) -->
  <circle cx="129.62" cy="127.38" r="54.62" fill="#7DA771" stroke="#5A85B5" stroke-width="12"/>
  <!-- Pupil -->
  <circle cx="130" cy="127" r="22" fill="#000000"/>
  <!-- Catchlight -->
  <circle cx="137.97" cy="111.32" r="4.32" fill="#EDEAE4"/>
</svg>
