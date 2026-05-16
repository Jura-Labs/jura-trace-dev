<script lang="ts">
  // On-page navigation anchors for the eight persona sections.
  const sections = [
    { id: 'museums',     label: 'Museum and Archive Staff' },
    { id: 'journalists', label: 'Journalists and Fact-Checkers' },
    { id: 'creators',    label: 'Content Creators' },
    { id: 'researchers', label: 'Researchers and Analysts' },
    { id: 'legal',       label: 'Legal and Compliance Professionals' },
    { id: 'insurance',   label: 'Insurance and Claims Investigation' },
    { id: 'corporate',   label: 'Corporate Communications and Security' },
    { id: 'human-rights', label: 'Human Rights Documentation' },
  ] as const;
</script>

<!-- Page header -->
<header class="mb-8">
  <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
    Usage Guides
  </h1>
  <p class="text-base text-flint-dark dark:text-flint-light leading-relaxed max-w-2xl">
    Jura Trace serves different communities with different needs. These guides provide
    practical workflows tailored to your role.
  </p>
</header>

<!-- Earth-line divider -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<!-- On-page navigation -->
<nav aria-label="On this page" class="mb-8">
  <p class="text-xs font-medium uppercase tracking-wider text-flint-dark dark:text-flint-light mb-3">
    On this page
  </p>
  <ul role="list" class="space-y-1">
    {#each sections as section}
      <li>
        <a
          href="#{section.id}"
          class="
            text-sm text-flint-dark dark:text-flint-light
            hover:text-lapis dark:hover:text-lapis dark:text-lapis-light
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

  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    You manage collections of cultural heritage material — photographs, manuscripts,
    artworks — and need to protect them from unauthorised reproduction while maintaining
    provenance records. Jura Trace gives you a bedrock of cryptographic evidence and
    invisible watermarking that travels with each asset across its life in your collection.
  </p>

  <!-- Recommended workflow -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended workflow
  </h3>
  <ol class="list-decimal pl-6 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>
      Import your collection into Jura Trace via the
      <strong class="text-text-light dark:text-text-dark">Protect</strong> page.
    </li>
    <li>
      Sign each asset with C2PA provenance — this creates a cryptographic
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
  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <strong class="text-text-light dark:text-text-dark">Deep</strong> — runs the full
    detector suite, including the four regional detectors needed for composite-image
    analysis (Segmented ELA, Shadow Consistency, Colour Temperature, Splice Boundary).
  </p>

  <!-- Key features -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Key features to explore
  </h3>
  <ul class="list-disc pl-6 space-y-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>Batch watermarking for large-scale collection processing</li>
    <li>C2PA signing with institution name and timestamp</li>
    <li>Configurable database location for shared-network deployments</li>
    <li>PDF trust reports suitable for accession records and audit trails</li>
  </ul>

  <!-- Tips callout -->
  <div class="bg-obsidian/5 dark:bg-obsidian/50 rounded border border-border-light dark:border-border-dark p-4 mt-4">
    <p class="text-xs font-semibold uppercase tracking-wider text-flint-dark dark:text-flint-light mb-2">
      Tips
    </p>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed">
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

  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    You verify images, videos, and documents received from sources or found online.
    You need to assess authenticity quickly and document your findings clearly. Each
    analysis builds another layer of verifiable evidence — a provenance chain you can
    cite in your records.
  </p>

  <!-- Recommended workflow -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended workflow
  </h3>
  <ol class="list-decimal pl-6 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
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
  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <strong class="text-text-light dark:text-text-dark">Standard</strong> for initial
    triage.
    <strong class="text-text-light dark:text-text-dark">Deep</strong> for stories
    requiring full analysis before publication.
  </p>

  <!-- Key features -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Key features to explore
  </h3>
  <ul class="list-disc pl-6 space-y-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>URL verification — paste a link without downloading the file first</li>
    <li>Three-way verdict classification: Authentic, Inconclusive, or Synthetic</li>
    <li>Signal Agreement dashboard — per-detector consensus at a glance</li>
    <li>PDF trust reports and ZIP case export for editorial records</li>
    <li>Reverse image search integration with source-protection disclosure</li>
  </ul>

  <!-- Tips callout -->
  <div class="bg-obsidian/5 dark:bg-obsidian/50 rounded border border-border-light dark:border-border-dark p-4 mt-4">
    <p class="text-xs font-semibold uppercase tracking-wider text-flint-dark dark:text-flint-light mb-2">
      Tips
    </p>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed">
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

  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    You create original photographs, illustrations, or videos and want to prove your
    authorship before publishing. C2PA provenance and invisible watermarking
    deposit a durable record of your creative origin — one that survives the compression
    and cropping common across social media platforms.
  </p>

  <!-- Recommended workflow -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended workflow
  </h3>
  <ol class="list-decimal pl-6 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>
      Import your finished work into Jura Trace via the
      <strong class="text-text-light dark:text-text-dark">Protect</strong> page.
    </li>
    <li>
      Sign with C2PA provenance — this embeds your creator information and a
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
  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
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
  <ul class="list-disc pl-6 space-y-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>C2PA signing with creator name and timestamp</li>
    <li>Invisible watermarking at three strength levels (Low, Medium, High)</li>
    <li>Asset catalogue with perceptual fingerprints for near-duplicate detection</li>
    <li>PDF trust reports as portable proof of provenance</li>
  </ul>

  <!-- Tips callout -->
  <div class="bg-obsidian/5 dark:bg-obsidian/50 rounded border border-border-light dark:border-border-dark p-4 mt-4">
    <p class="text-xs font-semibold uppercase tracking-wider text-flint-dark dark:text-flint-light mb-2">
      Tips
    </p>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed">
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

  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
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
  <ol class="list-decimal pl-6 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>
      Import your corpus into Jura Trace via the
      <strong class="text-text-light dark:text-text-dark">Protect</strong> page to
      register all items and generate perceptual fingerprints.
    </li>
    <li>
      Run verification in
      <strong class="text-text-light dark:text-text-dark">Deep</strong> mode for
      maximum signal coverage — this includes the regional detectors needed for
      composite analysis.
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
  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <strong class="text-text-light dark:text-text-dark">Deep</strong> — runs the full
    detector suite including the four regional detectors (Segmented ELA, Shadow
    Consistency, Colour Temperature, Splice Boundary) needed for composite-image
    analysis.
  </p>

  <!-- Key features -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Key features to explore
  </h3>
  <ul class="list-disc pl-6 space-y-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>Signal Agreement dashboard — per-detector agreement and disagreement at a glance</li>
    <li>Methodology transparency page — detector approaches, weightings, and known limitations</li>
    <li>ZIP case export — complete evidence bundle per item</li>
    <li>Per-detector detail panels including forensic heatmaps</li>
    <li>Regional analysis section: Segmented ELA, Shadow Consistency, Colour Temperature, Splice Boundary</li>
    <li>False positive reporting to improve classifier performance over time</li>
  </ul>

  <!-- Tips callout -->
  <div class="bg-obsidian/5 dark:bg-obsidian/50 rounded border border-border-light dark:border-border-dark p-4 mt-4">
    <p class="text-xs font-semibold uppercase tracking-wider text-flint-dark dark:text-flint-light mb-2">
      Tips
    </p>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      <li>
        The
        <a
          href="/help/methodology"
          class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:text-lapis/80 dark:hover:text-lapis dark:text-lapis-light/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian motion-safe:transition-colors"
        >How Analysis Works</a> page documents every detector's approach, limitations, and
        known false positive triggers. Cite it in your research methodology when
        describing the analysis pipeline.
      </li>
      <li>
        The current GBM (Gradient Boosting Machine) classifier is v4, trained on a
        10,709-image corpus (5,724 authentic, 4,985 AI-generated from 14 generator
        families) with a cross-validation AUC-ROC of 0.9868 and authentic false-positive
        rate of 4.54%. It is combined with the UnivFD v10onnx probe (AUC-ROC 0.9929,
        authentic FP 3.87%) into an ensemble score. Be aware of the training-data scope
        when drawing conclusions about content types or generators outside that
        distribution.
      </li>
      <li>
        Use the false positive report button on verification results to flag
        incorrectly-classified items. Reports are stored locally in the database and
        can help inform future model calibration.
      </li>
    </ul>
  </div>
</section>

<!-- ─── 5. Legal and Compliance Professionals ─────────────────────────────── -->
<section
  id="legal"
  aria-labelledby="heading-legal"
  class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6 mb-6"
>
  <h2
    id="heading-legal"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3"
  >
    Legal and Compliance Professionals
  </h2>

  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    You authenticate digital evidence for litigation, regulatory submissions, or compliance
    investigations. You need methodology-disclosed analysis that can withstand legal challenge.
    Jura Trace produces a traceable stratum of forensic evidence: every detector version,
    every signal score, and every action recorded in a hash-chained audit trail.
  </p>

  <!-- Recommended workflow -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended workflow
  </h3>
  <ol class="list-decimal pl-6 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>
      Receive digital evidence (photographs, documents, PDFs).
    </li>
    <li>
      Import into Jura Trace to create a fingerprint and audit trail entry.
    </li>
    <li>
      Run verification in
      <strong class="text-text-light dark:text-text-dark">Deep</strong> mode for full
      detector coverage including the four regional detectors.
    </li>
    <li>
      Review the verdict and per-detector signal scores.
    </li>
    <li>
      Export a PDF trust report with your analyst declaration — name, firm, and case reference.
    </li>
    <li>
      The methodology statement and detector versions are included in every report automatically.
    </li>
    <li>
      Archive the ZIP case export for your file.
    </li>
  </ol>

  <!-- Recommended mode -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended investigation mode
  </h3>
  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <strong class="text-text-light dark:text-text-dark">Deep</strong> — runs the full
    detector suite including the four regional detectors needed for composite-image
    analysis. Use Standard only for initial triage where regional coverage is not yet
    required.
  </p>

  <!-- Key features -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Key features to explore
  </h3>
  <ul class="list-disc pl-6 space-y-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>Analyst declaration in PDF exports (name, firm, case reference)</li>
    <li>Methodology versioning — detector versions recorded with every report</li>
    <li>Raw signal scores for all detectors</li>
    <li>Audit trail with SHA-256 hash chain integrity</li>
    <li>C2PA credential verification</li>
  </ul>

  <!-- Tips callout -->
  <div class="bg-obsidian/5 dark:bg-obsidian/50 rounded border border-border-light dark:border-border-dark p-4 mt-4">
    <p class="text-xs font-semibold uppercase tracking-wider text-flint-dark dark:text-flint-light mb-2">
      Tips
    </p>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      <li>
        The trust score is a forensic indicator, not legal proof — frame it accordingly
        in submissions and expert statements.
      </li>
      <li>
        "Inconclusive" means automated analysis could not make a confident determination.
        It does not mean the evidence is compromised.
      </li>
      <li>
        Export both PDF (for submissions) and ZIP (for your file) after every analysis.
      </li>
      <li>
        The
        <a
          href="/help/methodology"
          class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:text-lapis/80 dark:hover:text-lapis dark:text-lapis-light/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian motion-safe:transition-colors"
        >How Analysis Works</a> page documents every detector — cite it in your methodology
        statement when describing the analysis pipeline used.
      </li>
    </ul>
  </div>
</section>

<!-- ─── 6. Insurance and Claims Investigation ─────────────────────────────── -->
<section
  id="insurance"
  aria-labelledby="heading-insurance"
  class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6 mb-6"
>
  <h2
    id="heading-insurance"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3"
  >
    Insurance and Claims Investigation
  </h2>

  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    You investigate potentially fraudulent claims by analysing submitted photographs and
    documents for signs of manipulation, cloning, or AI generation. Copy-move detection
    and JPEG ghost analysis are the two detectors most relevant to composite fraud images —
    both are active in Deep mode.
  </p>

  <!-- Recommended workflow -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended workflow
  </h3>
  <ol class="list-decimal pl-6 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>
      Receive claims photographs from the adjuster or claimant.
    </li>
    <li>
      Import all case images via the
      <strong class="text-text-light dark:text-text-dark">Protect</strong> page for cataloguing.
    </li>
    <li>
      Switch to
      <strong class="text-text-light dark:text-text-dark">Verify → Batch</strong> tab for
      multi-image analysis.
    </li>
    <li>
      Run verification in
      <strong class="text-text-light dark:text-text-dark">Deep</strong> mode across all
      case images.
    </li>
    <li>
      Review copy-move detection and ELA results — these are the most relevant signals
      for manipulation in claims photographs.
    </li>
    <li>
      Check for JPEG ghost anomalies — these indicate regions originating from a source
      saved at a different quality setting, which is characteristic of composite images.
    </li>
    <li>
      Export per-image PDF reports with analyst declaration for the case file.
    </li>
  </ol>

  <!-- Recommended mode -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended investigation mode
  </h3>
  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <strong class="text-text-light dark:text-text-dark">Deep</strong> — covers copy-move
    detection, JPEG ghost, and regional analysis alongside the full forensic pipeline.
  </p>

  <!-- Key features -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Key features to explore
  </h3>
  <ul class="list-disc pl-6 space-y-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>Batch verification for multi-image case processing</li>
    <li>Copy-move detection — flags cloned regions within an image</li>
    <li>JPEG ghost analysis — reveals regions composited from a differently-compressed source</li>
    <li>Error Level Analysis (ELA)</li>
    <li>Analyst declaration in PDF reports</li>
  </ul>

  <!-- Tips callout -->
  <div class="bg-obsidian/5 dark:bg-obsidian/50 rounded border border-border-light dark:border-border-dark p-4 mt-4">
    <p class="text-xs font-semibold uppercase tracking-wider text-flint-dark dark:text-flint-light mb-2">
      Tips
    </p>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      <li>
        Focus on copy-move detection — this catches cloned regions such as duplicated
        damage patterns, a common indicator of manipulated claims photographs.
      </li>
      <li>
        JPEG ghost reveals when a region was saved at a different compression quality —
        common in composite fraud images where genuine and manipulated areas originate
        from different source files.
      </li>
      <li>
        Pay attention to EXIF timestamps — inconsistent dates across a set of claim
        photographs are a significant red flag alongside forensic signal findings.
      </li>
      <li>
        Results should be used as supporting evidence alongside other investigative
        findings, not as sole evidence of fraud.
      </li>
      <li>
        Export reports with your name and case reference before closing the analysis
        session.
      </li>
    </ul>
  </div>
</section>

<!-- ─── 7. Corporate Communications and Security ──────────────────────────── -->
<section
  id="corporate"
  aria-labelledby="heading-corporate"
  class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6 mb-6"
>
  <h2
    id="heading-corporate"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3"
  >
    Corporate Communications and Security
  </h2>

  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    You need to verify whether content purporting to show company executives or official
    communications is authentic, and respond rapidly to potential deepfake incidents.
    Jura Trace provides a three-way classification verdict — Authentic, Inconclusive, or
    Synthetic — giving you an immediate action framework during crisis response.
  </p>

  <!-- Recommended workflow -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended workflow
  </h3>
  <ol class="list-decimal pl-6 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>
      Receive suspicious content — a video call recording, social media post, or forwarded image.
    </li>
    <li>
      Drop the file on the
      <strong class="text-text-light dark:text-text-dark">Verify</strong> page.
    </li>
    <li>
      For video: use
      <strong class="text-text-light dark:text-text-dark">Standard</strong> mode for rapid
      triage (approximately 15 seconds). Switch to
      <strong class="text-text-light dark:text-text-dark">Deep</strong> for full analysis
      once triage indicates a concern.
    </li>
    <li>
      Review the verdict — the three-way classification gives you an immediate basis for
      your incident response decision.
    </li>
    <li>
      Check C2PA credentials — legitimate corporate content from modern production tools
      may carry provenance records you can verify.
    </li>
    <li>
      Export a PDF report for your internal communications team or board briefing.
    </li>
  </ol>

  <!-- Recommended mode -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended investigation mode
  </h3>
  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <strong class="text-text-light dark:text-text-dark">Standard</strong> for rapid triage
    during active incidents.
    <strong class="text-text-light dark:text-text-dark">Deep</strong> for thorough
    investigation before escalation or external disclosure.
  </p>

  <!-- Key features -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Key features to explore
  </h3>
  <ul class="list-disc pl-6 space-y-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>Video deepfake detection with temporal consistency signals (under evaluation for a future release)</li>
    <li>C2PA AI declaration detection — flags content declared as AI-generated by its creator tool</li>
    <li>Three-way verdict classification for clear incident response framing</li>
    <li>Rapid Standard mode for crisis triage</li>
  </ul>

  <!-- Tips callout -->
  <div class="bg-obsidian/5 dark:bg-obsidian/50 rounded border border-border-light dark:border-border-dark p-4 mt-4">
    <p class="text-xs font-semibold uppercase tracking-wider text-flint-dark dark:text-flint-light mb-2">
      Tips
    </p>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      <li>
        "Inconclusive" on a deepfake video means automated analysis could not confirm
        authenticity — do not treat it as clearance to declare the content genuine.
      </li>
      <li>
        For video, the frame timeline shows per-frame deepfake scores. Consistent
        flagging across many frames is more significant than isolated spikes in one
        or two frames.
      </li>
      <li>
        If C2PA credentials declare AI generation — for example, content originating
        from Google Gemini — Jura Trace will flag this clearly in the credentials panel.
      </li>
      <li>
        Consider signing your own genuine corporate communications with C2PA Content
        Credentials to establish a provenance baseline. Legitimate content that carries
        verifiable credentials is harder to dispute.
      </li>
    </ul>
  </div>
</section>

<!-- ─── 8. Human Rights Documentation ────────────────────────────────────── -->
<section
  id="human-rights"
  aria-labelledby="heading-human-rights"
  class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6 mb-6"
>
  <h2
    id="heading-human-rights"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3"
  >
    Human Rights Documentation
  </h2>

  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    You document human rights abuses using photographs, video, and audio from conflict
    zones and crisis environments. Your evidence must be tamper-evident and its analysis
    methodology transparent for international accountability mechanisms. Jura Trace
    processes everything locally — no content leaves your device, and the SHA-256 hash
    chain creates a verifiable record of every action taken on your evidence from the
    moment of intake.
  </p>

  <!-- Recommended workflow -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended workflow
  </h3>
  <ol class="list-decimal pl-6 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>
      Import field evidence into Jura Trace immediately upon receipt.
    </li>
    <li>
      Sign with C2PA provenance to establish a provenance record from the point
      of intake — the timestamp attests to when you first received the evidence.
    </li>
    <li>
      Embed an invisible watermark at
      <strong class="text-text-light dark:text-text-dark">Medium</strong> strength for
      tracking.
    </li>
    <li>
      Run verification in
      <strong class="text-text-light dark:text-text-dark">Deep</strong> mode for full
      detector coverage including the four regional detectors needed for composite
      analysis.
    </li>
    <li>
      Review the full signal breakdown — for evidence destined for legal proceedings,
      every detector's output is part of the record.
    </li>
    <li>
      Export ZIP case files bundling all analysis outputs for submission or secure
      long-term storage.
    </li>
    <li>
      Verify the audit chain integrity via
      <strong class="text-text-light dark:text-text-dark">Settings → Audit Log</strong>
      before submitting evidence to a mechanism or tribunal.
    </li>
  </ol>

  <!-- Recommended mode -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Recommended investigation mode
  </h3>
  <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <strong class="text-text-light dark:text-text-dark">Deep</strong> — full detector
    suite including the four regional detectors (Segmented ELA, Shadow Consistency,
    Colour Temperature, Splice Boundary) needed for composite analysis. Field-captured
    material with chain-of-custody requirements should always run in Deep mode.
  </p>

  <!-- Key features -->
  <h3 class="text-sm font-semibold text-text-light dark:text-text-dark mb-2">
    Key features to explore
  </h3>
  <ul class="list-disc pl-6 space-y-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
    <li>C2PA provenance chain — cryptographic attestation from point of intake</li>
    <li>Audit trail with SHA-256 hash chain — tamper-evident record of every action</li>
    <li>Batch processing for large evidence sets</li>
    <li>Local-first architecture — no content is transmitted externally under any circumstances</li>
    <li>ZIP case export bundling all forensic outputs</li>
    <li>Full offline operation — Ollama and FFmpeg are optional enhancements only</li>
  </ul>

  <!-- Tips callout -->
  <div class="bg-obsidian/5 dark:bg-obsidian/50 rounded border border-border-light dark:border-border-dark p-4 mt-4">
    <p class="text-xs font-semibold uppercase tracking-wider text-flint-dark dark:text-flint-light mb-2">
      Tips
    </p>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      <li>
        All processing runs on your device — no content is transmitted to any external
        service. This is critical when handling evidence from hostile or restricted
        environments.
      </li>
      <li>
        The SHA-256 hash chain audit log creates a tamper-evident record of every
        action taken on your evidence. Verify its integrity in
        <strong class="text-text-light dark:text-text-dark">Settings → Audit Log</strong>
        before any submission.
      </li>
      <li>
        For field deployment without internet connectivity: Jura Trace operates fully
        offline. Ollama (LLM descriptions) and FFmpeg (video/audio metadata) are optional
        enhancements that are not required for core forensic analysis.
      </li>
      <li>
        Sign evidence as early as possible in the chain of custody. The C2PA timestamp
        establishes when you attested to the evidence — earlier is always stronger for
        accountability purposes.
      </li>
      <li>
        See
        <a
          href="/help/methodology"
          class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:text-lapis/80 dark:hover:text-lapis dark:text-lapis-light/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian motion-safe:transition-colors"
        >Help → How Analysis Works</a> for how Jura Trace's methodology aligns with the
        Berkeley Protocol on Digital Open Source Investigations.
      </li>
    </ul>
  </div>
</section>

<!-- Footer navigation -->
<div class="earth-line mt-10 mb-8" role="separator" aria-hidden="true"></div>
<nav aria-label="Help section navigation" class="flex flex-wrap gap-4 text-sm">
  <a
    href="/help"
    class="text-flint-dark dark:text-flint-light hover:text-lapis dark:hover:text-lapis dark:text-lapis-light motion-safe:transition-colors motion-safe:duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
  >
    ← Help home
  </a>
  <a
    href="/help/methodology"
    class="text-flint-dark dark:text-flint-light hover:text-lapis dark:hover:text-lapis dark:text-lapis-light motion-safe:transition-colors motion-safe:duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
  >
    How Analysis Works →
  </a>
</nav>
