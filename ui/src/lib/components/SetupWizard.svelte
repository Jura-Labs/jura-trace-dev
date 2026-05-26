<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { checkSidecarHealth, getSidecarStartupStatus, onSidecarStatusChanged } from '$lib/api';
  import type { SidecarHealth, SidecarStartupStatus } from '$lib/types';

  // ── Props ──────────────────────────────────────────────────────────
  interface Props {
    onComplete: () => void;
  }

  const { onComplete }: Props = $props();

  // ── Constants ──────────────────────────────────────────────────────
  // JTV-138 (2026-05-02): wizard reduced from 5 steps to 2 after the
  // video/audio scope drop. Speech transcription, FFmpeg install, and
  // Local AI (Ollama) prompts have been moved out of first-launch:
  //   • Video/Audio + Speech Transcription — out of v1.0 entirely
  //     (re-add gated under JTV-139 with calibration matrix)
  //   • Local AI / Ollama — moved to Settings under JTV-132 so the
  //     install flow can be deferred to a moment of intent rather than
  //     blocking first launch (Aisha-bandwidth + Tom-RAM personas).
  const TOTAL_STEPS = 2;
  const HEALTH_TIMEOUT_MS = 3000;

  // ── State ──────────────────────────────────────────────────────────
  let currentStep = $state(0);
  let health = $state<SidecarHealth | null>(null);
  let healthChecking = $state(true);
  // Live sidecar startup status and elapsed time (JTV-184 snapshot API).
  let startupStatus = $state<SidecarStartupStatus>('connecting');
  let startupElapsed = $state(0);
  let _unlistenStartup: (() => void) | null = null;
  let _elapsedTick: ReturnType<typeof setInterval> | null = null;

  // Reference to the dialog element for focus trap
  let dialogEl: HTMLElement | null = $state(null);
  // Primary action button on each step — receives focus on step change
  let primaryActionEl: HTMLElement | null = $state(null);

  // ── Derived flags ──────────────────────────────────────────────────
  const sidecarOnline = $derived(health !== null);
  const isFirstStep = $derived(currentStep === 0);
  const isLastStep = $derived(currentStep === TOTAL_STEPS - 1);

  // ── Navigation ─────────────────────────────────────────────────────
  async function goNext() {
    if (isLastStep) {
      onComplete();
      return;
    }
    currentStep = Math.min(currentStep + 1, TOTAL_STEPS - 1);
    await focusPrimaryAction();
  }

  async function goBack() {
    if (isFirstStep) return;
    currentStep = Math.max(currentStep - 1, 0);
    await focusPrimaryAction();
  }

  async function goToStep(index: number) {
    currentStep = index;
    await focusPrimaryAction();
  }

  async function focusPrimaryAction() {
    await tick();
    primaryActionEl?.focus();
  }

  // ── Keyboard: wizard navigation + focus trap ───────────────────────
  function handleKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case 'Escape':
        event.preventDefault();
        onComplete();
        break;
      case 'ArrowRight':
        // Only advance if focus is NOT on a button (avoids double-firing)
        if (!(event.target instanceof HTMLButtonElement)) {
          event.preventDefault();
          goNext();
        }
        break;
      case 'ArrowLeft':
        if (!(event.target instanceof HTMLButtonElement)) {
          event.preventDefault();
          goBack();
        }
        break;
    }
  }

  function handleFocusTrap(event: KeyboardEvent) {
    if (event.key !== 'Tab' || !dialogEl) return;

    const focusableSelectors =
      'a[href], button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])';
    const focusable = Array.from(
      dialogEl.querySelectorAll<HTMLElement>(focusableSelectors)
    ).filter((el) => el.offsetParent !== null);

    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (event.shiftKey) {
      if (document.activeElement === first) {
        event.preventDefault();
        last.focus();
      }
    } else {
      if (document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
  }

  // ── Health check ───────────────────────────────────────────────────
  async function refreshHealth() {
    healthChecking = true;
    try {
      const result = await Promise.race([
        checkSidecarHealth(),
        new Promise<null>((resolve) => setTimeout(() => resolve(null), HEALTH_TIMEOUT_MS)),
      ]);
      health = result;
    } catch {
      health = null;
    } finally {
      healthChecking = false;
    }
  }

  // ── Lifecycle ──────────────────────────────────────────────────────
  let previouslyFocused: HTMLElement | null = null;

  onMount(async () => {
    previouslyFocused = document.activeElement as HTMLElement | null;
    focusPrimaryAction();
    // Fetch initial sidecar startup snapshot and subscribe to transitions.
    const snapshot = await getSidecarStartupStatus();
    startupStatus = snapshot.status;
    startupElapsed = snapshot.elapsedSecs;
    _unlistenStartup = await onSidecarStatusChanged((status) => {
      startupStatus = status;
    });
    // Increment the elapsed counter locally every second while connecting.
    // Cleanup is in onDestroy to handle premature component removal.
    _elapsedTick = setInterval(() => {
      if (startupStatus === 'connecting') {
        startupElapsed += 1;
      }
    }, 1000);
    await refreshHealth();
  });

  // Auto-advance step 0 two seconds after the health check resolves and the
  // sidecar is confirmed online. The $effect re-runs whenever healthChecking
  // or sidecarOnline changes and cleans up the timer if the user navigates away.
  $effect(() => {
    if (!healthChecking && sidecarOnline && currentStep === 0) {
      const timer = setTimeout(() => {
        if (currentStep === 0) goNext();
      }, 2000);
      return () => clearTimeout(timer);
    }
  });

  onDestroy(() => {
    _unlistenStartup?.();
    if (_elapsedTick !== null) clearInterval(_elapsedTick);
    previouslyFocused?.focus();
  });

  // ── Helpers ────────────────────────────────────────────────────────
  // Step labels used by the step-indicator nav
  const stepLabels = [
    'Analysis Engine',
    'Ready',
  ];
</script>

<!--
  SetupWizard — first-launch service setup guide.

  WCAG 2.2 AA:
  - role="dialog" aria-modal="true" boundary for screen readers
  - Focus trapped within dialog; restored on close
  - Focus moves to primary action on each step change
  - Step indicators carry aria-label and aria-current="step"
  - All text meets 4.5:1 contrast on graphite/obsidian backgrounds
  - 44px minimum touch targets on all interactive elements
  - Amber/cinnabar/malachite states communicated with text, not colour alone
  - Keyboard: Escape = dismiss, Arrow right/left = navigate (when focus not on button)
-->

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center backdrop-blur-sm"
  style="background: rgba(30,33,40,0.95);"
  role="dialog"
  aria-modal="true"
  aria-label="Jura Trace setup: confirm your services"
  aria-describedby="wizard-step-description"
  tabindex="-1"
  bind:this={dialogEl}
  onkeydown={(e) => { handleKeydown(e); handleFocusTrap(e); }}
>
  <!-- Card — wider than onboarding to accommodate status rows -->
  <div
    class="relative w-full max-w-xl mx-4 rounded-xl overflow-hidden"
    style="background: #272B34; border: 1px solid rgba(122,119,112,0.2);"
  >

    <!-- ── Step indicator bar ──────────────────────────────────────── -->
    <div class="px-8 pt-7 pb-0">
      <nav aria-label="Setup progress">
        <ol class="flex items-center gap-1.5" role="list">
          {#each stepLabels as label, index}
            <li class="flex items-center gap-1.5">
              <button
                onclick={() => goToStep(index)}
                class="flex items-center justify-center w-6 h-6 rounded-full transition-all duration-200
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                       focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
                aria-label="Go to step {index + 1}: {label}"
                aria-current={index === currentStep ? 'step' : undefined}
              >
                <span
                  class="block rounded-full transition-all duration-200
                         {index === currentStep
                           ? 'w-5 h-2 bg-lapis'
                           : index < currentStep
                             ? 'w-2 h-2 bg-lapis/40'
                             : 'w-2 h-2 bg-graphite-light'}"
                  aria-hidden="true"
                ></span>
              </button>
              {#if index < stepLabels.length - 1}
                <span class="w-4 h-px bg-graphite-light/50" aria-hidden="true"></span>
              {/if}
            </li>
          {/each}
        </ol>
      </nav>

      <!-- Step counter for screen readers -->
      <p class="sr-only" id="wizard-step-description">
        Step {currentStep + 1} of {TOTAL_STEPS}: {stepLabels[currentStep]}
      </p>
    </div>

    <!-- ── Step content area ──────────────────────────────────────── -->
    <div class="min-h-[300px] flex flex-col">

      <!-- ── Step 0: Analysis Engine ───────────────────────────────── -->
      {#if currentStep === 0}
        <div
          class="flex-1 flex flex-col px-8 pt-6 pb-6 motion-safe:animate-[fadeIn_200ms_ease-out]"
          role="group"
          aria-labelledby="step0-heading"
        >
          <p class="text-xs font-medium text-lapis dark:text-lapis-light uppercase tracking-widest mb-3">
            Step 1 of {TOTAL_STEPS}
          </p>

          <h2 id="step0-heading" class="font-heading text-xl font-semibold text-quartz leading-tight mb-5" style="letter-spacing: -0.01em;">
            Analysis Engine
          </h2>

          {#if healthChecking}
            <!-- Checking state -->
            <div class="flex items-center gap-3 rounded-lg px-4 py-3.5" style="background: rgba(30,33,40,0.6); border: 1px solid rgba(122,119,112,0.15);">
              <!-- Animated pulse dot -->
              <span
                class="flex-shrink-0 w-2.5 h-2.5 rounded-full bg-amber motion-safe:animate-pulse"
                aria-hidden="true"
              ></span>
              <p class="text-sm text-quartz">Checking analysis engine…</p>
            </div>

          {:else if sidecarOnline}
            <!-- Online state -->
            <div
              class="flex items-center gap-3 rounded-lg px-4 py-3.5"
              style="background: rgba(91,138,95,0.1); border: 1px solid rgba(91,138,95,0.25);"
              role="status"
              aria-live="polite"
            >
              <!-- Checkmark icon -->
              <svg class="flex-shrink-0 w-5 h-5 text-malachite-light" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 10l4 4 8-8" />
              </svg>
              <div>
                <p class="text-sm font-medium text-malachite-light">Analysis engine ready</p>
                <p class="text-xs text-flint-light mt-0.5">Full forensic analysis is available. Advancing in a moment…</p>
              </div>
            </div>

          {:else}
            <!-- Offline/connecting state -->
            <div
              class="flex items-start gap-3 rounded-lg px-4 py-3.5"
              style="background: rgba(212,148,58,0.08); border: 1px solid rgba(212,148,58,0.25);"
              role="status"
              aria-live="polite"
            >
              <!-- Warning icon -->
              <svg class="flex-shrink-0 w-5 h-5 text-amber-light mt-0.5" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M10 3L2 16h16L10 3z" />
                <path stroke="currentColor" stroke-linecap="round" stroke-width="1.75" d="M10 9v4" />
                <circle cx="10" cy="15" r="0.5" fill="currentColor" stroke="none" />
              </svg>
              <div>
                <p class="text-sm font-medium text-amber-light">
                  Starting…{startupStatus === 'connecting' && startupElapsed > 0 ? ` (${startupElapsed}s elapsed)` : ''}
                </p>
                <p class="text-xs text-flint-light mt-1 leading-relaxed">
                  First launch can take up to about 90 seconds while the analysis
                  engine unpacks. This is normal.
                </p>
                <p class="text-xs text-malachite-light mt-1.5 leading-relaxed">
                  Protect and EXIF metadata checks work now while the engine
                  finishes loading.
                </p>
                <button
                  onclick={refreshHealth}
                  disabled={healthChecking}
                  class="mt-2 text-xs px-2.5 py-1 rounded border border-lapis/40 text-lapis dark:text-lapis-light hover:bg-lapis/10 transition-colors
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-graphite
                         disabled:opacity-50"
                >
                  {healthChecking ? 'Checking…' : 'Re-check'}
                </button>
              </div>
            </div>
          {/if}
        </div>

      <!-- ── Step 1: Ready ─────────────────────────────────────────── -->
      {:else if currentStep === 1}
        <div
          class="flex-1 flex flex-col px-8 pt-6 pb-6 motion-safe:animate-[fadeIn_200ms_ease-out]"
          role="group"
          aria-labelledby="step1-heading"
        >
          <p class="text-xs font-medium text-malachite-dark dark:text-malachite-light uppercase tracking-widest mb-3">
            All done
          </p>

          <h2 id="step1-heading" class="font-heading text-xl font-semibold text-quartz leading-tight mb-5" style="letter-spacing: -0.01em;">
            Ready to use Jura Trace
          </h2>

          <!-- Summary list -->
          <ul class="space-y-2.5 mb-6" aria-label="Service availability summary">

            <!-- Core verification — always available -->
            <li class="flex items-center gap-3">
              <svg class="flex-shrink-0 w-4 h-4 text-malachite-light" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l3 3 7-7" />
              </svg>
              <span class="text-sm text-quartz">Core verification: C2PA, EXIF, perceptual hash</span>
            </li>

            <!-- Forensic analysis -->
            <li class="flex items-center gap-3">
              {#if sidecarOnline}
                <svg class="flex-shrink-0 w-4 h-4 text-malachite-light" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l3 3 7-7" />
                </svg>
              {:else}
                <svg class="flex-shrink-0 w-4 h-4 text-flint-light" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4l8 8M12 4l-8 8" />
                </svg>
              {/if}
              <span class="text-sm {sidecarOnline ? 'text-quartz' : 'text-flint-light'}">
                Forensic analysis {sidecarOnline ? ': available' : ': not available (start the Analysis Engine)'}
              </span>
            </li>

          </ul>

          <!-- Optional Ollama hint — points to Settings rather than prompting an
               install during first launch. JTV-132 will land the in-app
               install flow. -->
          <p class="text-xs text-flint-light leading-relaxed mb-6">
            Optional: install Ollama from <strong class="text-quartz">Settings</strong>
            to enable AI image descriptions and claim verification. Core verification
            and forensic analysis run without it.
          </p>

          <!-- Primary CTA -->
          <button
            bind:this={primaryActionEl}
            onclick={onComplete}
            class="w-full min-h-[44px] py-3 px-6 bg-lapis hover:bg-lapis-dark text-white text-sm font-medium
                   rounded-lg transition-colors duration-150 mt-auto
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                   focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
          >
            All set. Start using Jura Trace
          </button>
        </div>
      {/if}

    </div>

    <!-- ── Footer: navigation controls ───────────────────────────── -->
    <div
      class="flex items-center justify-between px-8 py-5 border-t"
      style="border-color: rgba(122,119,112,0.2);"
    >

      <!-- Back button -->
      <div class="w-16">
        {#if !isFirstStep && !isLastStep}
          <button
            onclick={goBack}
            class="min-h-[44px] px-2 text-xs text-flint-light hover:text-quartz transition-colors duration-150
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                   focus-visible:ring-offset-2 focus-visible:ring-offset-graphite rounded"
            aria-label="Go back to the previous step"
          >
            Back
          </button>
        {/if}
      </div>

      <!-- Step label (centre) -->
      <p class="text-xs text-flint-light" aria-hidden="true">
        {stepLabels[currentStep]}
      </p>

      <!-- Right-side controls: Skip / Continue -->
      <div class="flex items-center gap-3 w-auto">

        {#if isLastStep}
          <!-- Last step: primary CTA is inside the content area — no footer button needed -->
          <div></div>

        {:else}
          <!-- Continue button in footer -->
          <button
            bind:this={primaryActionEl}
            onclick={goNext}
            class="min-h-[44px] py-1.5 px-4 bg-lapis hover:bg-lapis-dark text-white text-xs font-medium
                   rounded-md transition-colors duration-150
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                   focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
          >
            Continue
          </button>
        {/if}

      </div>
    </div>

  </div>
</div>

<style>
  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(4px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  @media (prefers-reduced-motion: reduce) {
    @keyframes fadeIn {
      from { opacity: 1; transform: none; }
      to   { opacity: 1; transform: none; }
    }
  }
</style>
