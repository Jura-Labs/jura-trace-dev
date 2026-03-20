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
  <circle cx="16" cy="16" r="14" fill="#5A85B5"/>
  <circle cx="16" cy="16" r="9" fill="#EDEAE4"/>
  <circle cx="16" cy="16" r="6" fill="#D8D5CE"/>
  <circle cx="16" cy="16" r="3.5" fill="#1E2128"/>
</svg>
