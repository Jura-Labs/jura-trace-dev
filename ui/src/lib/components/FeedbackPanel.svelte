<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { getVersion, getLicenceTier } from '$lib/api';

  // ── Props ─────────────────────────────────────────────────────────────
  interface Props {
    onClose: () => void;
  }

  const { onClose }: Props = $props();

  // ── State ─────────────────────────────────────────────────────────────
  type FeedbackKind = 'bug' | 'ux' | 'idea';

  let kind = $state<FeedbackKind>('bug');
  let description = $state('');
  let email = $state('');
  let copied = $state(false);
  let copyError = $state('');

  // Auto-collected context
  let appVersion = $state('');
  let platform = $state('');
  let tier = $state('');
  let currentPage = $state('');

  // Focus management
  let dialogEl: HTMLElement | null = $state(null);
  let closeButtonEl: HTMLElement | null = $state(null);
  let previouslyFocused: HTMLElement | null = null;

  // ── Derived ───────────────────────────────────────────────────────────
  const kindLabel = $derived(
    kind === 'bug' ? 'Bug Report' : kind === 'ux' ? 'UX Issue' : 'Feature Request'
  );

  // ── Lifecycle ─────────────────────────────────────────────────────────
  onMount(async () => {
    previouslyFocused = document.activeElement as HTMLElement | null;

    // Gather auto-collected context
    appVersion = await getVersion();
    tier = await getLicenceTier();
    currentPage = typeof window !== 'undefined' ? window.location.pathname : '';
    platform =
      typeof navigator !== 'undefined'
        ? (navigator as Navigator & { userAgentData?: { platform: string } })
            .userAgentData?.platform ?? navigator.platform ?? 'Unknown'
        : 'Unknown';

    // Move focus into the dialog
    await tick();
    closeButtonEl?.focus();
  });

  onDestroy(() => {
    previouslyFocused?.focus();
  });

  // ── Keyboard handling ─────────────────────────────────────────────────
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key === 'Tab') {
      handleFocusTrap(event);
    }
  }

  function handleFocusTrap(event: KeyboardEvent) {
    if (!dialogEl) return;
    const focusableSelectors =
      'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])';
    const focusableElements = Array.from(
      dialogEl.querySelectorAll<HTMLElement>(focusableSelectors)
    ).filter((el) => el.offsetParent !== null);

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

  // ── Clipboard ─────────────────────────────────────────────────────────
  function buildMarkdown(): string {
    const date = new Date().toISOString();
    const contactLine = email.trim() ? email.trim() : 'not provided';
    const descriptionBody = description.trim() || '(no description provided)';

    return [
      `## Feedback: ${kindLabel}`,
      '',
      `**Description:**`,
      descriptionBody,
      '',
      `**Contact:** ${contactLine}`,
      '',
      '---',
      '**Context (auto-collected):**',
      `- App version: ${appVersion}`,
      `- Platform: ${platform}`,
      `- Tier: ${tier}`,
      `- Page: ${currentPage}`,
      `- Date: ${date}`,
    ].join('\n');
  }

  async function copyToClipboard() {
    copyError = '';
    try {
      await navigator.clipboard.writeText(buildMarkdown());
      copied = true;
      setTimeout(() => {
        copied = false;
      }, 2000);
    } catch {
      copyError =
        'Could not access the clipboard. Please copy the text manually from the context box below.';
    }
  }
</script>

<!--
  FeedbackPanel — clipboard-based feedback form for pilot testers.

  WCAG 2.2 AA:
  - role="dialog" aria-modal="true" with descriptive aria-labelledby
  - Focus trap within the dialog; focus restored on close
  - Initial focus on close button so the heading is announced first
  - Escape key dismisses the dialog
  - Radio group with fieldset/legend for the feedback kind selector
  - All form inputs have associated visible labels
  - Success/error states use role="status" and role="alert" with aria-live
  - 44px minimum touch targets on all interactive elements
  - focus-visible rings on all interactive elements
  - Reduced-motion: fade-in animation suppressed when prefers-reduced-motion
-->

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center backdrop-blur-sm"
  style="background: rgba(30,33,40,0.92);"
  role="dialog"
  aria-modal="true"
  aria-labelledby="feedback-panel-heading"
  tabindex="-1"
  bind:this={dialogEl}
  onkeydown={handleKeydown}
>
  <!-- Card -->
  <div
    class="relative w-full max-w-md mx-4 rounded-xl overflow-hidden motion-safe:animate-[feedbackFadeIn_180ms_ease-out]"
    style="background: #272B34; border: 1px solid rgba(122,119,112,0.2);"
  >

    <!-- ── Header ─────────────────────────────────────────────────────── -->
    <div
      class="flex items-center justify-between px-6 pt-6 pb-5"
      style="border-bottom: 1px solid rgba(122,119,112,0.15);"
    >
      <h2
        id="feedback-panel-heading"
        class="font-heading text-lg font-semibold text-quartz"
        style="letter-spacing: -0.01em;"
      >
        Send Feedback
      </h2>
      <button
        bind:this={closeButtonEl}
        onclick={onClose}
        class="min-w-[44px] min-h-[44px] flex items-center justify-center rounded-lg text-flint-light hover:text-quartz transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
        aria-label="Close feedback panel"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- ── Body ───────────────────────────────────────────────────────── -->
    <div class="px-6 py-5 flex flex-col gap-5">

      <!-- Feedback kind -->
      <fieldset>
        <legend class="text-xs font-medium text-quartz mb-3">
          What kind of feedback?
        </legend>
        <div class="flex flex-col gap-2" role="radiogroup">
          {#each ([
            { value: 'bug',  label: 'Something went wrong (bug)' },
            { value: 'ux',   label: 'Something was confusing (UX)' },
            { value: 'idea', label: 'Something I\'d like to see (idea)' },
          ] as const) as option}
            <label
              class="flex items-center gap-3 cursor-pointer min-h-[44px] px-3 rounded-lg transition-colors duration-150
                     {kind === option.value
                       ? 'bg-lapis/10 border border-lapis/30'
                       : 'border border-transparent hover:bg-obsidian/40'}"
            >
              <input
                type="radio"
                name="feedback-kind"
                value={option.value}
                checked={kind === option.value}
                onchange={() => { kind = option.value; }}
                class="w-4 h-4 shrink-0 accent-lapis focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1 focus-visible:ring-offset-graphite"
              />
              <span class="text-sm {kind === option.value ? 'text-quartz' : 'text-flint-light'}">
                {option.label}
              </span>
            </label>
          {/each}
        </div>
      </fieldset>

      <!-- Description textarea -->
      <div class="flex flex-col gap-1.5">
        <label for="feedback-description" class="text-xs font-medium text-quartz">
          Tell us more:
        </label>
        <textarea
          id="feedback-description"
          bind:value={description}
          rows={4}
          placeholder="Describe what happened, what you expected, or what you'd like to see…"
          class="w-full rounded-lg px-3 py-2.5 text-sm text-quartz placeholder:text-flint resize-none
                 bg-obsidian/60 border border-[rgba(122,119,112,0.2)] hover:border-[rgba(122,119,112,0.35)]
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                 focus-visible:ring-offset-1 focus-visible:ring-offset-graphite
                 transition-colors duration-150"
        ></textarea>
      </div>

      <!-- Email input -->
      <div class="flex flex-col gap-1.5">
        <label for="feedback-email" class="text-xs font-medium text-quartz">
          Email
          <span class="text-flint-light font-normal ml-1">(optional)</span>
        </label>
        <input
          id="feedback-email"
          type="email"
          bind:value={email}
          placeholder="your@email.org"
          autocomplete="email"
          class="w-full rounded-lg px-3 py-2.5 text-sm text-quartz placeholder:text-flint
                 bg-obsidian/60 border border-[rgba(122,119,112,0.2)] hover:border-[rgba(122,119,112,0.35)]
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                 focus-visible:ring-offset-1 focus-visible:ring-offset-graphite
                 transition-colors duration-150"
        />
      </div>

      <!-- Auto-collected context box -->
      <div class="flex flex-col gap-1.5">
        <p class="text-xs font-medium text-quartz" id="context-label">
          Context
          <span class="text-flint-light font-normal ml-1">(auto-collected, shown for transparency)</span>
        </p>
        <div
          class="bg-obsidian/50 rounded-lg border border-[rgba(122,119,112,0.2)] p-3 text-xs font-mono text-flint-light leading-relaxed"
          aria-labelledby="context-label"
          role="region"
        >
          <div>Version: {appVersion || '…'}</div>
          <div>Platform: {platform || '…'}</div>
          <div>Tier: {tier || '…'}</div>
          <div>Page: {currentPage || '…'}</div>
        </div>
      </div>

      <!-- Clipboard action + cancel -->
      <div class="flex items-center gap-3">
        <button
          onclick={copyToClipboard}
          class="flex-1 min-h-[44px] py-2.5 px-4 rounded-lg text-sm font-medium transition-colors duration-150
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                 focus-visible:ring-offset-2 focus-visible:ring-offset-graphite
                 {copied
                   ? 'bg-malachite/20 border border-malachite/30 text-malachite-light'
                   : 'bg-lapis hover:bg-lapis-dark text-white'}"
          aria-label={copied ? 'Feedback copied to clipboard' : 'Copy feedback to clipboard'}
        >
          {#if copied}
            Copied!
          {:else}
            Copy to Clipboard
          {/if}
        </button>
        <button
          onclick={onClose}
          class="min-h-[44px] py-2.5 px-4 rounded-lg text-sm text-flint-light hover:text-quartz
                 border border-[rgba(122,119,112,0.2)] hover:border-[rgba(122,119,112,0.4)]
                 transition-colors duration-150
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                 focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
        >
          Cancel
        </button>
      </div>

      <!-- Clipboard error -->
      {#if copyError}
        <p
          role="alert"
          aria-live="assertive"
          class="text-xs text-cinnabar leading-relaxed"
        >
          {copyError}
        </p>
      {/if}

      <!-- Instruction text -->
      <p class="text-xs text-flint-light leading-relaxed">
        Paste this into an email to
        <a
          href="mailto:feedback@juralabs.org"
          class="text-lapis-light hover:text-quartz underline underline-offset-2 transition-colors duration-150
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
        >feedback@juralabs.org</a>
        or into a GitHub issue.
      </p>

    </div>
    <!-- end body -->

  </div>
  <!-- end card -->
</div>

<!-- Screen reader announcement for copy success -->
<div
  role="status"
  aria-live="polite"
  aria-atomic="true"
  class="sr-only"
>
  {#if copied}Feedback copied to clipboard.{/if}
</div>

<style>
  @keyframes feedbackFadeIn {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  @media (prefers-reduced-motion: reduce) {
    @keyframes feedbackFadeIn {
      from { opacity: 1; transform: none; }
      to   { opacity: 1; transform: none; }
    }
  }
</style>
