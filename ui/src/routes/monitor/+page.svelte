<script lang="ts">
  import { onMount } from 'svelte';
  import { getMonitorOverview, getAuditLog, listMonitorUrls, addMonitorUrl, removeMonitorUrl, getMonitorEvents, updateMonitorCaseStatus } from '$lib/api';
  import type { MonitorOverview, AuditLogEntry, MonitorUrl, MonitorEvent } from '$lib/types';
  import ContextualHelpLink from '$lib/components/ContextualHelpLink.svelte';

  // ── State ──────────────────────────────────────────────────────

  let overview = $state<MonitorOverview | null>(null);
  let auditEntries = $state<AuditLogEntry[]>([]);
  let activeFilter = $state<string>('all');
  let auditOffset = $state(0);
  let loading = $state(true);
  let loadingMore = $state(false);

  const PAGE_SIZE = 50;

  // ── Watchlist state ───────────────────────────────────────────

  let watchlistUrls = $state<MonitorUrl[]>([]);
  let watchlistLoading = $state(false);
  let watchlistError = $state<string | null>(null);

  // Add URL form state
  let showAddForm = $state(false);
  let newUrl = $state('');
  let newLabel = $state('');
  let newFrequency = $state('daily');
  let addingUrl = $state(false);
  let addError = $state<string | null>(null);

  // Per-row expanded events
  let expandedUrlId = $state<string | null>(null);
  let expandedEvents = $state<MonitorEvent[]>([]);
  let eventsLoading = $state(false);

  // ── Lifecycle ──────────────────────────────────────────────────

  onMount(async () => {
    [overview, auditEntries, watchlistUrls] = await Promise.all([
      getMonitorOverview(),
      getAuditLog(PAGE_SIZE),
      listMonitorUrls(),
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

  // ── Watchlist helpers ─────────────────────────────────────────

  async function handleAddUrl() {
    const trimmed = newUrl.trim();
    if (!trimmed) {
      addError = 'Please enter a URL.';
      return;
    }
    try {
      new URL(trimmed);
    } catch {
      addError = 'Please enter a valid URL including the scheme (e.g. https://…).';
      return;
    }
    addingUrl = true;
    addError = null;
    try {
      const added = await addMonitorUrl(trimmed, newLabel.trim() || undefined, newFrequency);
      watchlistUrls = [added, ...watchlistUrls];
      newUrl = '';
      newLabel = '';
      newFrequency = 'daily';
      showAddForm = false;
    } catch (err) {
      addError = err instanceof Error ? err.message : String(err);
    } finally {
      addingUrl = false;
    }
  }

  async function handleRemoveUrl(urlId: string) {
    try {
      await removeMonitorUrl(urlId);
      watchlistUrls = watchlistUrls.filter(u => u.urlId !== urlId);
      if (expandedUrlId === urlId) {
        expandedUrlId = null;
        expandedEvents = [];
      }
    } catch (err) {
      watchlistError = err instanceof Error ? err.message : String(err);
    }
  }

  async function toggleExpanded(urlId: string) {
    if (expandedUrlId === urlId) {
      expandedUrlId = null;
      expandedEvents = [];
      return;
    }
    expandedUrlId = urlId;
    expandedEvents = [];
    eventsLoading = true;
    try {
      expandedEvents = await getMonitorEvents(urlId, 10);
    } finally {
      eventsLoading = false;
    }
  }

  function statusBadgeClass(status: string | null): string {
    switch (status) {
      case 'ok':      return 'bg-malachite/15 text-malachite dark:bg-malachite/20 dark:text-malachite-light';
      case 'changed': return 'bg-amber/15 text-amber dark:bg-amber/20 dark:text-amber-light';
      case 'missing':
      case 'error':   return 'bg-cinnabar/15 text-cinnabar dark:bg-cinnabar/20 dark:text-cinnabar-light';
      default:        return 'bg-graphite/30 text-flint dark:bg-graphite-light/30 dark:text-flint-light';
    }
  }

  function statusLabel(status: string | null): string {
    switch (status) {
      case 'ok':      return 'OK';
      case 'changed': return 'Changed';
      case 'missing': return 'Missing';
      case 'error':   return 'Error';
      default:        return 'Not checked';
    }
  }

  function eventTypelabel(et: string): string {
    const labels: Record<string, string> = {
      check_ok:        'Content unchanged',
      content_changed: 'Content changed',
      c2pa_stripped:   'C2PA credentials removed',
      watermark_lost:  'Watermark not found',
      http_error:      'HTTP error',
      timeout:         'Request timed out',
    };
    return labels[et] ?? et;
  }

  function caseStatusBadgeClass(cs: string): string {
    switch (cs) {
      case 'investigating': return 'bg-amber/15 text-amber dark:bg-amber/20 dark:text-amber-light';
      case 'resolved':      return 'bg-malachite/15 text-malachite dark:bg-malachite/20 dark:text-malachite-light';
      case 'escalated':     return 'bg-cinnabar/15 text-cinnabar dark:bg-cinnabar/20 dark:text-cinnabar-light';
      case 'dismissed':     return 'bg-graphite/30 text-flint dark:bg-graphite-light/30 dark:text-flint-light';
      default:              return 'bg-lapis/10 text-lapis dark:text-lapis-light';
    }
  }

  function truncateUrl(url: string, maxLen = 60): string {
    if (url.length <= maxLen) return url;
    return url.slice(0, maxLen - 1) + '…';
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
      style="letter-spacing: -0.01em; line-height: 1.3;"
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

  <!-- ── AI training limitation notice ────────────────────────── -->
  <div
    role="note"
    aria-label="AI training detection limitation"
    class="mx-auto max-w-2xl mb-8 rounded-lg border border-lapis/30 bg-lapis/10 px-5 py-4"
  >
    <div class="flex gap-3">
      <!-- Circle-i info icon -->
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 20 20"
        fill="currentColor"
        class="mt-0.5 h-4 w-4 flex-shrink-0 text-lapis dark:text-lapis-light"
        aria-hidden="true"
      >
        <path
          fill-rule="evenodd"
          d="M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0Zm-7-4a1 1 0 1 1-2 0 1 1 0 0 1 2 0ZM9 9a.75.75 0 0 0 0 1.5h.253a.25.25 0 0 1 .244.304l-.459 2.066A1.75 1.75 0 0 0 10.747 15H11a.75.75 0 0 0 0-1.5h-.253a.25.25 0 0 1-.244-.304l.459-2.066A1.75 1.75 0 0 0 9.253 9H9Z"
          clip-rule="evenodd"
        />
      </svg>
      <p class="text-xs leading-relaxed text-lapis dark:text-lapis-light">
        Content monitoring cannot detect whether your content has been used to train AI models.
        Watermarks and C2PA Content Credentials do not survive AI model training — they are
        designed to detect republication and unauthorised hosting, not extraction into training
        datasets.{' '}
        <a
          href="/help/methodology"
          class="underline underline-offset-2 hover:opacity-80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded-sm"
        >
          Learn more
        </a>
      </p>
    </div>
  </div>

  <!-- ── Earth line ────────────────────────────────────────────── -->
  <div class="earth-line" aria-hidden="true"></div>

  <!-- ── URL Watchlist ─────────────────────────────────────────── -->
  <section
    class="py-12 border-t border-border-light dark:border-[rgba(122,119,112,0.15)]"
    aria-labelledby="watchlist-heading"
  >
    <div class="flex items-baseline gap-4 mb-2">
      <span
        class="text-xs uppercase tracking-widest text-flint dark:text-[#A09D95] flex-shrink-0 w-20"
        aria-hidden="true"
      >
        Watch
      </span>
      <h2
        id="watchlist-heading"
        class="font-heading text-2xl font-normal text-text-light dark:text-quartz"
        style="letter-spacing: -0.01em;"
      >
        URL Watchlist
      </h2>
    </div>

    <p class="text-sm text-flint dark:text-[#9B9890] leading-relaxed pl-24 max-w-2xl mb-6">
      Register URLs where your protected content is published. Jura Trace will periodically check
      for changes to content, credentials, and watermarks.
    </p>

    <div class="pl-24">

      <!-- How URL monitoring works explanation -->
      <div class="rounded-lg border border-lapis/20 bg-lapis/5 px-4 py-3 mb-6 max-w-2xl">
        <p class="text-sm text-lapis dark:text-lapis-light font-medium mb-1">How URL Monitoring Works</p>
        <p class="text-xs text-flint dark:text-flint-light leading-relaxed">
          Add URLs where your protected content is published (for example, your website, social media profiles,
          or any page hosting your images). Jura Trace will periodically check each URL for changes
          and verify that your content credentials remain intact.
        </p>
        <p class="text-xs text-flint dark:text-flint-light leading-relaxed mt-1">
          <span class="font-medium">Note:</span> Domain-wide monitoring (scanning every page on a website)
          is planned for a future release. Currently, each URL is monitored individually.
        </p>
      </div>

      <!-- Add URL button / inline form -->
      {#if !showAddForm}
        <button
          onclick={() => { showAddForm = true; addError = null; }}
          class="min-h-[44px] px-5 py-2.5 text-sm rounded-full border border-lapis/40 text-lapis dark:text-lapis-light dark:border-lapis-light/40 hover:bg-lapis/10 dark:hover:bg-lapis-light/10 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian mb-6"
          aria-label="Add a URL to monitor"
        >
          Add URL
        </button>
      {:else}
        <div
          class="mb-6 rounded-lg border border-border-light dark:border-[rgba(122,119,112,0.2)] bg-white dark:bg-graphite p-5 max-w-2xl"
          role="form"
          aria-label="Add URL form"
        >
          <div class="space-y-3">
            <div>
              <label
                for="watchlist-url"
                class="block text-xs text-flint dark:text-flint-light mb-1.5"
              >
                URL <span class="text-cinnabar" aria-hidden="true">*</span>
              </label>
              <p class="text-xs text-flint dark:text-flint-light mb-1.5">
                Enter the exact URL of a page or image you want to monitor
              </p>
              <input
                id="watchlist-url"
                type="url"
                bind:value={newUrl}
                placeholder="https://example.com/gallery/my-photo.jpg"
                class="w-full rounded-md border border-border-light dark:border-[rgba(122,119,112,0.3)] bg-transparent px-3 py-2 text-sm text-text-light dark:text-quartz placeholder:text-flint/50 dark:placeholder:text-flint-light/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                aria-required="true"
                autocomplete="url"
              />
            </div>
            <div>
              <label
                for="watchlist-label"
                class="block text-xs text-flint dark:text-flint-light mb-1.5"
              >
                Label <span class="text-flint/50 dark:text-flint-light/50 font-normal">(optional)</span>
              </label>
              <input
                id="watchlist-label"
                type="text"
                bind:value={newLabel}
                placeholder="e.g. Museum collection page"
                class="w-full rounded-md border border-border-light dark:border-[rgba(122,119,112,0.3)] bg-transparent px-3 py-2 text-sm text-text-light dark:text-quartz placeholder:text-flint/50 dark:placeholder:text-flint-light/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                autocomplete="off"
              />
            </div>
            <div>
              <label
                for="watchlist-frequency"
                class="block text-xs text-flint dark:text-flint-light mb-1.5"
              >
                Check frequency
              </label>
              <select
                id="watchlist-frequency"
                bind:value={newFrequency}
                class="rounded-md border border-border-light dark:border-[rgba(122,119,112,0.3)] bg-white dark:bg-graphite px-3 py-2 text-sm text-text-light dark:text-quartz focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
              >
                <option value="hourly">Hourly</option>
                <option value="daily">Daily</option>
                <option value="weekly">Weekly</option>
              </select>
            </div>
          </div>

          {#if addError}
            <p class="mt-3 text-xs text-cinnabar dark:text-cinnabar-light" role="alert">{addError}</p>
          {/if}

          <div class="mt-4 flex gap-3">
            <button
              onclick={handleAddUrl}
              disabled={addingUrl}
              class="min-h-[44px] px-5 py-2 text-sm rounded-full bg-lapis text-white dark:bg-lapis-light dark:text-obsidian hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed transition-opacity focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
            >
              {addingUrl ? 'Adding…' : 'Add URL'}
            </button>
            <button
              onclick={() => { showAddForm = false; addError = null; newUrl = ''; newLabel = ''; newFrequency = 'daily'; }}
              class="min-h-[44px] px-5 py-2 text-sm rounded-full text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz border border-border-light dark:border-[rgba(122,119,112,0.3)] hover:border-flint/40 dark:hover:border-flint-light/40 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
            >
              Cancel
            </button>
          </div>
        </div>
      {/if}

      {#if watchlistError}
        <p class="mb-4 text-sm text-cinnabar dark:text-cinnabar-light" role="alert">{watchlistError}</p>
      {/if}

      <!-- Watchlist table -->
      {#if watchlistUrls.length === 0}
        <div
          class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-10 text-center max-w-md"
        >
          <h3
            class="font-heading text-lg font-normal text-text-light dark:text-quartz mb-3"
            style="letter-spacing: -0.01em;"
          >
            No URLs being monitored
          </h3>
          <div class="earth-line mb-5" aria-hidden="true"></div>
          <p class="text-sm text-flint dark:text-[#9B9890] leading-relaxed mb-2">
            Register URLs where your protected content is published.
            Jura Trace will track changes and verify that your content
            credentials remain intact.
          </p>
          <p class="text-sm text-flint/70 dark:text-flint leading-relaxed mt-4">
            Add your first URL using the form above.
          </p>
        </div>
      {:else}
        <ul class="list-none p-0 m-0 max-w-4xl" aria-label="Monitored URLs">
          {#each watchlistUrls as entry (entry.urlId)}
            <li class="border-b border-border-light dark:border-[rgba(122,119,112,0.12)] last:border-0">
              <!-- URL row -->
              <div class="py-4">
                <div class="flex items-start gap-3 flex-wrap">
                  <!-- Expand/collapse button -->
                  <button
                    onclick={() => toggleExpanded(entry.urlId)}
                    class="flex-1 min-w-0 text-left group focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                    aria-expanded={expandedUrlId === entry.urlId}
                    aria-controls="events-{entry.urlId}"
                  >
                    <div class="flex items-center gap-2 min-w-0">
                      <!-- Chevron -->
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        viewBox="0 0 20 20"
                        fill="currentColor"
                        class="h-3.5 w-3.5 flex-shrink-0 text-flint dark:text-flint-light transition-transform {expandedUrlId === entry.urlId ? 'rotate-90' : ''}"
                        aria-hidden="true"
                      >
                        <path
                          fill-rule="evenodd"
                          d="M7.21 14.77a.75.75 0 0 1 .02-1.06L11.168 10 7.23 6.29a.75.75 0 1 1 1.04-1.08l4.5 4.25a.75.75 0 0 1 0 1.08l-4.5 4.25a.75.75 0 0 1-1.06-.02Z"
                          clip-rule="evenodd"
                        />
                      </svg>
                      <div class="min-w-0">
                        {#if entry.label}
                          <p class="text-sm font-medium text-text-light dark:text-quartz truncate">
                            {entry.label}
                          </p>
                          <p class="text-xs text-flint dark:text-flint-light mt-0.5 truncate">
                            {truncateUrl(entry.url)}
                          </p>
                        {:else}
                          <p class="text-sm text-text-light dark:text-quartz truncate">
                            {truncateUrl(entry.url)}
                          </p>
                        {/if}
                      </div>
                    </div>
                  </button>

                  <!-- Status badge + meta + remove -->
                  <div class="flex items-center gap-2 flex-shrink-0 flex-wrap">
                    <span
                      class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {statusBadgeClass(entry.lastStatus)}"
                    >
                      {statusLabel(entry.lastStatus)}
                    </span>

                    {#if entry.lastCheckedAt}
                      <span class="text-xs text-flint dark:text-flint-light">
                        <time datetime={entry.lastCheckedAt}>
                          {formatDateTime(entry.lastCheckedAt)}
                        </time>
                      </span>
                    {/if}

                    <span class="text-xs text-flint/60 dark:text-flint-light/60">
                      {entry.checkFrequency}
                    </span>

                    <button
                      onclick={() => handleRemoveUrl(entry.urlId)}
                      class="min-h-[44px] min-w-[44px] flex items-center justify-center rounded text-flint/50 dark:text-flint-light/50 hover:text-cinnabar dark:hover:text-cinnabar-light transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar"
                      aria-label="Remove {entry.label ?? entry.url} from watchlist"
                    >
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        viewBox="0 0 20 20"
                        fill="currentColor"
                        class="h-4 w-4"
                        aria-hidden="true"
                      >
                        <path
                          fill-rule="evenodd"
                          d="M8.75 1A2.75 2.75 0 0 0 6 3.75v.443c-.795.077-1.584.176-2.365.298a.75.75 0 1 0 .23 1.482l.149-.022.841 10.518A2.75 2.75 0 0 0 7.596 19h4.807a2.75 2.75 0 0 0 2.742-2.53l.841-10.52.149.023a.75.75 0 0 0 .23-1.482A41.03 41.03 0 0 0 14 4.193V3.75A2.75 2.75 0 0 0 11.25 1h-2.5ZM10 4c.84 0 1.673.025 2.5.075V3.75c0-.69-.56-1.25-1.25-1.25h-2.5c-.69 0-1.25.56-1.25 1.25v.325C8.327 4.025 9.16 4 10 4ZM8.58 7.72a.75.75 0 0 0-1.5.06l.3 7.5a.75.75 0 1 0 1.5-.06l-.3-7.5Zm4.34.06a.75.75 0 1 0-1.5-.06l-.3 7.5a.75.75 0 1 0 1.5.06l.3-7.5Z"
                          clip-rule="evenodd"
                        />
                      </svg>
                    </button>
                  </div>
                </div>
              </div>

              <!-- Expanded events panel -->
              {#if expandedUrlId === entry.urlId}
                <div
                  id="events-{entry.urlId}"
                  class="pb-4 pl-5"
                  aria-label="Recent check events for {entry.label ?? entry.url}"
                >
                  {#if eventsLoading}
                    <p class="text-xs text-flint dark:text-flint-light py-2">Loading events…</p>
                  {:else if expandedEvents.length === 0}
                    <p class="text-xs text-flint dark:text-[#9B9890] py-2">
                      No check events recorded yet. Events will appear here once Jura Trace has performed its first check.
                    </p>
                  {:else}
                    <ul class="list-none p-0 m-0 space-y-2" aria-label="Check events">
                      {#each expandedEvents as ev (ev.eventId)}
                        <li class="rounded-md border border-border-light dark:border-[rgba(122,119,112,0.15)] bg-gray-50 dark:bg-graphite/50 px-4 py-3">
                          <div class="flex items-start gap-3 flex-wrap">
                            <div class="flex-1 min-w-0">
                              <div class="flex items-center gap-2 flex-wrap">
                                <span class="text-xs font-medium text-text-light dark:text-quartz">
                                  {eventTypelabel(ev.eventType)}
                                </span>
                                <span
                                  class="inline-flex items-center px-2 py-0.5 rounded-full text-xs {caseStatusBadgeClass(ev.caseStatus)}"
                                >
                                  {ev.caseStatus}
                                </span>
                              </div>
                              <p class="text-xs text-flint dark:text-flint-light mt-1">
                                <time datetime={ev.checkedAt}>{formatDateTime(ev.checkedAt)}</time>
                                {#if ev.httpStatus}
                                  &middot; HTTP {ev.httpStatus}
                                {/if}
                                {#if ev.responseTimeMs}
                                  &middot; {ev.responseTimeMs} ms
                                {/if}
                              </p>
                              {#if ev.caseNotes}
                                <p class="text-xs text-flint/80 dark:text-flint mt-1.5 italic">
                                  {ev.caseNotes}
                                </p>
                              {/if}
                            </div>

                            <!-- Case status controls -->
                            {#if ev.caseStatus === 'new' || ev.caseStatus === 'investigating'}
                              <div class="flex gap-1.5 flex-shrink-0 flex-wrap">
                                {#if ev.caseStatus === 'new'}
                                  <button
                                    onclick={async () => {
                                      await updateMonitorCaseStatus(ev.eventId, 'investigating');
                                      expandedEvents = expandedEvents.map(e =>
                                        e.eventId === ev.eventId ? { ...e, caseStatus: 'investigating' } : e
                                      );
                                    }}
                                    class="min-h-[44px] px-3 py-1 text-xs rounded-full border border-amber/40 text-amber dark:text-amber-light hover:bg-amber/10 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-amber"
                                  >
                                    Investigate
                                  </button>
                                {/if}
                                <button
                                  onclick={async () => {
                                    await updateMonitorCaseStatus(ev.eventId, 'resolved');
                                    expandedEvents = expandedEvents.map(e =>
                                      e.eventId === ev.eventId ? { ...e, caseStatus: 'resolved' } : e
                                    );
                                  }}
                                  class="min-h-[44px] px-3 py-1 text-xs rounded-full border border-malachite/40 text-malachite dark:text-malachite-light hover:bg-malachite/10 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-malachite"
                                >
                                  Resolve
                                </button>
                                <button
                                  onclick={async () => {
                                    await updateMonitorCaseStatus(ev.eventId, 'dismissed');
                                    expandedEvents = expandedEvents.map(e =>
                                      e.eventId === ev.eventId ? { ...e, caseStatus: 'dismissed' } : e
                                    );
                                  }}
                                  class="min-h-[44px] px-3 py-1 text-xs rounded-full border border-border-light dark:border-[rgba(122,119,112,0.3)] text-flint dark:text-flint-light hover:bg-gray-100 dark:hover:bg-graphite-light/40 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis"
                                >
                                  Dismiss
                                </button>
                              </div>
                            {/if}
                          </div>
                        </li>
                      {/each}
                    </ul>
                  {/if}
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
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
        style="letter-spacing: -0.01em;"
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
            style="letter-spacing: -0.02em;"
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
            style="letter-spacing: -0.02em;"
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
            style="letter-spacing: -0.02em;"
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
        style="letter-spacing: -0.01em;"
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
          style="letter-spacing: -0.02em;"
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
      <div class="flex items-center gap-2">
        <h2
          id="activity-heading"
          class="font-heading text-2xl font-normal text-text-light dark:text-quartz"
          style="letter-spacing: -0.01em;"
        >
          Everything that happened here
        </h2>
        <ContextualHelpLink href="/help/monitor" label="Learn about the monitor" />
      </div>
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
        style="letter-spacing: -0.01em;"
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
