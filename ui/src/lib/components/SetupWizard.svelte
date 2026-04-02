<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { checkSidecarHealth } from '$lib/api';
  import type { SidecarHealth } from '$lib/types';

  // ── Props ──────────────────────────────────────────────────────────
  interface Props {
    onComplete: () => void;
  }

  const { onComplete }: Props = $props();

  // ── Constants ──────────────────────────────────────────────────────
  const TOTAL_STEPS = 5;
  const HEALTH_TIMEOUT_MS = 3000;

  // ── State ──────────────────────────────────────────────────────────
  let currentStep = $state(0);
  let health = $state<SidecarHealth | null>(null);
  let healthChecking = $state(true);

  // Reference to the dialog element for focus trap
  let dialogEl: HTMLElement | null = $state(null);
  // Primary action button on each step — receives focus on step change
  let primaryActionEl: HTMLElement | null = $state(null);

  // ── Model pull state ──────────────────────────────────────────────
  let pullingModel = $state<string | null>(null);
  let pullError = $state<string | null>(null);

  // ── Derived capability flags ───────────────────────────────────────
  const sidecarOnline = $derived(health !== null);
  const ffmpegAvailable = $derived(health?.capabilities.videoMetadata === true);
  const transcriptionAvailable = $derived(health?.capabilities.transcription === true);
  const ollamaAvailable = $derived(health?.ollama !== null && health?.ollama !== undefined);

  // ── Derived Ollama model flags ─────────────────────────────────────
  const ollamaModels = $derived(health?.ollamaModels ?? []);
  const llavaInstalled = $derived(ollamaModels.some((m) => m.startsWith('llava')));
  const qwenInstalled = $derived(ollamaModels.some((m) => m.startsWith('qwen2.5')));
  const allModelsReady = $derived(llavaInstalled && qwenInstalled);

  const isFirstStep = $derived(currentStep === 0);
  const isLastStep = $derived(currentStep === TOTAL_STEPS - 1);

  // ── Platform detection ─────────────────────────────────────────────
  // navigator.platform is deprecated but still functional for this purpose;
  // we only need a coarse OS hint for the FFmpeg install hint text.
  function detectPlatform(): 'mac' | 'windows' | 'linux' {
    if (typeof navigator === 'undefined') return 'linux';
    const p = navigator.userAgent.toLowerCase();
    if (p.includes('win')) return 'windows';
    if (p.includes('mac')) return 'mac';
    return 'linux';
  }

  const platform = detectPlatform();

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

  // ── External link ──────────────────────────────────────────────────
  async function openOllamaDownload() {
    const url = 'https://ollama.com';
    try {
      // Use Tauri shell plugin if available, otherwise let the browser handle it
      if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
        const { open } = await import('@tauri-apps/plugin-shell');
        await open(url);
      } else {
        window.open(url, '_blank', 'noopener,noreferrer');
      }
    } catch {
      // Fallback: open in the current webview as a last resort
      window.open(url, '_blank', 'noopener,noreferrer');
    }
  }

  // ── Ollama URL resolution ──────────────────────────────────────────
  // Read the user-configured Ollama URL from Settings (localStorage),
  // falling back to the default localhost address.
  function getOllamaUrl(): string {
    if (typeof localStorage !== 'undefined') {
      const saved = localStorage.getItem('jura-ollama-url');
      if (saved) return saved.replace(/\/+$/, '');
    }
    return 'http://127.0.0.1:11434';
  }

  // ── Ollama model pull ──────────────────────────────────────────────
  async function pullOllamaModel(modelName: string) {
    pullingModel = modelName;
    pullError = null;
    try {
      const ollamaUrl = getOllamaUrl();
      const resp = await fetch(`${ollamaUrl}/api/pull`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: modelName, stream: false }),
      });
      if (!resp.ok) throw new Error(`Pull failed: ${resp.status}`);
      await refreshHealth();
    } catch (e) {
      pullError = `Failed to download ${modelName}. Please check Ollama is running and try again.`;
    } finally {
      pullingModel = null;
    }
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
    previouslyFocused?.focus();
  });

  // ── Helpers ────────────────────────────────────────────────────────
  // Step labels used by the step-indicator nav
  const stepLabels = [
    'Analysis Engine',
    'Video and Audio',
    'Speech Transcription',
    'Local AI',
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
  aria-label="Jura Trace setup — configure your services"
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
          <p class="text-xs font-medium text-lapis-light uppercase tracking-widest mb-3">
            Step 1 of 5
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
            <!-- Offline state -->
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
                <p class="text-sm font-medium text-amber-light">Analysis engine starting</p>
                <p class="text-xs text-flint-light mt-1 leading-relaxed">
                  The analysis engine may take a moment to start on first launch. Core features
                  (C2PA signing, EXIF metadata) work without it. If it remains offline, check
                  Settings for details.
                </p>
                <button
                  onclick={refreshHealth}
                  disabled={healthChecking}
                  class="mt-2 text-xs px-2.5 py-1 rounded border border-lapis/40 text-lapis-light hover:bg-lapis/10 transition-colors
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-graphite
                         disabled:opacity-50"
                >
                  {healthChecking ? 'Checking…' : 'Re-check'}
                </button>
              </div>
            </div>
          {/if}
        </div>

      <!-- ── Step 1: Video and Audio ───────────────────────────────── -->
      {:else if currentStep === 1}
        <div
          class="flex-1 flex flex-col px-8 pt-6 pb-6 motion-safe:animate-[fadeIn_200ms_ease-out]"
          role="group"
          aria-labelledby="step1-heading"
        >
          <p class="text-xs font-medium text-lapis-light uppercase tracking-widest mb-3">
            Step 2 of 5
          </p>

          <h2 id="step1-heading" class="font-heading text-xl font-semibold text-quartz leading-tight mb-5" style="letter-spacing: -0.01em;">
            Video and Audio
          </h2>

          {#if !sidecarOnline}
            <!-- Sidecar offline — can't assess FFmpeg -->
            <div
              class="flex items-start gap-3 rounded-lg px-4 py-3.5"
              style="background: rgba(30,33,40,0.6); border: 1px solid rgba(122,119,112,0.15);"
            >
              <svg class="flex-shrink-0 w-5 h-5 text-flint-light mt-0.5" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                <circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="1.75" />
                <path stroke="currentColor" stroke-linecap="round" stroke-width="1.75" d="M10 7v4" />
                <circle cx="10" cy="14" r="0.5" fill="currentColor" stroke="none" />
              </svg>
              <p class="text-sm text-flint-light">
                Video and audio status requires the analysis engine to be running.
              </p>
            </div>

          {:else if ffmpegAvailable}
            <!-- FFmpeg available -->
            <div
              class="flex items-center gap-3 rounded-lg px-4 py-3.5"
              style="background: rgba(91,138,95,0.1); border: 1px solid rgba(91,138,95,0.25);"
              role="status"
              aria-live="polite"
            >
              <svg class="flex-shrink-0 w-5 h-5 text-malachite-light" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 10l4 4 8-8" />
              </svg>
              <div>
                <p class="text-sm font-medium text-malachite-light">Video and audio analysis ready</p>
                <p class="text-xs text-flint-light mt-0.5">FFmpeg is available. Video metadata, frame extraction, and audio analysis are enabled.</p>
              </div>
            </div>

          {:else}
            <!-- FFmpeg missing -->
            <div
              class="flex items-start gap-3 rounded-lg px-4 py-3.5"
              style="background: rgba(212,148,58,0.08); border: 1px solid rgba(212,148,58,0.25);"
              role="status"
              aria-live="polite"
            >
              <svg class="flex-shrink-0 w-5 h-5 text-amber-light mt-0.5" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M10 3L2 16h16L10 3z" />
                <path stroke="currentColor" stroke-linecap="round" stroke-width="1.75" d="M10 9v4" />
                <circle cx="10" cy="15" r="0.5" fill="currentColor" stroke="none" />
              </svg>
              <div>
                <p class="text-sm font-medium text-amber-light">FFmpeg not installed</p>
                <p class="text-xs text-flint-light mt-1 mb-2 leading-relaxed">
                  Video and audio analysis will be unavailable. You can install it now or later.
                </p>
                <div class="flex items-center gap-2 mt-2">
                  <code
                    class="inline-block text-xs font-mono px-2.5 py-1 rounded"
                    style="background: rgba(30,33,40,0.8); color: #EDEAE4; border: 1px solid rgba(122,119,112,0.2);"
                  >
                    {#if platform === 'mac'}
                      brew install ffmpeg
                    {:else if platform === 'windows'}
                      See Settings for a download link
                    {:else}
                      sudo apt install ffmpeg
                    {/if}
                  </code>
                  <button
                    onclick={refreshHealth}
                    disabled={healthChecking}
                    class="text-xs px-2.5 py-1 rounded border border-lapis/40 text-lapis-light hover:bg-lapis/10 transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-graphite
                           disabled:opacity-50"
                  >
                    {healthChecking ? 'Checking…' : 'Re-check'}
                  </button>
                </div>
              </div>
            </div>
          {/if}
        </div>

      <!-- ── Step 2: Speech Transcription ──────────────────────────── -->
      {:else if currentStep === 2}
        <div
          class="flex-1 flex flex-col px-8 pt-6 pb-6 motion-safe:animate-[fadeIn_200ms_ease-out]"
          role="group"
          aria-labelledby="step2-heading"
        >
          <p class="text-xs font-medium text-lapis-light uppercase tracking-widest mb-3">
            Step 3 of 5
          </p>

          <h2 id="step2-heading" class="font-heading text-xl font-semibold text-quartz leading-tight mb-5" style="letter-spacing: -0.01em;">
            Speech Transcription
          </h2>

          {#if !sidecarOnline}
            <div
              class="flex items-start gap-3 rounded-lg px-4 py-3.5"
              style="background: rgba(30,33,40,0.6); border: 1px solid rgba(122,119,112,0.15);"
            >
              <svg class="flex-shrink-0 w-5 h-5 text-flint-light mt-0.5" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                <circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="1.75" />
                <path stroke="currentColor" stroke-linecap="round" stroke-width="1.75" d="M10 7v4" />
                <circle cx="10" cy="14" r="0.5" fill="currentColor" stroke="none" />
              </svg>
              <p class="text-sm text-flint-light">
                Transcription status requires the analysis engine to be running.
              </p>
            </div>

          {:else if transcriptionAvailable}
            <!-- Transcription ready -->
            <div
              class="flex items-center gap-3 rounded-lg px-4 py-3.5"
              style="background: rgba(91,138,95,0.1); border: 1px solid rgba(91,138,95,0.25);"
              role="status"
              aria-live="polite"
            >
              <svg class="flex-shrink-0 w-5 h-5 text-malachite-light" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 10l4 4 8-8" />
              </svg>
              <div>
                <p class="text-sm font-medium text-malachite-light">Speech transcription ready</p>
                <p class="text-xs text-flint-light mt-0.5">faster-whisper is installed and the model is available.</p>
              </div>
            </div>

          {:else}
            <!-- faster-whisper present but model needs download — or not installed -->
            <div
              class="flex items-start gap-3 rounded-lg px-4 py-3.5"
              style="background: rgba(30,33,40,0.6); border: 1px solid rgba(122,119,112,0.15);"
              role="status"
              aria-live="polite"
            >
              <svg class="flex-shrink-0 w-5 h-5 text-lapis-light mt-0.5" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                <circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="1.75" />
                <path stroke="currentColor" stroke-linecap="round" stroke-width="1.75" d="M10 7v4" />
                <circle cx="10" cy="14" r="0.5" fill="currentColor" stroke="none" />
              </svg>
              <div>
                <p class="text-sm font-medium text-quartz">Speech transcription model not loaded</p>
                <p class="text-xs text-flint-light mt-1 leading-relaxed">
                  The Whisper model (approximately 150 MB) downloads automatically on first use.
                  No action is needed now.
                </p>
              </div>
            </div>
          {/if}
        </div>

      <!-- ── Step 3: Local AI (Ollama + models) ────────────────────── -->
      {:else if currentStep === 3}
        <div
          class="flex-1 flex flex-col px-8 pt-6 pb-6 motion-safe:animate-[fadeIn_200ms_ease-out]"
          role="group"
          aria-labelledby="step3-heading"
        >
          <p class="text-xs font-medium text-lapis-light uppercase tracking-widest mb-3">
            Step 4 of 5
          </p>

          <h2 id="step3-heading" class="font-heading text-xl font-semibold text-quartz leading-tight mb-5" style="letter-spacing: -0.01em;">
            Local AI
          </h2>

          {#if !ollamaAvailable}
            <!-- ── Ollama not running ─────────────────────────────── -->
            <div
              class="flex items-start gap-3 rounded-lg px-4 py-3.5 mb-4"
              style="background: rgba(30,33,40,0.6); border: 1px solid rgba(122,119,112,0.15);"
              role="status"
              aria-live="polite"
            >
              <svg class="flex-shrink-0 w-5 h-5 text-flint-light mt-0.5" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                <circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="1.75" />
                <path stroke="currentColor" stroke-linecap="round" stroke-width="1.75" d="M10 7v4" />
                <circle cx="10" cy="14" r="0.5" fill="currentColor" stroke="none" />
              </svg>
              <div>
                <p class="text-sm font-medium text-quartz">Ollama is not running</p>
                <p class="text-xs text-flint-light mt-1 leading-relaxed">
                  Ollama runs AI models locally on your machine — no data leaves your device.
                  It powers image descriptions and claim verification.
                  Jura Trace works fully without it.
                </p>
              </div>
            </div>

            <div class="flex items-center gap-3 flex-wrap">
              <button
                onclick={openOllamaDownload}
                class="min-h-[44px] flex items-center gap-2 px-4 py-2.5 rounded-lg text-sm font-medium
                       bg-lapis hover:bg-lapis-dark text-white transition-colors duration-150
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                       focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
                aria-label="Download Ollama — opens ollama.com in your browser"
              >
                <svg class="w-4 h-4" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M6 3H3a1 1 0 00-1 1v9a1 1 0 001 1h9a1 1 0 001-1v-3" />
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 3h4v4" />
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 9L13 3" />
                </svg>
                Download Ollama
              </button>

              <button
                onclick={refreshHealth}
                disabled={healthChecking}
                class="min-h-[44px] px-4 py-2.5 rounded-lg text-sm font-medium
                       border border-lapis/40 text-lapis-light hover:bg-lapis/10 transition-colors duration-150
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                       focus-visible:ring-offset-2 focus-visible:ring-offset-graphite
                       disabled:opacity-50"
              >
                {healthChecking ? 'Checking…' : "I've installed it — re-check"}
              </button>

              <button
                bind:this={primaryActionEl}
                onclick={goNext}
                class="min-h-[44px] px-4 py-2.5 rounded-lg text-sm text-flint-light hover:text-quartz
                       transition-colors duration-150
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                       focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
              >
                Skip — I don't need this
              </button>
            </div>

          {:else}
            <!-- ── Ollama running — show model checklist ───────────── -->

            <!-- Privacy notice -->
            <div
              class="flex items-start gap-2 rounded-md px-3 py-2.5 mb-4"
              style="background: rgba(55,99,153,0.1); border: 1px solid rgba(55,99,153,0.25);"
            >
              <svg class="flex-shrink-0 w-4 h-4 text-lapis-light mt-0.5" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5" />
                <path stroke="currentColor" stroke-linecap="round" stroke-width="1.5" d="M8 6v4" />
                <circle cx="8" cy="5" r="0.5" fill="currentColor" stroke="none" />
              </svg>
              <p class="text-xs text-lapis-light leading-relaxed">
                These models run entirely on your machine. No data is sent to external servers.
              </p>
            </div>

            <!-- Model checklist -->
            <ul class="space-y-3 mb-4" aria-label="Required AI models">

              <!-- llava:7b -->
              <li
                class="flex items-center gap-3 rounded-lg px-4 py-3"
                style="{llavaInstalled
                  ? 'background: rgba(91,138,95,0.08); border: 1px solid rgba(91,138,95,0.2);'
                  : 'background: rgba(30,33,40,0.6); border: 1px solid rgba(122,119,112,0.18);'}"
              >
                <!-- Status icon -->
                {#if llavaInstalled}
                  <svg class="flex-shrink-0 w-5 h-5 text-malachite-light" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                    <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 10l4 4 8-8" />
                  </svg>
                {:else if pullingModel === 'llava:7b'}
                  <!-- Spinner while downloading -->
                  <svg
                    class="flex-shrink-0 w-5 h-5 text-lapis-light motion-safe:animate-spin"
                    fill="none" viewBox="0 0 20 20" aria-hidden="true"
                  >
                    <circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="2" stroke-dasharray="22 22" />
                  </svg>
                {:else}
                  <svg class="flex-shrink-0 w-5 h-5 text-amber-light" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                    <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M10 3L2 16h16L10 3z" />
                    <path stroke="currentColor" stroke-linecap="round" stroke-width="1.75" d="M10 9v4" />
                    <circle cx="10" cy="15" r="0.5" fill="currentColor" stroke="none" />
                  </svg>
                {/if}

                <!-- Model info -->
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-medium {llavaInstalled ? 'text-malachite-light' : 'text-quartz'}">
                    <code class="font-mono">llava:7b</code>
                    <span class="ml-1.5 text-xs font-normal text-flint-light">~4.7 GB</span>
                  </p>
                  <p class="text-xs text-flint-light mt-0.5">
                    {#if llavaInstalled}
                      Installed — image descriptions enabled
                    {:else if pullingModel === 'llava:7b'}
                      Downloading… this may take 5–10 minutes
                    {:else}
                      Not installed — required for image descriptions
                    {/if}
                  </p>
                </div>

                <!-- Download button (only when not installed and not already pulling this model) -->
                {#if !llavaInstalled && pullingModel !== 'llava:7b'}
                  <button
                    onclick={() => pullOllamaModel('llava:7b')}
                    disabled={pullingModel !== null}
                    class="flex-shrink-0 min-h-[44px] px-3 py-2 rounded-md text-xs font-medium
                           bg-lapis hover:bg-lapis-dark text-white transition-colors duration-150
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                           focus-visible:ring-offset-2 focus-visible:ring-offset-graphite
                           disabled:opacity-40 disabled:cursor-not-allowed"
                    aria-label="Download llava:7b (approximately 4.7 gigabytes)"
                  >
                    Download
                  </button>
                {/if}
              </li>

              <!-- qwen2.5:7b-instruct -->
              <li
                class="flex items-center gap-3 rounded-lg px-4 py-3"
                style="{qwenInstalled
                  ? 'background: rgba(91,138,95,0.08); border: 1px solid rgba(91,138,95,0.2);'
                  : 'background: rgba(30,33,40,0.6); border: 1px solid rgba(122,119,112,0.18);'}"
              >
                <!-- Status icon -->
                {#if qwenInstalled}
                  <svg class="flex-shrink-0 w-5 h-5 text-malachite-light" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                    <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 10l4 4 8-8" />
                  </svg>
                {:else if pullingModel === 'qwen2.5:7b-instruct'}
                  <svg
                    class="flex-shrink-0 w-5 h-5 text-lapis-light motion-safe:animate-spin"
                    fill="none" viewBox="0 0 20 20" aria-hidden="true"
                  >
                    <circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="2" stroke-dasharray="22 22" />
                  </svg>
                {:else}
                  <svg class="flex-shrink-0 w-5 h-5 text-amber-light" fill="none" viewBox="0 0 20 20" aria-hidden="true">
                    <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M10 3L2 16h16L10 3z" />
                    <path stroke="currentColor" stroke-linecap="round" stroke-width="1.75" d="M10 9v4" />
                    <circle cx="10" cy="15" r="0.5" fill="currentColor" stroke="none" />
                  </svg>
                {/if}

                <!-- Model info -->
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-medium {qwenInstalled ? 'text-malachite-light' : 'text-quartz'}">
                    <code class="font-mono">qwen2.5:7b-instruct</code>
                    <span class="ml-1.5 text-xs font-normal text-flint-light">~4.7 GB</span>
                  </p>
                  <p class="text-xs text-flint-light mt-0.5">
                    {#if qwenInstalled}
                      Installed — claim verification enabled
                    {:else if pullingModel === 'qwen2.5:7b-instruct'}
                      Downloading… this may take 5–10 minutes
                    {:else}
                      Not installed — required for claim verification
                    {/if}
                  </p>
                </div>

                {#if !qwenInstalled && pullingModel !== 'qwen2.5:7b-instruct'}
                  <button
                    onclick={() => pullOllamaModel('qwen2.5:7b-instruct')}
                    disabled={pullingModel !== null}
                    class="flex-shrink-0 min-h-[44px] px-3 py-2 rounded-md text-xs font-medium
                           bg-lapis hover:bg-lapis-dark text-white transition-colors duration-150
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                           focus-visible:ring-offset-2 focus-visible:ring-offset-graphite
                           disabled:opacity-40 disabled:cursor-not-allowed"
                    aria-label="Download qwen2.5:7b-instruct (approximately 4.7 gigabytes)"
                  >
                    Download
                  </button>
                {/if}
              </li>
            </ul>

            <!-- Pull error (shown below the list, outside the list items) -->
            {#if pullError}
              <div
                class="flex items-start gap-2 rounded-md px-3 py-2.5 mb-3"
                style="background: rgba(180,60,60,0.08); border: 1px solid rgba(180,60,60,0.25);"
                role="alert"
                aria-live="assertive"
              >
                <svg class="flex-shrink-0 w-4 h-4 text-cinnabar-light mt-0.5" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8 2L1 13h14L8 2z" />
                  <path stroke="currentColor" stroke-linecap="round" stroke-width="1.5" d="M8 7v3" />
                  <circle cx="8" cy="12" r="0.5" fill="currentColor" stroke="none" />
                </svg>
                <p class="text-xs text-cinnabar-light leading-relaxed">{pullError}</p>
              </div>
            {/if}

            <!-- All models ready banner -->
            {#if allModelsReady}
              <div
                class="flex items-center gap-2 rounded-md px-3 py-2.5"
                style="background: rgba(91,138,95,0.1); border: 1px solid rgba(91,138,95,0.25);"
                role="status"
                aria-live="polite"
              >
                <svg class="flex-shrink-0 w-4 h-4 text-malachite-light" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l3 3 7-7" />
                </svg>
                <p class="text-xs text-malachite-light font-medium">All models ready — AI features are fully enabled</p>
              </div>
            {/if}

            <!-- Continue button (when Ollama is available, footer's Continue won't show; provide one here) -->
            {#if allModelsReady}
              <button
                bind:this={primaryActionEl}
                onclick={goNext}
                class="w-full min-h-[44px] py-2.5 px-4 bg-lapis hover:bg-lapis-dark text-white text-sm font-medium
                       rounded-lg transition-colors duration-150 mt-4
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                       focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
              >
                Continue
              </button>
            {/if}
          {/if}
        </div>

      <!-- ── Step 4: Ready ──────────────────────────────────────────── -->
      {:else if currentStep === 4}
        <div
          class="flex-1 flex flex-col px-8 pt-6 pb-6 motion-safe:animate-[fadeIn_200ms_ease-out]"
          role="group"
          aria-labelledby="step4-heading"
        >
          <p class="text-xs font-medium text-malachite uppercase tracking-widest mb-3">
            All done
          </p>

          <h2 id="step4-heading" class="font-heading text-xl font-semibold text-quartz leading-tight mb-5" style="letter-spacing: -0.01em;">
            Ready to use Jura Trace
          </h2>

          <!-- Summary list -->
          <ul class="space-y-2.5 mb-6" aria-label="Service availability summary">

            <!-- Core verification — always available -->
            <li class="flex items-center gap-3">
              <svg class="flex-shrink-0 w-4 h-4 text-malachite-light" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l3 3 7-7" />
              </svg>
              <span class="text-sm text-quartz">Core verification — always available</span>
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
                Forensic analysis {sidecarOnline ? '— available' : '— not available (start the Analysis Engine)'}
              </span>
            </li>

            <!-- Video and audio -->
            <li class="flex items-center gap-3">
              {#if ffmpegAvailable}
                <svg class="flex-shrink-0 w-4 h-4 text-malachite-light" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l3 3 7-7" />
                </svg>
              {:else}
                <svg class="flex-shrink-0 w-4 h-4 text-flint-light" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4l8 8M12 4l-8 8" />
                </svg>
              {/if}
              <span class="text-sm {ffmpegAvailable ? 'text-quartz' : 'text-flint-light'}">
                Video and audio {ffmpegAvailable ? '— available' : '— not available (install FFmpeg)'}
              </span>
            </li>

            <!-- Transcription -->
            <li class="flex items-center gap-3">
              {#if transcriptionAvailable}
                <svg class="flex-shrink-0 w-4 h-4 text-malachite-light" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l3 3 7-7" />
                </svg>
              {:else}
                <svg class="flex-shrink-0 w-4 h-4 text-flint-light" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4l8 8M12 4l-8 8" />
                </svg>
              {/if}
              <span class="text-sm {transcriptionAvailable ? 'text-quartz' : 'text-flint-light'}">
                Speech transcription {transcriptionAvailable ? '— available' : '— downloads on first use'}
              </span>
            </li>

            <!-- AI descriptions and claim verification -->
            <li class="flex items-center gap-3">
              {#if allModelsReady}
                <svg class="flex-shrink-0 w-4 h-4 text-malachite-light" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l3 3 7-7" />
                </svg>
              {:else}
                <svg class="flex-shrink-0 w-4 h-4 text-flint-light" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                  <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4l8 8M12 4l-8 8" />
                </svg>
              {/if}
              <span class="text-sm {allModelsReady ? 'text-quartz' : 'text-flint-light'}">
                {#if allModelsReady}
                  AI descriptions and claim verification — available
                {:else if ollamaAvailable}
                  AI descriptions and claim verification — models not yet downloaded (optional)
                {:else}
                  AI descriptions and claim verification — not available (optional)
                {/if}
              </span>
            </li>

          </ul>

          <!-- Primary CTA -->
          <button
            bind:this={primaryActionEl}
            onclick={onComplete}
            class="w-full min-h-[44px] py-3 px-6 bg-lapis hover:bg-lapis-dark text-white text-sm font-medium
                   rounded-lg transition-colors duration-150 mt-auto
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                   focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
          >
            All set — start using Jura Trace
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

        {:else if currentStep === 3 && (!ollamaAvailable || allModelsReady)}
          <!-- Step 3: CTAs are in the content area (Ollama install CTA or all-models-ready Continue) -->
          <div></div>

        {:else}
          <!-- Skip / Continue button in footer -->
          {#if !isLastStep}
            <!-- Skip (ghost) -->
            {#if currentStep < TOTAL_STEPS - 2}
              <button
                onclick={onComplete}
                class="text-xs text-flint-light hover:text-flint transition-colors duration-150
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                       focus-visible:ring-offset-2 focus-visible:ring-offset-graphite rounded px-1"
              >
                Skip setup
              </button>
            {/if}

            <!-- Continue -->
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
