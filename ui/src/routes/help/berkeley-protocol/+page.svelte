<script lang="ts">
  // No reactive state required — this is a static compliance reference page.
  // All content is authored directly; no Tauri IPC calls are made.
</script>

<!--
  Help — International Evidence Standards (Berkeley Protocol alignment)
  ======================================================================
  Maps Jura Trace capabilities to the Berkeley Protocol on Digital Open
  Source Investigations (2022). Written for human rights documentation
  coordinators who need to brief legal teams on the tool's evidentiary
  design.

  Audience: documentation coordinators, legal teams, compliance officers.
  Style: Sanctuary theme, Georgia serif headings, Tailwind utility classes.
  British spelling throughout. No emojis.
-->

<!-- ── Page header ──────────────────────────────────────────────────── -->
<header class="mb-8">
  <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
    International Evidence Standards
  </h1>
  <p class="text-base text-flint dark:text-flint-light leading-relaxed max-w-2xl">
    Jura Trace's forensic analysis pipeline and evidence management features align with
    established international standards for digital open source investigations, including
    the Berkeley Protocol on Digital Open Source Investigations (2022). This page maps
    those features to the Protocol's requirements to help legal teams and compliance
    officers assess the tool's suitability for human rights documentation work.
  </p>
</header>

<!-- Earth-line divider -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<!-- ── Table of contents ─────────────────────────────────────────────── -->
<nav aria-label="Page contents" class="mb-10">
  <p class="text-xs font-semibold uppercase tracking-widest text-flint dark:text-flint-light mb-3">
    On this page
  </p>
  <ol class="space-y-1 text-sm">
    <li>
      <a href="#what-is-the-protocol"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        1. What is the Berkeley Protocol?
      </a>
    </li>
    <li>
      <a href="#alignment-overview"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        2. Alignment Overview
      </a>
    </li>
    <li>
      <a href="#chain-of-custody"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        3. Chain of Custody (§5.3)
      </a>
    </li>
    <li>
      <a href="#content-integrity"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        4. Content Integrity (§5.4)
      </a>
    </li>
    <li>
      <a href="#methodology-documentation"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        5. Methodology Documentation (§6.2)
      </a>
    </li>
    <li>
      <a href="#analyst-accountability"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        6. Analyst Accountability (§6.3)
      </a>
    </li>
    <li>
      <a href="#privacy-and-security"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        7. Privacy and Security (§4)
      </a>
    </li>
    <li>
      <a href="#preservation-of-evidence"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        8. Preservation of Original Evidence (§5.2)
      </a>
    </li>
    <li>
      <a href="#recommended-workflow"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        9. Recommended Workflow for Human Rights Documentation
      </a>
    </li>
    <li>
      <a href="#disclaimer"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        10. Important Disclaimer
      </a>
    </li>
  </ol>
</nav>

<!-- ══════════════════════════════════════════════════════════════════
     1. What is the Berkeley Protocol?
     ══════════════════════════════════════════════════════════════════ -->
<section id="what-is-the-protocol" class="mb-12" aria-labelledby="what-is-the-protocol-heading">
  <h2
    id="what-is-the-protocol-heading"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
  >
    1. What is the Berkeley Protocol?
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The Berkeley Protocol on Digital Open Source Investigations was published jointly by the
    United Nations Human Rights Office and the Human Rights Center at UC Berkeley School of
    Law in 2022. It provides a comprehensive methodological framework for collecting,
    preserving, and analysing digital information for accountability purposes. The Protocol
    drew on the expertise of international lawyers, investigators, technologists, and
    civil society practitioners to establish a baseline of good practice applicable across
    diverse institutional contexts.
  </p>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The Protocol is used by international criminal tribunals, United Nations investigative
    mechanisms, national human rights commissions, and civil society organisations
    worldwide. It addresses the full lifecycle of digital evidence: from collection and
    preservation through analysis, documentation, and submission to legal proceedings.
    Its standards apply whether the evidence is a photograph, a video, a document, or
    a social media post.
  </p>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Four areas of the Protocol are directly relevant to how Jura Trace is designed:
    chain of custody (§5.3), content integrity (§5.4), methodology documentation (§6.2),
    and analyst accountability (§6.3). The Protocol's provisions on privacy, security
    (§4), and evidence preservation (§5.2) are also material for organisations working
    in sensitive contexts.
  </p>

  <!-- Callout: external resource -->
  <div
    class="rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite p-4"
    role="note"
  >
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Full text of the Protocol.</strong>
      The Berkeley Protocol is freely available at the UN Human Rights Office website and
      the Human Rights Center at UC Berkeley School of Law. Jura Trace recommends that
      every organisation using the tool for human rights documentation read the Protocol
      directly and apply their own institutional compliance assessment.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     2. Alignment Overview
     ══════════════════════════════════════════════════════════════════ -->
