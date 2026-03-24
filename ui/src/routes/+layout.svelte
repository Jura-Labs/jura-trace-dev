<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import OnboardingOverlay from '$lib/components/OnboardingOverlay.svelte';
  import LogoMark from '$lib/components/LogoMark.svelte';

  let { children } = $props();

  let darkMode = $state(true);
  let showOnboarding = $state(false);
  let mobileMenuOpen = $state(false);
  let currentPath = $state('/');

  onMount(() => {
    const stored = localStorage.getItem('jura-dark-mode');
    // Treat absence or 'true' as dark (dark-first default)
    darkMode = stored === null ? true : stored === 'true';
    applyTheme(darkMode);

    currentPath = window.location.pathname;

    // Show onboarding on first launch (no prior completion recorded)
    if (!localStorage.getItem('jura-onboarded')) {
      showOnboarding = true;
    }
  });

  function completeOnboarding() {
    localStorage.setItem('jura-onboarded', 'true');
    showOnboarding = false;
  }

  function applyTheme(dark: boolean) {
    if (dark) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }

  function toggleDarkMode() {
    darkMode = !darkMode;
    localStorage.setItem('jura-dark-mode', String(darkMode));
    applyTheme(darkMode);
  }

  function toggleMobileMenu() {
    mobileMenuOpen = !mobileMenuOpen;
    // Lock body scroll when menu is open
    if (typeof document !== 'undefined') {
      document.body.style.overflow = mobileMenuOpen ? 'hidden' : '';
    }
  }

  function closeMobileMenu() {
    mobileMenuOpen = false;
    if (typeof document !== 'undefined') {
      document.body.style.overflow = '';
    }
  }

  function handleNavClick(href: string) {
    currentPath = href;
    closeMobileMenu();
  }

  // Navigation items
  const navItems = [
    { href: '/',         label: 'Dashboard', title: 'Overview and statistics' },
    { href: '/protect',  label: 'Protect',   title: 'Safeguard digital assets' },
    { href: '/verify',   label: 'Verify',    title: 'Check content authenticity' },
    { href: '/monitor',  label: 'Monitor',   title: 'Track content protection and verification' },
    { href: '/settings', label: 'Settings',  title: 'Application preferences' },
    { href: '/help',     label: 'Help',      title: 'Documentation and guidance' },
  ];
</script>

<!-- Skip navigation -->
<a
  href="#main-content"
  class="sr-only focus:not-sr-only focus:fixed focus:top-4 focus:left-4 focus:z-50 focus:px-4 focus:py-2.5 focus:bg-lapis focus:text-white focus:rounded-lg focus:ring-2 focus:ring-lapis-light focus:shadow-lg focus:text-sm"
>
  Skip to main content
</a>

