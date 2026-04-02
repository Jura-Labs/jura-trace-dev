<script lang="ts">
  // No reactive state needed — this is a static help page.
  // Section IDs are used for deep linking from contextual help buttons.
</script>

<!-- ── Page header ────────────────────────────────────────────────── -->
<header class="mb-8">
  <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
    Understanding the Monitor
  </h1>
  <p class="text-base text-flint dark:text-flint-light leading-relaxed max-w-2xl">
    The Monitor section gives you a complete record of every protection and verification
    action Jura Trace has performed. Your Protection Chronicle, Trust Landscape, and
    Activity Record form a layered audit trail — held entirely on your device, never
    transmitted elsewhere.
  </p>
</header>

<!-- Earth-line divider -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<!-- ── Table of contents ───────────────────────────────────────────── -->
<nav aria-label="Page contents" class="mb-10">
  <p class="text-xs font-semibold uppercase tracking-widest text-flint dark:text-flint-light mb-3">
    On this page
  </p>
  <ol class="space-y-1 text-sm">
    <li>
      <a href="#what-monitor-does"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        1. What Monitor Does
      </a>
    </li>
    <li>
      <a href="#the-dashboard"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        2. The Dashboard
      </a>
    </li>
    <li>
      <a href="#the-audit-trail"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        3. The Audit Trail
      </a>
    </li>
    <li>
      <a href="#hash-chain-integrity"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        4. Hash Chain Integrity
      </a>
    </li>
    <li>
      <a href="#filtering-and-searching"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        5. Filtering and Searching
      </a>
    </li>
    <li>
      <a href="#ai-training"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        6. AI Training Detection Limitation
      </a>
    </li>
    <li>
      <a href="#url-watchlist"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        7. URL Watchlist
      </a>
    </li>
    <li>
      <a href="#best-practices"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        8. Best Practices
      </a>
    </li>
  </ol>
</nav>

<!-- ══════════════════════════════════════════════════════════════════
     1. What Monitor Does
     ══════════════════════════════════════════════════════════════════ -->
<section id="what-monitor-does" class="mb-12" aria-labelledby="heading-what-monitor-does">
  <h2 id="heading-what-monitor-does" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    1. What Monitor Does
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Monitor is the observational layer of Jura Trace. While Protect and Verify are active
    operations, Monitor gives you a view of what has already happened — across your entire
    collection and over time. It answers three questions at a glance:
  </p>

  <ul class="space-y-3 mb-6">
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">How well protected is my collection?</strong>
        The Protection Chronicle shows how many assets have been imported, signed with C2PA
        credentials, and watermarked.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">What does my verification history show?</strong>
        The Trust Landscape displays the distribution of verification verdicts — authentic,
        inconclusive, and synthetic — across all checks performed.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">What actions were taken, and when?</strong>
        The Activity Record is a filterable log of every protect and verify action, drawn
        from the local audit database.
      </span>
    </li>
  </ul>

  <!-- Callout: local-first -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">All data is local.</strong>
      The Monitor draws entirely from your on-device SQLite database. No activity data,
      no statistics, and no audit records are transmitted to any external service.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     2. The Dashboard
     ══════════════════════════════════════════════════════════════════ -->
