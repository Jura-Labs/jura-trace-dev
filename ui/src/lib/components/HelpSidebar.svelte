<script lang="ts">
  // Props
  let { currentPath }: { currentPath: string } = $props();

  // Sidebar navigation sections
  const sections = [
    {
      groupLabel: 'Overview',
      items: [
        { href: '/help',            label: 'Getting Started',     title: 'Introduction and first steps' },
      ],
    },
    {
      groupLabel: 'Features',
      // Protect + Signing Modes restored 2026-04-28 alongside the
      // top-nav restoration — Validator evaluation phase complete.
      items: [
        { href: '/help/protect',         label: 'Protect',             title: 'Sign content with Content Credentials' },
        { href: '/help/bedrock-signing', label: 'Signing Modes',       title: 'Sovereign vs Conformant signing' },
        { href: '/help/verify',          label: 'Verify',              title: 'Forensic analysis and trust scores' },
        { href: '/help/forensic-detectors', label: 'Detector Reference', title: 'Per-detector reference guide' },
        { href: '/help/monitor',         label: 'Monitor',             title: 'Activity log and trust landscape' },
        { href: '/help/settings',        label: 'Settings',            title: 'Configuration and deployment' },
      ],
    },
    {
      groupLabel: 'Understanding',
      items: [
        { href: '/help/how-it-works', label: 'How It Works',       title: 'Plain-English guide for non-technical users' },
        { href: '/help/methodology',  label: 'How Analysis Works', title: 'Methodology and trust score computation' },
        { href: '/help/model-cards',  label: 'Model Cards',        title: 'ML classifier training data and performance' },
        { href: '/help/glossary',     label: 'Glossary',           title: 'Definitions of technical terms' },
        { href: '/help/personas',     label: 'Usage Guides',       title: 'Workflows for different use cases' },
      ],
    },
    {
      groupLabel: 'Compliance',
      items: [
        { href: '/help/berkeley-protocol', label: 'Berkeley Protocol',    title: 'International evidence standards alignment' },
        { href: '/help/continuity',        label: 'Continuity Promise',   title: 'Data portability and long-term availability commitments' },
        { href: '/help/compliance',        label: 'IT Security Summary',  title: 'Information security and data protection overview' },
        { href: '/help/compliance#dpia',   label: 'DPIA Template',        title: 'Data Protection Impact Assessment guidance' },
        { href: '/help/open-source',       label: 'Open Source Licences', title: 'Licence notices and third-party attributions' },
      ],
    },
  ];

  function isActive(href: string): boolean {
    // Exact match for the index; prefix match for sub-routes
    if (href === '/help') {
      return currentPath === '/help' || currentPath === '/help/';
    }
    return currentPath === href || currentPath.startsWith(href + '/');
  }
</script>

<nav aria-label="Help topics">
  {#each sections as section}
    <div class="mb-6">
      <!-- Section group label -->
      <p
        class="px-2 mb-1 text-[0.65rem] section-label uppercase tracking-widest select-none"
        aria-hidden="true"
      >
        {section.groupLabel}
      </p>

      <ul role="list">
        {#each section.items as item}
          <li>
            <a
              href={item.href}
              title={item.title}
              aria-current={isActive(item.href) ? 'page' : undefined}
              class="
                flex items-center min-h-[44px] px-2 py-1.5 rounded text-sm
                transition-colors duration-150
                focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                {isActive(item.href)
                  ? 'text-lapis dark:text-lapis-light font-medium bg-lapis/5 dark:bg-lapis/10'
                  : 'text-flint-dark dark:text-flint-light hover:text-lapis dark:hover:text-lapis dark:text-lapis-light hover:bg-gray-50 dark:hover:bg-graphite-light/30'}
              "
            >
              {item.label}
            </a>
          </li>
        {/each}
      </ul>
    </div>
  {/each}
</nav>
