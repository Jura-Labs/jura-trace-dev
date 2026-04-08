<script lang="ts">
  // No reactive state required — this is a static transparency page.
  // All content is authored directly; no Tauri IPC calls are made.
</script>

<!--
  Help — Methodology Transparency Page
  =====================================
  "How Analysis Works" explains every forensic detector, the trust scoring
  formula, investigation modes, and the known limitations of automated analysis.

  Audience: all users — from museum curators to technical investigators.
  Style: Sanctuary theme, Georgia serif headings, Tailwind utility classes.
  British spelling throughout. No emojis. Native <details>/<summary> for
  keyboard-accessible detector disclosures.
-->

<article aria-labelledby="methodology-heading">

  <!-- ── Page heading ──────────────────────────────────────────────────── -->
  <header class="mb-10">
    <h1
      id="methodology-heading"
      class="font-heading text-3xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      How Analysis Works
    </h1>
    <p class="text-base text-flint dark:text-flint-light leading-relaxed max-w-2xl">
      Jura Trace uses multiple independent forensic detectors to assess content
      authenticity. No single detector is conclusive — the trust score reflects
      the combined weight of all available signals. This page explains each
      detector, how scores are computed, and the known limitations of automated
      analysis.
    </p>
    <div class="earth-line mt-6" aria-hidden="true"></div>
  </header>

  <!-- ── Table of contents ─────────────────────────────────────────────── -->
  <nav aria-label="Page contents" class="mb-10">
    <p class="text-xs text-flint dark:text-flint-light uppercase tracking-nav font-medium mb-3">Contents</p>
    <ol class="space-y-1 text-sm">
      {#each [
        { href: '#trust-score',         label: 'Trust Score' },
        { href: '#investigation-modes', label: 'Investigation Modes' },
        { href: '#detector-reference',  label: 'Detector Reference' },
        { href: '#limitations',         label: 'What This Does Not Prove' },
        { href: '#signal-weighting',    label: 'Signal Weighting' },
      ] as item}
        <li>
          <a
            href={item.href}
            class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none
                   focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >
            {item.label}
          </a>
        </li>
      {/each}
    </ol>
  </nav>

  <!-- ── 1. Trust Score ────────────────────────────────────────────────── -->
  <section aria-labelledby="trust-score-heading" class="mb-12" id="trust-score">

    <h2
      id="trust-score-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      Trust Score
    </h2>

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
      Every analysis produces a trust score between 0% and 100%. The score
      summarises how consistent the available evidence is with an unmanipulated,
      authentically captured piece of content. Think of it as the bedrock reading
      from all detection layers combined — the stronger and more consistent the
      signals, the higher the score.
    </p>

    <!-- Trust levels -->
    <h3 class="font-heading text-lg text-text-light dark:text-quartz mb-3 mt-6 leading-tight">
      Trust Levels and Verdicts
    </h3>

    <div class="overflow-x-auto mb-6">
      <table class="w-full text-sm border-collapse" aria-label="Trust level thresholds">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="text-left py-2 pr-6 font-medium text-text-light dark:text-quartz">Level</th>
            <th class="text-left py-2 pr-6 font-medium text-text-light dark:text-quartz">Score range</th>
            <th class="text-left py-2 pr-6 font-medium text-text-light dark:text-quartz">Verdict</th>
            <th class="text-left py-2 font-medium text-text-light dark:text-quartz">Meaning</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border-light dark:divide-border-dark">
          <tr>
            <td class="py-2.5 pr-6">
              <span class="inline-flex items-center gap-1.5">
                <span class="w-2 h-2 rounded-full bg-malachite dark:bg-malachite-light flex-shrink-0" aria-hidden="true"></span>
                <span class="text-text-light dark:text-quartz font-medium">High Trust</span>
              </span>
            </td>
            <td class="py-2.5 pr-6 tabular-nums text-flint dark:text-flint-light">71% – 100%</td>
            <td class="py-2.5 pr-6 text-malachite dark:text-malachite-light font-medium">Authentic</td>
            <td class="py-2.5 text-flint dark:text-flint-light">No detectors flagged anomalies. Content appears consistent with authentic capture.</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6">
              <span class="inline-flex items-center gap-1.5">
                <span class="w-2 h-2 rounded-full bg-amber dark:bg-amber-light flex-shrink-0" aria-hidden="true"></span>
                <span class="text-text-light dark:text-quartz font-medium">Moderate Trust</span>
              </span>
            </td>
            <td class="py-2.5 pr-6 tabular-nums text-flint dark:text-flint-light">40% – 70%</td>
            <td class="py-2.5 pr-6 text-amber dark:text-amber-light font-medium">Inconclusive</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Some signals raised concerns but evidence is not definitive. Human review is recommended.</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6">
              <span class="inline-flex items-center gap-1.5">
                <span class="w-2 h-2 rounded-full bg-cinnabar dark:bg-cinnabar-light flex-shrink-0" aria-hidden="true"></span>
                <span class="text-text-light dark:text-quartz font-medium">Low Trust</span>
              </span>
            </td>
            <td class="py-2.5 pr-6 tabular-nums text-flint dark:text-flint-light">0% – 39%</td>
            <td class="py-2.5 pr-6 text-cinnabar dark:text-cinnabar-light font-medium">Synthetic</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Multiple strong signals indicate manipulation or AI generation.</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Scoring formula -->
    <h3 class="font-heading text-lg text-text-light dark:text-quartz mb-3 mt-6 leading-tight">
      Scoring Formula
    </h3>

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
      The overall trust score is built from two components:
    </p>

    <ul class="space-y-2 text-sm text-flint dark:text-flint-light mb-4 pl-4">
      <li class="flex gap-2">
        <span class="flex-shrink-0 text-lapis dark:text-lapis-light mt-0.5" aria-hidden="true">&#8594;</span>
        <span>
          <span class="font-medium text-text-light dark:text-quartz">40% — EXIF metadata trust.</span>
          Derived from the 12-rule EXIF anomaly check. Missing camera data, GPS/timestamp
          mismatches, and software editor signatures each reduce this component.
        </span>
      </li>
      <li class="flex gap-2">
        <span class="flex-shrink-0 text-lapis dark:text-lapis-light mt-0.5" aria-hidden="true">&#8594;</span>
        <span>
          <span class="font-medium text-text-light dark:text-quartz">60% — Forensic analysis trust.</span>
          The worst-case result across all manipulation signals and the deepfake ensemble
          score. Taking the worst case ensures that a single strong negative signal cannot
          be averaged away by clean results elsewhere.
        </span>
      </li>
    </ul>

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
      Two adjustments are then applied:
    </p>

    <ul class="space-y-2 text-sm text-flint dark:text-flint-light mb-6 pl-4">
      <li class="flex gap-2">
        <span class="flex-shrink-0 text-lapis dark:text-lapis-light mt-0.5" aria-hidden="true">&#8594;</span>
        <span>
          <span class="font-medium text-text-light dark:text-quartz">C2PA provenance bonus: +10%.</span>
          When a cryptographically valid C2PA provenance manifest is present — providing
          a verifiable record of the content's origin — the score receives a 10% uplift,
          up to a maximum of 100%.
        </span>
      </li>
      <li class="flex gap-2">
        <span class="flex-shrink-0 text-lapis dark:text-lapis-light mt-0.5" aria-hidden="true">&#8594;</span>
        <span>
          <span class="font-medium text-text-light dark:text-quartz">Regional composite amplification cap: 0.55 maximum.</span>
          When two or more regional detectors (Segmented ELA, Shadow Consistency,
          Colour Temperature, Splice Boundary) independently flag anomalies, the overall
          trust score is capped at 55%, regardless of other signals. Two independent
          regional detectors agreeing is a strong indicator of compositing.
        </span>
      </li>
    </ul>

    <!-- Document scoring -->
    <h3 class="font-heading text-lg text-text-light dark:text-quartz mb-3 mt-6 leading-tight">
      Document Scoring
    </h3>

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
      PDF documents cannot be subjected to pixel-level forensic analysis. For PDFs,
      the score is derived from C2PA provenance alone:
    </p>

    <div class="overflow-x-auto mb-6">
      <table class="w-full text-sm border-collapse" aria-label="PDF document scoring rules">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="text-left py-2 pr-6 font-medium text-text-light dark:text-quartz">Condition</th>
            <th class="text-left py-2 font-medium text-text-light dark:text-quartz">Score</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border-light dark:divide-border-dark">
          <tr>
            <td class="py-2.5 pr-6 text-flint dark:text-flint-light">Valid C2PA manifest present</td>
            <td class="py-2.5 tabular-nums text-malachite dark:text-malachite-light font-medium">82%</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-flint dark:text-flint-light">C2PA manifest present but invalid signature</td>
            <td class="py-2.5 tabular-nums text-cinnabar dark:text-cinnabar-light font-medium">25%</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-flint dark:text-flint-light">No C2PA manifest</td>
            <td class="py-2.5 tabular-nums text-amber dark:text-amber-light font-medium">50%</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Not legal proof callout -->
    <div
      class="rounded border border-amber/25 bg-amber/5 dark:border-amber/20 dark:bg-amber/5 px-4 py-3 text-sm text-flint dark:text-flint-light leading-relaxed"
      role="note"
    >
      <span class="font-medium text-text-light dark:text-quartz">Important: </span>
      The trust score is a confidence indicator, not legal proof. A score of 100%
      means no detectors found anomalies — it does not mean the content is
      definitively authentic. See
      <a href="#limitations" class="text-lapis dark:text-lapis-light hover:underline">What This Does Not Prove</a>
      below.
    </div>

  </section>

  <div class="earth-line mb-10" aria-hidden="true"></div>

  <!-- ── 2. Investigation Modes ─────────────────────────────────────────── -->
  <section aria-labelledby="investigation-modes-heading" class="mb-12" id="investigation-modes">

    <h2
      id="investigation-modes-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      Investigation Modes
    </h2>

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
      Choose a mode before running analysis. Faster modes are suitable for routine
      triage; deeper modes are appropriate when you need the fullest possible
      picture of a file's provenance strata.
    </p>

    <div class="overflow-x-auto mb-4">
      <table class="w-full text-sm border-collapse" aria-label="Investigation modes comparison">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="text-left py-2 pr-6 font-medium text-text-light dark:text-quartz">Mode</th>
            <th class="text-left py-2 pr-6 font-medium text-text-light dark:text-quartz">Typical time</th>
            <th class="text-left py-2 pr-6 font-medium text-text-light dark:text-quartz">Detectors active</th>
            <th class="text-left py-2 font-medium text-text-light dark:text-quartz">Best for</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border-light dark:divide-border-dark">
          <tr>
            <td class="py-2.5 pr-6 font-medium text-text-light dark:text-quartz">Standard</td>
            <td class="py-2.5 pr-6 tabular-nums text-flint dark:text-flint-light">~15 s</td>
            <td class="py-2.5 pr-6 text-flint dark:text-flint-light">EXIF anomaly, C2PA, ELA, AI detection</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Routine triage and quick authenticity checks</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 font-medium text-text-light dark:text-quartz">Deep</td>
            <td class="py-2.5 pr-6 tabular-nums text-flint dark:text-flint-light">~60 s</td>
            <td class="py-2.5 pr-6 text-flint dark:text-flint-light">All detectors, including regional analysis</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Investigating specific concerns or disputed content</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 font-medium text-text-light dark:text-quartz">Archival</td>
            <td class="py-2.5 pr-6 tabular-nums text-flint dark:text-flint-light">~90 s+</td>
            <td class="py-2.5 pr-6 text-flint dark:text-flint-light">Deep analysis with scanner-calibrated tolerances; chain-of-custody logging</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Preservation workflows, institutional archives, evidence preservation</td>
          </tr>
        </tbody>
      </table>
    </div>

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      Regional detectors (Segmented ELA, Shadow Consistency, Colour Temperature,
      Splice Boundary) are only active in Deep and Archival modes. Standard mode
      returns <code class="font-mono text-xs bg-graphite/10 dark:bg-graphite/60 px-1 rounded">null</code>
      for those fields.
    </p>

  </section>

  <div class="earth-line mb-10" aria-hidden="true"></div>

  <!-- ── 3. Detector Reference ──────────────────────────────────────────── -->
  <section aria-labelledby="detector-reference-heading" class="mb-12" id="detector-reference">

    <h2
      id="detector-reference-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-2 leading-tight tracking-heading"
    >
      Detector Reference
    </h2>

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
      Expand each detector to learn what it measures, how it works, and when it
      may produce false positives. All 16 detectors run locally on your device —
      no data is transmitted externally.
    </p>

    <div class="space-y-2">

      <!-- 1. EXIF Anomaly Detection -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">01</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">EXIF Anomaly Detection</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Consistency of the metadata embedded in a file at the time of capture — including camera model, GPS coordinates, timestamps, software tags, and resolution values.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Applies 12 consistency rules to the file's EXIF data: checking whether timestamps are plausible, whether GPS data matches declared location, whether a software editor tag has been added after capture, and whether resolution values are internally consistent.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                One or more metadata fields are missing, inconsistent, or contain signatures associated with editing software. This may indicate the metadata was stripped or altered after the original capture.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Intentional EXIF stripping for privacy (common before sharing images online), CMS or social media platforms that remove or rewrite metadata, and screenshots (which lack camera data by design).
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Standard &#183; Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Limited to metadata embedded at capture time. Cannot detect modifications to the image content itself — only inconsistencies in the surrounding metadata. Files stripped of all metadata produce no signal.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 2. C2PA Provenance -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">02</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">C2PA Provenance</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Whether the file carries a cryptographically signed provenance record — a digital certificate of origin created at the point of capture or production, following the Coalition for Content Provenance and Authenticity (C2PA) open standard.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Reads and cryptographically verifies the embedded C2PA manifest. If valid, extracts the claim generator field to detect whether a known AI creation tool signed the credentials. An AI-generated image signed by its creator will carry valid credentials — those credentials are then treated as evidence of AI origin rather than evidence of authenticity.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                A valid manifest means the file's provenance chain is intact and has not been broken since signing. An invalid or absent manifest means the file cannot be verified via this standard — it does not mean the content is inauthentic.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Most existing images and documents do not carry C2PA credentials. Absence of credentials is not a negative finding — it simply means verification via this standard is not possible.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Standard &#183; Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Jura Trace uses a self-signed certificate. Manifests signed by Jura Trace are valid but not trusted by third-party C2PA verifiers such as Adobe's Content Authenticity web tool. Institutional trust requires a certificate from a C2PA Trust List authority.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 3. Error Level Analysis -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">03</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Error Level Analysis (ELA)</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Differences in compression error across regions of an image. Every time a JPEG image is saved, it loses a predictable amount of information. ELA amplifies those differences to reveal regions that have been saved a different number of times from the rest of the image.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Re-compresses the image at a known quality level and subtracts the result from the original. In an unedited image, error levels are uniform across the frame. Regions pasted in from another source — or edited after the original compression — show higher error levels than the surrounding image and appear brighter in the ELA heatmap.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Uneven error levels suggest one or more regions were modified after the original file was created, or were composited from a source with a different compression history.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                High-detail areas (foliage, fabric, hair) naturally produce higher ELA values. Multiple rounds of social media recompression can produce uniform but elevated error levels across the whole image, reducing sensitivity. Modern codecs (AVIF, WebP) may show atypical patterns.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Standard &#183; Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Unreliable on multiply-compressed JPEG images. Social media platforms compress images multiple times, creating ELA artefacts indistinguishable from manipulation. Weight reduced to 1.0 (from 2.0) in trust scoring to reflect this limitation.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 4. Noise Analysis -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">04</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Noise Analysis</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                The distribution and consistency of noise grain across the image. Cameras introduce a characteristic noise pattern at the sensor level. AI-generated images often lack this natural grain distribution.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Divides the image into blocks and measures variance within each block. Compares the variance distribution across blocks against expected natural camera noise patterns. Blocks with significantly higher or lower noise than their neighbours are flagged as anomalous.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Unnaturally uniform noise across the image may indicate AI generation. Localised noise anomalies between regions may indicate compositing from sources with different sensor noise profiles.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Heavily sharpened images, images with applied noise reduction, screenshots, and illustrations all have non-camera-like noise profiles and may trigger this detector without indicating manipulation.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Standard &#183; Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                May flag authentic images with intentional grain (film scans, night photography, high-ISO captures) as anomalous. Cannot distinguish artistic noise from manipulation-related noise inconsistency.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 5. Copy-Move Detection -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">05</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Copy-Move Detection</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Whether any region of an image has been duplicated from another part of the same image — a technique commonly used to clone out unwanted content or replicate objects.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Extracts feature descriptors from image patches and performs nearest-neighbour matching across all patch pairs. Matched pairs that are sufficiently separated in the image but have similar visual content are flagged as potential clone regions.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                One or more regions appear to have been copied from elsewhere in the same image. This is a reliable indicator of manual editing with a clone stamp or healing brush tool.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Images with naturally repeating patterns — wallpaper, tiling, fabric, crowd scenes — may produce false matches. Very small images or images with few distinguishable features are also more susceptible.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Standard &#183; Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Uses ORB feature matching, which can miss small, rotated, or scaled copied regions. SIFT upgrade planned (patent expired 2020). Performance degrades on heavily compressed images where keypoints are destroyed.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 6. AI Generation Detection -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">06</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">AI Generation Detection</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Statistical patterns in the image that distinguish AI-generated imagery from photographs taken with a real camera. AI generation models leave characteristic fingerprints in noise, texture, and spectral distributions.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Extracts an 80-feature vector covering noise statistics (LSB randomness, LSB entropy), spectral decay patterns, Local Binary Pattern (LBP) texture descriptors, and Grey-Level Co-occurrence Matrix (GLCM) contrast measures. A GradientBoosting classifier — trained on 545 images, cross-validation AUC&#8209;ROC 0.945 — assigns a probability score, which is blended with a heuristic signal at a 65%/35% ratio. The pipeline also checks for invisible watermarks from known AI generators.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                The statistical properties of the image are more consistent with AI generation than camera capture. Higher-confidence findings indicate multiple independent features pointing in the same direction.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Heavily processed photographs, CGI renders, composite illustrations, and images that have undergone multiple rounds of compression may exhibit AI-like statistical properties. The trained false positive rate on authentic press photos is approximately 14% — human review is always warranted.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Standard &#183; Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Trained on 709 images from generators available before April 2026. May underperform on outputs from newer generators. Minimum image size: 128&#215;128 pixels. See the <a href="/help/model-cards#gbm-classifier" class="text-lapis dark:text-lapis-light underline hover:no-underline">GBM model card</a> for full training data documentation.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 7. NPR -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">07</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Neighbouring Pixel Relationships (NPR)</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                How adjacent pixels relate to one another. Natural photographs have characteristic correlation patterns between neighbouring pixels, arising from optical blur, sensor interpolation, and scene continuity. AI generators produce pixels through a fundamentally different process that disturbs these relationships.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Computes horizontal and vertical pixel correlation coefficients, the variance of pixel differences, and high-frequency energy ratios. These values are compared against empirical distributions from authentic photographs.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Pixel relationship statistics deviate significantly from natural camera output. This is a complementary AI-detection signal that is independent of ELA and noise analysis.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Heavily upscaled images, images with strong sharpening filters, and artwork or illustrations all exhibit non-photographic pixel relationships and will typically trigger this detector.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Standard &#183; Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Computationally intensive. Less effective on highly compressed content where pixel neighbour relationships are already disrupted by quantisation. Best suited to high-quality source images.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 8. Chromatic Aberration -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">08</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Chromatic Aberration</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Whether the image contains the radial colour fringing pattern produced by real camera lenses. All physical lenses bend different wavelengths of light by slightly different amounts, producing a characteristic colour fringe — strongest at the image edges — that AI generators do not replicate.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Measures the offset between the red and blue colour channels at a sample of high-contrast edge points across the image. Fits a radial distortion model and computes the R-squared coefficient of determination. A high R-squared value indicates a consistent lens aberration pattern consistent with a real camera.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                The image lacks a consistent radial chromatic aberration pattern. AI-generated images and heavily post-processed photographs typically fail this check.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Camera software that corrects chromatic aberration in-camera (common in modern smartphones and mirrorless cameras), screenshots, illustrations, and CGI renders will all fail this check without being manipulated.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Standard &#183; Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Rated 1/5 accuracy in forensic audit (March 2026). Included in Deep mode only as an investigative signal. Results should not be weighted heavily and are excluded from trust scoring.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 9. JPEG Ghost -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">09</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">JPEG Ghost</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Evidence of double compression — the signature left when a region of an image was previously saved as a JPEG at a different quality level before being composited into the final file.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Re-compresses the image at multiple quality levels and measures the deviation from the original in each block. Regions that show a minimum deviation at an unexpected quality level — different from the rest of the image — are flagged as potential JPEG ghosts, indicating they carry a different compression history from the surrounding content.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Part of the image appears to have been saved at a different JPEG quality setting from the rest, consistent with being spliced in from a separately compressed source.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Overlaid graphics or watermarks added in a different application, images assembled from multiple sources for legitimate purposes (collages, contact sheets), and very low-quality JPEG files where compression dominates the signal.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Standard &#183; Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Only applicable to JPEG files. Produces no signal on PNG, WebP, TIFF, or other non-JPEG formats. The detector section is greyed out for non-JPEG inputs.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 10. Segmented ELA -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">10</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Segmented ELA</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Regional variation in compression error levels across a grid of image segments. Where standard ELA analyses the whole image uniformly, Segmented ELA examines whether specific regions are inconsistent with their neighbours — the forensic equivalent of reading the strata in individual rock layers rather than the whole formation at once.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Divides the image into an 8&#215;8 grid (64 cells) and runs ELA independently on each cell. Applies cluster analysis to identify groups of cells with anomalously high error levels compared to the rest of the image.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Specific regions show compression error levels inconsistent with adjacent areas, suggesting those regions may have been added from a differently compressed source.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Images with highly varied content types within a single frame (a person against a smooth background, text overlaid on a photograph) naturally produce regional ELA variation. Intentionally added text, logos, or watermarks will flag strongly.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Grid-based analysis divides images into 8&#215;8 cells. Artefacts in a single cell may indicate localised compression differences rather than deliberate manipulation. Consider alongside other regional detectors.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 11. Shadow Consistency -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">11</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Shadow Consistency</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Whether the implied direction of light is consistent across different regions of the image. In an authentic photograph, shadows and highlights all point away from the same light source. Composite images — where elements were photographed under different lighting conditions — frequently fail this check.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Divides the image into regions and computes a gradient-weighted estimate of light direction (expressed as an angle) for each region. Compares estimated light directions across regions. Significant angular disagreement — weighted by the strength of the gradient signal — is treated as evidence of inconsistent lighting.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Different parts of the image appear to have been lit from different directions, suggesting elements were photographed or generated separately and composited together.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Scenes with multiple artificial light sources (studio setups, concert photography, street scenes at night), reflective surfaces, and images with strong background/foreground separation can legitimately show regional lighting inconsistencies.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Removed from trust scoring due to high false positive rate. Displayed as an investigative signal in Expert View only. Useful for manual inspection of light direction but not reliable for automated detection.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 12. Colour Temperature -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">12</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Colour Temperature</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Whether the colour temperature — the warm or cool quality of the light — is consistent across different segments of the image. Elements photographed under different lighting conditions carry different colour casts even after global white balance adjustments.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Converts the image to the CIELAB perceptual colour space and segments it into regions. Analyses the warm/cool balance (the a and b channels) of each segment. Significant divergence between segments — particularly between foreground and background — is treated as an inconsistency indicator.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Different regions appear to have been captured under different lighting conditions. Combined with other regional signals, this increases confidence in a composite manipulation finding.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Images with naturally warm foreground and cool background (outdoor portrait photography, golden-hour scenes), intentional colour grading, and images that mix daylight and artificial light sources will show natural colour temperature inconsistencies.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Assumes single-illuminant scenes. Mixed lighting conditions (e.g. tungsten + daylight, indoor/outdoor transitions) produce false positives. Results are most meaningful for outdoor scenes with consistent natural light.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 13. Splice Boundary -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">13</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Splice Boundary</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                The physical boundary where one image region ends and another begins — the cut edge produced when elements are composited. Three independent edge signals are combined to localise these boundaries.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Analyses three signals simultaneously: (1) JPEG DCT grid discontinuities — abrupt changes in the compression block pattern at potential splice points; (2) noise level changes — sudden shifts in noise grain across a boundary; (3) feathering artefacts — the soft-edge signature left by selection tools and layer masking. Agreement between multiple signals at the same location substantially increases confidence.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                One or more signals detected an anomalous edge inconsistent with natural image content. When corroborated by Segmented ELA flagging the same region, this is the highest-confidence composite manipulation signal Jura Trace can produce.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Hard vignettes, image borders, intentionally added frames or decorative edges, and embedded watermarks or logos all create artificial boundaries that can trigger this detector. Cropped images may show strong grid discontinuities at the crop boundary.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Deep &#183; Archival</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Removed from trust scoring due to high false positive rate on images with natural sharp edges (architecture, text, geometric patterns). Displayed as an investigative signal in Expert View only.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 14. CLIP Detection -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">14</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">
              CLIP Detection
              <span class="ml-1.5 text-xs font-normal text-flint dark:text-flint-light">(optional)</span>
            </span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Semantic-level characteristics of the image using a large-scale vision-language model. Rather than analysing low-level pixel statistics, CLIP Detection asks whether the overall image content appears consistent with AI-generated or authentic photographic output.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Passes the image through an OpenCLIP ViT&#8209;B/32 vision encoder and performs zero-shot classification against text descriptions of AI-generated versus authentic photography. Cosine similarity scores for each description are used to derive a confidence-weighted verdict. This detector requires an optional ~350 MB model download and loads lazily on first use.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                The image's visual semantics are more consistent with AI-generated content than with authentic photography. This is a complementary signal that operates at a different level of abstraction to the statistical detectors.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Highly stylised or artistic photography, heavily post-processed images, macro photography, and subjects statistically over-represented in AI training datasets may produce elevated AI probability scores.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Deep &#183; Archival — only when the optional model is installed</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                2.7% false positive rate on authentic images. Non-photographic content (paintings, digital illustrations) may still trigger false positives. Requires the optional CLIP ViT&#8209;B/32 model (~350 MB). See the <a href="/help/model-cards#univfd-probe" class="text-lapis dark:text-lapis-light underline hover:no-underline">UnivFD model card</a> for full documentation.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 15. RAG Claim Checker -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">15</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">
              RAG Claim Checker
              <span class="ml-1.5 text-xs font-normal text-flint dark:text-flint-light">(optional — requires Ollama)</span>
            </span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Factual claims made in audio or video content, checked against a local knowledge base. This detector does not assess visual authenticity — it assesses whether speech content is consistent with verifiable facts.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Audio tracks are transcribed locally via the faster-whisper model. Discrete factual claims are extracted from the transcript. Each claim is evaluated against a local knowledge base using TF-IDF retrieval (no web access), and a Qwen2.5 language model running via Ollama produces a verdict: supported, disputed, unverified, or unavailable. All processing occurs entirely on-device.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                One or more claims in the audio content conflict with material in the local knowledge base. A disputed finding does not prove the claim is false — the knowledge base may be incomplete on the topic in question.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                The local knowledge base is a small preliminary corpus covering a limited domain. Claims about topics not represented in the corpus return <em>insufficient context</em> &mdash; the tool does not reason from its training data about claims the corpus cannot support. This is a retrieval match, not a fact-check; see the <a href="/help/model-cards#kb-retrieval" class="text-lapis dark:text-lapis-light underline hover:no-underline">model card</a> for scope, limitations, and the explicit non-warranty. Transcription errors may also lead to incorrect claim extraction.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Deep &#183; Archival — only when Ollama is running and faster-whisper is installed</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Requires Ollama running locally with a Qwen2.5 model downloaded. Knowledge base is limited to ~150 passages across 6 documents — claims outside this domain cannot be verified. Accuracy depends on transcription quality (faster-whisper). Not a replacement for professional fact-checking — results indicate consistency with the local knowledge base only.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 16. Video Deepfake Analysis -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">16</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Video Deepfake Analysis</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Whether a video file contains AI-generated or manipulated content, assessed frame by frame and across the temporal sequence. Temporal analysis catches inconsistencies between frames that would be invisible in a single-frame check.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Extracts evenly-spaced frames (6 in Standard mode, 20 in Deep, 40 in Archival) and runs the full AI Generation Detection pipeline on each frame. Three temporal consistency signals are then computed across the frame sequence: noise drift, spectral drift, and LBP texture drift. The overall score aggregates as: 50% mean frame score + 30% worst-case frame score + 20% temporal drift score.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Either individual frames exhibit AI-generation patterns, or the statistical properties of the video change over time in ways inconsistent with natural camera footage. High temporal drift alongside clean individual frames can indicate face-swap manipulation applied to otherwise authentic footage.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Video with intentional visual effects or colour grading transitions, footage assembled from multiple clips with different camera settings, and heavily compressed video (especially at low bitrates) can all produce temporal drift signals unrelated to manipulation.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint dark:text-flint-light">Standard &#183; Deep &#183; Archival — video files only</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint dark:text-flint-light leading-relaxed">
                Analyses a sample of frames (6 in Standard, 20 in Deep, 40 in Archival) — not every frame. Manipulation confined to un-sampled frames may be missed. Temporal consistency signals (noise drift, spectral drift, LBP drift) require sufficient frame count for meaningful measurement. Requires FFmpeg for frame extraction.
              </dd>
            </div>
          </dl>
        </div>
      </details>

    </div><!-- /detector list -->
  </section>

  <div class="earth-line mb-10" aria-hidden="true"></div>

  <!-- ── 4. What This Does Not Prove ──────────────────────────────────────── -->
  <section aria-labelledby="limitations-heading" class="mb-12" id="limitations">

    <h2
      id="limitations-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      What This Does Not Prove
    </h2>

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
      Jura Trace is a forensic aid. Like all forensic tools, it has limits. The
      following limitations apply to every analysis.
    </p>

    <ul class="space-y-5">

      <li class="flex gap-3 text-sm">
        <span
          class="flex-shrink-0 mt-0.5 w-4 text-center font-bold text-cinnabar dark:text-cinnabar-light"
          aria-hidden="true"
        >&#215;</span>
        <div>
          <p class="font-medium text-text-light dark:text-quartz mb-1">Automated analysis cannot prove authenticity.</p>
          <p class="text-flint dark:text-flint-light leading-relaxed">
            A high trust score means no detectors found anomalies. It does not mean the
            content is definitively authentic — it means analysis found nothing to indicate
            otherwise.
          </p>
        </div>
      </li>

      <li class="flex gap-3 text-sm">
        <span
          class="flex-shrink-0 mt-0.5 w-4 text-center font-bold text-cinnabar dark:text-cinnabar-light"
          aria-hidden="true"
        >&#215;</span>
        <div>
          <p class="font-medium text-text-light dark:text-quartz mb-1">A low trust score may reflect legitimate processing.</p>
          <p class="text-flint dark:text-flint-light leading-relaxed">
            Social media platforms, content management systems, and publishing workflows
            routinely recompress, resize, and strip metadata from images. These processes
            produce forensic artefacts that are indistinguishable from some manipulation
            signals. A low trust score on a file with a documented processing history is
            not necessarily evidence of tampering.
          </p>
        </div>
      </li>

      <li class="flex gap-3 text-sm">
        <span
          class="flex-shrink-0 mt-0.5 w-4 text-center font-bold text-cinnabar dark:text-cinnabar-light"
          aria-hidden="true"
        >&#215;</span>
        <div>
          <p class="font-medium text-text-light dark:text-quartz mb-1">Scores should inform human judgement, not replace it.</p>
          <p class="text-flint dark:text-flint-light leading-relaxed">
            No forensic detector has a zero false positive or false negative rate. Results
            must be interpreted by a human, in context, alongside other available evidence.
            The "Know What's Real" tagline reflects an aspiration — not a guarantee that
            analysis will always reach the correct conclusion.
          </p>
        </div>
      </li>

      <li class="flex gap-3 text-sm">
        <span
          class="flex-shrink-0 mt-0.5 w-4 text-center font-bold text-cinnabar dark:text-cinnabar-light"
          aria-hidden="true"
        >&#215;</span>
        <div>
          <p class="font-medium text-text-light dark:text-quartz mb-1">Jura Trace is not a legal authority.</p>
          <p class="text-flint dark:text-flint-light leading-relaxed">
            Analysis results are not legal evidence and should not be presented as
            definitive findings in legal proceedings without appropriate expert
            interpretation. For evidential purposes, results should be treated as
            investigative indicators that warrant further examination by a qualified
            forensic specialist.
          </p>
        </div>
      </li>

      <li class="flex gap-3 text-sm">
        <span
          class="flex-shrink-0 mt-0.5 w-4 text-center font-bold text-cinnabar dark:text-cinnabar-light"
          aria-hidden="true"
        >&#215;</span>
        <div>
          <p class="font-medium text-text-light dark:text-quartz mb-1">Detectors are trained on current AI generation techniques.</p>
          <p class="text-flint dark:text-flint-light leading-relaxed">
            AI generation technology advances rapidly. Detectors trained on current model
            outputs may be less effective against future techniques. Jura Trace is updated
            regularly, but a period of reduced sensitivity should be expected whenever a
            new generation technique becomes widespread.
          </p>
        </div>
      </li>

    </ul>

  </section>

  <div class="earth-line mb-10" aria-hidden="true"></div>

  <!-- ── 5. Signal Weighting ────────────────────────────────────────────── -->
  <section aria-labelledby="signal-weighting-heading" class="mb-12" id="signal-weighting">

    <h2
      id="signal-weighting-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      Signal Weighting
    </h2>

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
      When multiple detectors run simultaneously, their results are combined using
      a weighted ensemble. The weights reflect each detector's empirical reliability
      as a manipulation indicator, established through calibration against known
      authentic and manipulated image corpora.
    </p>

    <div class="overflow-x-auto mb-6">
      <table class="w-full text-sm border-collapse" aria-label="Detector signal weights">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="text-left py-2 pr-6 font-medium text-text-light dark:text-quartz">Detector</th>
            <th class="text-left py-2 pr-6 font-medium text-text-light dark:text-quartz">Weight</th>
            <th class="text-left py-2 font-medium text-text-light dark:text-quartz">Notes</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border-light dark:divide-border-dark">
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Error Level Analysis</td>
            <td class="py-2.5 pr-6 tabular-nums font-medium text-text-light dark:text-quartz">2.0</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Most reliable single manipulation indicator</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Segmented ELA</td>
            <td class="py-2.5 pr-6 tabular-nums font-medium text-text-light dark:text-quartz">1.5</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Regional variant; high specificity for compositing</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Colour Temperature</td>
            <td class="py-2.5 pr-6 tabular-nums font-medium text-text-light dark:text-quartz">1.5</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Strong composite indicator when consistent with other regional signals</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Noise Analysis</td>
            <td class="py-2.5 pr-6 tabular-nums font-medium text-text-light dark:text-quartz">1.0</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Standard weight; useful for both AI detection and compositing</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Copy-Move Detection</td>
            <td class="py-2.5 pr-6 tabular-nums font-medium text-text-light dark:text-quartz">1.0</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Standard weight; specific to clone stamp manipulation</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Shadow Consistency</td>
            <td class="py-2.5 pr-6 tabular-nums font-medium text-text-light dark:text-quartz">1.0</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Standard weight; higher false positive rate in complex lighting</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Splice Boundary</td>
            <td class="py-2.5 pr-6 tabular-nums font-medium text-text-light dark:text-quartz">1.0</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Most meaningful when corroborated by Segmented ELA</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">AI Generation Detection</td>
            <td class="py-2.5 pr-6 text-flint dark:text-flint-light italic">Independent</td>
            <td class="py-2.5 text-flint dark:text-flint-light">Considered via worst-case combination with manipulation score; not pooled into the weighted sum</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Concordance check -->
    <h3 class="font-heading text-lg text-text-light dark:text-quartz mb-3 mt-6 leading-tight">
      Concordance Check
    </h3>

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
      When ELA and the AI Generation Detection ensemble both return clean results —
      but other signals (noise analysis, copy-move) flag concerns — Jura Trace
      applies a concordance dampening factor. When the two most reliable detectors
      agree that the image is clean, the weight of disagreeing secondary signals
      is reduced.
    </p>

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      This logic is based on the observation that mixed signals alongside a clean
      ELA result are most commonly caused by codec artefacts (AVIF, WebP, HEIC)
      and heavy recompression rather than genuine manipulation. The dampening
      reduces false positives in these cases while preserving sensitivity when
      the primary detectors also fire.
    </p>

  </section>

  <!-- ── Footer navigation ──────────────────────────────────────────────── -->
  <nav
    aria-label="Help section navigation"
    class="pt-6 mt-4 border-t border-border-light dark:border-border-dark"
  >
    <div class="flex items-center justify-between gap-4 text-sm">
      <a
        href="/help/verify"
        class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none
               focus-visible:ring-2 focus-visible:ring-lapis rounded"
      >
        &#8592; Verify Guide
      </a>
      <a
        href="/help/glossary"
        class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none
               focus-visible:ring-2 focus-visible:ring-lapis rounded"
      >
        Glossary &#8594;
      </a>
    </div>
  </nav>

</article>