<section id="the-dashboard" class="mb-12" aria-labelledby="heading-the-dashboard">
  <h2 id="heading-the-dashboard" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    2. The Dashboard
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
    The Monitor dashboard presents three panels. Each draws live from the local database
    and updates as you add assets or perform verifications.
  </p>

  <div class="space-y-6">

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Protection Chronicle
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
        A summary of your protected collection. The Chronicle shows three counts:
      </p>
      <ul class="space-y-2 text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
        <li class="flex gap-2">
          <span class="text-malachite dark:text-malachite-light flex-none">✓</span>
          <span>
            <strong class="text-text-light dark:text-text-dark">Assets imported</strong> —
            the total number of files registered in your local database.
          </span>
        </li>
        <li class="flex gap-2">
          <span class="text-malachite dark:text-malachite-light flex-none">✓</span>
          <span>
            <strong class="text-text-light dark:text-text-dark">Signed with C2PA</strong> —
            files that carry a valid Content Credentials manifest.
          </span>
        </li>
        <li class="flex gap-2">
          <span class="text-malachite dark:text-malachite-light flex-none">✓</span>
          <span>
            <strong class="text-text-light dark:text-text-dark">Watermarked</strong> —
            files that have had an invisible frequency-domain watermark embedded.
          </span>
        </li>
      </ul>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        The difference between "imported" and "signed" or "watermarked" indicates assets
        that have been catalogued but not yet fully protected. Use this as a prompt to
        complete protection for any outstanding items.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Trust Landscape
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
        A distribution bar showing the spread of verification verdicts across all files
        you have checked. The three verdict categories are:
      </p>
      <div class="overflow-x-auto mb-3">
        <table class="w-full text-sm border-collapse">
          <thead>
            <tr class="border-b border-border-light dark:border-border-dark">
              <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Verdict</th>
              <th class="text-left py-2 text-text-light dark:text-text-dark font-semibold">Meaning</th>
            </tr>
          </thead>
          <tbody class="text-flint dark:text-flint-light">
            <tr class="border-b border-border-light/50 dark:border-border-dark/50">
              <td class="py-2 pr-4 font-semibold text-malachite dark:text-malachite-light">Authentic</td>
              <td class="py-2">Signals consistent with camera capture; no significant anomalies detected.</td>
            </tr>
            <tr class="border-b border-border-light/50 dark:border-border-dark/50">
              <td class="py-2 pr-4 font-semibold text-amber-600 dark:text-amber-400">Inconclusive</td>
              <td class="py-2">Mixed or insufficient signals; manual review recommended.</td>
            </tr>
            <tr>
              <td class="py-2 pr-4 font-semibold text-cinnabar dark:text-cinnabar-light">Synthetic</td>
              <td class="py-2">Multiple signals consistent with AI generation or manipulation detected.</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        A high proportion of "inconclusive" results often indicates a collection of highly
        compressed or heavily processed images. Consider running those files through the
        Verify page on Deep or Archival mode for a more thorough analysis.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Activity Record
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        A reverse-chronological list of all logged actions. Each entry shows the action
        type (protect, verify, database change), the file or asset involved, the timestamp,
        and the outcome. The Activity Record is drawn directly from the audit log in the
        local database. See <a href="#the-audit-trail" class="text-lapis dark:text-lapis-light underline decoration-lapis/30 hover:decoration-lapis">The Audit Trail</a>
        for a full description of what gets recorded.
      </p>
    </div>

  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     3. The Audit Trail
     ══════════════════════════════════════════════════════════════════ -->