<section id="alignment-overview" class="mb-12" aria-labelledby="alignment-overview-heading">
  <h2
    id="alignment-overview-heading"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
  >
    2. Alignment Overview
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
    The table below gives a summary of how Jura Trace maps to the Protocol's key
    requirements. Detailed explanations for each area follow in subsequent sections.
  </p>

  <div class="overflow-x-auto mb-6">
    <table class="w-full text-sm border-collapse" aria-label="Berkeley Protocol alignment summary">
      <thead>
        <tr class="border-b border-border-light dark:border-border-dark">
          <th class="text-left py-2 pr-6 font-semibold text-text-light dark:text-text-dark">Protocol requirement</th>
          <th class="text-left py-2 pr-6 font-semibold text-text-light dark:text-text-dark">Section</th>
          <th class="text-left py-2 font-semibold text-text-light dark:text-text-dark">Jura Trace capability</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-border-light dark:divide-border-dark">
        <tr>
          <td class="py-2.5 pr-6 text-text-light dark:text-text-dark font-medium">Chain of custody</td>
          <td class="py-2.5 pr-6 tabular-nums text-flint dark:text-flint-light">§5.3</td>
          <td class="py-2.5 text-flint dark:text-flint-light">SHA-256 hash chain audit log with tamper-evident integrity verification</td>
        </tr>
        <tr>
          <td class="py-2.5 pr-6 text-text-light dark:text-text-dark font-medium">Content integrity</td>
          <td class="py-2.5 pr-6 tabular-nums text-flint dark:text-flint-light">§5.4</td>
          <td class="py-2.5 text-flint dark:text-flint-light">C2PA Content Credentials, perceptual hashing, invisible watermarking</td>
        </tr>
        <tr>
          <td class="py-2.5 pr-6 text-text-light dark:text-text-dark font-medium">Methodology documentation</td>
          <td class="py-2.5 pr-6 tabular-nums text-flint dark:text-flint-light">§6.2</td>
          <td class="py-2.5 text-flint dark:text-flint-light">21 forensic detectors documented in-app; trust score formula fully disclosed</td>
        </tr>
        <tr>
          <td class="py-2.5 pr-6 text-text-light dark:text-text-dark font-medium">Analyst accountability</td>
          <td class="py-2.5 pr-6 tabular-nums text-flint dark:text-flint-light">§6.3</td>
          <td class="py-2.5 text-flint dark:text-flint-light">Analyst declaration in PDF export; deterministic, reproducible pipeline</td>
        </tr>
        <tr>
          <td class="py-2.5 pr-6 text-text-light dark:text-text-dark font-medium">Privacy and security</td>
          <td class="py-2.5 pr-6 tabular-nums text-flint dark:text-flint-light">§4</td>
          <td class="py-2.5 text-flint dark:text-flint-light">Local-first architecture; no cloud dependency; no telemetry</td>
        </tr>
        <tr>
          <td class="py-2.5 pr-6 text-text-light dark:text-text-dark font-medium">Preservation of original evidence</td>
          <td class="py-2.5 pr-6 tabular-nums text-flint dark:text-flint-light">§5.2</td>
          <td class="py-2.5 text-flint dark:text-flint-light">Original files never modified; C2PA signing creates a new file; metadata preserved</td>
        </tr>
      </tbody>
    </table>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     3. Chain of Custody
     ══════════════════════════════════════════════════════════════════ -->
