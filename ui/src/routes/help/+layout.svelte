<script lang="ts">
  import { onMount } from 'svelte';
  import HelpSidebar from '$lib/components/HelpSidebar.svelte';

  let { children } = $props();

  // Track current path for sidebar active-link highlighting.
  // Uses window.location.pathname in SPA mode (no $app/state page store in static adapter).
  let currentPath = $state('/help');

  onMount(() => {
    currentPath = window.location.pathname;
  });
</script>

<!--
  Help section layout: two-column on desktop (sidebar + content),
  horizontal scrollable tab strip on mobile.
-->
<div class="flex flex-col md:flex-row gap-0 md:gap-10 min-h-[60vh]">

  <!-- Desktop sidebar -->
  <aside
    class="hidden md:block flex-none w-[200px] pt-1"
    aria-label="Help navigation panel"
  >
    <div class="sticky top-24">
      <HelpSidebar {currentPath} />
    </div>
  </aside>

  <!-- Mobile: horizontal scrollable nav strip -->
  <nav
    class="md:hidden overflow-x-auto -mx-6 px-6 pb-3 mb-6 border-b border-border-light dark:border-[rgba(122,119,112,0.15)]"
    aria-label="Help topics"
  >
    <ul role="list" class="flex gap-1 min-w-max">
      {#each [
        { href: '/help',                 label: 'Getting Started' },
        { href: '/help/how-it-works',    label: 'How It Works' },
        // Protect + Signing Modes restored 2026-04-28 alongside the
        // top-nav restoration — Validator evaluation phase complete.
        { href: '/help/protect',         label: 'Protect' },
        { href: '/help/bedrock-signing', label: 'Signing Modes' },
        { href: '/help/verify',          label: 'Verify' },
        { href: '/help/forensic-detectors', label: 'Detector Reference' },
        { href: '/help/monitor',         label: 'Monitor' },
        { href: '/help/settings',        label: 'Settings' },
        { href: '/help/methodology',        label: 'How Analysis Works' },
        { href: '/help/model-cards',        label: 'Model Cards' },
        { href: '/help/glossary',            label: 'Glossary' },
        { href: '/help/personas',            label: 'Usage Guides' },
        { href: '/help/berkeley-protocol',   label: 'Berkeley Protocol' },
        { href: '/help/continuity',          label: 'Continuity Promise' },
        { href: '/help/compliance',          label: 'IT Security' },
        { href: '/help/open-source',         label: 'Open Source Licences' },
      ] as item}
        <li>
          <a
            href={item.href}
            aria-current={
              (item.href === '/help'
                ? currentPath === '/help' || currentPath === '/help/'
                : currentPath === item.href || currentPath.startsWith(item.href + '/'))
                ? 'page'
                : undefined
            }
            class="
              inline-flex items-center min-h-[44px] px-3 py-1 rounded text-sm whitespace-nowrap
              transition-colors duration-150
              focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
              focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
              {(item.href === '/help'
                ? currentPath === '/help' || currentPath === '/help/'
                : currentPath === item.href || currentPath.startsWith(item.href + '/'))
                ? 'text-lapis dark:text-lapis-light font-medium bg-lapis/5 dark:bg-lapis/10'
                : 'muted-help hover:text-lapis dark:hover:text-lapis'}
            "
          >
            {item.label}
          </a>
        </li>
      {/each}
    </ul>
  </nav>

  <!-- Main content area -->
  <div class="flex-1 min-w-0">
    {@render children()}
  </div>
</div>