<section id="the-audit-trail" class="mb-12" aria-labelledby="heading-the-audit-trail">
  <h2 id="heading-the-audit-trail" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    3. The Audit Trail
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Every significant action in Jura Trace is written to the audit log — a tamper-evident
    record stored in your local database. The audit trail is the sedimentary record of your
    work: each layer deposited in sequence, each one referencing the layer beneath it.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    What gets recorded
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
    The following action types are logged automatically. You do not need to take any
    additional steps to enable logging.
  </p>

  <div class="space-y-4 mb-6">

    <div>
      <h4 class="text-sm font-semibold text-text-light dark:text-text-dark mb-1">
        Protect actions
      </h4>
      <ul class="space-y-1 text-sm text-flint dark:text-flint-light leading-relaxed">
        <li class="flex gap-2">
          <span class="text-lapis dark:text-lapis-light flex-none">→</span>
          <span><strong class="text-text-light dark:text-text-dark">Asset import</strong> — file path, detected format, perceptual fingerprints, and timestamp.</span>
        </li>
        <li class="flex gap-2">
          <span class="text-lapis dark:text-lapis-light flex-none">→</span>
          <span><strong class="text-text-light dark:text-text-dark">C2PA signing</strong> — output file path, manifest claim generator, content hash, and timestamp.</span>
        </li>
        <li class="flex gap-2">
          <span class="text-lapis dark:text-lapis-light flex-none">→</span>
          <span><strong class="text-text-light dark:text-text-dark">Watermark embedding</strong> — asset ID, watermark strength level, institution name encoded, and timestamp.</span>
        </li>
        <li class="flex gap-2">
          <span class="text-lapis dark:text-lapis-light flex-none">→</span>
          <span><strong class="text-text-light dark:text-text-dark">Batch watermarking</strong> — count of files processed, any failures, and overall completion timestamp.</span>
        </li>
      </ul>
    </div>

    <div>
      <h4 class="text-sm font-semibold text-text-light dark:text-text-dark mb-1">
        Verify actions
      </h4>
      <ul class="space-y-1 text-sm text-flint dark:text-flint-light leading-relaxed">
        <li class="flex gap-2">
          <span class="text-lapis dark:text-lapis-light flex-none">→</span>
          <span><strong class="text-text-light dark:text-text-dark">File verification</strong> — file path, investigation mode, trust score, verdict, and timestamp.</span>
        </li>
        <li class="flex gap-2">
          <span class="text-lapis dark:text-lapis-light flex-none">→</span>
          <span><strong class="text-text-light dark:text-text-dark">URL verification</strong> — the URL submitted (with any query strings redacted for privacy), verdict, and timestamp.</span>
        </li>
        <li class="flex gap-2">
          <span class="text-lapis dark:text-lapis-light flex-none">→</span>
          <span><strong class="text-text-light dark:text-text-dark">Batch verification</strong> — file count, individual verdicts, and completion timestamp.</span>
        </li>
      </ul>
    </div>

    <div>
      <h4 class="text-sm font-semibold text-text-light dark:text-text-dark mb-1">
        Database changes
      </h4>
      <ul class="space-y-1 text-sm text-flint dark:text-flint-light leading-relaxed">
        <li class="flex gap-2">
          <span class="text-lapis dark:text-lapis-light flex-none">→</span>
          <span><strong class="text-text-light dark:text-text-dark">Asset deletion</strong> — asset ID, file name, and timestamp of removal from the catalogue.</span>
        </li>
        <li class="flex gap-2">
          <span class="text-lapis dark:text-lapis-light flex-none">→</span>
          <span><strong class="text-text-light dark:text-text-dark">Database relocation</strong> — old path, new path, and timestamp.</span>
        </li>
      </ul>
    </div>

  </div>

  <!-- Callout: what is not recorded -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Privacy note:</strong>
      The audit log records file paths and actions, not file contents. The full pixel data,
      media data, or document text of any file is never written to the audit database.
      URL query strings are redacted before logging.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     4. Hash Chain Integrity
     ══════════════════════════════════════════════════════════════════ -->
<section id="hash-chain-integrity" class="mb-12" aria-labelledby="heading-hash-chain">
  <h2 id="heading-hash-chain" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    4. Hash Chain Integrity
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The audit log uses a SHA-256 hash chain to make tampering detectable. Each entry
    contains a hash of the previous entry. If any entry is altered — or if entries are
    deleted or reordered — the chain breaks, and the integrity check will report a failure.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    How the chain works
  </h3>

  <ol class="space-y-4 mb-6">
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">1</span>
      <span>
        When the first audit entry is written, Jura Trace computes a SHA-256 hash of its
        content and stores that hash alongside the entry.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">2</span>
      <span>
        Each subsequent entry includes the hash of the entry immediately before it. The
        new entry's own hash is then computed over both its content and that prior hash.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20 text-lapis dark:text-lapis-light text-xs font-semibold flex items-center justify-center">3</span>
      <span>
        To verify integrity, Jura Trace replays the chain from the first entry to the last,
        recomputing each hash and comparing it to the stored value. A mismatch at any point
        indicates that the log has been modified.
      </span>
    </li>
  </ol>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Checking integrity
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The integrity status is shown at the top of the Activity Record. When the chain is
    intact, the status reads <strong class="text-text-light dark:text-text-dark">Verified</strong>.
    If the check fails, the status reads <strong class="text-text-light dark:text-text-dark">Chain broken</strong>
    and indicates approximately where in the log the discrepancy was found.
  </p>

  <!-- Callout: intended use -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Using the chain as evidence.</strong>
      The hash chain is designed to give you confidence that your audit log has not been
      modified since it was written. If you need to demonstrate the integrity of your
      verification history — for example, in a dispute or institutional review — you can
      export the log and present the chain verification status alongside it.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     5. Filtering and Searching
     ══════════════════════════════════════════════════════════════════ -->
