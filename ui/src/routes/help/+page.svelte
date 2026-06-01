<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';

  // Topic cards shown on the help index.
  //
  // Protect + Monitor restored 2026-04-28 alongside the top-nav
  // restoration — Validator evaluation and pilot-tester walkthrough
  // phases are complete.
  const cards = [
    {
      href: '/help/protect',
      title: 'Protect',
      description: 'Sign your work with Content Credentials using Local Signing (per-install certificate). Conformant signing is under evaluation for a future release.',
    },
    {
      href: '/help/verify',
      title: 'Verify',
      description: 'Understand the forensic analysis pipeline, how trust scores are computed, and the difference between investigation modes.',
    },
    {
      href: '/help/monitor',
      title: 'Monitor',
      description: 'Review your Protection Chronicle, explore the Trust Landscape distribution, and browse the full activity record.',
    },
    {
      href: '/help/settings',
      title: 'Settings',
      description: 'Choose a deployment profile, manage data storage, review service status, and configure updates.',
    },
    {
      href: '/help/how-it-works',
      title: 'How It Works — Start Here',
      description: 'A plain-English walk-through of what Jura Trace checks, why there are two AI checks, and what "Experimental" means. Written for non-technical users.',
    },
    {
      href: '/help/methodology',
      title: 'How Analysis Works',
      description: 'Transparency on every detector, how scores are weighted, composite signal amplification, and known limitations.',
    },
    {
      href: '/help/format-support',
      title: 'Format Support',
      description: 'Which file formats Jura Trace v1.0 can verify and protect, what each format gets in the pipeline, and what is explicitly out of scope.',
    },
    {
      href: '/help/glossary',
      title: 'Glossary',
      description: 'Definitions of technical terms used throughout the application — from ELA and C2PA to perceptual hashing.',
    },
    {
      href: '/help/personas',
      title: 'Usage Guides',
      description: 'Practical workflows tailored for museum curators, investigative journalists, fact-checkers, and IT administrators.',
    },
    {
      href: '/help/model-cards',
      title: 'Model Cards',
      description: 'Training data, performance metrics, known limitations, and version history for the GBM and UnivFD classifiers.',
    },
    {
      href: '/help/continuity',
      title: 'Continuity Promise',
      description: 'Our commitments to data portability, source code release, and long-term availability.',
    },
    {
      href: '/help/compliance',
      title: 'IT and Compliance',
      description: 'Information security, data protection, and regulatory compliance for institutional deployment.',
    },
    {
      href: '/help/open-source',
      title: 'Open Source Licences',
      description: 'Jura Trace’s own licence notice and the attributions for the open source software it is built with.',
    },
  ] as const;

  // ── Help search (Tier 1) ────────────────────────────────────────────────
  // Filters the card list by title or description (case-insensitive substring).
  // When ?search=1 is present in the URL (set by the native "Search Help…" menu
  // item via menu:search-help → goto('/help?search=1')), the input is focused on
  // mount so the user can type immediately.

  let searchQuery = $state('');

  // Mutable copy so we can work around `as const` inference restriction.
  type Card = { href: string; title: string; description: string };
  const allCards: readonly Card[] = cards;

  const filteredCards = $derived(
    searchQuery.trim() === ''
      ? allCards
      : allCards.filter((c) => {
          const q = searchQuery.trim().toLowerCase();
          return c.title.toLowerCase().includes(q) || c.description.toLowerCase().includes(q);
        }),
  );

  let searchInput: HTMLInputElement | undefined = $state();

  onMount(() => {
    // Focus the search input when the page is navigated to with ?search=1.
    // This is set by the native OS menu "Search Help…" item.
    if ($page.url.searchParams.get('search') === '1') {
      searchInput?.focus();
    }
  });
</script>

<!-- Page heading -->
<header class="mb-8">
  <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
    Documentation and Guidance
  </h1>
  <p class="text-base text-text-light dark:text-quartz leading-relaxed max-w-2xl">
    Jura Trace runs 13 forensic detectors to verify content authenticity and embeds
    tamper-evident credentials to protect your digital assets. These guides explain how
    each feature works, the methodology behind our analysis, and practical workflows for
    different use cases.
  </p>
