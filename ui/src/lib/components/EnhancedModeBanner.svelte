<script lang="ts">
  /**
   * Enhanced-mode first-run nudge.
   *
   * Shown on Dashboard and Verify when the user is still on the default
   * Standard (offline) NetworkMode. One-click upgrades to Enhanced; a
   * "Not now" button dismisses for this install via localStorage.
   * Auto-hides once NetworkMode === 'enhanced'.
   *
   * Rationale: the Standard default preserves the local-first USP, but
   * full C2PA validation (OCSP revocation checks, remote manifest fetch,
   * §15.9 audit items) needs network access. This banner surfaces the
   * choice contextually without forcing a wizard detour.
   */
  import { onMount } from 'svelte';
  import { getNetworkMode, setNetworkMode } from '$lib/api';
  import type { NetworkMode } from '$lib/types';

  const DISMISSED_KEY = 'jura-enhanced-banner-dismissed';

  let mode = $state<NetworkMode>('standard');
  let dismissed = $state(false);
  let busy = $state(false);
  let feedback = $state<{ ok: boolean; text: string } | null>(null);

  const visible = $derived(mode === 'standard' && !dismissed && feedback === null);

  onMount(async () => {
    try {
      mode = await getNetworkMode();
    } catch {
      mode = 'standard';
    }
    if (typeof window !== 'undefined') {
      dismissed = window.localStorage.getItem(DISMISSED_KEY) === 'true';
    }
  });

  async function enableEnhanced() {
    busy = true;
    try {
      const resolved = await setNetworkMode('enhanced');
      mode = resolved;
      feedback = { ok: true, text: 'Enhanced mode enabled. Online validation is now active.' };
      // Auto-hide the feedback after a moment so it doesn't linger.
      setTimeout(() => { feedback = null; }, 4000);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      feedback = { ok: false, text: `Could not enable Enhanced mode: ${msg}` };
    } finally {
      busy = false;
    }
  }

  function dismiss() {
    dismissed = true;
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(DISMISSED_KEY, 'true');
    }
  }
</script>

{#if visible}
  <div
    role="note"
    class="rounded-lg border border-lapis/30 bg-lapis/5 dark:bg-lapis/10 px-4 py-3 mb-5 flex items-start gap-3"
  >
    <svg
      class="flex-shrink-0 mt-0.5 text-lapis dark:text-lapis-light"
      width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor"
      stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"
    >
      <circle cx="12" cy="12" r="10"/>
      <path d="M12 16v-4M12 8h.01"/>
    </svg>

    <div class="flex-1 min-w-0">
      <p class="text-sm text-obsidian dark:text-quartz leading-relaxed">
        <strong class="font-semibold">Enable Enhanced mode for full Content Credentials validation?</strong>
      </p>
      <p class="text-xs text-flint dark:text-flint-light leading-relaxed mt-1 max-w-prose">
        Jura Trace runs fully offline by default. Enhanced mode adds online certificate
        revocation checks (OCSP/CRL) and remote Content Credentials retrieval — recommended
        for real-world content verification and standards-conformant validation.
      </p>

      <div class="flex flex-wrap items-center gap-2 mt-3">
        <button
          class="px-3 py-1.5 min-h-[32px] text-xs font-medium bg-lapis text-white rounded
                 hover:bg-lapis-dark transition-colors disabled:opacity-50
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          onclick={enableEnhanced}
          disabled={busy}
        >
          {busy ? 'Enabling…' : 'Enable Enhanced Mode'}
        </button>
        <button
          class="px-3 py-1.5 min-h-[32px] text-xs text-flint dark:text-flint-light
                 hover:text-obsidian dark:hover:text-quartz transition-colors underline underline-offset-2
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          onclick={dismiss}
        >
          Not now
        </button>
        <a
          href="/settings#network-access"
          class="px-1 py-1.5 min-h-[32px] text-xs text-lapis dark:text-lapis-light underline underline-offset-2
                 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
        >
          Learn more in Settings
        </a>
      </div>
    </div>
  </div>
{:else if feedback}
  <div
    role="status"
    aria-live="polite"
    class="rounded-lg border px-4 py-3 mb-5 text-sm leading-relaxed
           {feedback.ok
             ? 'border-malachite/30 bg-malachite/5 text-malachite dark:text-malachite-light'
             : 'border-cinnabar/30 bg-cinnabar/5 text-cinnabar dark:text-cinnabar-light'}"
  >
    {feedback.text}
  </div>
{/if}