<section id="filtering-and-searching" class="mb-12" aria-labelledby="heading-filtering">
  <h2 id="heading-filtering" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    5. Filtering and Searching
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The Activity Record can be narrowed using the filter controls above the log. You can
    apply filters individually or combine them.
  </p>

  <div class="space-y-5">

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Action type
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Filter by the category of action: <strong class="text-text-light dark:text-text-dark">Protect</strong>
        (imports, signing, watermarking),
        <strong class="text-text-light dark:text-text-dark">Verify</strong>
        (file, URL, and batch verifications), or
        <strong class="text-text-light dark:text-text-dark">Database</strong>
        (deletions, relocations). Selecting "All" clears the action type filter.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Date range
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Set a start date, an end date, or both to restrict the log to a specific period.
        This is useful when preparing a report for a particular project or incident window.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Content type
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Filter by the format of the file involved: image, video, audio, or document. This
        lets you focus on, for example, all video verifications performed during a particular
        period.
      </p>
    </div>

  </div>

  <!-- Callout: clear filters -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4 mt-6">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Tip:</strong>
      Use the <strong class="text-text-light dark:text-text-dark">Clear filters</strong>
      button to return to the full Activity Record. Active filters are shown as badges
      above the log — each badge can be removed individually by clicking the
      <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">✗</span>
      on it.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     6. AI Training Detection Limitation
     ══════════════════════════════════════════════════════════════════ -->
<section id="ai-training" class="mb-12" aria-labelledby="heading-ai-training">
  <h2 id="heading-ai-training" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    6. AI Training Detection Limitation
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Jura Trace can protect your content against republication and metadata stripping. It
    cannot detect whether your content has been used to train an AI model. This distinction
    matters, and it deserves a clear explanation.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Why watermarks and C2PA do not survive AI training
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    When an image is used to train a generative model, the model does not preserve or
    transmit the image itself. Instead, it adjusts billions of internal numerical weights
    in response to the visual features it sees — shapes, textures, colours, compositional
    patterns. The training process:
  </p>

  <ul class="space-y-3 mb-6 text-sm text-flint dark:text-flint-light leading-relaxed">
    <li class="flex gap-3">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold mt-0.5">→</span>
      <span>
        Discards all file-level metadata, including C2PA manifests, EXIF data, and any
        embedded rights information. The model sees only decoded pixel values.
      </span>
    </li>
    <li class="flex gap-3">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold mt-0.5">→</span>
      <span>
        Does not preserve frequency-domain signals. Invisible watermarks are embedded in
        the DCT or DWT coefficient domain. Training pipelines typically decode images to
        raw pixel data and apply augmentation (resize, crop, colour jitter) that erases
        these signals before the model ever processes them.
      </span>
    </li>
    <li class="flex gap-3">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold mt-0.5">→</span>
      <span>
        Produces no output that can be traced back to a specific source image. An image
        generated by a trained model reflects the statistical aggregate of many inputs,
        not the fingerprint of any single one.
      </span>
    </li>
  </ul>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    What Monitor can and cannot detect
  </h3>

  <div class="overflow-x-auto mb-6">
    <table class="w-full text-sm border-collapse">
      <thead>
        <tr class="border-b border-border-light dark:border-border-dark">
          <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Scenario</th>
          <th class="text-left py-2 text-text-light dark:text-text-dark font-semibold">Can Monitor detect it?</th>
        </tr>
      </thead>
      <tbody class="text-flint dark:text-flint-light">
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4">Republication of the original file on a website</td>
          <td class="py-2 text-malachite dark:text-malachite-light">✓ Via URL Watchlist — re-verify registered URLs on demand</td>
        </tr>
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4">Metadata stripping before redistribution</td>
          <td class="py-2 text-malachite dark:text-malachite-light">✓ C2PA verification will show manifest missing</td>
        </tr>
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4">Watermark surviving JPEG compression and redistribution</td>
          <td class="py-2 text-malachite dark:text-malachite-light">✓ Watermark extraction in Verify pipeline</td>
        </tr>
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4">Use of your image to train an AI model</td>
          <td class="py-2 text-cinnabar dark:text-cinnabar-light">✗ Not detectable — no traceable signal survives</td>
        </tr>
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4">Style transfer producing a visual derivative</td>
          <td class="py-2 text-cinnabar dark:text-cinnabar-light">✗ Style alone cannot be fingerprinted</td>
        </tr>
        <tr>
          <td class="py-2 pr-4">AI-generated image that resembles your content</td>
          <td class="py-2 text-amber-600 dark:text-amber-400">~ Perceptual similarity search (planned feature)</td>
        </tr>
      </tbody>
    </table>
  </div>

  <!-- Callout: honest limitation -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">An honest limitation.</strong>
      No current technology can reliably detect whether a specific image was included in
      a model's training data. Research in this area — including membership inference
      attacks — remains an active and unsettled field. Jura Trace will incorporate
      advances in this space as they mature and become reliable enough to present to
      users as meaningful evidence.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     7. URL Watchlist
     ══════════════════════════════════════════════════════════════════ -->
