<script lang="ts">
  import { onMount } from 'svelte';
  import { getMonitorOverview, getAuditLog } from '$lib/api';
  import type { MonitorOverview, AuditLogEntry } from '$lib/types';

  // ── State ──────────────────────────────────────────────────────

  let overview = $state<MonitorOverview | null>(null);
  let auditEntries = $state<AuditLogEntry[]>([]);
  let activeFilter = $state<string>('all');
  let auditOffset = $state(0);
  let loading = $state(true);
  let loadingMore = $state(false);

  const PAGE_SIZE = 50;

  // ── Lifecycle ──────────────────────────────────────────────────

  onMount(async () => {
    [overview, auditEntries] = await Promise.all([
      getMonitorOverview(),
      getAuditLog(PAGE_SIZE),
    ]);
    auditOffset = auditEntries.length;
    loading = false;
  });

  // ── Derived ────────────────────────────────────────────────────

  const filteredAudit = $derived(
    activeFilter === 'all'
      ? auditEntries
      : auditEntries.filter(e => e.action === activeFilter),
  );

  // ── Helpers ────────────────────────────────────────────────────

  const ACTION_VERBS: Record<string, string> = {
    import: 'Imported',
    verify: 'Verified',
    sign: 'Signed',
    delete: 'Removed',
    fingerprint: 'Fingerprinted',
    false_positive: 'Reported false positive for',
  };

  function formatAction(entry: AuditLogEntry): string {
    const verb = ACTION_VERBS[entry.action] ?? entry.action;
    return `${verb} ${entry.targetId}`;
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
    });
  }

  function formatDateTime(iso: string): string {
    return new Date(iso).toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'short',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function trustColour(score: number): string {
    if (score >= 0.7) return 'text-malachite dark:text-malachite-light';
    if (score >= 0.4) return 'text-amber dark:text-amber-light';
    return 'text-cinnabar dark:text-cinnabar-light';
  }

  function trustLabel(score: number): string {
    if (score >= 0.7) return 'High confidence';
    if (score >= 0.4) return 'Under review';
    return 'Concern raised';
  }

  async function loadMore() {
    loadingMore = true;
    const filter = activeFilter === 'all' ? undefined : activeFilter;
    const more = await getAuditLog(PAGE_SIZE, filter);
    // Append only entries not already present (dedup by logId)
    const existingIds = new Set(auditEntries.map(e => e.logId));
    const fresh = more.filter(e => !existingIds.has(e.logId));
    auditEntries = [...auditEntries, ...fresh];
    auditOffset += fresh.length;
    loadingMore = false;
  }

  // Filter tab definitions
  const filterTabs = [
    { key: 'all',       label: 'All' },
    { key: 'import',    label: 'Import' },
    { key: 'verify',    label: 'Verify' },
    { key: 'sign',      label: 'Sign' },
  ];
</script>

<!-- ── Loading skeleton ─────────────────────────────────────────── -->
{#if loading}
  <div class="py-32 text-center" aria-live="polite" aria-busy="true">
    <p class="text-sm text-flint dark:text-flint-light">Loading activity record…</p>
  </div>

{:else}
<div class="space-y-0">

  <!-- ── Hero ──────────────────────────────────────────────────── -->
  <section class="text-center py-16 pb-12">
    <p class="text-xs text-flint dark:text-[#A09D95] uppercase tracking-widest mb-5">
      Content story
    </p>
    <h1
      class="text-4xl font-heading text-text-light dark:text-quartz mb-5 font-normal"
      style="font-family: Georgia, 'Times New Roman', serif; letter-spacing: -0.01em; line-height: 1.3;"
    >
      What has happened to your work
    </h1>
    <p class="text-base text-flint dark:text-[#9B9890] max-w-md mx-auto mb-3 leading-relaxed">
      A record of every file you have protected and every claim you have examined.
    </p>
    <p class="text-sm text-flint/70 dark:text-flint max-w-sm mx-auto italic leading-relaxed">
      Everything stored locally. Nothing leaves this machine.
    </p>
  </section>

  <!-- ── Earth line ────────────────────────────────────────────── -->
  <div class="earth-line" aria-hidden="true"></div>

  <!-- ── Protection Chronicle ──────────────────────────────────── -->
  <section
    class="py-12 border-t border-border-light dark:border-[rgba(122,119,112,0.15)]"
    aria-labelledby="chronicle-heading"
  >
    <div class="flex items-baseline gap-4 mb-4">
      <span
        class="text-xs uppercase tracking-widest text-flint dark:text-[#A09D95] flex-shrink-0 w-20"
        aria-hidden="true"
      >
        Archive
      </span>
      <h2
        id="chronicle-heading"
        class="font-heading text-2xl font-normal text-text-light dark:text-quartz"
        style="font-family: Georgia, 'Times New Roman', serif; letter-spacing: -0.01em;"
      >
        <a
          href="/protect"
          class="hover:text-lapis dark:hover:text-lapis-light transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
        >
          Your protected collection
        </a>
      </h2>
    </div>

    {#if overview && overview.protection.totalAssets > 0}
      {@const p = overview.protection}
      <p class="text-sm text-flint dark:text-[#9B9890] leading-relaxed pl-24 max-w-2xl mb-8">
        {p.totalAssets.toLocaleString()} {p.totalAssets === 1 ? 'file' : 'files'} in your archive,
        {p.c2paSigned.toLocaleString()} with Content Credentials,
        {p.fingerprinted.toLocaleString()} fingerprinted.
        {#if p.earliestAt}
          First protected {formatDate(p.earliestAt)}.
        {/if}
      </p>

      <!-- Quiet figures -->
      <div
        class="pl-24 flex gap-12 flex-wrap"
        aria-label="Protection statistics"
      >
        <div class="text-center">
          <p
            class="font-heading text-3xl font-normal text-lapis-light dark:text-[#8AABBF] tracking-tight"
            style="font-family: Georgia, 'Times New Roman', serif; letter-spacing: -0.02em;"
            aria-label="{p.totalAssets.toLocaleString()} total assets"
          >
            {p.totalAssets.toLocaleString()}
          </p>
          <p class="text-xs text-flint dark:text-[#A09D95] mt-1.5 tracking-wide lowercase">
            assets
          </p>
        </div>

        <div class="text-center">
          <p
            class="font-heading text-3xl font-normal text-lapis-light dark:text-[#8AABBF] tracking-tight"
            style="font-family: Georgia, 'Times New Roman', serif; letter-spacing: -0.02em;"
            aria-label="{p.c2paSigned.toLocaleString()} with Content Credentials"
          >
            {p.c2paSigned.toLocaleString()}
          </p>
          <p class="text-xs text-flint dark:text-[#A09D95] mt-1.5 tracking-wide lowercase">
            C2PA signed
          </p>
        </div>

        <div class="text-center">
          <p
            class="font-heading text-3xl font-normal text-lapis-light dark:text-[#8AABBF] tracking-tight"
            style="font-family: Georgia, 'Times New Roman', serif; letter-spacing: -0.02em;"
            aria-label="{p.fingerprinted.toLocaleString()} fingerprinted"
          >
            {p.fingerprinted.toLocaleString()}
          </p>
          <p class="text-xs text-flint dark:text-[#A09D95] mt-1.5 tracking-wide lowercase">
            fingerprinted
          </p>
        </div>
      </div>

      <!-- Content type breakdown -->
      {#if Object.keys(p.byContentType).length > 0}
        <div class="pl-24 mt-8 flex flex-wrap gap-2" aria-label="Content types in collection">
          {#each Object.entries(p.byContentType) as [type, count]}
            <span class="inline-flex items-center gap-1.5 text-xs px-3 py-1 rounded-full bg-gray-100 dark:bg-graphite-light text-flint dark:text-flint-light">
              <span class="font-medium text-text-light dark:text-quartz">{count}</span>
              {type}
            </span>
          {/each}
        </div>
      {/if}

    {:else}
      <p class="text-sm text-flint dark:text-[#9B9890] leading-relaxed pl-24 max-w-2xl">
        No files protected yet.
        <a
          href="/protect"
          class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
        >
          Visit the Protect page
        </a>
        to import your first content.
      </p>
    {/if}
  </section>

  <!-- ── Earth line ────────────────────────────────────────────── -->
  <div class="earth-line" aria-hidden="true"></div>

  <!-- ── Trust Landscape ───────────────────────────────────────── -->
  <section
    class="py-12 border-t border-border-light dark:border-[rgba(122,119,112,0.15)]"
    aria-labelledby="trust-heading"
  >
    <div class="flex items-baseline gap-4 mb-4">
      <span
        class="text-xs uppercase tracking-widest text-flint dark:text-[#A09D95] flex-shrink-0 w-20"
        aria-hidden="true"
      >
        Trust
      </span>
      <h2
        id="trust-heading"
        class="font-heading text-2xl font-normal text-text-light dark:text-quartz"
        style="font-family: Georgia, 'Times New Roman', serif; letter-spacing: -0.01em;"
      >
        How verified content is holding up
      </h2>
    </div>

    {#if overview && overview.trust.total > 0}
      {@const t = overview.trust}

      <p class="text-sm text-flint dark:text-[#9B9890] leading-relaxed pl-24 max-w-2xl mb-6">
        Of {t.total.toLocaleString()} {t.total === 1 ? 'verification' : 'verifications'},
        {t.highCount.toLocaleString()} returned high confidence,
        {t.mediumCount.toLocaleString()} {t.mediumCount === 1 ? 'was' : 'were'} reviewed,
        and {t.lowCount.toLocaleString()} raised {t.lowCount === 1 ? 'a concern' : 'concerns'}.
        {#if t.latestAt}
          Last checked {formatDate(t.latestAt)}.
        {/if}
      </p>

      <!-- Trust distribution bar -->
      <div class="pl-24 max-w-lg" aria-label="Trust distribution">
        <div
          class="flex h-2 rounded-full overflow-hidden mb-3 bg-gray-100 dark:bg-graphite-light/30"
          role="img"
          aria-label="Trust distribution: {t.highCount} high, {t.mediumCount} medium, {t.lowCount} low"
        >
          {#if t.highCount > 0}
            <div
              class="bg-malachite dark:bg-malachite-light"
              style="width: {(t.highCount / t.total) * 100}%"
            ></div>
          {/if}
          {#if t.mediumCount > 0}
            <div
              class="bg-amber dark:bg-amber-light"
              style="width: {(t.mediumCount / t.total) * 100}%"
            ></div>
          {/if}
          {#if t.lowCount > 0}
            <div
              class="bg-cinnabar dark:bg-cinnabar-light"
              style="width: {(t.lowCount / t.total) * 100}%"
            ></div>
          {/if}
        </div>

        <!-- Bar legend -->
        <div class="flex gap-4 text-xs text-flint dark:text-flint-light" aria-hidden="true">
          <span class="flex items-center gap-1.5">
            <span class="w-2 h-2 rounded-full bg-malachite dark:bg-malachite-light flex-shrink-0"></span>
            High ({t.highCount})
          </span>
          <span class="flex items-center gap-1.5">
            <span class="w-2 h-2 rounded-full bg-amber dark:bg-amber-light flex-shrink-0"></span>
            Review ({t.mediumCount})
          </span>
          <span class="flex items-center gap-1.5">
            <span class="w-2 h-2 rounded-full bg-cinnabar dark:bg-cinnabar-light flex-shrink-0"></span>
            Concern ({t.lowCount})
          </span>
        </div>
      </div>

      <!-- Average trust -->
      <div class="pl-24 mt-8">
        <p
          class="font-heading text-4xl font-normal tracking-tight {trustColour(t.averageTrust)}"
          style="font-family: Georgia, 'Times New Roman', serif; letter-spacing: -0.02em;"
          aria-label="Average trust score: {Math.round(t.averageTrust * 100)} per cent"
        >
          {Math.round(t.averageTrust * 100)}%
        </p>
        <p class="text-xs text-flint dark:text-[#A09D95] mt-1.5 tracking-wide lowercase">
          average trust score
        </p>
      </div>

    {:else}
      <p class="text-sm text-flint dark:text-[#9B9890] leading-relaxed pl-24 max-w-2xl">
        No verifications yet.
        <a
          href="/verify"
          class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
        >
          Visit the Verify page
        </a>
        to examine your first file.
      </p>
    {/if}
  </section>

  <!-- ── Earth line ────────────────────────────────────────────── -->
  <div class="earth-line" aria-hidden="true"></div>

  <!-- ── Activity Record ───────────────────────────────────────── -->
  <section
    class="py-12 border-t border-border-light dark:border-[rgba(122,119,112,0.15)]"
    aria-labelledby="activity-heading"
  >
    <div class="flex items-baseline gap-4 mb-6">
      <span
        class="text-xs uppercase tracking-widest text-flint dark:text-[#A09D95] flex-shrink-0 w-20"
        aria-hidden="true"
      >
        Record
      </span>
      <h2
        id="activity-heading"
        class="font-heading text-2xl font-normal text-text-light dark:text-quartz"
        style="font-family: Georgia, 'Times New Roman', serif; letter-spacing: -0.01em;"
      >
        Everything that happened here
      </h2>
    </div>

    <!-- Filter tabs -->
    <div
      class="pl-24 flex gap-2 mb-6 flex-wrap"
      role="group"
      aria-label="Filter activity by action type"
    >
      {#each filterTabs as tab}
        <button
          onclick={() => { activeFilter = tab.key; }}
          class="min-h-[44px] px-4 py-2 text-xs rounded-full transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                 {activeFilter === tab.key
                   ? 'bg-lapis text-white dark:bg-lapis-light dark:text-obsidian'
                   : 'text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz bg-transparent hover:bg-gray-100 dark:hover:bg-graphite-light/40'}"
          aria-pressed={activeFilter === tab.key}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <!-- Entries -->
    <div class="pl-24">
      {#if filteredAudit.length === 0}
        <p class="text-sm text-flint dark:text-[#9B9890] py-4">
          {activeFilter === 'all' ? 'No activity recorded yet.' : `No ${activeFilter} activity recorded.`}
        </p>
      {:else}
        <ul aria-label="Activity log entries" class="list-none p-0 m-0">
          {#each filteredAudit as entry (entry.logId)}
            <li class="py-3 border-b border-border-light dark:border-[rgba(122,119,112,0.12)] last:border-0">
              <div class="flex items-baseline gap-3">
                <span class="text-xs text-flint dark:text-flint-light flex-shrink-0 w-28">
                  <time datetime={entry.createdAt}>{formatDateTime(entry.createdAt)}</time>
                </span>
                <span class="text-sm text-text-light dark:text-quartz leading-relaxed">
                  {formatAction(entry)}
                </span>
              </div>
              {#if entry.details}
                <p class="text-xs text-flint/70 dark:text-flint mt-1 ml-31 pl-[calc(theme(spacing.28)+theme(spacing.3))] leading-relaxed">
                  {entry.details}
                </p>
              {/if}
            </li>
          {/each}
        </ul>

        <!-- Show more -->
        {#if auditEntries.length >= PAGE_SIZE}
          <div class="mt-6">
            <button
              onclick={loadMore}
              disabled={loadingMore}
              class="min-h-[44px] px-5 py-2.5 text-sm text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors rounded-full border border-border-light dark:border-[rgba(122,119,112,0.3)] hover:border-flint/40 dark:hover:border-flint-light/40 disabled:opacity-50 disabled:cursor-not-allowed focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
              aria-label="Load more activity entries"
            >
              {loadingMore ? 'Loading…' : 'Show more'}
            </button>
          </div>
        {/if}
      {/if}
    </div>
  </section>

  <!-- ── Philosophy anchor ──────────────────────────────────────── -->
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
      <p class="text-xs text-flint dark:text-[#A09D95] uppercase tracking-widest mb-6">
        The human centre
      </p>
      <blockquote
        class="font-heading text-xl font-normal text-text-light dark:text-quartz leading-relaxed mb-5"
        style="font-family: Georgia, 'Times New Roman', serif; letter-spacing: -0.01em;"
      >
        Keep people at the heart of every decision.<br>
        <em class="text-lapis-light dark:text-[#8AABBF] not-italic">
          Use technology to support and guide, not to take over.
        </em>
      </blockquote>
      <p class="text-xs text-flint/70 dark:text-flint tracking-wide">
        Juralabs CIC — Reclaiming Technology for Society
      </p>
    </div>
  </section>

</div>
{/if}
