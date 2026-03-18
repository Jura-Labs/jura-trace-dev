<script lang="ts">
  let { size = 24, className = '' }: { size?: number; className?: string } = $props();
</script>

<svg
  viewBox="0 0 32 32"
  xmlns="http://www.w3.org/2000/svg"
  width={size}
  height={size}
  class={className}
  aria-hidden="true"
  role="img"
>
  <polygon points="6,7 26,7 24,12 8,12" fill="#6B96CC"/>
  <polygon points="4,13 28,13 26,18 6,18" fill="#3E6FA8"/>
  <polygon points="3,19 29,19 27,24 5,24" fill="#3A6499"/>
  <line x1="24" y1="7" x2="8" y2="24" stroke="white" stroke-opacity="0.35" stroke-width="1.5" stroke-linecap="round"/>
</svg>
