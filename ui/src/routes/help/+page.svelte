<script lang="ts">
  // Topic cards shown on the help index
  const cards = [
    {
      href: '/help/protect',
      title: 'Protect',
      description: 'Learn how to safeguard your digital assets with C2PA provenance credentials and invisible frequency-domain watermarking.',
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
      description: 'Configure Ollama, choose a deployment profile, manage data storage, and review the service status panel.',
    },
    {
      href: '/help/settings#ollama',
      title: 'Ollama AI Features',
      description: 'How to use text reading and claim verification powered by local AI models.',
    },
    {
      href: '/help/methodology',
      title: 'How Analysis Works',
      description: 'Transparency on every detector, how scores are weighted, composite signal amplification, and known limitations.',
    },
    {
      href: '/help/glossary',
      title: 'Glossary',
      description: 'Definitions of technical terms used throughout the application — from ELA and C2PA to perceptual hashing and DWT watermarking.',
    },
    {
      href: '/help/personas',
      title: 'Usage Guides',
      description: 'Practical workflows tailored for museum curators, investigative journalists, fact-checkers, and IT administrators.',
    },
    {
      href: '/help/compliance',
      title: 'IT and Compliance',
      description: 'Information security, data protection, and regulatory compliance for institutional deployment.',
    },
  ] as const;
</script>

<!-- Page heading -->
<header class="mb-8">
  <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
    Documentation and Guidance
  </h1>
  <p class="text-base text-flint dark:text-flint-light leading-relaxed max-w-2xl">
    Jura Trace runs 16+ forensic detectors to verify content authenticity and embeds
    tamper-evident credentials to protect your digital assets. These guides explain how
    each feature works, the methodology behind our analysis, and practical workflows for
    different use cases.
  </p>
</header>

<!-- Earth-line section divider -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<!-- Start here -->
<section aria-label="Getting started steps" class="mb-8">
  <p class="text-xs font-semibold uppercase tracking-widest text-flint dark:text-flint-light mb-4">
    Start here
  </p>
  <ol class="space-y-3">
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">1</span>
      <span>Complete the <strong class="text-text-light dark:text-text-dark">Setup Wizard</strong> on first launch — it checks the Analysis Engine, FFmpeg, and Ollama so everything is ready before you start.</span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">2</span>
      <span>Import your first asset on the <a href="/help/protect" class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"><strong class="text-text-light dark:text-text-dark">Protect</strong> page</a> to apply C2PA provenance or an invisible watermark.</span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">3</span>
      <span>Run your first verification on the <a href="/help/verify" class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"><strong class="text-text-light dark:text-text-dark">Verify</strong> page</a> to see the forensic pipeline in action.</span>
    </li>
  </ol>
</section>

<!-- Earth-line section divider -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<!-- Topic card grid -->
<section aria-label="Help topics">
  <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
    {#each cards as card}
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
        <h2 class="text-base font-heading font-semibold text-text-light dark:text-text-dark mb-2 tracking-heading group-hover:text-lapis dark:group-hover:text-lapis-light motion-safe:transition-colors motion-safe:duration-150">
          {card.title}
        </h2>
        <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
          {card.description}
        </p>
      </a>
    {/each}
  </div>
</section>
