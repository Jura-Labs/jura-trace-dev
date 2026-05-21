<script lang="ts">
  // No reactive state required — this is a static compliance reference page.
  // All content is authored directly; no Tauri IPC calls are made.
</script>

<!--
  Help — IT and Compliance
  ========================
  In-app version of the Information Security Summary and DPIA guidance,
  written for IT administrators, data protection officers, and procurement
  teams evaluating Jura Trace for institutional deployment.

  Audience: IT administrators, DPOs, procurement teams.
  Style: Sanctuary theme, Georgia serif headings, Tailwind utility classes.
  British spelling throughout. No emojis.
  Section IDs support deep linking from the sidebar (e.g. #dpia).
-->

<article aria-labelledby="compliance-heading">

  <!-- ── Page header ──────────────────────────────────────────────────── -->
  <header class="mb-8">
    <h1
      id="compliance-heading"
      class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3"
    >
      IT and Compliance
    </h1>
    <p class="text-base text-flint-dark dark:text-flint-light leading-relaxed max-w-2xl">
      Resources for IT administrators, data protection officers, and procurement
      teams evaluating Jura Trace for institutional deployment.
    </p>
  </header>

  <!-- Earth-line divider -->
  <div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

  <!-- ── Table of contents ─────────────────────────────────────────────── -->
  <nav aria-label="Page contents" class="mb-10">
    <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-3">
      On this page
    </p>
    <ol class="space-y-1 text-sm">
      <li>
        <a href="#security"
           class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
          1. Information Security Summary
        </a>
      </li>
      <li>
        <a href="#dpia"
           class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
          2. Data Protection Impact Assessment
        </a>
      </li>
      <li>
        <a href="#regulatory"
           class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
          3. Regulatory Compliance Overview
        </a>
      </li>
      <li>
        <a href="#audit-status"
           class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
          4. Security Audit Status
        </a>
      </li>
    </ol>
  </nav>

  <!-- ── Section 1: Information Security Summary ───────────────────────── -->
  <section id="security" aria-labelledby="security-heading" class="mb-12">
    <h2
      id="security-heading"
      class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
    >
      Information Security Summary
    </h2>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      This section summarises the key security properties of Jura Trace for institutional
      procurement review. A detailed Information Security Summary document (v1.0, 25 March 2026)
      is available for download from your Jura Trace account manager or at
      <a
        href="https://juralabs.org/compliance"
        class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
      >juralabs.org/compliance</a>.
    </p>

    <!-- Architecture overview -->
    <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2 mt-6">
      Architecture overview
    </h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      Jura Trace is a native desktop application built on the Tauri v2 framework (Rust backend,
      SvelteKit frontend). It runs on macOS 13+, Windows 10+, and Ubuntu 22.04+. All processing
      occurs on-device. The Analysis Engine binds to an OS-assigned ephemeral port on the
      <code class="font-mono text-xs bg-gray-100 dark:bg-graphite-light px-1 py-0.5 rounded">127.0.0.1</code>
      loopback interface only, so it is not reachable from other machines on the network and the
      port is not fixed across launches.
    </p>
    <div
      class="rounded-lg border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40 p-4 mb-4 font-mono text-xs text-flint-dark dark:text-flint-light leading-relaxed"
      role="img"
      aria-label="Architecture diagram: User Device contains the Tauri application, local SQLite database, Analysis Engine on a loopback ephemeral port assigned at launch, and optional Ollama on localhost port 11434. No external network traffic for core functionality."
    >
      <p>[User Device]</p>
      <p class="ml-4">├── Tauri App (Rust + SvelteKit)</p>
      <p class="ml-8">├── Local SQLite Database</p>
      <p class="ml-8">└── Analysis Engine (127.0.0.1, OS-assigned ephemeral port)</p>
      <p class="ml-12">└── Ollama LLM Runtime (127.0.0.1:11434, optional)</p>
      <p class="ml-4">└── [No external network traffic for core functionality]</p>
    </div>

    <!-- Data processing summary -->
    <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2 mt-6">
      Data processing summary
    </h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      When a file is submitted for analysis, it is read from disk by the Tauri Rust process.
      Forensic analysis is performed locally; the file is passed over the loopback interface
      to the Analysis Engine. A perceptual fingerprint is computed and stored in the local
      database alongside analysis results. The original file is never copied, moved, or
      transmitted. <strong class="font-semibold text-text-light dark:text-text-dark">Original file contents are not stored in the database</strong>: only metadata,
      hashes, and analysis results are persisted.
    </p>

    <!-- Data at rest -->
    <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2 mt-6">
      Data at rest
    </h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      All asset metadata, fingerprints, verification results, and audit logs are stored in a
      single SQLite database file under the user's application data directory. Default locations:
    </p>
    <div class="overflow-x-auto mb-4">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="text-left py-2 pr-4 font-semibold text-text-light dark:text-text-dark">Platform</th>
            <th class="text-left py-2 font-semibold text-text-light dark:text-text-dark">Default path</th>
          </tr>
        </thead>
        <tbody class="text-flint-dark dark:text-flint-light">
          <tr class="border-b border-border-light dark:border-border-dark">
            <td class="py-2 pr-4">macOS</td>
            <td class="py-2 font-mono text-xs">~/Library/Application Support/Jura Trace/jura_trace.db</td>
          </tr>
          <tr class="border-b border-border-light dark:border-border-dark">
            <td class="py-2 pr-4">Windows</td>
            <td class="py-2 font-mono text-xs">%APPDATA%\Jura Trace\jura_trace.db</td>
          </tr>
          <tr>
            <td class="py-2 pr-4">Linux</td>
            <td class="py-2 font-mono text-xs">~/.local/share/jura-trace/jura_trace.db</td>
          </tr>
        </tbody>
      </table>
    </div>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      The database is not encrypted at the application level. The data-at-rest control
      is OS-level full-disk encryption: FileVault on macOS (default on macOS Catalina
      and later), BitLocker on Windows (default on Windows 11 24H2 and later), and LUKS
      or equivalent on Linux. We recommend deployments verify FDE is enabled.
    </p>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      The database location can be overridden via the <code class="font-mono text-xs bg-gray-100 dark:bg-graphite-light px-1 py-0.5 rounded">JURA_DB_PATH</code> environment variable
      or the Change Location button in Settings, allowing placement on a network share,
      managed drive, or encrypted volume.
    </p>

    <!-- Data in transit -->
    <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2 mt-6">
      Data in transit
    </h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      All communication between the desktop application and the Analysis Engine occurs over
      the loopback interface and never leaves the device. There is no telemetry, no usage
      analytics, and no crash reporting. Two optional features involve external network
      communication, both requiring explicit user action:
    </p>
    <ul class="space-y-2 text-sm text-flint-dark dark:text-flint-light mb-3 ml-4 list-disc">
      <li>
        <strong class="font-semibold text-text-light dark:text-text-dark">Auto-update check:</strong>
        A single HTTPS GET to GitHub Releases containing only the current version number and
        platform identifier. No user data or file content is transmitted. Can be disabled
        for air-gapped deployments.
      </li>
      <li>
        <strong class="font-semibold text-text-light dark:text-text-dark">Reverse image search</strong>
        (BYOK, off by default): if you provide your own Google Vision API key, you can
        send a perceptual hash (and, on per-call confirmation, a thumbnail-sized crop)
        to your Google Vision endpoint. Off by default. Requires explicit per-analysis
        confirmation. Additional provider integrations are under evaluation for future releases.
      </li>
    </ul>

    <!-- Authentication -->
    <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2 mt-6">
      Authentication and access control
    </h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      The Analysis Engine requires an API key on every request via the
      <code class="font-mono text-xs bg-gray-100 dark:bg-graphite-light px-1 py-0.5 rounded">X-Jura-API-Key</code> header,
      preventing other processes on the same machine from calling the Analysis Engine without
      authorisation. In standard single-user installations, the key is generated automatically
      at startup and requires no configuration.
    </p>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      Jura Trace has no user account system, no login screen, and no cloud authentication.
      Application access is controlled entirely by OS-level user account permissions. File
      system access is restricted to paths explicitly chosen by the user via the native OS
      file picker, enforced at the Tauri capability level.
    </p>
  </section>

  <div class="earth-line mb-10" role="separator" aria-hidden="true"></div>

  <!-- ── Section 2: DPIA ───────────────────────────────────────────────── -->
  <section id="dpia" aria-labelledby="dpia-heading" class="mb-12">
    <h2
      id="dpia-heading"
      class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
    >
      Data Protection Impact Assessment
    </h2>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      A Data Protection Impact Assessment (DPIA) is required under UK GDPR Article 35 when
      processing is likely to result in a high risk to the rights and freedoms of individuals,
      for example, when processing special category data at scale, using automated decision
      making with significant effects, or systematically monitoring public areas.
    </p>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      A pre-filled DPIA template following ICO guidance is available for download from
      <a
        href="https://juralabs.org/compliance"
        class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
      >juralabs.org/compliance</a>. The template addresses Jura Trace's local-first
      architecture and covers the residual risks identified below.
    </p>

    <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3 mt-6">
      Key findings from the pre-filled template
    </h3>

    <!-- Key points list -->
    <ul class="space-y-3 text-sm text-flint-dark dark:text-flint-light mb-6">
      <li class="flex gap-3">
        <span
          class="mt-0.5 flex-none w-2 h-2 rounded-full bg-malachite"
          aria-hidden="true"
        ></span>
        <span>
          <strong class="font-semibold text-text-light dark:text-text-dark">Local-first architecture significantly reduces data protection risk.</strong>
          No personal data is transmitted to Juralabs or any third party during core operation.
          Juralabs holds no record of who is using the software or what content is being
          processed.
        </span>
      </li>
      <li class="flex gap-3">
        <span
          class="mt-0.5 flex-none w-2 h-2 rounded-full bg-malachite"
          aria-hidden="true"
        ></span>
        <span>
          <strong class="font-semibold text-text-light dark:text-text-dark">No external data transfers.</strong>
          All forensic analysis, C2PA signing, watermarking, and report generation occur on-device.
          No content crosses the network boundary during normal operation.
        </span>
      </li>
      <li class="flex gap-3">
        <span
          class="mt-0.5 flex-none w-2 h-2 rounded-full bg-malachite"
          aria-hidden="true"
        ></span>
        <span>
          <strong class="font-semibold text-text-light dark:text-text-dark">File contents are not stored in the database.</strong>
          Only metadata, perceptual hashes, and analysis results are persisted. The original
          image, video, or audio file is read from disk and analysed in memory; it is not
          copied or retained.
        </span>
      </li>
      <li class="flex gap-3">
        <span
          class="mt-0.5 flex-none w-2 h-2 rounded-full bg-amber"
          aria-hidden="true"
        ></span>
        <span>
          <strong class="font-semibold text-text-light dark:text-text-dark">Residual risk: unencrypted SQLite database.</strong>
          The database is not encrypted at the application level. The mitigation is
          OS-level full-disk encryption (FileVault, BitLocker, or LUKS), which is the
          default on modern macOS and Windows. Deployments should verify FDE is enabled.
        </span>
      </li>
      <li class="flex gap-3">
        <span
          class="mt-0.5 flex-none w-2 h-2 rounded-full bg-lapis dark:bg-lapis-light"
          aria-hidden="true"
        ></span>
        <span>
          <strong class="font-semibold text-text-light dark:text-text-dark">Data controller responsibility.</strong>
          Where the content processed through Jura Trace includes images of identifiable
          individuals, the processing organisation is the data controller for that activity.
          Jura Trace acts as a local processing tool; Juralabs is not a data processor.
        </span>
      </li>
    </ul>

    <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3 mt-6">
      When is a DPIA required?
    </h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      Most use cases for Jura Trace (including archival cataloguing, press photo verification,
      and content provenance tagging) do not require a formal DPIA. A DPIA is more likely
      to be required when:
    </p>
    <ul class="space-y-1 text-sm text-flint-dark dark:text-flint-light ml-4 list-disc mb-3">
      <li>Processing images of identifiable individuals at scale (e.g. systematic monitoring of public content).</li>
      <li>Using verification outputs as part of automated decision-making with significant effects on individuals.</li>
      <li>Deploying the tool across a large organisation with centralised audit log storage.</li>
    </ul>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      The pre-filled template provides a structured starting point that your data protection
      officer can adapt to your institution's specific context.
    </p>
  </section>

  <div class="earth-line mb-10" role="separator" aria-hidden="true"></div>

  <!-- ── Section 3: Regulatory Compliance Overview ────────────────────── -->
  <section id="regulatory" aria-labelledby="regulatory-heading" class="mb-12">
    <h2
      id="regulatory-heading"
      class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
    >
      Regulatory Compliance Overview
    </h2>

    <!-- UK GDPR / DPA 2018 -->
    <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2 mt-6">
      UK GDPR and Data Protection Act 2018
    </h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      Jura Trace's local-first architecture substantially reduces the data protection risk
      profile for institutional deployments. No personal data is transmitted to Juralabs or
      any third party as part of core functionality. The application does not create user
      accounts and Juralabs holds no record of who is using the software or what content is
      being processed. Local processing does not trigger the international transfer restrictions
      of UK GDPR Chapter V.
    </p>

    <!-- EU AI Act -->
    <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2 mt-6">
      EU AI Act
    </h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      Jura Trace <em>detects</em> AI-generated content but does not itself generate AI content.
      The application is not subject to the provider obligations under the EU AI Act applicable
      to general-purpose AI models or high-risk AI systems.
    </p>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      The optional Ollama integration (LLaVA and Qwen2.5 models) runs entirely on-device.
      Juralabs does not operate these models as a service. Article 50 transparency obligations
      (disclosure that content is AI-generated) apply to organisations using these features
      to generate content descriptions, not to Jura Trace as a tool.
    </p>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      Organisations using verification reports in automated decision-making should assess their
      obligations under Article 14 (human oversight). Jura Trace supports human oversight
      through the visual inspection checklist and signal agreement dashboard on the Verify page.
      An EU AI Act Article 50 audit-log export is planned for the v1.0.1 release (~early August 2026, as Article 50 binds 2 August 2026).
    </p>

    <!-- Online Safety Act 2023 -->
    <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2 mt-6">
      Online Safety Act 2023
    </h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      The Online Safety Act 2023 places duties on regulated services to prevent harmful content.
      Jura Trace is a local desktop tool, not a regulated service. However, organisations subject
      to the Act (including news publishers, online platforms, and content archives) may find
      Jura Trace's forensic verification capabilities useful as part of their content moderation
      and authenticity verification workflows, helping to demonstrate that appropriate steps
      have been taken to assess the provenance of content before publication.
    </p>

    <!-- AGPL Licence -->
    <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2 mt-6">
      Licence terms: AGPL-3.0-or-later
    </h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      Jura Trace is licenced under the GNU Affero General Public License, version 3 or later.
      You may use, study, modify, and redistribute the software for any purpose (including
      commercial use), provided you comply with the AGPL's terms. The two requirements that
      typically matter are: source disclosure if you operate a modified version as a hosted
      network service, and copyleft on any larger product that incorporates Jura Trace.
    </p>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      A separate <strong class="font-semibold text-text-light dark:text-text-dark">commercial licence</strong>
      is available for organisations whose use case cannot operate under the AGPL, for
      example, integration into closed-source products, internal modified deployments without
      source release, or cases requiring contractual indemnification beyond what the AGPL
      provides. Contact
      <a
        href="mailto:commercial@juralabs.org"
        class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
      >commercial@juralabs.org</a> for the commercial-licence process.
    </p>
  </section>

  <div class="earth-line mb-10" role="separator" aria-hidden="true"></div>

  <!-- ── Section 4: Security Audit Status ─────────────────────────────── -->
  <section id="audit-status" aria-labelledby="audit-heading" class="mb-12">
    <h2
      id="audit-heading"
      class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
    >
      Security Audit Status
    </h2>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      A full OWASP-aligned security penetration test was conducted on 25 March 2026 (Sprint 19),
      covering the Tauri IPC boundary, Analysis Engine API, Content Security Policy, capability
      configuration, data at rest, and dependency surfaces.
    </p>

    <!-- Summary table -->
    <div class="overflow-x-auto mb-6">
      <table class="w-full text-sm border-collapse">
        <caption class="sr-only">Security audit findings by severity, Sprint 19</caption>
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="text-left py-2 pr-4 font-semibold text-text-light dark:text-text-dark">Severity</th>
            <th class="text-left py-2 pr-4 font-semibold text-text-light dark:text-text-dark">Findings</th>
            <th class="text-left py-2 pr-4 font-semibold text-text-light dark:text-text-dark">Open</th>
            <th class="text-left py-2 font-semibold text-text-light dark:text-text-dark">Fixed</th>
          </tr>
        </thead>
        <tbody class="text-flint-dark dark:text-flint-light">
          <tr class="border-b border-border-light dark:border-border-dark">
            <td class="py-2 pr-4">Critical</td>
            <td class="py-2 pr-4">0</td>
            <td class="py-2 pr-4">0</td>
            <td class="py-2">—</td>
          </tr>
          <tr class="border-b border-border-light dark:border-border-dark">
            <td class="py-2 pr-4">High</td>
            <td class="py-2 pr-4">4</td>
            <td class="py-2 pr-4">0</td>
            <td class="py-2">4</td>
          </tr>
          <tr class="border-b border-border-light dark:border-border-dark">
            <td class="py-2 pr-4">Medium</td>
            <td class="py-2 pr-4">5</td>
            <td class="py-2 pr-4">0</td>
            <td class="py-2">5</td>
          </tr>
          <tr class="border-b border-border-light dark:border-border-dark">
            <td class="py-2 pr-4">Low</td>
            <td class="py-2 pr-4">6</td>
            <td class="py-2 pr-4">0</td>
            <td class="py-2">6</td>
          </tr>
          <tr>
            <td class="py-2 pr-4">Informational</td>
            <td class="py-2 pr-4">4</td>
            <td class="py-2 pr-4">0</td>
            <td class="py-2">4</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Status callout -->
    <div
      class="rounded-lg border border-malachite/30 bg-malachite/5 dark:bg-malachite/10 p-4 mb-6"
      role="status"
      aria-label="Security audit status"
    >
      <p class="text-sm font-semibold text-malachite-dark dark:text-malachite-light">
        All security audit findings have been remediated. There are 0 open items as of v0.9.0-rc.1.
      </p>
    </div>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      The Sprint 19 audit confirmed that all findings from the Sprint 14 audit (3 critical, 6 high)
      remain remediated with no regression. The full Sprint 19 audit report is available on request
      via <a
        href="mailto:commercial@juralabs.org"
        class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
      >commercial@juralabs.org</a>.
    </p>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      For detector methodology documentation, including how each forensic signal is computed and
      weighted, see the
      <a
        href="/help/methodology"
        class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
      >How Analysis Works</a> page.
    </p>
  </section>

</article>
