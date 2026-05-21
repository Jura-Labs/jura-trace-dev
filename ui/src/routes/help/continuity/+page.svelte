<script lang="ts">
  // No reactive state required — static policy page.
</script>

<!--
  Help — Continuity Promise
  =========================
  TRIED Pillar 5: "responsible retirement: removal from public access when
  maintenance ceases"
  Addresses institutional procurement concern: "what if you go bust?"

  Sanctuary theme. British spelling. No emojis. WCAG 2.2 AA.
-->

<article aria-labelledby="continuity-heading">

  <!-- ── Page heading ──────────────────────────────────────────────────── -->
  <header class="mb-10">
    <h1
      id="continuity-heading"
      class="font-heading text-3xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      Continuity Promise
    </h1>
    <p class="text-base text-text-light dark:text-quartz leading-relaxed max-w-2xl">
      Jura Trace is built by Juralabs Community Interest Company (CIC), a UK
      social enterprise. We understand that institutions and organisations need
      confidence that the tools they depend on will remain available, and that
      their data will remain accessible regardless of what happens to the company
      behind the software.
    </p>
    <div class="earth-line mt-6" aria-hidden="true"></div>
  </header>

  <!-- ── Our commitments ──────────────────────────────────────────────── -->
  <section aria-labelledby="commitments-heading" class="mb-12">
    <h2
      id="commitments-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-6 leading-tight tracking-heading"
    >
      Our Commitments
    </h2>

    <div class="space-y-6">

      <!-- 1. Open source release -->
      <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6">
        <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">1. Source Code Release</h3>
        <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
          Jura Trace's source code is published under AGPL-3.0-or-later today —
          the codebase is already public and forkable. The CIC's continuity
          commitment is that if Jura Labs CIC ceases operations, the source will
          remain available under AGPL-3.0-or-later in perpetuity (this is a
          consequence of the AGPL licence terms and the CIC's asset-locked
          status — it cannot be retracted by any future change of ownership).
        </p>
      </div>

      <!-- 2. Model weights -->
      <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6">
        <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">2. Model Weight Preservation</h3>
        <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
          All trained model weights (the GBM deepfake classifier, UnivFD linear
          probe, and any future classifiers) will be deposited in a public archive
          such as HuggingFace Hub or Zenodo. These weights are essential for
          continued operation of the detection pipeline and will be freely
          available under terms compatible with the AGPL-3.0-or-later licence.
        </p>
      </div>

      <!-- 3. Data portability -->
      <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6">
        <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">3. Data Portability</h3>
        <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
          Jura Trace stores all data in a local SQLite database on your device.
          SQLite is an open, well-documented format readable by hundreds of tools.
          Your asset records, verification results, fingerprints, annotations, and
          audit logs are always accessible — with or without the Jura Trace
          application. No data is stored on our servers.
        </p>
      </div>

      <!-- 4. Methodology documentation -->
      <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6">
        <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">4. Methodology Documentation</h3>
        <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
          The C2PA signing methodology, trust scoring formula, detector
          descriptions, model cards, and all technical documentation will be
          archived alongside the source code. This ensures that future users and
          researchers can understand how results were produced and assess their
          validity.
        </p>
      </div>

      <!-- 5. Notice period -->
      <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6">
        <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">5. Advance Notice</h3>
        <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
          A minimum 90-day notice period will be provided before any service
          discontinuation, including cessation of model updates, auto-updater
          shutdown, or end of support. This gives institutions time to plan
          transitions and export any needed data.
        </p>
      </div>

    </div>
  </section>

  <div class="earth-line mb-12" aria-hidden="true"></div>

  <!-- ── Governance ───────────────────────────────────────────────────── -->
  <section aria-labelledby="governance-heading" class="mb-12">
    <h2
      id="governance-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-6 leading-tight tracking-heading"
    >
      Governance Structure
    </h2>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      Juralabs is registered as a Community Interest Company (CIC) in the United
      Kingdom. The CIC structure provides additional safeguards:
    </p>

    <ul class="list-disc pl-5 space-y-2 text-sm text-text-light dark:text-quartz leading-relaxed">
      <li><strong>Asset lock:</strong> CIC regulations include an asset lock that prevents the company's assets from being distributed to directors or shareholders for private benefit. If the company is dissolved, assets must be transferred to another CIC, charity, or community body with similar objectives.</li>
      <li><strong>Community benefit purpose:</strong> The company exists to benefit the community. Its articles of association require that it operates for the benefit of journalists, fact-checkers, cultural institutions, and civil society organisations working on content authenticity.</li>
      <li><strong>Regulator oversight:</strong> CICs are regulated by the Office of the Regulator of Community Interest Companies, which reviews annual community interest reports.</li>
    </ul>
  </section>

  <div class="earth-line mb-12" aria-hidden="true"></div>

  <!-- ── Legal note ───────────────────────────────────────────────────── -->
  <section aria-labelledby="legal-heading" class="mb-12">
    <h2
      id="legal-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-6 leading-tight tracking-heading"
    >
      Important Note
    </h2>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
      This page describes Juralabs' intentions and commitments. It is not a
      legally binding contract. The commitments above reflect the values and
      governance structure of the organisation. For institutions requiring formal
      contractual guarantees, please
      <a href="https://juralabs.org/contact" target="_blank" rel="noopener noreferrer"
         class="text-lapis dark:text-lapis-light underline hover:no-underline">contact us</a>
      to discuss institutional licensing arrangements.
    </p>
  </section>

</article>
