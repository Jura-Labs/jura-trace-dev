<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import OnboardingOverlay from '$lib/components/OnboardingOverlay.svelte';
  import SetupWizard from '$lib/components/SetupWizard.svelte';
  import LogoMark from '$lib/components/LogoMark.svelte';
  import FeedbackPanel from '$lib/components/FeedbackPanel.svelte';
  import { getSkipWizard, getVersion, checkSidecarHealth } from '$lib/api';

  let { children } = $props();

  let darkMode = $state(true);
  let showOnboarding = $state(false);
  let showSetupWizard = $state(false);
  let showFeedback = $state(false);
  let showSidecarReminder = $state(false);
  let sidecarReminderDismissed = $state(false);
  let mobileMenuOpen = $state(false);
  let currentPath = $state('/');

  // ── Webview zoom ──────────────────────────────────────────────────────────
  // Persists across launches via localStorage.  Steps 0.1 in range [0.8, 1.5].
  // Applied via the Tauri Webview.setZoom() API (available in @tauri-apps/api/webview).
  // In plain-browser dev mode the API import is skipped gracefully.
  const ZOOM_MIN = 0.8;
  const ZOOM_MAX = 1.5;
  const ZOOM_STEP = 0.1;
  const ZOOM_DEFAULT = 1.0;
  const ZOOM_KEY = 'jura-ui-zoom';

  let zoomFactor = $state(ZOOM_DEFAULT);

  async function applyZoom(factor: number) {
    const tauriAvailable = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
    if (!tauriAvailable) return;
    try {
      const { getCurrentWebview } = await import('@tauri-apps/api/webview');
      await getCurrentWebview().setZoom(factor);
    } catch {
      // Non-fatal — setZoom may not be available in dev/mock builds
    }
  }

  function clampZoom(f: number): number {
    return Math.round(Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, f)) * 10) / 10;
  }

  function handleZoom(direction: 'in' | 'out' | 'reset') {
    let next: number;
    if (direction === 'reset') {
      next = ZOOM_DEFAULT;
    } else if (direction === 'in') {
      next = clampZoom(zoomFactor + ZOOM_STEP);
    } else {
      next = clampZoom(zoomFactor - ZOOM_STEP);
    }
    zoomFactor = next;
    localStorage.setItem(ZOOM_KEY, String(next));
    applyZoom(next);
  }

  // Unlisten functions for native OS menu events.
  // Populated in onMount (Tauri env only), cleaned up in onDestroy.
  let menuUnlisteners: Array<() => void> = [];

  onDestroy(() => {
    for (const off of menuUnlisteners) {
      off();
    }
    menuUnlisteners = [];
  });

  onMount(async () => {
    const stored = localStorage.getItem('jura-dark-mode');
    // Treat absence or 'true' as dark (dark-first default)
    darkMode = stored === null ? true : stored === 'true';
    applyTheme(darkMode);

    // Restore persisted zoom level
    const storedZoom = localStorage.getItem(ZOOM_KEY);
    if (storedZoom !== null) {
      const parsed = parseFloat(storedZoom);
      if (!isNaN(parsed)) {
        zoomFactor = clampZoom(parsed);
        applyZoom(zoomFactor);
      }
    }

    currentPath = window.location.pathname;

    // Managed deployments can set skip_setup_wizard=true in config.json to
    // suppress the wizard for all users on that machine. When the flag is set
    // we also write the localStorage key so subsequent mounts are fast and do
    // not re-call the backend on every navigation.
    const skipWizard = await getSkipWizard();
    if (skipWizard) {
      // Mark as complete in localStorage so future loads skip the IPC call
      // and the wizard is never shown, even after the flag is read.
      localStorage.setItem('jura-setup-complete', 'managed');
    }

    // Version-gated wizard re-trigger: if the app version has changed since
    // setup was last completed (e.g. reinstall or upgrade), clear the setup
    // flag so the wizard runs again automatically.
    if (!skipWizard && localStorage.getItem('jura-setup-complete')) {
      try {
        const currentVersion = await getVersion();
        const setupVersion = localStorage.getItem('jura-setup-version');
        if (setupVersion && setupVersion !== currentVersion) {
          localStorage.removeItem('jura-setup-complete');
          localStorage.removeItem('jura-setup-version');
        }
      } catch {
        // getVersion failed (browser dev mode) — don't clear setup state
      }
    }

    // Show onboarding on first launch (no prior completion recorded)
    if (!localStorage.getItem('jura-onboarded')) {
      showOnboarding = true;
    } else if (!skipWizard && !localStorage.getItem('jura-setup-complete')) {
      // Already onboarded but setup not completed (e.g. app relaunched mid-setup)
      showSetupWizard = true;
    }

    // Sidecar offline reminder: if setup is complete but the sidecar is
    // not running, show a non-blocking banner so the user knows forensic
    // analysis is unavailable — unless they've permanently dismissed it.
    if (
      !showOnboarding &&
      !showSetupWizard &&
      localStorage.getItem('jura-setup-complete') &&
      localStorage.getItem('jura-sidecar-reminder-dismissed') !== 'true'
    ) {
      // Give the sidecar a moment to start (it auto-launches in production)
      setTimeout(async () => {
        try {
          const health = await checkSidecarHealth();
          if (!health || health.status !== 'ok') {
            showSidecarReminder = true;
          }
        } catch {
          showSidecarReminder = true;
        }
      }, 5000);
    }

    // ── Native OS menu event listeners ─────────────────────────────────────
    // Only wire up when running inside Tauri (browser dev mode has no menu).
    // Each listener returns an unlisten function stored in menuUnlisteners so
    // onDestroy can clean them up and prevent duplicate registrations on
    // hot-reload.
    const tauriAvailable = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
    if (tauriAvailable) {
      const { listen } = await import('@tauri-apps/api/event');

      // menu:navigate — route to any app tab
      menuUnlisteners.push(
        await listen<string>('menu:navigate', (event) => {
          goto(event.payload);
        }),
      );

      // menu:show-feedback — open the feedback panel
      menuUnlisteners.push(
        await listen('menu:show-feedback', () => {
          showFeedback = true;
        }),
      );

      // menu:check-updates — navigate to Settings then trigger the updater
      menuUnlisteners.push(
        await listen('menu:check-updates', async () => {
          await goto('/settings');
          // Dynamically import to keep the updater out of the initial bundle.
          try {
            const { checkForUpdate, makeTauriDeps } = await import('$lib/updater');
            const deps = await makeTauriDeps();
            // No-op status handler here — Settings page owns the status UI.
            // This call triggers the underlying check; Settings will render
            // the result on its next mount / via its own poll on re-mount.
            await checkForUpdate(() => {}, deps);
          } catch {
            // Non-fatal: Settings page will show its own update UI.
          }
        }),
      );

      // menu:search-help — navigate to help with search focus flag
      menuUnlisteners.push(
        await listen('menu:search-help', () => {
          goto('/help?search=1');
        }),
      );

      // menu:zoom — adjust webview zoom level
      menuUnlisteners.push(
        await listen<string>('menu:zoom', (event) => {
          const direction = event.payload as 'in' | 'out' | 'reset';
          handleZoom(direction);
        }),
      );
    }
  });

  function completeOnboarding() {
    localStorage.setItem('jura-onboarded', 'true');
    showOnboarding = false;
    // Show the setup wizard unless the user has already completed it
    if (!localStorage.getItem('jura-setup-complete')) {
      showSetupWizard = true;
    }
  }

  async function completeSetup() {
    localStorage.setItem('jura-setup-complete', 'true');
    try {
      const currentVersion = await getVersion();
      localStorage.setItem('jura-setup-version', currentVersion);
    } catch {
      // Browser dev mode — skip version tracking
    }
    showSetupWizard = false;
  }

  function handleSidecarSetUp() {
    showSidecarReminder = false;
    // Clear setup-complete so the wizard re-opens
    localStorage.removeItem('jura-setup-complete');
    showSetupWizard = true;
  }

  function handleSidecarNotNow() {
    showSidecarReminder = false;
  }

  function handleSidecarDontRemind() {
    localStorage.setItem('jura-sidecar-reminder-dismissed', 'true');
    showSidecarReminder = false;
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

  // Navigation items.
  //
  // Protect was hidden during the C2PA Validator evaluation
  // (2026-04-23) and Monitor was hidden during the pilot-tester
  // walkthrough (2026-04-26).  Both restored 2026-04-28 — the
  // evaluation phases are complete and pilots want the full
  // navigation surface.  Page contents themselves were never
  // changed; only the nav entries were suppressed.
  const navItems = [
    { href: '/',         label: 'Dashboard', title: 'Overview and statistics' },
    { href: '/protect',  label: 'Protect',   title: 'Sign content with Content Credentials' },
    { href: '/verify',   label: 'Verify',    title: 'Check content authenticity' },
    { href: '/monitor',  label: 'Monitor',   title: 'Activity log and trust landscape' },
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
        class="flex items-center gap-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded"
        aria-label="Jura Trace — home"
      >
        <LogoMark size={64} />
        <span class="brand-name text-[21px] text-text-light dark:text-text-dark">Jura Trace</span>
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
                     : 'text-flint-dark dark:text-flint-light hover:text-lapis dark:hover:text-[#8AABBF]'}"
            title={item.title}
            aria-current={currentPath === item.href ? 'page' : undefined}
          >
            {item.label}
          </a>
        {/each}

        <!-- Dark mode toggle -->
        <button
          onclick={toggleDarkMode}
          class="min-w-[44px] min-h-[44px] flex items-center justify-center text-flint-dark dark:text-flint-light hover:text-lapis dark:hover:text-lapis dark:text-lapis-light transition-colors text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded px-2"
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
          class="min-w-[44px] min-h-[44px] flex items-center justify-center text-flint-dark dark:text-flint-light hover:text-lapis dark:hover:text-lapis dark:text-lapis-light transition-colors text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded"
          aria-label={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}
          aria-pressed={darkMode}
        >
          {darkMode ? 'Light' : 'Dark'}
        </button>

        <!-- Hamburger button -->
        <button
          onclick={toggleMobileMenu}
          class="min-w-[44px] min-h-[44px] flex items-center justify-center text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-text-dark transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded"
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
                       : 'text-flint-dark dark:text-flint-light hover:text-lapis dark:hover:text-lapis dark:text-lapis-light'}"
              aria-current={currentPath === item.href ? 'page' : undefined}
            >
              {item.label}
            </a>
          {/each}
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

  <!-- Sidecar offline reminder banner -->
  {#if showSidecarReminder}
    <div
      class="sticky bottom-0 z-30 border-t border-amber/30 bg-amber/10 dark:bg-amber/5"
      role="status"
      aria-live="polite"
    >
      <div class="max-w-4xl mx-auto px-6 lg:px-8 py-4 flex flex-col sm:flex-row items-start sm:items-center gap-3">
        <div class="flex-1 min-w-0">
          <p class="text-sm font-medium text-text-light dark:text-quartz">
            Analysis Engine is offline
          </p>
          <p class="text-xs muted-help mt-0.5">
            Full forensic analysis (AI detection, noise analysis, copy-move detection) requires the Analysis Engine.
            Core features like C2PA signing and EXIF metadata still work without it.
          </p>
        </div>
        <div class="flex items-center gap-2 shrink-0">
          <button
            onclick={handleSidecarSetUp}
            class="px-4 py-2 min-h-[44px] rounded text-xs font-medium bg-lapis text-white hover:bg-lapis-dark transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          >
            Set up now
          </button>
          <button
            onclick={handleSidecarNotNow}
            class="px-4 py-2 min-h-[44px] rounded text-xs font-medium border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          >
            Not now
          </button>
          <button
            onclick={handleSidecarDontRemind}
            class="px-4 py-2 min-h-[44px] rounded text-xs text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          >
            Don't remind me
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Footer -->
  <footer class="border-t border-border-light dark:border-[rgba(122,119,112,0.15)] mt-auto">
    <div class="max-w-4xl mx-auto px-6 lg:px-8 py-8">
      <div class="flex flex-col sm:flex-row items-center justify-between gap-4 text-sm text-flint-dark dark:text-flint-light">
        <div class="flex items-center gap-3">
          <LogoMark size={24} />
          <span class="brand-name text-xs text-text-light dark:text-text-dark">Jura Trace</span>
          <span class="text-xs">v0.9.0</span>
        </div>
        <p class="text-xs text-center muted-help">Know What's Real</p>
        <div class="flex items-center gap-4 text-xs">
          <button
            onclick={() => showFeedback = true}
            class="text-xs text-flint-dark dark:text-flint-light hover:text-lapis dark:hover:text-lapis dark:text-lapis-light transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded"
          >
            Feedback
          </button>
          <a
            href="https://juralabs.org"
            target="_blank"
            rel="noopener noreferrer"
            class="underline underline-offset-2 hover:no-underline hover:text-lapis dark:hover:text-lapis dark:text-lapis-light transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >
            Jura Labs CIC
            <span class="sr-only">(opens in new tab)</span>
          </a>
          <a
            href="/help/open-source"
            class="underline underline-offset-2 hover:no-underline hover:text-lapis dark:hover:text-lapis dark:text-lapis-light transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >
            Licences
          </a>
          <a
            href="https://codeberg.org/jura-labs/jura-trace"
            target="_blank"
            rel="noopener noreferrer"
            class="underline underline-offset-2 hover:no-underline hover:text-lapis dark:hover:text-lapis dark:text-lapis-light transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >
            Source
            <span class="sr-only">(opens in new tab)</span>
          </a>
        </div>
      </div>
      <!-- AGPL-3.0 §5(d) Appropriate Legal Notices -->
      <p class="mt-4 text-[0.7rem] leading-relaxed text-center muted-help">
        Copyright &copy; 2025{new Date().getFullYear() > 2025 ? `–${new Date().getFullYear()}` : ''} Paul Griffiths, published by Jura Labs CIC.
        Free software under
        <a
          href="/help/open-source"
          class="underline underline-offset-2 hover:text-lapis dark:hover:text-lapis-light focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >AGPL-3.0-or-later</a
        >, distributed WITHOUT WARRANTY.
      </p>
    </div>
  </footer>
</div>

{#if showOnboarding}
  <OnboardingOverlay onComplete={completeOnboarding} />
{/if}

{#if showSetupWizard}
  <SetupWizard onComplete={completeSetup} />
{/if}

{#if showFeedback}
  <FeedbackPanel onClose={() => showFeedback = false} />
{/if}
