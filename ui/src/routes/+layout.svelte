<script lang="ts">
  import '../app.css';

  let darkMode = true;

  function toggleDarkMode() {
    darkMode = !darkMode;
    if (darkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }

  // Navigation items
  const navItems = [
    { href: '/', label: 'Dashboard', title: 'Overview and statistics' },
    { href: '/protect', label: 'Protect', title: 'Safeguard digital assets' },
    { href: '/verify', label: 'Verify', title: 'Check content authenticity' },
    { href: '/settings', label: 'Settings', title: 'Application preferences' },
  ];
</script>

<div class="min-h-screen bg-surface-light dark:bg-surface-dark text-text-light dark:text-text-dark">
  <!-- Header -->
  <header class="border-b border-border-light dark:border-border-dark">
    <nav class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
      <!-- Brand -->
      <a href="/" class="brand-name text-lg text-text-light dark:text-text-dark">
        Jura Archive
      </a>

      <!-- Navigation -->
      <div class="flex items-center gap-6">
        {#each navItems as item}
          <a
            href={item.href}
            class="nav-link text-flint dark:text-flint-light hover:text-lapis dark:hover:text-lapis-light transition-colors"
            title={item.title}
          >
            {item.label}
          </a>
        {/each}

        <!-- Dark mode toggle -->
        <button
          onclick={toggleDarkMode}
          class="text-flint dark:text-flint-light hover:text-lapis dark:hover:text-lapis-light transition-colors text-sm"
          title={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}
          aria-label={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}
        >
          {darkMode ? 'Light' : 'Dark'}
        </button>
      </div>
    </nav>
  </header>

  <!-- Main content -->
  <main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <slot />
  </main>

  <!-- Footer -->
  <footer class="border-t border-border-light dark:border-border-dark mt-auto">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4 text-center text-sm text-flint dark:text-flint-light">
      <span class="brand-name text-xs">Jura Archive</span>
      <span class="mx-2">v0.1.0-dev</span>
      <span class="mx-2">|</span>
      <span>Local-first. Your data stays here.</span>
    </div>
  </footer>
</div>