</header>

<!-- Earth-line section divider -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<!-- Start here -->
<section aria-label="Getting started steps" class="mb-8">
  <p class="text-xs section-label uppercase tracking-widest mb-4">
    Start here
  </p>
  <ol class="space-y-3">
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis text-white dark:bg-lapis-light dark:text-obsidian text-xs font-semibold flex items-center justify-center">1</span>
      <span>Confirm the <strong class="text-text-light dark:text-text-dark">Analysis Engine</strong> is online during the brief first-launch wizard. The core forensic pipeline is fully available once the Analysis Engine is running.</span>
    </li>
    <li class="flex gap-3 text-sm text-text-light dark:text-quartz leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis text-white dark:bg-lapis-light dark:text-obsidian text-xs font-semibold flex items-center justify-center">2</span>
      <span>Run your first verification on the <a href="/help/verify" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"><strong class="text-text-light dark:text-text-dark">Verify</strong> page</a> to see the forensic pipeline in action.</span>
    </li>
  </ol>
</section>

<!-- Earth-line section divider -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<!-- Help search -->
<div class="mb-6">
  <label
    for="help-search"
    class="block text-xs section-label uppercase tracking-widest mb-2"
  >
    Search Help
  </label>
  <input
    bind:this={searchInput}
    bind:value={searchQuery}
    id="help-search"
    type="search"
    placeholder="Search topics..."
    autocomplete="off"
    spellcheck="false"
    class="
      w-full max-w-sm px-3 py-2 rounded-lg text-sm
      bg-white dark:bg-graphite
      border border-border-light dark:border-border-dark
      text-text-light dark:text-text-dark
      placeholder-flint-dark dark:placeholder-flint-light
      focus:outline-none focus:ring-2 focus:ring-lapis focus:border-transparent
      dark:focus:ring-lapis-light
    "
    aria-label="Search help topics"
  />
  {#if searchQuery.trim() !== '' && filteredCards.length === 0}
    <p class="mt-3 text-sm muted-help" role="status">
      No topics match <strong class="text-text-light dark:text-text-dark">"{searchQuery.trim()}"</strong>. Try a shorter term.
    </p>
  {:else if searchQuery.trim() !== ''}
    <p class="mt-2 text-xs muted-help" role="status" aria-live="polite">
      {filteredCards.length} {filteredCards.length === 1 ? 'topic' : 'topics'} found
    </p>
  {/if}
</div>

<!-- Topic card grid -->
<section aria-label="Help topics">
  <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
    {#each filteredCards as card}
      <a
        href={card.href}
        class="
          group block p-6 rounded-lg border
          bg-white dark:bg-graphite
          border-border-light dark:border-border-dark
          hover:border-lapis dark:hover:border-lapis-light
          motion-safe:transition-colors motion-safe:duration-150
          focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
          focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
        "
      >
        <h2 class="text-base font-heading font-semibold text-text-light dark:text-text-dark mb-2 tracking-heading group-hover:text-lapis dark:group-hover:text-lapis dark:text-lapis-light motion-safe:transition-colors motion-safe:duration-150">
          {card.title}
        </h2>
        <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
          {card.description}
        </p>
      </a>
    {/each}
  </div>
</section>

<!-- Earth-line section divider -->
<div class="earth-line mt-10 mb-8" role="separator" aria-hidden="true"></div>

<!-- Equity positioning -->
<section aria-labelledby="equity-heading" class="mb-8">
  <h2 id="equity-heading" class="font-heading text-lg text-text-light dark:text-quartz mb-3 tracking-heading">
    Built for Those Who Need It Most
  </h2>
  <p class="text-sm text-text-light dark:text-quartz leading-relaxed max-w-2xl">
    Jura Trace is designed to work where verification is most urgent and resources
    are most constrained — offline, on-device, without sending sensitive content to
    any server. It is free for journalists, fact-checkers, and human rights
    organisations, because the communities facing the greatest threat from synthetic
    media should not face a paywall to verify it. The local-first architecture means
    no internet connection is required, no data leaves your machine, and no cloud
    account is needed.
  </p>
</section>