<section id="url-watchlist" class="mb-12" aria-labelledby="heading-url-watchlist">
  <h2 id="heading-url-watchlist" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    7. URL Watchlist
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The URL Watchlist lets you register public URLs for on-demand re-verification. Once a
    URL is added, Jura Trace re-runs C2PA manifest verification and watermark extraction
    against it each time you trigger a check, logging the result in the Activity Record.
    All checking runs entirely on your device — no cloud infrastructure is involved.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Adding a URL
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Navigate to the Monitor tab and enter a public URL in the
    <strong class="text-text-light dark:text-text-dark">Add URL</strong> field, then click
    <strong class="text-text-light dark:text-text-dark">Add</strong>. The URL appears in
    your watchlist immediately with a status badge reflecting the last known check result.
  </p>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Status badges
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Each URL entry carries a status badge that updates after every check:
  </p>

  <ul class="space-y-2 mb-4 text-sm text-flint dark:text-flint-light leading-relaxed">
    <li class="flex gap-3">
      <span class="flex-none font-semibold text-malachite dark:text-malachite-light">Active</span>
      <span>The URL is reachable and its last verification result was recorded successfully.</span>
    </li>
    <li class="flex gap-3">
      <span class="flex-none font-semibold text-amber dark:text-amber-light">Changed</span>
      <span>The C2PA status or watermark result differs from a previous check — review the event history.</span>
    </li>
    <li class="flex gap-3">
      <span class="flex-none font-semibold text-cinnabar dark:text-cinnabar-light">Error</span>
      <span>The URL could not be reached or the verification pipeline encountered a problem.</span>
    </li>
  </ul>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Detection history and case management
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Click any watchlist entry to expand it and view the full detection history — each event
    row shows the timestamp, C2PA manifest result, and watermark extraction outcome. You can
    assign a case status to any entry (for example, marking it as
    <strong class="text-text-light dark:text-text-dark">Under Review</strong> or
    <strong class="text-text-light dark:text-text-dark">Resolved</strong>) to track your
    investigative response over time.
  </p>

  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Local-first design.</strong>
      All watchlist data — registered URLs, event history, and case notes — is stored in the
      application database on your device. Nothing is sent to any external server.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     8. Best Practices
     ══════════════════════════════════════════════════════════════════ -->
<section id="best-practices" class="mb-12" aria-labelledby="heading-best-practices">
  <h2 id="heading-best-practices" class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    8. Best Practices
  </h2>

  <ul class="space-y-4">
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Review your audit trail regularly.</strong>
        A monthly review of the Activity Record helps you spot gaps — assets that have
        been imported but not yet signed, or verification sessions that were started but
        not completed.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Export verification reports before sharing findings.</strong>
        When communicating verification results to a colleague, client, or institution,
        export a PDF trust report or a case ZIP from the Verify page. This captures the
        full evidence package — including signal scores, the verdict, and a timestamp —
        in a shareable format.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Use the hash chain to demonstrate evidence integrity.</strong>
        If you need to present your audit trail in a formal context — such as a content
        dispute, a journalistic source verification review, or an institutional compliance
        check — run the integrity check immediately before export. A verified chain
        demonstrates that the log has not been altered since the actions were recorded.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Keep the Protection Chronicle balanced.</strong>
        Aim to sign and watermark every catalogued asset. Unsigned assets provide no
        cryptographic proof of origin if challenged later. The Chronicle makes it easy
        to see at a glance how complete your protection coverage is.
      </span>
    </li>
  </ul>
</section>