<section id="chain-of-custody" class="mb-12" aria-labelledby="chain-of-custody-heading">
  <h2
    id="chain-of-custody-heading"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
  >
    3. Chain of Custody <span class="text-base font-normal text-flint dark:text-flint-light ml-2">Berkeley Protocol §5.3</span>
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The Berkeley Protocol requires that organisations maintain a reliable record of who
    accessed, processed, or transferred evidence — and when. Every action must be
    logged so that the integrity of the evidentiary chain can be demonstrated to a
    tribunal or reviewing body.
  </p>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    Jura Trace addresses this requirement through a tamper-evident audit log stored in
    the local SQLite database. Every consequential action — file import, C2PA signing,
    verification, watermark embedding, and export — writes an entry to the log. Each
    entry records:
  </p>

  <ul class="space-y-3 mb-6">
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span><strong class="text-text-light dark:text-text-dark">Action type</strong> — what operation was performed on the file.</span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span><strong class="text-text-light dark:text-text-dark">Timestamp</strong> — date and time of the operation, recorded in UTC.</span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span><strong class="text-text-light dark:text-text-dark">Operator context</strong> — the analyst or institution identifier associated with the session.</span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span><strong class="text-text-light dark:text-text-dark">SHA-256 hash chain link</strong> — each entry includes a hash that incorporates the previous entry's hash, forming an unbroken chain. Altering any past entry invalidates all subsequent hashes.</span>
    </li>
  </ul>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    You can verify the integrity of the entire audit chain at any time using the
    <strong class="text-text-light dark:text-text-dark">Verify Audit Chain</strong> function
    in Settings. If the chain is intact, every entry is confirmed as unmodified. If any
    entry has been altered or deleted, the check reports the point of failure. This gives
    you a bedrock of verifiable provenance that you can present alongside your evidence.
  </p>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    PDF trust reports generated by Jura Trace include the analysis timestamp, the
    pipeline version, and — on the Professional tier — an analyst declaration. This
    creates a human-readable chain of custody record suitable for attachment to legal
    submissions.
  </p>

  <!-- Callout: audit integrity command -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Before any legal submission.</strong>
      Run the audit chain integrity check from Settings &#8594; Audit Log &#8594; Verify
      Chain Integrity. Keep a screenshot or printed record of the result alongside your
      evidence bundle.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     4. Content Integrity
     ══════════════════════════════════════════════════════════════════ -->
<section id="content-integrity" class="mb-12" aria-labelledby="content-integrity-heading">
  <h2
    id="content-integrity-heading"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
  >
    4. Content Integrity <span class="text-base font-normal text-flint dark:text-flint-light ml-2">Berkeley Protocol §5.4</span>
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The Protocol requires that organisations be able to demonstrate a file has not been
    altered since it was first collected. This means creating an integrity record at the
    moment of acquisition and verifying it remains consistent through the evidence
    lifecycle.
  </p>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    Jura Trace provides three complementary mechanisms for content integrity assurance.
    Together these layers form a strata of cryptographic and perceptual evidence that
    can withstand scrutiny from legal and technical reviewers.
  </p>

  <div class="space-y-5 mb-6">

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        C2PA Content Credentials
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        The Coalition for Content Provenance and Authenticity (C2PA) standard embeds a
        cryptographically signed manifest directly into the file. The manifest records the
        signing identity, the time of signing, and a hash of the content at the moment of
        signing. Any subsequent modification to the file — even a single byte — invalidates
        the manifest hash, which Jura Trace detects and reports during verification. C2PA
        is an open, ISO-aligned standard used by camera manufacturers, news agencies, and
        content platforms worldwide.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Perceptual hashing
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        In addition to cryptographic hashing, Jura Trace generates three perceptual hash
        values for each image or video frame: aHash (average hash), dHash (difference
        hash), and pHash (discrete cosine transform hash). Perceptual hashes remain
        stable under minor format changes — such as recompression or metadata stripping
        — that would break a cryptographic hash. They enable near-duplicate detection: if
        a manipulated version of a file is submitted, the perceptual distance from the
        registered original can be computed and reported.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Invisible watermarking
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Jura Trace can embed an invisible frequency-domain watermark (DWT-DCT-SVD method)
        carrying a 128-bit UUID and your institution's identifier. The watermark survives
        JPEG recompression at quality 70 and above, moderate resizing, and up to 30% crop.
        If a watermarked file is later submitted for verification — even by a third party —
        the watermark extraction reveals the originating institution and the asset identifier.
        This provides a further layer of provenance evidence independent of file metadata.
      </p>
    </div>

  </div>

  <!-- Callout: combined use -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Best practice.</strong>
      For the strongest integrity record, use all three mechanisms together: sign with C2PA
      at acquisition, embed a watermark, and register the file's fingerprint in the local
      database. Each mechanism works independently — if one is stripped, the others remain.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     5. Methodology Documentation
     ══════════════════════════════════════════════════════════════════ -->
