<script lang="ts">
  interface Props {
    href: string;
    label: string;
    /** Size variant.  `md` (default) is the card-header size used at the AI
        panel and similar.  `sm` is the dense-row size for the integrity-card
        detector rows where vertical space is tight. */
    size?: 'sm' | 'md';
  }
  let { href, label, size = 'md' }: Props = $props();

  const sizeClass = $derived(
    size === 'sm'
      ? 'w-5 h-5 text-[10px]'
      : 'w-6 h-6 text-xs'
  );
</script>

<a
  {href}
  class="inline-flex items-center justify-center {sizeClass} rounded-full border border-flint/30 text-flint-dark dark:text-flint-light shrink-0
         hover:border-lapis hover:text-lapis dark:hover:text-lapis dark:text-lapis-light transition-colors motion-safe:transition-colors
         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
  aria-label={label}
  title={label}
>?</a>