<div class="min-h-screen flex flex-col bg-surface-light dark:bg-surface-dark text-text-light dark:text-text-dark">
  <!-- Header -->
  <header class="border-b border-border-light dark:border-[rgba(122,119,112,0.15)] sticky top-0 z-40 bg-surface-light dark:bg-surface-dark">
    <div class="max-w-5xl mx-auto px-6 lg:px-8 h-16 flex items-center justify-between">

      <!-- Brand: logo + name -->
      <a
        href="/"
        onclick={() => handleNavClick('/')}
        class="flex items-center gap-2.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded"
        aria-label="Jura Trace — home"
      >
        <LogoMark size={24} />
        <span class="brand-name text-sm text-text-light dark:text-text-dark">Jura Trace</span>
      </a>

      <!-- Desktop nav -->
      <nav class="hidden md:flex items-center gap-6" aria-label="Main navigation">
        {#each navItems as item}
          <a
            href={item.href}
            onclick={() => handleNavClick(item.href)}
            class="nav-link transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded
                   {currentPath === item.href
                     ? 'text-lapis dark:text-[#8AABBF]'
                     : 'text-flint dark:text-flint-light hover:text-lapis dark:hover:text-[#8AABBF]'}"
            title={item.title}
            aria-current={currentPath === item.href ? 'page' : undefined}
          >
            {item.label}
          </a>
        {/each}

        <!-- External link -->
        <a
          href="https://juralabs.org"
          target="_blank"
          rel="noopener noreferrer"
          class="text-xs text-flint dark:text-flint-light hover:text-lapis dark:hover:text-lapis-light transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded"
        >
          Juralabs.org
          <span class="sr-only">(opens in new tab)</span>
        </a>

        <!-- Dark mode toggle -->
        <button
          onclick={toggleDarkMode}
          class="min-w-[44px] min-h-[44px] flex items-center justify-center text-flint dark:text-flint-light hover:text-lapis dark:hover:text-lapis-light transition-colors text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded px-2"
          title={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}
          aria-label={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}
          aria-pressed={darkMode}
        >
          {darkMode ? 'Light' : 'Dark'}
        </button>
      </nav>

      <!-- Mobile controls -->
      <div class="flex items-center gap-2 md:hidden">
        <!-- Dark mode toggle (mobile) -->
        <button
          onclick={toggleDarkMode}
          class="min-w-[44px] min-h-[44px] flex items-center justify-center text-flint dark:text-flint-light hover:text-lapis dark:hover:text-lapis-light transition-colors text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded"
          aria-label={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}
          aria-pressed={darkMode}
        >
          {darkMode ? 'Light' : 'Dark'}
        </button>

        <!-- Hamburger button -->
        <button
          onclick={toggleMobileMenu}
          class="min-w-[44px] min-h-[44px] flex items-center justify-center text-flint dark:text-flint-light hover:text-text-light dark:hover:text-text-dark transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded"
          aria-label={mobileMenuOpen ? 'Close navigation menu' : 'Open navigation menu'}
          aria-expanded={mobileMenuOpen}
          aria-controls="mobile-menu"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
            {#if mobileMenuOpen}
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            {:else}
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
            {/if}
          </svg>
        </button>
      </div>

    </div>

    <!-- Mobile menu -->
    {#if mobileMenuOpen}
      <div
        id="mobile-menu"
        class="md:hidden border-t border-border-light dark:border-[rgba(122,119,112,0.15)] bg-surface-light dark:bg-surface-dark"
      >
        <nav class="flex flex-col py-2 max-w-4xl mx-auto px-6" aria-label="Mobile navigation">
          {#each navItems as item}
            <a
              href={item.href}
              onclick={() => handleNavClick(item.href)}
              class="flex items-center h-[44px] px-2 text-sm nav-link transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded
                     {currentPath === item.href
                       ? 'text-lapis dark:text-lapis-light'
                       : 'text-flint dark:text-flint-light hover:text-lapis dark:hover:text-lapis-light'}"
              aria-current={currentPath === item.href ? 'page' : undefined}
            >
              {item.label}
            </a>
          {/each}
          <a
            href="https://juralabs.org"
            target="_blank"
            rel="noopener noreferrer"
            class="flex items-center h-[44px] px-2 text-sm text-flint dark:text-flint-light hover:text-lapis dark:hover:text-lapis-light transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
          >
            Juralabs.org
            <span class="sr-only">(opens in new tab)</span>
          </a>
        </nav>
      </div>
    {/if}
  </header>

  <!-- Main content — constrained to Sanctuary max-width of ~900px -->
  <main
    id="main-content"
    tabindex="-1"
    class="flex-1 max-w-4xl w-full mx-auto px-6 lg:px-8 py-10 outline-none"
  >
    {@render children()}
  </main>

  <!-- Footer -->
  <footer class="border-t border-border-light dark:border-[rgba(122,119,112,0.15)] mt-auto">
    <div class="max-w-4xl mx-auto px-6 lg:px-8 py-8">
      <div class="flex flex-col sm:flex-row items-center justify-between gap-4 text-sm text-flint dark:text-flint-light">
        <div class="flex items-center gap-3">
          <LogoMark size={16} />
          <span class="brand-name text-xs text-text-light dark:text-text-dark">Jura Trace</span>
          <span class="text-xs">v0.5.0-dev</span>
        </div>
        <p class="text-xs text-center italic text-flint dark:text-flint-light max-w-sm leading-relaxed">
          Keep people at the heart of every decision. Use technology to support and guide, not to take over.
        </p>
        <div class="flex items-center gap-4 text-xs">
          <a
            href="https://juralabs.org"
            target="_blank"
            rel="noopener noreferrer"
            class="hover:text-lapis dark:hover:text-lapis-light transition-colors underline-offset-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >
            Juralabs CIC
            <span class="sr-only">(opens in new tab)</span>
          </a>
          <a
            href="https://juralabs.org/jura-trace"
            target="_blank"
            rel="noopener noreferrer"
            class="hover:text-lapis dark:hover:text-lapis-light transition-colors underline-offset-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >
            About
            <span class="sr-only">(opens in new tab)</span>
          </a>
        </div>
      </div>
    </div>
  </footer>
</div>

{#if showOnboarding}
  <OnboardingOverlay onComplete={completeOnboarding} />
{/if}