<section id="methodology-documentation" class="mb-12" aria-labelledby="methodology-documentation-heading">
  <h2
    id="methodology-documentation-heading"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
  >
    5. Methodology Documentation <span class="text-base font-normal text-flint dark:text-flint-light ml-2">Berkeley Protocol §6.2</span>
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The Protocol requires that analysts document the tools and methods used in their
    investigation so that findings can be independently assessed, challenged, and reproduced.
    A finding that cannot be explained or replicated carries limited evidentiary weight.
  </p>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    Jura Trace is designed with methodological transparency as a first principle. The
    analysis pipeline is fully documented within the application.
  </p>

  <ul class="space-y-3 mb-6">
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Detector reference.</strong>
        The
        <a href="/help/methodology"
           class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:text-lapis/80 dark:hover:text-lapis-light/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian motion-safe:transition-colors">
          How Analysis Works
        </a>
        page documents all 21 forensic detectors: what each one measures, how it works,
        its known false-positive triggers, and the investigation modes in which it runs.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Trust score formula.</strong>
        The weighting applied to each signal is published in-app: 40% EXIF metadata
        analysis, 60% forensic signal analysis, with C2PA bonus and penalty adjustments
        and a composite amplification cap when two or more regional detectors fire.
        Nothing in the scoring is hidden.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Threshold transparency.</strong>
        Verdict thresholds are published: a deepfake score above 0.65 returns Synthetic;
        0.30–0.65 returns Inconclusive; below 0.30 returns Authentic. Analysts can quote
        these thresholds when explaining findings to a legal team.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Pipeline version recording.</strong>
        Every PDF trust report includes the Jura Trace version number at the time of
        analysis. This allows a reviewer to identify precisely which detector versions
        and model weights were used, and to reproduce the analysis if required.
      </span>
    </li>
  </ul>

  <!-- Callout: link to methodology -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      When submitting findings to legal proceedings, include a printed copy of the
      <a href="/help/methodology"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:text-lapis/80 dark:hover:text-lapis-light/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian motion-safe:transition-colors">
        How Analysis Works
      </a>
      page alongside the PDF trust report. This gives reviewers a plain-language
      explanation of every signal that contributed to the verdict.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     6. Analyst Accountability
     ══════════════════════════════════════════════════════════════════ -->
<section id="analyst-accountability" class="mb-12" aria-labelledby="analyst-accountability-heading">
  <h2
    id="analyst-accountability-heading"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
  >
    6. Analyst Accountability <span class="text-base font-normal text-flint dark:text-flint-light ml-2">Berkeley Protocol §6.3</span>
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The Protocol requires that individuals conducting investigations be identifiable,
    and that their findings not overstate the strength of the evidence. This protects
    the integrity of proceedings and establishes a clear line of professional
    responsibility for the analysis.
  </p>

  <div class="space-y-5 mb-6">

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Analyst declaration in PDF exports
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        The Professional tier includes an analyst declaration block in every PDF trust
        report. This records the analyst's name, organisation, case reference number,
        and the date of analysis. The declaration can serve as a signed statement of the
        investigator's identity and the context in which the analysis was conducted.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Deterministic and reproducible pipeline
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Given the same file, the same investigation mode, and the same version of Jura
        Trace, the analysis pipeline produces the same result. There is no probabilistic
        variation between runs. This means a legal reviewer or opposing counsel can
        independently reproduce the analysis and verify that the reported findings are
        accurate.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Three-way verdict design
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Jura Trace deliberately avoids binary authentic/fake verdicts. The three-way
        verdict — Authentic, Inconclusive, or Synthetic — reflects the Protocol's
        principle that findings should not overstate what the evidence supports.
        Inconclusive is the expected and correct verdict for the majority of real-world
        content. Analysts are encouraged to treat it as a signal for further investigation,
        not as a determination.
      </p>
    </div>

  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     7. Privacy and Security
     ══════════════════════════════════════════════════════════════════ -->
