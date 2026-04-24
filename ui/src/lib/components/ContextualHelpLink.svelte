<script lang="ts">
  let { href, label }: { href: string; label: string } = $props();
</script>

<a
  {href}
  class="inline-flex items-center justify-center w-6 h-6 rounded-full border border-flint/30 text-flint-dark dark:text-flint-light text-xs
         hover:border-lapis hover:text-lapis dark:hover:text-lapis dark:text-lapis-light transition-colors motion-safe:transition-colors
         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
  aria-label={label}
  title={label}
>?</a>
