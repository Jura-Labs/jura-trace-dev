<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';

  // ── Props ─────────────────────────────────────────────────────────────
  interface Props {
    onComplete: () => void;
  }

  const { onComplete }: Props = $props();

  // ── State ─────────────────────────────────────────────────────────────
  let currentSlide = $state(0);
  const totalSlides = 3;

  // Reference to the dialog element for initial focus management
  let dialogEl: HTMLElement | null = $state(null);
  // Reference to the primary action button on each slide (bind:this requires $state)
  let primaryActionEl: HTMLElement | null = $state(null);

  // ── Derived ───────────────────────────────────────────────────────────
  const isFirstSlide = $derived(currentSlide === 0);
  const isLastSlide = $derived(currentSlide === totalSlides - 1);

  // ── Navigation ────────────────────────────────────────────────────────
  async function goNext() {
    if (isLastSlide) {
      onComplete();
      return;
    }
    currentSlide = Math.min(currentSlide + 1, totalSlides - 1);
    await focusPrimaryAction();
  }

  async function goBack() {
    if (isFirstSlide) return;
    currentSlide = Math.max(currentSlide - 1, 0);
    await focusPrimaryAction();
  }

  async function goToSlide(index: number) {
    currentSlide = index;
    await focusPrimaryAction();
  }

  async function focusPrimaryAction() {
    await tick();
    primaryActionEl?.focus();
  }

  // ── Keyboard handler ──────────────────────────────────────────────────
  function handleKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case 'ArrowRight':
      case 'Enter':
        // Enter advances only when focus is NOT already on a button (prevents
        // double-firing the button's own click + this handler).
        if (event.key === 'ArrowRight' || !(event.target instanceof HTMLButtonElement || event.target instanceof HTMLAnchorElement)) {
          event.preventDefault();
          goNext();
        }
        break;
      case 'ArrowLeft':
        event.preventDefault();
        goBack();
        break;
      case 'Escape':
        event.preventDefault();
        onComplete();
        break;
    }
  }

  // ── Focus trap ────────────────────────────────────────────────────────
  function handleFocusTrap(event: KeyboardEvent) {
    if (event.key !== 'Tab' || !dialogEl) return;

    const focusableSelectors =
      'a[href], button:not([disabled]), [tabindex]:not([tabindex="-1"])';
    const focusableElements = Array.from(
      dialogEl.querySelectorAll<HTMLElement>(focusableSelectors)
    ).filter((el) => el.offsetParent !== null); // visible only

    if (focusableElements.length === 0) return;

    const first = focusableElements[0];
    const last = focusableElements[focusableElements.length - 1];

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

  // ── Lifecycle ─────────────────────────────────────────────────────────
  let previouslyFocused: HTMLElement | null = null;

  onMount(() => {
    previouslyFocused = document.activeElement as HTMLElement | null;
    // Focus the primary action button on mount
    focusPrimaryAction();
  });

  onDestroy(() => {
    // Restore focus to the element that was active before the overlay opened
    previouslyFocused?.focus();
  });
</script>

<!--
  OnboardingOverlay — three-slide introduction to Jura Trace.

  WCAG 2.2 AA:
  - role="dialog" aria-modal="true" for screen reader boundary
  - Focus trap within the dialog
  - Focus moved to primary action on each slide change
  - All text meets 4.5:1 contrast on obsidian backgrounds
  - Keyboard: Arrow right / Enter = next, Arrow left = back, Escape = skip
  - Progress dots have aria-label
  - Reduced-motion: slide transition is suppressed when prefers-reduced-motion
-->

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center backdrop-blur-sm"
  style="background: rgba(30,33,40,0.95);"
  role="dialog"
  aria-modal="true"
  aria-label="Welcome to Jura Trace — getting started guide"
  tabindex="-1"
  bind:this={dialogEl}
  onkeydown={(e) => { handleKeydown(e); handleFocusTrap(e); }}
>
  <!-- Card -->
  <div class="relative w-full max-w-lg mx-4 rounded-xl overflow-hidden" style="background: #272B34; border: 1px solid rgba(122,119,112,0.2);"  >

    <!-- ── Slides container ───────────────────────────────────────────── -->
    <div class="min-h-[360px] flex flex-col">

      <!-- ── Slide 0: Welcome ──────────────────────────────────────────── -->
      {#if currentSlide === 0}
        <div
          class="flex-1 flex flex-col px-8 pt-10 pb-6 motion-safe:animate-[fadeIn_200ms_ease-out]"
          role="group"
          aria-label="Slide 1 of 3: Welcome"
        >
          <!-- Eyebrow label -->
          <p class="text-xs font-medium text-lapis-light uppercase tracking-widest mb-4">
            Welcome to Jura Trace
          </p>

          <!-- Headline -->
          <h2 class="font-heading text-2xl font-semibold text-quartz leading-tight mb-4" style="letter-spacing: -0.01em;">
            Know What's Real
          </h2>

          <!-- Body -->
          <p class="text-sm text-quartz leading-relaxed mb-3">
            Jura Trace is a local-first tool for protecting your digital assets and
            verifying content authenticity. Everything runs on your device — no cloud,
            no accounts, no tracking.
          </p>

          <!-- Subtext -->
          <p class="text-xs text-flint dark:text-flint-light leading-relaxed mt-auto">
            Built by Juralabs CIC — free for non-commercial use.
          </p>
        </div>

      <!-- ── Slide 1: Two tools in one ──────────────────────────────────── -->
      {:else if currentSlide === 1}
        <div
          class="flex-1 flex flex-col px-8 pt-10 pb-6 motion-safe:animate-[fadeIn_200ms_ease-out]"
          role="group"
          aria-label="Slide 2 of 3: Two tools in one"
        >
          <!-- Eyebrow label -->
          <p class="text-xs font-medium text-lapis-light uppercase tracking-widest mb-4">
            Two tools in one
          </p>

          <!-- Two-column feature grid -->
          <div class="grid grid-cols-2 gap-4 mb-5">

            <!-- PROTECT column -->
            <div class="rounded-lg p-4" style="background: rgba(30,33,40,0.6); border: 1px solid rgba(122,119,112,0.15);">
              <h2 class="text-xs font-semibold text-quartz uppercase tracking-widest mb-2 nav-link">
                Protect
              </h2>
              <p class="text-xs text-flint-light leading-relaxed">
                Stamp your images and documents with C2PA Content Credentials —
                machine-readable proof of origin.
              </p>
            </div>

            <!-- VERIFY column -->
            <div class="rounded-lg p-4" style="background: rgba(30,33,40,0.6); border: 1px solid rgba(122,119,112,0.15);">
              <h2 class="text-xs font-semibold text-quartz uppercase tracking-widest mb-2 nav-link">
                Verify
              </h2>
              <p class="text-xs text-flint-light leading-relaxed">
                Run forensic analysis on any image. Detect manipulation, AI generation,
                and provenance anomalies.
              </p>
            </div>

          </div>

          <!-- ML Sidecar note -->
          <div class="flex items-start gap-2.5 bg-amber/10 border border-amber/20 rounded-md px-3 py-2.5 mt-auto">
            <span class="flex-shrink-0 mt-px w-1.5 h-1.5 rounded-full bg-amber mt-1" aria-hidden="true"></span>
            <p class="text-xs text-flint-light leading-relaxed">
              The <span class="text-quartz font-medium">ML Sidecar</span> (optional) unlocks
              deep forensic analysis. Start it from the Settings page.
            </p>
          </div>
        </div>

      <!-- ── Slide 2: You're ready ──────────────────────────────────────── -->
      {:else if currentSlide === 2}
        <div
          class="flex-1 flex flex-col px-8 pt-10 pb-6 motion-safe:animate-[fadeIn_200ms_ease-out]"
          role="group"
          aria-label="Slide 3 of 3: You're ready"
        >
          <!-- Eyebrow label -->
          <p class="text-xs font-medium text-malachite uppercase tracking-widest mb-4">
            You're ready
          </p>

          <!-- Headline -->
          <h2 class="font-heading text-2xl font-semibold text-quartz leading-tight mb-4" style="letter-spacing: -0.01em;">
            Start with Verify or Protect
          </h2>

          <!-- Body -->
          <p class="text-sm text-quartz leading-relaxed mb-8">
            Drop an image on the Verify page to run your first analysis, or import
            files on the Protect page to begin cataloguing.
          </p>

          <!-- Get Started CTA -->
          <button
            bind:this={primaryActionEl}
            onclick={onComplete}
            class="w-full py-3 px-6 bg-lapis hover:bg-lapis-dark text-quartz text-sm font-medium rounded-lg
                   transition-colors duration-150 mt-auto
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                   focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
          >
            Get Started
          </button>
        </div>
      {/if}

    </div>

    <!-- ── Footer: progress + navigation ─────────────────────────────── -->
    <div class="flex items-center justify-between px-8 py-5 border-t" style="border-color: rgba(122,119,112,0.2);">

      <!-- Progress dots -->
      <nav aria-label="Slide progress">
        <ol class="flex items-center gap-2" role="list">
          {#each Array(totalSlides) as _, index}
            <li>
              <button
                onclick={() => goToSlide(index)}
                class="flex items-center justify-center w-6 h-6 rounded-full transition-all duration-200
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                       focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
                aria-label="Go to slide {index + 1}"
                aria-current={index === currentSlide ? 'step' : undefined}
              >
                <span
                  class="block rounded-full transition-all duration-200
                         {index === currentSlide
                           ? 'w-5 h-2 bg-lapis'
                           : 'w-2 h-2 bg-graphite-light group-hover:bg-flint'}"
                  aria-hidden="true"
                ></span>
              </button>
            </li>
          {/each}
        </ol>
      </nav>

      <!-- Right-side controls: Skip / Back + Next -->
      <div class="flex items-center gap-4">

        <!-- Skip link — visible on slides 0 and 1 only -->
        {#if !isLastSlide}
          <button
            onclick={onComplete}
            class="text-xs text-flint dark:text-flint-light hover:text-flint-light transition-colors duration-150
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                   focus-visible:ring-offset-2 focus-visible:ring-offset-graphite rounded"
          >
            Skip
          </button>
        {/if}

        <!-- Back button — hidden on first slide -->
        {#if !isFirstSlide && !isLastSlide}
          <button
            onclick={goBack}
            class="text-xs text-flint dark:text-flint-light hover:text-quartz transition-colors duration-150
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                   focus-visible:ring-offset-2 focus-visible:ring-offset-graphite rounded"
            aria-label="Go back to previous slide"
          >
            Back
          </button>
        {/if}

        <!-- Next button — hidden on last slide (replaced by Get Started CTA) -->
        {#if !isLastSlide}
          <button
            bind:this={primaryActionEl}
            onclick={goNext}
            class="py-1.5 px-4 bg-lapis hover:bg-lapis-dark text-white text-xs font-medium rounded-md
                   transition-colors duration-150
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                   focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
            aria-label="Next slide"
          >
            Next
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