<section id="privacy-and-security" class="mb-12" aria-labelledby="privacy-and-security-heading">
  <h2
    id="privacy-and-security-heading"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
  >
    7. Privacy and Security <span class="text-base font-normal text-flint dark:text-flint-light ml-2">Berkeley Protocol §4</span>
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The Protocol requires that organisations consider the security of their investigation
    infrastructure and take reasonable steps to protect evidence, sources, and analysts —
    particularly when working in contexts where state surveillance or hostile actors
    present a realistic threat.
  </p>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-5">
    Jura Trace's local-first architecture was designed with exactly these contexts in mind.
    It is a desktop application that requires no internet connection, no user account,
    and no cloud infrastructure to function.
  </p>

  <ul class="space-y-3 mb-6">
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">No content leaves the device.</strong>
        All forensic analysis — including the ML pipeline and AI-generation detection —
        runs entirely on the analyst's machine. No file data, no metadata, and no results
        are transmitted to any external server at any point during analysis.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">No user accounts or telemetry.</strong>
        Jura Trace does not require registration, does not collect usage analytics, and
        does not report any operational data to Juralabs or any third party.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Air-gapped operation supported.</strong>
        Jura Trace can operate on a machine with no network connectivity. Optional features
        that require external models (Ollama AI descriptions) degrade gracefully — the
        core analysis pipeline functions without them.
      </span>
    </li>
    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="text-lapis dark:text-lapis-light mt-0.5 flex-none font-semibold">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Sidecar API authentication.</strong>
        The internal Python ML sidecar communicates exclusively on localhost (port 8200)
        and requires an API key for all requests. Network traffic from the sidecar is
        blocked by the application's Content Security Policy from reaching any external
        address.
      </span>
    </li>
  </ul>

  <!-- Callout: URL verification caution -->
  <div
    class="rounded border border-amber/25 bg-amber/5 dark:border-amber/20 dark:bg-amber/5 px-4 py-3 text-sm text-flint dark:text-flint-light leading-relaxed"
    role="note"
  >
    <span class="font-medium text-text-light dark:text-quartz">Note on URL verification: </span>
    The URL verification feature fetches a remote file for analysis. This requires an
    outbound network request from the device. If your operational security policy
    prohibits outbound requests from investigation infrastructure, use the file or
    batch input methods instead and transfer files via a separate, air-gapped process.
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     8. Preservation of Original Evidence
     ══════════════════════════════════════════════════════════════════ -->
