<script lang="ts">
  import { onMount } from 'svelte';
  import { getStats, checkSidecarHealth } from '$lib/api';
  import type { AppStats, SidecarHealth } from '$lib/types';

  let stats: AppStats = $state({
    totalAssets: 0,
    totalFingerprints: 0,
    totalVerifications: 0,
    c2paSignedCount: 0,
  });

  let sidecarHealth = $state<SidecarHealth | null>(null);

  const sidecarAvailable = $derived(sidecarHealth?.status === 'ok');
  const hasAssets = $derived(stats.totalAssets > 0);
  const isFirstRun = $derived(stats.totalAssets === 0 && stats.totalVerifications === 0);

  onMount(async () => {
    [stats, sidecarHealth] = await Promise.all([
      getStats(),
      checkSidecarHealth(),
    ]);
  });
</script>

<div class="space-y-0">

  <!-- Hero -->
  <section class="text-center py-16 pb-12">
    <p class="text-xs text-flint dark:text-[#A09D95] uppercase tracking-widest mb-5">12 forensic detectors. Everything stays on your device.</p>
    <h1
      class="text-4xl font-heading text-text-light dark:text-quartz mb-5 font-normal"
      style="letter-spacing: -0.01em; line-height: 1.3;"
    >
      Know What's Real
    </h1>
    <p class="text-base text-flint dark:text-[#9B9890] max-w-md mx-auto mb-3 leading-relaxed">
      AI-generated content has made verification essential. Jura Trace gives you 12 forensic detectors, C2PA provenance verification, and invisible watermarking — all running locally, with no cloud and no accounts.
    </p>
  </section>

  <!-- Earth line -->
  <div class="earth-line" aria-hidden="true"></div>

  <!-- Quiet accomplishments -->
  <section
    class="py-14 flex justify-center gap-16 flex-wrap"
    aria-label="Summary statistics"
  >
    <div class="text-center">
      <p
        class="font-heading text-3xl font-normal text-lapis-light dark:text-[#8AABBF] tracking-tight"
        style="letter-spacing: -0.02em;"
        aria-label="{stats.totalAssets.toLocaleString()} assets protected"
      >
        {stats.totalAssets.toLocaleString()}
      </p>
      <p class="text-xs text-flint dark:text-[#A09D95] mt-1.5 tracking-wide lowercase">assets protected</p>
    </div>

    <div class="text-center">
      <p
        class="font-heading text-3xl font-normal text-lapis-light dark:text-[#8AABBF] tracking-tight"
        style="letter-spacing: -0.02em;"
        aria-label="{stats.c2paSignedCount.toLocaleString()} C2PA-signed files"
      >
        {stats.c2paSignedCount.toLocaleString()}
      </p>
      <p class="text-xs text-flint dark:text-[#A09D95] mt-1.5 tracking-wide lowercase">C2PA-signed files</p>
    </div>

    <div class="text-center">
      <p
        class="font-heading text-3xl font-normal text-lapis-light dark:text-[#8AABBF] tracking-tight"
        style="letter-spacing: -0.02em;"
        aria-label="{stats.totalFingerprints.toLocaleString()} fingerprints"
      >
        {stats.totalFingerprints.toLocaleString()}
      </p>
      <p class="text-xs text-flint dark:text-[#A09D95] mt-1.5 tracking-wide lowercase">fingerprints</p>
    </div>

    <div class="text-center">
      <p
        class="font-heading text-3xl font-normal text-lapis-light dark:text-[#8AABBF] tracking-tight"
        style="letter-spacing: -0.02em;"
        aria-label="{stats.totalVerifications.toLocaleString()} verifications"
      >
        {stats.totalVerifications.toLocaleString()}
      </p>
      <p class="text-xs text-flint dark:text-[#A09D95] mt-1.5 tracking-wide lowercase">verifications</p>
    </div>
  </section>

  <!-- Earth line -->
  <div class="earth-line" aria-hidden="true"></div>

  <!-- First-run welcome state -->
  {#if isFirstRun}
    <section
      class="py-14 flex justify-center"
      aria-label="Getting started"
    >
      <div
        class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-10 text-center max-w-md w-full"
      >
        <h2
          class="font-heading text-2xl font-normal text-text-light dark:text-quartz mb-3"
          style="letter-spacing: -0.01em;"
        >
          Welcome to Jura Trace
        </h2>
        <div class="earth-line mb-6" aria-hidden="true"></div>
        <p class="text-sm text-flint dark:text-[#9B9890] leading-relaxed mb-2">
          Start by verifying an image or protecting your content.
        </p>
        <p class="text-sm text-flint dark:text-[#9B9890] leading-relaxed mb-8">
          Your activity will appear here as you use the tool.
        </p>
        <div class="flex flex-col sm:flex-row gap-3 justify-center">
          <a
            href="/verify/v2"
            class="inline-flex items-center justify-center px-5 py-2.5 min-h-[44px] rounded border border-lapis text-lapis dark:text-lapis-light dark:border-lapis-light text-sm font-medium
                   hover:bg-lapis hover:text-white dark:hover:bg-lapis-light dark:hover:text-obsidian transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
          >
            Verify Content
          </a>
          <a
            href="/protect"
            class="inline-flex items-center justify-center px-5 py-2.5 min-h-[44px] rounded border border-lapis text-lapis dark:text-lapis-light dark:border-lapis-light text-sm font-medium
                   hover:bg-lapis hover:text-white dark:hover:bg-lapis-light dark:hover:text-obsidian transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
          >
            Protect Content
          </a>
        </div>
      </div>
    </section>
  {/if}

  <!-- Analysis services status -->
  <div class="py-8 text-center" aria-live="polite" aria-atomic="true">
    {#if sidecarAvailable}
      <span
        class="inline-flex items-center gap-2 text-xs px-5 py-2 rounded-full border"
        style="color: #6B8F5F; background: rgba(107,143,95,0.06); border-color: rgba(107,143,95,0.12);"
      >
        <span class="w-1.5 h-1.5 rounded-full bg-malachite" aria-hidden="true"></span>
        Analysis services connected
      </span>
    {:else}
      <span
        class="inline-flex items-center gap-2 text-xs px-5 py-2 rounded-full border text-flint dark:text-flint border-border-light dark:border-border-dark"
      >
        <span class="w-1.5 h-1.5 rounded-full bg-flint/40" aria-hidden="true"></span>
        Analysis services offline
      </span>
    {/if}
  </div>

  <!-- Earth line -->
  <div class="earth-line" aria-hidden="true"></div>

  <!-- Narrative chapters -->
  <section class="pt-2 pb-12" aria-label="What you can do">

    <!-- Protect -->
    <div class="py-12 border-t border-border-light dark:border-[rgba(122,119,112,0.15)]">
      <div class="flex items-baseline gap-4 mb-4">
        <span class="text-xs uppercase tracking-widest text-flint dark:text-[#A09D95] flex-shrink-0 w-20">Protect</span>
        <h2
          class="font-heading text-2xl font-normal text-text-light dark:text-quartz"
          style="letter-spacing: -0.01em;"
        >
          <a
            href="/protect"
            class="hover:text-lapis dark:hover:text-lapis-light transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >
            Safeguard your content
          </a>
        </h2>
      </div>
      <p class="text-sm text-flint dark:text-[#9B9890] leading-relaxed pl-24 max-w-2xl">
        Import your files and let Jura Trace catalogue them with care. Sign with a C2PA
        provenance manifest so your work carries proof of origin wherever it travels. Generate
        perceptual fingerprints that persist even when images are cropped, resized, or screenshotted.
      </p>

      <!-- Asset hint -->
      <p class="text-xs text-flint/60 dark:text-flint mt-4 pl-24">
        {#if hasAssets}
          {stats.totalAssets} {stats.totalAssets === 1 ? 'file' : 'files'} catalogued — <a
            href="/protect"
            class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >view all</a>
        {:else}
          No content catalogued yet.
        {/if}
      </p>
    </div>

    <!-- Verify -->
    <div class="py-12 border-t border-border-light dark:border-[rgba(122,119,112,0.15)]">
      <div class="flex items-baseline gap-4 mb-4">
        <span class="text-xs uppercase tracking-widest text-flint dark:text-[#A09D95] flex-shrink-0 w-20">Verify</span>
        <h2
          class="font-heading text-2xl font-normal text-text-light dark:text-quartz"
          style="letter-spacing: -0.01em;"
        >
          <a
            href="/verify/v2"
            class="hover:text-lapis dark:hover:text-lapis-light transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >
            Check what you're looking at
          </a>
        </h2>
      </div>
      <p class="text-sm text-flint dark:text-[#9B9890] leading-relaxed pl-24 max-w-2xl">
        Drop an image, paste a URL, or describe a claim. Jura Trace examines the evidence
        layer by layer — forensic analysis, metadata inspection, region-based composite
        detection — and tells you what it finds. Honestly. No certainty where none exists.
      </p>
    </div>

    <!-- Monitor -->
    <div class="py-12 border-t border-border-light dark:border-[rgba(122,119,112,0.15)]">
      <div class="flex items-baseline gap-4 mb-4">
        <span class="text-xs uppercase tracking-widest text-flint dark:text-[#A09D95] flex-shrink-0 w-20">Monitor</span>
        <h2
          class="font-heading text-2xl font-normal text-text-light dark:text-quartz"
          style="letter-spacing: -0.01em;"
        >
          <a
            href="/monitor"
            class="hover:text-lapis dark:hover:text-lapis-light transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >
            See what has happened to your work
          </a>
        </h2>
      </div>
      <p class="text-sm text-flint dark:text-[#9B9890] leading-relaxed pl-24 max-w-2xl">
        Review your content's protection history, track verification outcomes, and follow
        the complete audit trail of every action taken. A narrative record of your archive,
        kept entirely on your machine.
      </p>
    </div>

  </section>

</div>
