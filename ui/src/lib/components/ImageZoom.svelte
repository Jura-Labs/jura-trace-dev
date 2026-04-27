<script lang="ts">
  interface Props {
    src: string;
    alt: string;
    /** Tailwind classes for the rendered thumbnail. Defaults to a block-level
        full-width image capped at 12rem high — the same treatment used by the
        newer detector heatmaps (colour temperature, shadow, splice, NPR). */
    thumbClass?: string;
    /** Optional caption rendered beneath the thumbnail in small italic. */
    caption?: string;
    /** Optional override for the click button's aria-label.  Defaults to
        "{alt} — click to enlarge". */
    label?: string;
  }

  let {
    src,
    alt,
    thumbClass = 'w-full max-h-48 object-contain rounded border border-border-light dark:border-border-dark',
    caption,
    label,
  }: Props = $props();

  let open = $state(false);

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) open = false;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<button
  type="button"
  class="block group cursor-zoom-in rounded focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
  aria-label={label ?? `${alt} — click to enlarge`}
  onclick={() => (open = true)}
>
  <img {src} {alt} class="{thumbClass} group-hover:opacity-90 motion-safe:transition-opacity motion-safe:duration-150" />
  {#if caption}
    <span class="block text-[10px] text-flint-dark dark:text-flint-light mt-1 italic">{caption}</span>
  {/if}
</button>

{#if open}
  <!-- Backdrop is a button so the click-to-close interaction is exposed as an
       accessible control without an extra noninteractive role attribute. -->
  <button
    type="button"
    class="fixed inset-0 z-[100] bg-black/85 flex items-center justify-center cursor-zoom-out p-4 focus:outline-none"
    aria-label="Close full-size image preview"
    onclick={() => (open = false)}
  ></button>
  <div
    class="fixed inset-0 z-[100] flex items-center justify-center p-4 pointer-events-none"
    role="dialog"
    aria-label="Full-size image preview — press Escape to close"
    aria-modal="true"
  >
    <img
      {src}
      alt="Full-size: {alt}"
      class="max-w-[90vw] max-h-[85vh] rounded-lg shadow-2xl pointer-events-auto"
    />
    {#if caption}
      <p class="absolute bottom-6 left-1/2 -translate-x-1/2 max-w-[80vw] text-center text-sm text-quartz bg-black/60 px-3 py-1.5 rounded pointer-events-none">
        {caption}
      </p>
    {/if}
    <button
      type="button"
      class="absolute top-4 right-4 w-10 h-10 flex items-center justify-center rounded-full bg-white/80 dark:bg-graphite/80 text-obsidian dark:text-quartz
             hover:bg-white dark:hover:bg-graphite focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light pointer-events-auto"
      aria-label="Close image preview"
      onclick={() => (open = false)}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <line x1="18" y1="6" x2="6" y2="18" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
    </button>
  </div>
{/if}