<section id="preservation-of-evidence" class="mb-12" aria-labelledby="preservation-of-evidence-heading">
  <h2
    id="preservation-of-evidence-heading"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
  >
    8. Preservation of Original Evidence <span class="text-base font-normal text-flint dark:text-flint-light ml-2">Berkeley Protocol §5.2</span>
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The Protocol requires that original evidence be preserved unaltered. Any analysis
    or processing should operate on a copy, never on the source file. This allows the
    original to be re-examined independently and ensures the chain of custody is not
    compromised by the investigation process itself.
  </p>

  <ul class="space-y-3 mb-6">
    <li class="flex gap-2 text-sm">
      <span class="text-malachite dark:text-malachite-light flex-none mt-0.5">✓</span>
      <span class="text-flint dark:text-flint-light leading-relaxed">
        <strong class="text-text-light dark:text-text-dark">Verification never modifies the source file.</strong>
        When you submit a file for analysis, Jura Trace reads it into memory. The
        forensic pipeline operates on the in-memory copy. The file on disk is not
        touched, renamed, or moved.
      </span>
    </li>
    <li class="flex gap-2 text-sm">
      <span class="text-malachite dark:text-malachite-light flex-none mt-0.5">✓</span>
      <span class="text-flint dark:text-flint-light leading-relaxed">
        <strong class="text-text-light dark:text-text-dark">C2PA signing creates a new file.</strong>
        When you sign a file with C2PA Content Credentials, Jura Trace writes a new file
        alongside the original, appending <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">_c2pa</span>
        to the filename. The original remains unmodified.
      </span>
    </li>
    <li class="flex gap-2 text-sm">
      <span class="text-malachite dark:text-malachite-light flex-none mt-0.5">✓</span>
      <span class="text-flint dark:text-flint-light leading-relaxed">
        <strong class="text-text-light dark:text-text-dark">Existing metadata is preserved during signing.</strong>
        The C2PA signing process preserves all existing IPTC, XMP, and EXIF metadata
        fields in the signed copy. No pre-existing provenance information is stripped
        or overwritten.
      </span>
    </li>
    <li class="flex gap-2 text-sm">
      <span class="text-malachite dark:text-malachite-light flex-none mt-0.5">✓</span>
      <span class="text-flint dark:text-flint-light leading-relaxed">
        <strong class="text-text-light dark:text-text-dark">Watermark embedding creates a new file.</strong>
        Invisible watermark embedding, like C2PA signing, writes to a new file. The
        original is not altered.
      </span>
    </li>
  </ul>

  <!-- Callout: storage advice -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Storage recommendation.</strong>
      Store original evidence files in a write-protected location (a read-only volume or
      a locked folder) before importing them into Jura Trace. This provides a physical
      guarantee that originals cannot be modified, independent of Jura Trace's own
      non-destructive design.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     9. Recommended Workflow
     ══════════════════════════════════════════════════════════════════ -->
<section id="recommended-workflow" class="mb-12" aria-labelledby="recommended-workflow-heading">
  <h2
    id="recommended-workflow-heading"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
  >
    9. Recommended Workflow for Human Rights Documentation
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
    The following workflow applies the Berkeley Protocol's principles to a typical Jura
    Trace investigation. Following these steps in sequence creates a complete, legally
    defensible evidence record.
  </p>

  <ol class="space-y-4 mb-8">

    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20
                   text-lapis dark:text-lapis-light text-xs font-semibold
                   flex items-center justify-center mt-0.5">1</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Import evidence files into Jura Trace.</strong>
        Use the Protect page to import files. Importing registers each file in the local
        database, generates a perceptual fingerprint, and creates the first audit log
        entry — establishing the start of the chain of custody.
      </span>
    </li>

    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20
                   text-lapis dark:text-lapis-light text-xs font-semibold
                   flex items-center justify-center mt-0.5">2</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Sign with C2PA Content Credentials.</strong>
        Apply a C2PA signature to each file. This creates a cryptographic provenance
        record that binds your institution's identity to the file at this moment. The
        signed copy is written alongside the original.
      </span>
    </li>

    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20
                   text-lapis dark:text-lapis-light text-xs font-semibold
                   flex items-center justify-center mt-0.5">3</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Embed an invisible watermark.</strong>
        Use the watermark feature to embed your institution's identifier into each file.
        Select a strength level appropriate to expected handling — Medium is suitable
        for most documentation workflows. This adds a secondary provenance layer that
        is robust to moderate file manipulation.
      </span>
    </li>

    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20
                   text-lapis dark:text-lapis-light text-xs font-semibold
                   flex items-center justify-center mt-0.5">4</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Run verification in Deep or Archival mode.</strong>
        Submit each file for verification. Use Deep mode (20 frames for video; full
        detector suite for images) or Archival mode for the most comprehensive analysis
        record. Standard mode is suitable for initial triage. The results are recorded
        in the audit log.
      </span>
    </li>

    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20
                   text-lapis dark:text-lapis-light text-xs font-semibold
                   flex items-center justify-center mt-0.5">5</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Export a PDF trust report with analyst declaration.</strong>
        Generate a PDF from the results panel. On the Professional tier, complete the
        analyst declaration fields — name, organisation, case reference, and date.
        The PDF includes the full analysis record: verdict, trust score, all detector
        findings, metadata, and pipeline version.
      </span>
    </li>

    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20
                   text-lapis dark:text-lapis-light text-xs font-semibold
                   flex items-center justify-center mt-0.5">6</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Export a ZIP case file.</strong>
        Use the ZIP export to bundle the original file, the PDF trust report, all
        forensic heatmap visualisations, and the full machine-readable JSON result. This
        self-contained archive is the primary deliverable for legal handover.
      </span>
    </li>

    <li class="flex gap-3 text-sm text-flint dark:text-flint-light leading-relaxed">
      <span class="flex-none w-6 h-6 rounded-full bg-lapis/10 dark:bg-lapis/20
                   text-lapis dark:text-lapis-light text-xs font-semibold
                   flex items-center justify-center mt-0.5">7</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Verify audit chain integrity before submission.</strong>
        Navigate to Settings &#8594; Audit Log &#8594; Verify Chain Integrity immediately
        before handing over the evidence package. Confirm the chain is intact and include
        a record of this check with your submission.
      </span>
    </li>

  </ol>

  <!-- Callout: methodology page -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      Include a printed copy of the
      <a href="/help/methodology"
         class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:text-lapis/80 dark:hover:text-lapis-light/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian motion-safe:transition-colors">
        How Analysis Works
      </a>
      page with every legal submission. It gives the reviewing authority a complete,
      plain-language account of the tools and methods used — directly addressing the
      Berkeley Protocol's methodology documentation requirement.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     10. Important Disclaimer
     ══════════════════════════════════════════════════════════════════ -->
