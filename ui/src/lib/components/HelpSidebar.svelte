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
      items: [
        { href: '/help/protect',    label: 'Protect',             title: 'Safeguard digital assets' },
        { href: '/help/verify',     label: 'Verify',              title: 'Forensic analysis and trust scores' },
        { href: '/help/monitor',    label: 'Monitor',             title: 'Activity log and trust landscape' },
        { href: '/help/settings',   label: 'Settings',            title: 'Configuration and deployment' },
      ],
    },
    {
      groupLabel: 'Understanding',
      items: [
        { href: '/help/methodology', label: 'How Analysis Works', title: 'Methodology and trust score computation' },
        { href: '/help/glossary',    label: 'Glossary',           title: 'Definitions of technical terms' },
        { href: '/help/personas',    label: 'Usage Guides',       title: 'Workflows for different use cases' },
      ],
    },
    {
      groupLabel: 'Compliance',
      items: [
        { href: '/help/berkeley-protocol', label: 'Berkeley Protocol',    title: 'International evidence standards alignment' },
        { href: '/help/compliance',        label: 'IT Security Summary',  title: 'Information security and data protection overview' },
        { href: '/help/compliance#dpia',   label: 'DPIA Template',        title: 'Data Protection Impact Assessment guidance' },
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
        class="px-2 mb-1 text-[0.65rem] font-semibold uppercase tracking-widest text-flint dark:text-flint-light select-none"
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
                  : 'text-flint dark:text-flint-light hover:text-lapis dark:hover:text-lapis-light hover:bg-gray-50 dark:hover:bg-graphite-light/30'}
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
