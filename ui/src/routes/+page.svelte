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
    <p class="text-xs text-flint dark:text-[#A09D95] uppercase tracking-widest mb-5">Local-first content integrity</p>
    <h1
      class="text-4xl font-heading text-text-light dark:text-quartz mb-5 font-normal"
      style="letter-spacing: -0.01em; line-height: 1.3;"
    >
      Know What's Real
    </h1>
    <p class="text-base text-flint dark:text-[#9B9890] max-w-md mx-auto mb-3 leading-relaxed">
      In a world of synthetic media, the ability to verify what you see matters more than ever.
    </p>
    <p class="text-sm text-flint/70 dark:text-flint max-w-sm mx-auto italic leading-relaxed">
      Everything happens on your machine. Nothing leaves.
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
        aria-label="{stats.c2paSignedCount.toLocaleString()} content credentials"
      >
        {stats.c2paSignedCount.toLocaleString()}
      </p>
      <p class="text-xs text-flint dark:text-[#A09D95] mt-1.5 tracking-wide lowercase">content credentials</p>
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
        Import your files and let Jura Trace catalogue them with care. Sign with C2PA Content
        Credentials so your work carries proof of origin wherever it travels. Generate perceptual
        fingerprints that persist even when images are cropped, resized, or screenshotted.
      </p>

      <!-- Asset hint -->
      <p class="text-xs text-flint/60 dark:text-flint mt-4 pl-24">
        {#if hasAssets}
          {stats.totalAssets} {stats.totalAssets === 1 ? 'file' : 'files'} catalogued — <a
            href="/protect"
            class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >view all</a>
        {:else}
          Import files on the Protect page to get started.
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
            href="/verify"
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

  <!-- Philosophy anchor -->
  <section
    class="py-20 text-center relative"
    aria-label="Our philosophy"
  >
    <!-- Vertical rule above -->
    <div
      class="absolute top-0 left-1/2 -translate-x-1/2 w-px h-10 bg-flint/20"
      aria-hidden="true"
    ></div>

    <div class="max-w-lg mx-auto">
      <p class="text-xs text-flint dark:text-[#A09D95] uppercase tracking-widest mb-6">The human centre</p>
      <blockquote
        class="font-heading text-xl font-normal text-text-light dark:text-quartz leading-relaxed mb-5"
        style="letter-spacing: -0.01em;"
      >
        Keep people at the heart of every decision.<br>
        <em class="text-lapis-light dark:text-[#8AABBF] not-italic">Use technology to support and guide, not to take over.</em>
      </blockquote>
      <p class="text-xs text-flint/70 dark:text-flint tracking-wide">Juralabs CIC — Reclaiming Technology for Society</p>
    </div>
  </section>

</div>