<section id="disclaimer" class="mb-12" aria-labelledby="disclaimer-heading">
  <h2
    id="disclaimer-heading"
    class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4"
  >
    10. Important Disclaimer
  </h2>

  <div
    class="rounded border border-amber/25 bg-amber/5 dark:border-amber/20 dark:bg-amber/5 px-4 py-4 text-sm text-flint dark:text-flint-light leading-relaxed"
    role="note"
  >
    <p class="mb-3">
      Jura Trace does not claim formal certification against the Berkeley Protocol on
      Digital Open Source Investigations. This page maps Jura Trace's design principles
      and technical capabilities to the Protocol's requirements, and is provided to
      support organisations conducting their own compliance assessments.
    </p>
    <p class="mb-3">
      The Berkeley Protocol is a methodological framework, not a certification scheme.
      Alignment with its principles is assessed by the organisations and legal bodies
      that use it — not by the developers of tools referenced in that work. Juralabs
      makes no legal representation that use of Jura Trace is sufficient, in itself, to
      satisfy any court, tribunal, or UN mechanism's evidentiary requirements.
    </p>
    <p>
      Organisations using Jura Trace for human rights documentation should conduct
      independent verification that the tool meets their specific evidentiary and
      operational requirements, consult their legal counsel, and consider the Protocol
      alongside other relevant frameworks and institutional guidelines.
    </p>
  </div>
</section>

<!-- ── Next steps ─────────────────────────────────────────────────── -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<section aria-label="Related pages" class="mb-4">
  <h2 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Related pages
  </h2>
  <ul class="space-y-2 text-sm">
    <li>
      <a href="/help/methodology"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        How Analysis Works
      </a>
      <span class="text-flint dark:text-flint-light"> — full documentation of all 21 forensic detectors, the trust score formula, and known limitations</span>
    </li>
    <li>
      <a href="/help/verify"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        Verifying Content Authenticity
      </a>
      <span class="text-flint dark:text-flint-light"> — step-by-step guide to submitting files for verification and reading the results</span>
    </li>
    <li>
      <a href="/help/protect"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        Protecting Your Content
      </a>
      <span class="text-flint dark:text-flint-light"> — C2PA signing, watermarking, and batch protection workflows</span>
    </li>
  </ul>
</section>
