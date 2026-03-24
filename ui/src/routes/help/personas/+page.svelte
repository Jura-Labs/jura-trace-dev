<script lang="ts">
  // On-page navigation anchors for the four persona sections.
  const sections = [
    { id: 'museums',     label: 'Museum and Archive Staff' },
    { id: 'journalists', label: 'Journalists and Fact-Checkers' },
    { id: 'creators',    label: 'Content Creators' },
    { id: 'researchers', label: 'Researchers and Analysts' },
  ] as const;
</script>

<!-- Page header -->
<header class="mb-8">
  <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
    Usage Guides
  </h1>
  <p class="text-base text-flint dark:text-flint-light leading-relaxed max-w-2xl">
    Jura Trace serves different communities with different needs. These guides provide
    practical workflows tailored to your role.
  </p>
</header>

<!-- Earth-line divider -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<!-- On-page navigation -->
<nav aria-label="On this page" class="mb-8">
  <p class="text-xs font-medium uppercase tracking-wider text-flint dark:text-flint-light mb-3">
    On this page
  </p>
  <ul role="list" class="space-y-1">
    {#each sections as section}
      <li>
        <a
          href="#{section.id}"
          class="
            text-sm text-flint dark:text-flint-light
            hover:text-lapis dark:hover:text-lapis-light
            motion-safe:transition-colors motion-safe:duration-150
            focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
            focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
          "
        >
          {section.label}
        </a>
      </li>
    {/each}
  </ul>
</nav>

<!-- ─── 1. Museum and Archive Staff ──────────────────────────────────────── -->
<section
  id="museums"
  aria-labelledby="heading-museums"
  class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6 mb-6"
>
  <h2
    id="heading-museums"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3"
  >
    Museum and Archive Staff
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    You manage collections of cultural heritage material — photographs, manuscripts,
    artworks — and need to protect them from unauthorised reproduction while maintaining
    provenance records. Jura Trace gives you a bedrock of cryptographic evidence and
    invisible watermarking that travels with each asset across its life in your collection.
  </p>

  <!-- Recommended workflow -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended workflow
  </h3>
  <ol class="list-decimal pl-6 space-y-2 text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    <li>
      Import your collection into Jura Trace via the
      <strong class="text-text-light dark:text-text-dark">Protect</strong> page.
    </li>
    <li>
      Sign each asset with C2PA Content Credentials (C2PA) — this creates a cryptographic
      record of your institution's ownership, including a timestamp and the signing
      authority.
    </li>
    <li>
      Apply invisible watermarks at <strong class="text-text-light dark:text-text-dark">Medium</strong>
      or <strong class="text-text-light dark:text-text-dark">High</strong> strength — these survive
      social media compression and moderate cropping.
    </li>
    <li>
      Use batch watermarking for large collections: select multiple assets and apply
      credentials in a single operation.
    </li>
    <li>
      Consider configuring the database location
      (<strong class="text-text-light dark:text-text-dark">Settings → Database Location</strong>)
      to a shared network drive so multiple workstations can access the same catalogue.
    </li>
  </ol>

  <!-- Recommended mode -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended investigation mode
  </h3>
  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    <strong class="text-text-light dark:text-text-dark">Archival</strong> — designed for
    preservation workflows with scanner-calibrated tolerances that account for the
    artefacts common in digitised heritage material.
  </p>

  <!-- Key features -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Key features to explore
  </h3>
  <ul class="list-disc pl-6 space-y-1 text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    <li>Batch watermarking for large-scale collection processing</li>
    <li>C2PA signing with institution name and timestamp</li>
    <li>Configurable database location for shared-network deployments</li>
    <li>PDF trust reports suitable for accession records and audit trails</li>
  </ul>

  <!-- Tips callout -->
  <div class="bg-obsidian/5 dark:bg-obsidian/50 rounded border border-border-light dark:border-border-dark p-4 mt-4">
    <p class="text-xs font-semibold uppercase tracking-wider text-flint dark:text-flint-light mb-2">
      Tips
    </p>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint dark:text-flint-light leading-relaxed">
      <li>
        Sign before watermarking — C2PA credentials attest to the pre-watermark original.
        Reversing the order means the credential captures a watermarked version rather
        than your master.
      </li>
      <li>
        Export trust reports as PDFs for your accession records. Each report includes the
        full fingerprint, watermark payload, and signing authority.
      </li>
      <li>
        Back up your database regularly — it holds all perceptual fingerprints and
        verification history. The default location is shown under
        <strong class="text-text-light dark:text-text-dark">Settings → Database Location</strong>.
      </li>
    </ul>
  </div>
</section>

<!-- ─── 2. Journalists and Fact-Checkers ─────────────────────────────────── -->
<section
  id="journalists"
  aria-labelledby="heading-journalists"
  class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6 mb-6"
>
  <h2
    id="heading-journalists"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3"
  >
    Journalists and Fact-Checkers
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    You verify images, videos, and documents received from sources or found online.
    You need to assess authenticity quickly and document your findings clearly. Each
    analysis builds another layer of verifiable evidence — a provenance chain you can
    cite in your records.
  </p>

  <!-- Recommended workflow -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended workflow
  </h3>
  <ol class="list-decimal pl-6 space-y-2 text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    <li>
      Receive an image or URL from a source.
    </li>
    <li>
      Drop the file on the
      <strong class="text-text-light dark:text-text-dark">Verify</strong> page, or
      paste the URL in the URL tab.
    </li>
    <li>
      Start with <strong class="text-text-light dark:text-text-dark">Standard</strong>
      mode for a quick initial assessment (approximately 15 seconds for images).
    </li>
    <li>
      If results are inconclusive, re-run in
      <strong class="text-text-light dark:text-text-dark">Deep</strong> mode for full
      forensic analysis across all detectors.
    </li>
    <li>
      Review the verdict, trust score, and individual detector findings. Pay particular
      attention to the Signal Agreement dashboard — it shows which detectors agree and
      which diverge.
    </li>
    <li>
      Check C2PA credentials: if present, verify the claim generator and signing
      authority. A valid C2PA manifest is strong evidence of provenance.
    </li>
    <li>
      Use "Investigate Further" for reverse image search. Note the privacy disclosure:
      sharing URLs with external search services may reveal your investigation to third
      parties.
    </li>
    <li>
      Export a PDF trust report or ZIP case file for your records. Both formats include
      the full evidence bundle.
    </li>
  </ol>

  <!-- Recommended mode -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended investigation mode
  </h3>
  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    <strong class="text-text-light dark:text-text-dark">Standard</strong> for initial
    triage.
    <strong class="text-text-light dark:text-text-dark">Deep</strong> for stories
    requiring full analysis before publication.
  </p>

  <!-- Key features -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Key features to explore
  </h3>
  <ul class="list-disc pl-6 space-y-1 text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    <li>URL verification — paste a link without downloading the file first</li>
    <li>Three-way verdict classification: Authentic, Inconclusive, or Synthetic</li>
    <li>Signal Agreement dashboard — per-detector consensus at a glance</li>
    <li>PDF trust reports and ZIP case export for editorial records</li>
    <li>Reverse image search integration with source-protection disclosure</li>
  </ul>

  <!-- Tips callout -->
  <div class="bg-obsidian/5 dark:bg-obsidian/50 rounded border border-border-light dark:border-border-dark p-4 mt-4">
    <p class="text-xs font-semibold uppercase tracking-wider text-flint dark:text-flint-light mb-2">
      Tips
    </p>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint dark:text-flint-light leading-relaxed">
      <li>
        <strong class="text-text-light dark:text-text-dark">"Inconclusive" is the most
        common verdict</strong> — it does not mean fake. It means the automated analysis
        cannot make a confident determination. Always apply editorial judgement alongside
        the forensic result.
      </li>
      <li>
        Missing or inconsistent EXIF metadata is common in manipulated images, but also
        in legitimate web-distributed content. Treat it as a signal, not a conclusion.
      </li>
      <li>
        The trust score is a forensic indicator. It is not proof of fabrication or
        authenticity. Use it to prioritise further investigation, not to end it.
      </li>
      <li>
        Be aware of source protection when using reverse image search. The privacy
        disclosure on the results page explains exactly what is shared with the search
        service.
      </li>
    </ul>
  </div>
</section>

<!-- ─── 3. Content Creators ───────────────────────────────────────────────── -->
<section
  id="creators"
  aria-labelledby="heading-creators"
  class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6 mb-6"
>
  <h2
    id="heading-creators"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3"
  >
    Content Creators
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    You create original photographs, illustrations, or videos and want to prove your
    authorship before publishing. C2PA Content Credentials and invisible watermarking
    deposit a durable record of your creative origin — one that survives the compression
    and cropping common across social media platforms.
  </p>

  <!-- Recommended workflow -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended workflow
  </h3>
  <ol class="list-decimal pl-6 space-y-2 text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    <li>
      Import your finished work into Jura Trace via the
      <strong class="text-text-light dark:text-text-dark">Protect</strong> page.
    </li>
    <li>
      Sign with C2PA Content Credentials — this embeds your creator information and a
      verifiable timestamp into the file.
    </li>
    <li>
      Apply an invisible watermark.
      <strong class="text-text-light dark:text-text-dark">Medium</strong> strength is
      recommended for web distribution: it balances visual quality against resilience to
      social media compression.
    </li>
    <li>
      Publish the signed, watermarked version.
    </li>
    <li>
      Keep the original file and your Jura Trace database as your provenance record.
      The database holds the perceptual fingerprint of every protected asset.
    </li>
  </ol>

  <!-- Recommended mode -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended investigation mode
  </h3>
  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    Not applicable — this workflow is centred on the
    <strong class="text-text-light dark:text-text-dark">Protect</strong> section.
    If you want to confirm your credentials are intact after publishing, run a
    <strong class="text-text-light dark:text-text-dark">Standard</strong> verification
    on the published version.
  </p>

  <!-- Key features -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Key features to explore
  </h3>
  <ul class="list-disc pl-6 space-y-1 text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    <li>C2PA signing with creator name and timestamp</li>
    <li>Invisible watermarking at three strength levels (Low, Medium, High)</li>
    <li>Asset catalogue with perceptual fingerprints for near-duplicate detection</li>
    <li>PDF trust reports as portable proof of provenance</li>
  </ul>

  <!-- Tips callout -->
  <div class="bg-obsidian/5 dark:bg-obsidian/50 rounded border border-border-light dark:border-border-dark p-4 mt-4">
    <p class="text-xs font-semibold uppercase tracking-wider text-flint dark:text-flint-light mb-2">
      Tips
    </p>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint dark:text-flint-light leading-relaxed">
      <li>
        Sign immediately after final editing — do not sign work-in-progress. The
        credential captures the exact state of the file at the moment of signing.
      </li>
      <li>
        Medium watermark strength survives social media compression on platforms such
        as Instagram and X while maintaining visual quality that is indistinguishable
        to the naked eye.
      </li>
      <li>
        Your perceptual hashes are stored locally and can be used to find near-duplicates.
        If you later encounter your work being reused without authorisation, the fingerprint
        provides a basis for a similarity comparison.
      </li>
      <li>
        Consider running a Standard verification on your published work periodically
        to confirm that C2PA credentials remain intact across the platforms where you
        distribute it.
      </li>
    </ul>
  </div>
</section>

<!-- ─── 4. Researchers and Analysts ──────────────────────────────────────── -->
<section
  id="researchers"
  aria-labelledby="heading-researchers"
  class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6 mb-6"
>
  <h2
    id="heading-researchers"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3"
  >
    Researchers and Analysts
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    You conduct systematic analysis of media content — misinformation research, digital
    forensics, or content authenticity studies. You need detailed, reproducible results
    with full visibility into each detector's methodology. Jura Trace exposes every layer
    of the analysis pipeline, giving you the strata of evidence needed to support
    rigorous academic or professional work.
  </p>

  <!-- Recommended workflow -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended workflow
  </h3>
  <ol class="list-decimal pl-6 space-y-2 text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    <li>
      Import your corpus into Jura Trace via the
      <strong class="text-text-light dark:text-text-dark">Protect</strong> page to
      register all items and generate perceptual fingerprints.
    </li>
    <li>
      Run verification in
      <strong class="text-text-light dark:text-text-dark">Deep</strong> or
      <strong class="text-text-light dark:text-text-dark">Archival</strong> mode for
      maximum signal coverage.
    </li>
    <li>
      Review the Signal Agreement dashboard to understand detector consensus — it shows
      which detectors flag an item and how they compare.
    </li>
    <li>
      Examine individual detector outputs. For composite detection, pay particular
      attention to the Regional Analysis section: Segmented ELA, Shadow Consistency,
      Colour Temperature, and Splice Boundary each analyse spatial inconsistencies
      independently.
    </li>
    <li>
      Use the Inspection Checklist for structured visual review alongside the automated
      analysis. It provides eight visual cues to look for manually.
    </li>
    <li>
      Export a ZIP case file for each analysed item — this bundles all forensic maps,
      metadata, the full verdict, and the trust score together in a single portable
      archive.
    </li>
    <li>
      Use the similar asset finder to compare perceptual hashes across your corpus
      and surface near-duplicate relationships.
    </li>
  </ol>

  <!-- Recommended mode -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended investigation mode
  </h3>
  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    <strong class="text-text-light dark:text-text-dark">Deep</strong> or
    <strong class="text-text-light dark:text-text-dark">Archival</strong> — both provide
    maximum detector coverage. Archival additionally applies scanner-calibrated tolerances
    suited to digitised heritage material and enables chain-of-custody logging.
  </p>

  <!-- Key features -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Key features to explore
  </h3>
  <ul class="list-disc pl-6 space-y-1 text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    <li>Signal Agreement dashboard — per-detector agreement and disagreement at a glance</li>
    <li>Methodology transparency page — detector approaches, weightings, and known limitations</li>
    <li>ZIP case export — complete evidence bundle per item</li>
    <li>Per-detector detail panels including forensic heatmaps</li>
    <li>Regional analysis section: Segmented ELA, Shadow Consistency, Colour Temperature, Splice Boundary</li>
    <li>False positive reporting to improve classifier performance over time</li>
  </ul>

  <!-- Tips callout -->
  <div class="bg-obsidian/5 dark:bg-obsidian/50 rounded border border-border-light dark:border-border-dark p-4 mt-4">
    <p class="text-xs font-semibold uppercase tracking-wider text-flint dark:text-flint-light mb-2">
      Tips
    </p>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint dark:text-flint-light leading-relaxed">
      <li>
        The
        <a
          href="/help/methodology"
          class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:text-lapis/80 dark:hover:text-lapis-light/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian motion-safe:transition-colors"
        >How Analysis Works</a> page documents every detector's approach, limitations, and
        known false positive triggers. Cite it in your research methodology when
        describing the analysis pipeline.
      </li>
      <li>
        The GBM (Gradient Boosting Machine) classifier was trained on a 545-image corpus
        (326 authentic, 219 AI-generated) with a cross-validation AUC-ROC of 0.945.
        Be aware of its training data scope when drawing conclusions about content types
        or generators outside that distribution.
      </li>
      <li>
        Use the false positive report button on verification results to flag
        incorrectly-classified items. Reports are stored locally in the database and
        can help inform future model calibration.
      </li>
      <li>
        Archival mode uses scanner-calibrated tolerances that account for digitisation
        artefacts common in heritage materials — useful when working with scanned
        analogue photographs or documents that would otherwise trigger noise-based
        detectors.
      </li>
    </ul>
  </div>
</section>

<!-- Footer navigation -->
<div class="earth-line mt-10 mb-8" role="separator" aria-hidden="true"></div>
<nav aria-label="Help section navigation" class="flex flex-wrap gap-4 text-sm">
  <a
    href="/help"
    class="text-flint dark:text-flint-light hover:text-lapis dark:hover:text-lapis-light motion-safe:transition-colors motion-safe:duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
  >
    ← Help home
  </a>
  <a
    href="/help/methodology"
    class="text-flint dark:text-flint-light hover:text-lapis dark:hover:text-lapis-light motion-safe:transition-colors motion-safe:duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
  >
    How Analysis Works →
  </a>
</nav>
