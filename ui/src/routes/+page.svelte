<script lang="ts">
  import { onMount } from 'svelte';
  import { getStats, getRecentAssets, checkSidecarHealth } from '$lib/api';
  import type { AppStats, Asset, SidecarHealth } from '$lib/types';
  import { formatFileSize, CONTENT_TYPE_LABELS } from '$lib/types';

  let stats: AppStats = $state({
    totalAssets: 0,
    totalFingerprints: 0,
    totalVerifications: 0,
    c2paSignedCount: 0,
  });

  let recentAssets: Asset[] = $state([]);
  let sidecarHealth = $state<SidecarHealth | null>(null);

  const sidecarAvailable = $derived(sidecarHealth?.status === 'ok');

  onMount(async () => {
    [stats, recentAssets, sidecarHealth] = await Promise.all([
      getStats(),
      getRecentAssets(5),
      checkSidecarHealth(),
    ]);
  });

  function contentTypeAbbr(type: string): string {
    switch (type) {
      case 'image':    return 'IMG';
      case 'document': return 'DOC';
      case 'video':    return 'VID';
      case 'audio':    return 'AUD';
      case '3d':       return '3D';
      case 'web':      return 'WEB';
      default:         return 'FILE';
    }
  }
</script>

<div class="space-y-8">
  <!-- Hero -->
  <section class="text-center py-12 border-b border-graphite-light">
    <p class="text-xs text-flint uppercase tracking-widest mb-3">Local-first content integrity</p>
    <h1 class="text-3xl font-heading text-quartz mb-4">What's Real</h1>
    <p class="text-base text-flint max-w-xl mx-auto">
      Protect your content from unauthorised AI extraction. Verify authenticity.
      Everything happens locally on your machine.
    </p>
  </section>

  <!-- Stats Grid -->
  <section aria-label="Summary statistics">
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
      <div class="bg-graphite rounded-lg p-6 border border-graphite-light">
        <p class="text-3xl font-heading text-lapis-light">{stats.totalAssets.toLocaleString()}</p>
        <p class="text-sm text-flint mt-1">Assets Protected</p>
      </div>
      <div class="bg-graphite rounded-lg p-6 border border-graphite-light">
        <p class="text-3xl font-heading text-malachite-light">{stats.c2paSignedCount.toLocaleString()}</p>
        <p class="text-sm text-flint mt-1">C2PA Signed</p>
      </div>
      <div class="bg-graphite rounded-lg p-6 border border-graphite-light">
        <p class="text-3xl font-heading text-lapis-light">{stats.totalFingerprints.toLocaleString()}</p>
        <p class="text-sm text-flint mt-1">Fingerprints</p>
      </div>
      <div class="bg-graphite rounded-lg p-6 border border-graphite-light">
        <p class="text-3xl font-heading text-amber-light">{stats.totalVerifications.toLocaleString()}</p>
        <p class="text-sm text-flint mt-1">Verifications</p>
      </div>
    </div>

    <!-- Sidecar status -->
    <div
      class="mt-4 flex items-center gap-2 text-xs px-3 py-2 rounded-lg border
             {sidecarAvailable
               ? 'bg-malachite/5 text-malachite-light border-malachite/15'
               : 'bg-graphite text-flint border-graphite-light'}"
    >
      <span
        class="w-2 h-2 rounded-full {sidecarAvailable ? 'bg-malachite' : 'bg-flint/40'}"
        aria-hidden="true"
      ></span>
      <span>
        ML Sidecar: {sidecarAvailable ? 'Connected' : 'Offline'}
      </span>
      {#if sidecarHealth?.capabilities}
        <span class="text-flint/60 ml-1">
          — ELA {sidecarHealth.capabilities.ela ? 'ready' : 'off'}
        </span>
      {/if}
    </div>
  </section>

  <!-- Recent Assets -->
  <section aria-label="Recent assets">
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-base font-heading text-quartz">Recent Assets</h2>
      <a
        href="/protect"
        class="text-xs text-lapis hover:text-lapis-light transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian rounded"
      >
        View all
      </a>
    </div>

    {#if recentAssets.length === 0}
      <div class="bg-graphite rounded-lg border border-graphite-light px-6 py-8 text-center">
        <p class="text-sm text-flint">No assets yet. Import files to get started.</p>
      </div>
    {:else}
      <div class="bg-graphite rounded-lg border border-graphite-light overflow-hidden">
        {#each recentAssets as asset (asset.assetId)}
          <div class="flex items-center gap-3 px-4 py-3 border-b border-graphite-light/50 last:border-b-0">
            <!-- Content type badge -->
            <span
              class="flex-shrink-0 text-xs font-mono px-1.5 py-0.5 rounded bg-graphite-light text-flint w-10 text-center"
              aria-label={CONTENT_TYPE_LABELS[asset.contentType] ?? asset.contentType}
            >
              {contentTypeAbbr(asset.contentType)}
            </span>

            <!-- File name -->
            <p class="flex-1 text-sm text-quartz truncate min-w-0" title={asset.fileName}>
              {asset.fileName}
            </p>

            <!-- File size -->
            <span class="flex-shrink-0 text-xs text-flint tabular-nums">
              {formatFileSize(asset.fileSize)}
            </span>

            <!-- Signed badge -->
            {#if asset.c2paSigned}
              <span
                class="flex-shrink-0 text-xs px-2 py-0.5 rounded bg-malachite/15 text-malachite-light"
                aria-label="C2PA signed"
              >
                Signed
              </span>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <!-- Quick Actions -->
  <section class="grid grid-cols-1 md:grid-cols-2 gap-6" aria-label="Quick actions">
    <a
      href="/protect"
      class="block bg-graphite rounded-lg p-8 border border-graphite-light hover:border-malachite transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
    >
      <p class="text-xs text-malachite-light uppercase tracking-wide mb-2">Protect</p>
      <h2 class="text-xl font-heading text-quartz mb-2">Safeguard Your Content</h2>
      <p class="text-sm text-flint">
        Import files, auto-catalogue with AI, sign with C2PA Content Credentials,
        and fingerprint for future tracking.
      </p>
    </a>

    <a
      href="/verify"
      class="block bg-graphite rounded-lg p-8 border border-graphite-light hover:border-lapis transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
    >
      <p class="text-xs text-lapis-light uppercase tracking-wide mb-2">Verify</p>
      <h2 class="text-xl font-heading text-quartz mb-2">Check Authenticity</h2>
      <p class="text-sm text-flint">
        Upload an image, paste a URL, or type a claim. Get forensic analysis,
        deepfake detection, and sourced verification.
      </p>
    </a>
  </section>
</div>
