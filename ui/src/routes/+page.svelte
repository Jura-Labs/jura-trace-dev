<script lang="ts">
  import { onMount } from 'svelte';
  import { getStats } from '$lib/api';
  import type { AppStats } from '$lib/types';

  let stats: AppStats = $state({
    totalAssets: 0,
    totalFingerprints: 0,
    totalVerifications: 0,
    c2paSignedCount: 0,
  });

  onMount(async () => {
    stats = await getStats();
  });
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
  <section class="grid grid-cols-2 md:grid-cols-4 gap-4">
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
  </section>

  <!-- Quick Actions -->
  <section class="grid grid-cols-1 md:grid-cols-2 gap-6">
    <a
      href="/protect"
      class="block bg-graphite rounded-lg p-8 border border-graphite-light hover:border-malachite transition-colors"
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
      class="block bg-graphite rounded-lg p-8 border border-graphite-light hover:border-lapis transition-colors"
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
