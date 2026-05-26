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
    <p class="text-base text-text-light dark:text-quartz leading-relaxed max-w-2xl">
      Jura Trace uses multiple independent forensic detectors to assess content
      authenticity. No single detector is conclusive: the trust score reflects
      the combined weight of all available signals. This page explains each
      detector, how scores are computed, and the known limitations of automated
      analysis.
    </p>
    <div class="earth-line mt-6" aria-hidden="true"></div>
  </header>

  <!-- ── Table of contents ─────────────────────────────────────────────── -->
  <nav aria-label="Page contents" class="mb-10">
    <p class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-nav font-medium mb-3">Contents</p>
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
            class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none
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

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
      Every analysis produces a trust score between 0% and 100%. The score
      summarises how consistent the available evidence is with an unmanipulated,
      authentically captured piece of content. Think of it as the bedrock reading
      from all detection layers combined: the stronger and more consistent the
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
            <td class="py-2.5 pr-6 tabular-nums text-flint-dark dark:text-flint-light">71% – 100%</td>
            <td class="py-2.5 pr-6 text-malachite-dark dark:text-malachite-light font-medium">Authentic</td>
            <td class="py-2.5 text-flint-dark dark:text-flint-light">No detectors flagged anomalies. Content appears consistent with authentic capture.</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6">
              <span class="inline-flex items-center gap-1.5">
                <span class="w-2 h-2 rounded-full bg-amber dark:bg-amber-light flex-shrink-0" aria-hidden="true"></span>
                <span class="text-text-light dark:text-quartz font-medium">Moderate Trust</span>
              </span>
            </td>
            <td class="py-2.5 pr-6 tabular-nums text-flint-dark dark:text-flint-light">40% – 70%</td>
            <td class="py-2.5 pr-6 text-amber-dark dark:text-amber-light font-medium">Inconclusive</td>
            <td class="py-2.5 text-flint-dark dark:text-flint-light">Some signals raised concerns but evidence is not definitive. Human review is recommended.</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6">
              <span class="inline-flex items-center gap-1.5">
                <span class="w-2 h-2 rounded-full bg-cinnabar dark:bg-cinnabar-light flex-shrink-0" aria-hidden="true"></span>
                <span class="text-text-light dark:text-quartz font-medium">Low Trust</span>
              </span>
            </td>
            <td class="py-2.5 pr-6 tabular-nums text-flint-dark dark:text-flint-light">0% – 39%</td>
            <td class="py-2.5 pr-6 text-cinnabar-dark dark:text-cinnabar-light font-medium">Synthetic</td>
            <td class="py-2.5 text-flint-dark dark:text-flint-light">Multiple strong signals indicate manipulation or AI generation.</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Scoring formula -->
    <h3 id="scoring-formula" class="font-heading text-lg text-text-light dark:text-quartz mb-3 mt-6 leading-tight scroll-mt-20">
      Scoring Formula
    </h3>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      The trust score combines five components in a defined order. No single
      component can override the others when it strongly disagrees with the
      evidence. The full algorithm is open-source under AGPL-3.0 at
      <code class="font-mono text-xs">src-tauri/src/lib.rs::compute_trust</code>.
    </p>

    <ol class="space-y-3 text-sm text-flint-dark dark:text-flint-light mb-6 pl-4 list-decimal list-inside">
      <li>
        <span class="font-medium text-text-light dark:text-quartz">Forensic signal analysis (primary).</span>
        The worst-case score across the pixel-level detectors: ELA, noise
        residuals, copy-move detection, JPEG ghost analysis, the deepfake
        ensemble (GBM v4 + UnivFD probe), segmented ELA, and colour temperature.
        The worst-case approach (a <code class="font-mono text-xs">min()</code>
        across signals) prevents a single strong negative signal from being
        averaged away by clean results elsewhere. This is the dominant
        contributor to the score.
      </li>
      <li>
        <span class="font-medium text-text-light dark:text-quartz">EXIF metadata consistency (corroborating, capped at 20% weight).</span>
        Derived from the EXIF anomaly check (twelve base rules plus a five-check
        injection-detection suite for templated timestamps, integer-degree GPS,
        programmatic pipeline software, missing MakerNote on mandatory-vendor
        cameras, and iPhone sRGB mismatch, plus a two-check XMP AI-provenance
        suite for DigitalSourceType and AI-tooling CreatorTool values).
        Weighted at 20% because EXIF is trivially edited by any free tool and
        is stripped by most social-media platforms. A clean EXIF block cannot
        rescue a low forensic-trust score.
      </li>
      <li>
        <span class="font-medium text-text-light dark:text-quartz">C2PA provenance adjustment.</span>
        When a cryptographically valid C2PA manifest is present, the score
        receives a +0.10 uplift (capped at 100%). When the manifest itself
        declares AI generation (IPTC <code class="font-mono text-xs">trainedAlgorithmicMedia</code>),
        the score is reduced by 0.25. A self-declared origin is the strongest
        provenance signal we recognise.
      </li>
      <li>
        <span class="font-medium text-text-light dark:text-quartz">Composite-evidence cap (0.55 maximum).</span>
        When two independent regional detectors (segmented ELA, colour
        temperature) flag anomalies in the same image, the overall trust
        score is capped at 55% regardless of the weighted sum. Two
        independent regional detectors agreeing is a strong indicator of
        compositing that overrides clean whole-image analysis. (Shadow
        consistency and splice boundary are available as on-demand
        investigation tools but do not contribute to this cap.)
      </li>
      <li>
        <span class="font-medium text-text-light dark:text-quartz">Deepfake verdict ceiling.</span>
        When the deepfake ensemble returns a "synthetic" verdict, the
        score is capped at: 25% for high confidence, 35% for medium,
        45% for low. An "inconclusive" verdict caps at 55%. Screenshots
        and documents bypass this ceiling because AI detection is
        suppressed for those content types. This ceiling prevents a
        clean EXIF block or a present-but-unrelated C2PA manifest from
        inflating the score when the AI detector has already flagged
        the content.
      </li>
    </ol>

    <p class="text-xs text-flint-dark dark:text-flint-light mb-6 italic">
      No single weighting ratio captures the full calculation. The
      headline 20% EXIF / 80% forensic ratio describes only step 2 above;
      the composite-evidence cap and verdict ceiling can collapse the
      score to 25%-55% independent of the weighted sum. A C2PA-declared
      AI image, for example, lands in the 25% band regardless of how
      clean its EXIF is.
    </p>

    <!-- Document scoring -->
    <h3 class="font-heading text-lg text-text-light dark:text-quartz mb-3 mt-6 leading-tight">
      Document Scoring
    </h3>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
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
            <td class="py-2.5 pr-6 text-flint-dark dark:text-flint-light">Valid C2PA manifest present</td>
            <td class="py-2.5 tabular-nums text-malachite-dark dark:text-malachite-light font-medium">82%</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-flint-dark dark:text-flint-light">C2PA manifest present but invalid signature</td>
            <td class="py-2.5 tabular-nums text-cinnabar-dark dark:text-cinnabar-light font-medium">25%</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-flint-dark dark:text-flint-light">No C2PA manifest</td>
            <td class="py-2.5 tabular-nums text-amber-dark dark:text-amber-light font-medium">50%</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Not legal proof callout -->
    <div
      class="rounded border border-amber/25 bg-amber/5 dark:border-amber/20 dark:bg-amber/5 px-4 py-3 text-sm text-text-light dark:text-quartz leading-relaxed"
      role="note"
    >
      <span class="font-medium text-text-light dark:text-quartz">Important: </span>
      The trust score is a confidence indicator, not legal proof. A score of 100%
      means no detectors found anomalies. It does not mean the content is
      definitively authentic. See
      <a href="#limitations" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline">What This Does Not Prove</a>
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

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
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
            <td class="py-2.5 pr-6 tabular-nums text-flint-dark dark:text-flint-light">~15 s</td>
            <td class="py-2.5 pr-6 text-flint-dark dark:text-flint-light">EXIF anomaly, C2PA, ELA, AI detection</td>
            <td class="py-2.5 text-flint-dark dark:text-flint-light">Routine triage and quick authenticity checks</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 font-medium text-text-light dark:text-quartz">Deep</td>
            <td class="py-2.5 pr-6 tabular-nums text-flint-dark dark:text-flint-light">~60 s</td>
            <td class="py-2.5 pr-6 text-flint-dark dark:text-flint-light">All detectors, including regional analysis</td>
            <td class="py-2.5 text-flint-dark dark:text-flint-light">Investigating specific concerns or disputed content</td>
          </tr>
        </tbody>
      </table>
    </div>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
      Regional detectors (Segmented ELA, Shadow Consistency, Colour Temperature,
      Splice Boundary) are only active in Deep mode. Standard mode leaves those sections blank.
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

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      Expand each detector to learn what it measures, how it works, and when it
      may produce false positives. All detectors run locally on your device.
      No data is transmitted externally.
    </p>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
      The reference is divided into two groups. The ten <strong class="text-text-light dark:text-quartz font-medium">automatic detectors</strong>
      run on every verification at the mode indicated in each entry's
      <em>Active in modes</em> line. Their findings feed into the numeric trust
      score. A twelfth entry (Video Deepfake) is included for reference only: it is
      <strong class="text-text-light dark:text-quartz font-medium">not available in this release</strong>.
      Jura Trace v1.0 analyses still images only; video files are not supported in the
      Verify pipeline. Below the automatic detectors, an amber-tinted panel
      lists the three
      <strong class="text-text-light dark:text-quartz font-medium">on-demand investigation tools</strong>:
      these are available in Expert View and can be triggered manually when the
      automatic signals are ambiguous or when a specific question needs a targeted
      probe. The on-demand tools do not contribute to the numeric trust score.
    </p>

    <!--
      Detector lineup after Sprint 28 tech-debt audit (April 2026) and the
      S28 follow-up Option 2 reconciliation (April 2026):

      Automatic (in trust scoring) — 10 detectors at v1.0, matching the
      Rust detectors_run writer vocabulary in src-tauri/src/lib.rs and the
      TypeScript DETECTOR_ID_LABELS map in ui/src/lib/detectorLabels.ts
      (modulo `transcription` which is preprocessing infrastructure, not
      a forensic detector, and so lives only in detectorLabels for DB
      recording purposes):
        1. EXIF Anomaly
        2. C2PA Provenance
        3. ELA
        4. Noise Analysis
        5. Copy-Move
        6. AI Generation (GBM v4 + UnivFD v10onnx ensemble)
        7. JPEG Ghost (0.5× weight — S28-4)
        8. Segmented ELA
        9. Colour Temperature
        10. CLIP Detection (blends into the AI ensemble)

      Watermark Extraction was the 11th automatic detector through rc.x.
      Deferred to v1.1 on 2026-05-21 alongside the watermark embed UI
      (feature-flag V1_SHOW_WATERMARK=false). Backend extract code path
      stays in tree; sidecar bundle no longer includes invisible-watermark
      (PyInstaller torch-import regression risk). Re-enable in v1.1 when
      a torch-free implementation is in place.

      Under evaluation (NOT in v1.0 trust scoring):
        Video Deepfake — dropped from v1.0 on 2 May 2026 pending Global
        Majority device calibration. Under evaluation for a future
        release once Sora / Runway Gen-3 / HeyGen / Synthesia recall
        and platform-forwarded robustness are published. The detector
        entry remains in the reference below for transparency with an
        "Under evaluation" tag.

      Knowledge base retrieval aid (Claim Checker, formerly RAG):
        Deferred from v1.0 on 2026-05-17 pending corpus expansion and
        formal accuracy evaluation. The Python service code remains
        for re-enablement; the sidecar capability flag rag is false
        in v1.0 builds.

      On-demand investigation tools (NOT in trust scoring):
        NPR — demoted S28-3 (April 2026)
        Shadow Consistency — demoted April 2026
        Splice Boundary — demoted April 2026

      Removed entirely (scoring detectors):
        Chromatic Aberration — S28-1 (April 2026), audit 1/5 accuracy
        Diffusion artefacts — S28-2 (April 2026), superseded by UnivFD probe (now v9)
        Seasonal indicators — Sprint 27 April 2026, pseudoscience
        Weather scoring detector — Sprint 27 April 2026, was browser-mock only
          (NOTE: a manual Weather Context lookup IS live in Enhanced mode via
           fetch_weather_context / Open-Meteo; it does not feed the trust score.
           Documented on the Verify help page §10.)
    -->

    <h3 class="font-heading text-lg text-text-light dark:text-quartz mb-3 mt-2 tracking-heading">
      Automatic detectors
    </h3>

    <div class="space-y-2">

      <!-- 1. EXIF Anomaly Detection -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint-dark dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">01</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">EXIF Anomaly Detection</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Consistency of the metadata embedded in a file at the time of capture, including camera model, GPS coordinates, timestamps, software tags, and resolution values.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Applies consistency rules to the file's EXIF data: checking whether timestamps are plausible, whether GPS data matches declared location, whether a software editor tag has been added after capture, and whether resolution values are internally consistent. A further set of injection-detection sub-checks looks for fabricated or reconstructed metadata blocks: programmatic imaging libraries in the Software field (Pillow, ImageMagick, OpenCV), canonical template timestamps, GPS at exact integer degrees, cameras whose firmware always writes a MakerNote but where none is present, and iPhones declaring an sRGB colour space without a MakerNote. The XMP packet is also parsed for AI-provenance fields (<code class="text-xs text-text-light dark:text-quartz">Iptc4xmpExt:DigitalSourceType</code>, the IPTC ground-truth AI declaration, and <code class="text-xs text-text-light dark:text-quartz">xmp:CreatorTool</code>, generator name and version), so files that self-declare AI origin are flagged directly. The <code class="text-xs text-text-light dark:text-quartz">xmpMM:History</code> edit-history stack is parsed to surface timestamped edit lineage: if manipulation tool signatures (clone stamp, content-aware fill, healing brush, generative fill) appear in the history, a High-severity finding is raised; a compound signal also fires when a phone-captured image shows multiple save actions from a desktop editor.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                One or more metadata fields are missing, inconsistent, or contain signatures associated with editing software or scripted pipelines. This may indicate the metadata was stripped, altered, or fabricated after the original capture.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Intentional EXIF stripping for privacy (common before sharing images online), CMS or social media platforms that remove or rewrite metadata, and screenshots (which lack camera data by design). The injection sub-checks can also fire on legitimate scientific or archival workflows that re-encode images through Pillow or ImageMagick, on timer-triggered or time-lapse rigs that produce round-second timestamps, on third-party iPhone camera apps that write sRGB intentionally, and on images that have been through Google Photos, WhatsApp or similar platforms which strip MakerNote data. XMP provenance is a self-declaration: a file can assert <code class="text-xs text-text-light dark:text-quartz">digitalCapture</code> in its XMP even if the pixels were in fact generated, and an AI-generated file whose XMP packet has been stripped will never trigger the XMP checks. Treat the absence of an AI-provenance declaration as inconclusive, never as confirmation of authenticity. The multi-save history compound check only triggers on phone-vendor captures with three or more save actions from a desktop editor. Multiple saves from Photoshop on a DSLR capture is a normal RAW workflow and does not trigger.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint-dark dark:text-flint-light">Standard &#183; Deep</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Limited to metadata embedded at capture time. Cannot detect modifications to the image content itself, only inconsistencies in the surrounding metadata. Files stripped of all metadata produce no signal. The injection-detection sub-checks (programmatic library, template timestamp, integer GPS, MakerNote absence, iPhone sRGB) fire only on JPEG files with EXIF metadata present; stripped metadata produces no injection signal. XMP AI-provenance checks rely on self-declared metadata: a file can assert <code class="text-xs text-text-light dark:text-quartz">digitalCapture</code> in XMP even if the pixels were generated, and an AI-generated file whose XMP has been stripped will never trigger the XMP checks. The <code class="text-xs text-text-light dark:text-quartz">xmpMM:History</code> edit-history parser reads standard Photoshop history entries, which record application-level saves but do not record tool-level detail; Class H fires on non-standard history entries that do carry manipulation tool names.
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
            <span class="text-xs font-mono tabular-nums text-flint-dark dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">02</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">C2PA Provenance</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Whether the file carries a cryptographically signed provenance record: a digital certificate of origin created at the point of capture or production, following the Coalition for Content Provenance and Authenticity (C2PA) open standard.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Reads and cryptographically verifies the embedded C2PA manifest. If valid, extracts the claim generator field to detect whether a known AI creation tool signed the credentials. An AI-generated image signed by its creator will carry valid credentials. Those credentials are then treated as evidence of AI origin rather than evidence of authenticity.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                A valid manifest means the file's provenance chain is intact and has not been broken since signing. An invalid or absent manifest means the file cannot be verified via this standard. It does not mean the content is inauthentic.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Most existing images and documents do not carry C2PA credentials. Absence of credentials is not a negative finding: it simply means verification via this standard is not possible.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint-dark dark:text-flint-light">Standard &#183; Deep</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
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
            <span class="text-xs font-mono tabular-nums text-flint-dark dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">03</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Error Level Analysis (ELA)</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Differences in compression error across regions of an image. Every time a JPEG image is saved, it loses a predictable amount of information. ELA amplifies those differences to reveal regions that have been saved a different number of times from the rest of the image.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Re-compresses the image at a known quality level and subtracts the result from the original. In an unedited image, error levels are uniform across the frame. Regions pasted in from another source, or edited after the original compression, show higher error levels than the surrounding image and appear brighter in the ELA heatmap.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Uneven error levels suggest one or more regions were modified after the original file was created, or were composited from a source with a different compression history.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                High-detail areas (foliage, fabric, hair) naturally produce higher ELA values. Multiple rounds of social media recompression can produce uniform but elevated error levels across the whole image, reducing sensitivity. Modern codecs (AVIF, WebP) may show atypical patterns.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint-dark dark:text-flint-light">Standard &#183; Deep</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Unreliable on multiply-compressed JPEG images. Social media platforms compress images multiple times, creating ELA artefacts indistinguishable from manipulation. ELA's effective contribution is tempered in trust scoring to reflect this limitation; see the Signal Weighting table below for the current weight in `compute_trust`.
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
            <span class="text-xs font-mono tabular-nums text-flint-dark dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">04</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Noise Analysis</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                The distribution and consistency of noise grain across the image. Cameras introduce a characteristic noise pattern at the sensor level. AI-generated images often lack this natural grain distribution.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Divides the image into blocks and measures variance within each block. Compares the variance distribution across blocks against expected natural camera noise patterns. Blocks with significantly higher or lower noise than their neighbours are flagged as anomalous.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Unnaturally uniform noise across the image may indicate AI generation. Localised noise anomalies between regions may indicate compositing from sources with different sensor noise profiles.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Heavily sharpened images, images with applied noise reduction, screenshots, and illustrations all have non-camera-like noise profiles and may trigger this detector without indicating manipulation.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint-dark dark:text-flint-light">Standard &#183; Deep</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
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
            <span class="text-xs font-mono tabular-nums text-flint-dark dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">05</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Copy-Move Detection</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Whether any region of an image has been duplicated from another part of the same image: a technique commonly used to clone out unwanted content or replicate objects.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Extracts SIFT (Scale-Invariant Feature Transform) descriptors from image patches and performs nearest-neighbour self-matching with Lowe's ratio test. Matched pairs that are spatially separated but visually similar are filtered through RANSAC geometric verification and DBSCAN clustering to identify coherent clone regions.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                One or more regions appear to have been copied from elsewhere in the same image. This is a reliable indicator of manual editing with a clone stamp or healing brush tool. SIFT detects clones even when the copied region has been rotated or scaled before pasting.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Images with naturally repeating patterns (wallpaper, tiling, fabric, crowd scenes) may produce false matches, though Lowe's ratio test and RANSAC geometric verification significantly reduce these. Very small images or images with few distinguishable features are also more susceptible.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint-dark dark:text-flint-light">Standard &#183; Deep</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Detection requires that the cloned region contains enough gradient structure for SIFT keypoints. Flat, low-texture regions (plain skies, smooth walls) may not yield enough keypoints to detect copying even when a forgery is present. Performance also degrades on heavily compressed images where keypoints are destroyed. Clones in images below approximately 128&times;128 pixels are not attempted.
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
            <span class="text-xs font-mono tabular-nums text-flint-dark dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">06</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">AI Generation Detection</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Statistical patterns in the image that distinguish AI-generated imagery from photographs taken with a real camera. AI generation models leave characteristic fingerprints in noise, texture, and spectral distributions.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Extracts an 84-feature vector across six feature classes: <strong>noise statistics</strong> (LSB randomness, LSB entropy, LF/HF ratio, anisotropy); <strong>spectral decay patterns</strong>; <strong>Local Binary Pattern (LBP) texture descriptors</strong>; <strong>Grey-Level Co-occurrence Matrix (GLCM) contrast measures</strong>; <strong>demosaic inter-channel coherence</strong>; and <strong>PRNU sensor pattern consistency</strong>. The PRNU class measures spatial autocorrelation of the noise residual following the Lukáš/Fridrich/Goljan 2006 PRNU framework, which exposes asymmetric horizontal-vertical noise patterns that distinguish diffusion-model outputs from real camera sensor noise. The PRNU asymmetry signal surfaces in the verify-page detector list when it triggers. A GradientBoosting classifier (GBM v4, trained on over 10,000 images from 14 generator families, cross-validation AUC&#8209;ROC 0.9868, authentic false-positive rate 4.54%, calibrated threshold 0.49) assigns a probability score. This is combined with the UnivFD v10onnx probe (a LogisticRegression classifier on CLIP ViT-B/32 embeddings, trained on over 50,000 samples including platform-forwarded and multi-format augmentation across PNG, TIFF, WebP and HEIC, AUC&#8209;ROC 0.9929, authentic FP rate 3.87%, recall 95.77%) into an ensemble score.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                The statistical properties of the image are more consistent with AI generation than camera capture. Higher-confidence findings indicate multiple independent features pointing in the same direction.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Heavily processed photographs, CGI renders, composite illustrations, and images that have undergone multiple rounds of compression may exhibit AI-like statistical properties. The ensemble authentic false positive rate is 4.54% (GBM v4) and 3.87% (UnivFD v10onnx) on the held-out test set; human review is always warranted.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint-dark dark:text-flint-light">Standard &#183; Deep</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                GBM v4 trained on over 10,000 images across 14 generator families; UnivFD v10onnx trained on over 50,000 samples (including platform-forwarded plus multi-format PNG/TIFF/WebP/HEIC augmentation). May underperform on outputs from generators not represented in the training corpus. Both models are retrained as new generator families are identified. Minimum image size: 128&#215;128 pixels. Per-format AUC for v10onnx: PNG 0.998 / TIFF 0.995 / WebP 0.993 / HEIC 0.990. Per-generator recall varies with model version; full per-generator breakdown in the <a href="/help/model-cards#univfd-probe" class="text-lapis dark:text-lapis-light underline hover:no-underline">model card</a>. See also the <a href="/help/model-cards#gbm-classifier" class="text-lapis dark:text-lapis-light underline hover:no-underline">GBM model card</a> for full training data documentation.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 7. JPEG Ghost -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint-dark dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">07</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">JPEG Ghost</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Evidence of double compression: the signature left when a region of an image was previously saved as a JPEG at a different quality level before being composited into the final file.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Re-compresses the image at multiple quality levels and measures the deviation from the original in each block. Regions that show a minimum deviation at an unexpected quality level (different from the rest of the image) are flagged as potential JPEG ghosts, indicating they carry a different compression history from the surrounding content.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Part of the image appears to have been saved at a different JPEG quality setting from the rest, consistent with being spliced in from a separately compressed source.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Overlaid graphics or watermarks added in a different application, images assembled from multiple sources for legitimate purposes (collages, contact sheets), and very low-quality JPEG files where compression dominates the signal.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint-dark dark:text-flint-light">Standard &#183; Deep</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Only applicable to JPEG files. Produces no signal on PNG, WebP, TIFF, or other non-JPEG formats. The detector section is greyed out for non-JPEG inputs. Quality-adaptive weight applied: the effective trust-score contribution scales with the estimated JPEG quality factor (<code class="text-xs">effective_weight = 0.5 × max(jpeg_quality / 100, 0.3)</code>) to mitigate a structural blind spot on platform-forwarded content (Twitter/WhatsApp re-encoding wipes differential ghost signatures entirely, making the signal indistinguishable from authentic content at those quality levels).
              </dd>
            </div>
            <div id="jpeg-ghost-calibration">
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Weight calibration status</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                JPEG Ghost contributes to the trust score at a 0.5&#215; weight (half that of ELA, noise analysis, and copy-move detection). This weight is a cross-review consensus value, not an empirically measured one. The Sprint 28 calibration sweep (S28-FU9) could not produce meaningful true positive rate data because the synthetic training corpus uses single-resave PIL composites, which equalise DCT coefficients across the frame when the save quality approximates the background quality, exactly the condition JPEG Ghost is designed to detect. Meaningful calibration requires real-world single-JPEG-resave splice forgeries from a research benchmark such as CASIA v2. This is tracked as backlog item&#160;#10 (post-v1.0 research track). Full calibration results are in
                <code class="font-mono text-xs bg-gray-100 dark:bg-graphite-light px-1 py-0.5 rounded">docs/calibration/s28-jpeg-ghost-weight.md</code>.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 8. Segmented ELA -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint-dark dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">08</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Segmented ELA</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Regional variation in compression error levels across a grid of image segments. Where standard ELA analyses the whole image uniformly, Segmented ELA examines whether specific regions are inconsistent with their neighbours: the forensic equivalent of reading the strata in individual rock layers rather than the whole formation at once.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Divides the image into an 8&#215;8 grid (64 cells) and runs ELA independently on each cell. Applies cluster analysis to identify groups of cells with anomalously high error levels compared to the rest of the image.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Specific regions show compression error levels inconsistent with adjacent areas, suggesting those regions may have been added from a differently compressed source.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Images with highly varied content types within a single frame (a person against a smooth background, text overlaid on a photograph) naturally produce regional ELA variation. Intentionally added text, logos, or watermarks will flag strongly.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint-dark dark:text-flint-light">Deep</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Grid-based analysis divides images into 8&#215;8 cells. Artefacts in a single cell may indicate localised compression differences rather than deliberate manipulation. Consider alongside other regional detectors.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 9. Colour Temperature -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint-dark dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">09</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Colour Temperature</span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Whether the colour temperature (the warm or cool quality of the light) is consistent across different segments of the image. Elements photographed under different lighting conditions carry different colour casts even after global white balance adjustments.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Converts the image to the CIELAB perceptual colour space and segments it into regions. Analyses the warm/cool balance (the a and b channels) of each segment. Significant divergence between segments (particularly between foreground and background) is treated as an inconsistency indicator.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Different regions appear to have been captured under different lighting conditions. Combined with other regional signals, this increases confidence in a composite manipulation finding.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Images with naturally warm foreground and cool background (outdoor portrait photography, golden-hour scenes), intentional colour grading, and images that mix daylight and artificial light sources will show natural colour temperature inconsistencies.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint-dark dark:text-flint-light">Deep</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Assumes single-illuminant scenes. Mixed lighting conditions (e.g. tungsten + daylight, indoor/outdoor transitions) produce false positives. Results are most meaningful for outdoor scenes with consistent natural light.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 10. CLIP Detection -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint-dark dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">10</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">
              CLIP Detection
              <span class="ml-1.5 text-xs font-normal text-flint-dark dark:text-flint-light">(optional)</span>
            </span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Semantic-level characteristics of the image using a large-scale vision-language model. Rather than analysing low-level pixel statistics, CLIP Detection asks whether the overall image content appears consistent with AI-generated or authentic photographic output.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Passes the image through an OpenCLIP ViT&#8209;B/32 vision encoder and performs zero-shot classification against text descriptions of AI-generated versus authentic photography. Cosine similarity scores for each description are used to derive a confidence-weighted verdict. This detector requires an optional ~350 MB model download and loads lazily on first use.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                The image's visual semantics are more consistent with AI-generated content than with authentic photography. This is a complementary signal that operates at a different level of abstraction to the statistical detectors.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Highly stylised or artistic photography, heavily post-processed images, macro photography, and subjects statistically over-represented in AI training datasets may produce elevated AI probability scores.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint-dark dark:text-flint-light">Deep (only when the optional model is installed)</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                UnivFD v10onnx authentic false positive rate: 3.87% (improved 0.25 pp from v9, down from 5.01% in v8 and 28.7% in v7). Multi-format augmentation training (PNG/TIFF/WebP/HEIC) added to the existing platform-forwarded augmentation; per-format AUC stays above 0.99 across all four lossless / modern-lossy codecs. Non-photographic content (paintings, digital illustrations) may still trigger false positives. Requires the optional CLIP ViT&#8209;B/32 model (bundled, ~580&nbsp;MB combined). See the <a href="/help/model-cards#univfd-probe" class="text-lapis dark:text-lapis-light underline hover:no-underline">UnivFD model card</a> for full documentation.
              </dd>
            </div>
            <div id="clip-detection">
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Note: class probabilities are currently experimental</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                The class probability bars shown in the verify results are produced by feeding raw cosine similarity scores directly into a softmax function without applying the CLIP logit scale multiplier. This causes near-uniform distributions (~20% per class) regardless of the image content; the values do not reliably discriminate between authentic and AI-generated images. The UnivFD v10onnx probe (a trained logistic regression classifier on the same CLIP ViT&#8209;B/32 embeddings, AUC-ROC 0.9929, authentic FP 3.87%, AI recall 95.77%) is the production-grade path and contributes to the trust score separately. The class probability display is retained as an exploratory signal pending a fix to the softmax temperature and is marked <em>Experimental, informational only</em> in the verify interface.
              </dd>
            </div>
          </dl>
        </div>
      </details>

      <!-- 12. Video Deepfake Analysis -->
      <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
        <summary
          class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
        >
          <span class="flex items-center gap-3">
            <span class="text-xs font-mono tabular-nums text-flint-dark dark:text-flint-light w-5 flex-shrink-0" aria-hidden="true">12</span>
            <span class="font-medium text-sm text-text-light dark:text-quartz">Video Deepfake Analysis</span>
            <span
              class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-medium bg-lapis/10 text-lapis dark:text-lapis-light border border-lapis/20"
            >
              Under evaluation
            </span>
          </span>
          <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
            <span class="hidden group-open:inline">Close</span>
            <span class="group-open:hidden">Details</span>
          </span>
        </summary>
        <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
          <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mb-3 italic">
            Not available in this release. The methodology below describes
            the intended implementation for reference.
          </p>
          <dl class="space-y-3 text-sm">
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Whether a video file contains AI-generated or manipulated content, assessed frame by frame and across the temporal sequence. Temporal analysis catches inconsistencies between frames that would be invisible in a single-frame check.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Extracts evenly-spaced frames (6 in Standard mode, 20 in Deep) and runs the full AI Generation Detection pipeline on each frame. Three temporal consistency signals are then computed across the frame sequence: noise drift, spectral drift, and LBP texture drift. The overall score aggregates as: 50% mean frame score + 30% worst-case frame score + 20% temporal drift score.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Either individual frames exhibit AI-generation patterns, or the statistical properties of the video change over time in ways inconsistent with natural camera footage. High temporal drift alongside clean individual frames can indicate face-swap manipulation applied to otherwise authentic footage.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Video with intentional visual effects or colour grading transitions, footage assembled from multiple clips with different camera settings, and heavily compressed video (especially at low bitrates) can all produce temporal drift signals unrelated to manipulation.
              </dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
              <dd class="text-flint-dark dark:text-flint-light">Standard &#183; Deep (video files only)</dd>
            </div>
            <div>
              <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
              <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                Analyses a sample of frames (6 in Standard, 20 in Deep), not every frame. Manipulation confined to un-sampled frames may be missed. Temporal consistency signals (noise drift, spectral drift, LBP drift) require sufficient frame count for meaningful measurement. Requires FFmpeg for frame extraction.
              </dd>
            </div>
          </dl>
        </div>
      </details>


      <!-- On-demand investigation tools — visually distinct block -->
      <div
        class="mt-8 rounded-lg border border-amber/30 dark:border-amber/20 bg-amber/5 dark:bg-amber/[0.04] p-4"
        aria-labelledby="on-demand-heading"
      >
        <h3
          id="on-demand-heading"
          class="font-heading text-lg text-text-light dark:text-quartz mb-1 tracking-heading"
        >
          On-demand investigation tools
        </h3>

        <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
          These tools are available in Expert View and can be triggered manually when
          the automatic signals are ambiguous or a specific question needs a targeted
          probe. They are <strong class="font-medium text-text-light dark:text-quartz">not</strong>
          part of the automatic pipeline and do not contribute to the numeric trust score.
        </p>

        <div class="space-y-2">

        <!-- On-demand: NPR -->
        <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
          <summary
            class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
          >
            <span class="flex items-center gap-3">
                <span class="font-medium text-sm text-text-light dark:text-quartz">Neighbouring Pixel Relationships (NPR)</span>
            </span>
            <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
              <span class="hidden group-open:inline">Close</span>
              <span class="group-open:hidden">Details</span>
            </span>
          </summary>
          <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
            <dl class="space-y-3 text-sm">
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  How adjacent pixels relate to one another. Natural photographs have characteristic correlation patterns between neighbouring pixels, arising from optical blur, sensor interpolation, and scene continuity. AI generators produce pixels through a fundamentally different process that disturbs these relationships.
                </dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  Computes horizontal and vertical pixel correlation coefficients, the variance of pixel differences, and high-frequency energy ratios. These values are compared against empirical distributions from authentic photographs.
                </dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  Pixel relationship statistics deviate significantly from natural camera output. This is a complementary AI-detection signal that is independent of ELA and noise analysis.
                </dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  Heavily upscaled images, images with strong sharpening filters, and artwork or illustrations all exhibit non-photographic pixel relationships and will typically trigger this detector.
                </dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
                <dd class="text-flint-dark dark:text-flint-light">On-demand investigation tool (not part of the automatic pipeline)</dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  Demoted to on-demand in Sprint 28 (April 2026). Content-authenticity-expert cross-review noted that the Tan et al. AAAI 2024 paper uses NPR features as input to a learned classifier, not as a standalone threshold, and that a hand-tuned NPR statistic is partially redundant with the UnivFD v10onnx probe which encodes upsampling artefacts at a higher level of abstraction via CLIP features. The sidecar endpoint remains available for manual investigation. Also computationally intensive and less effective on highly compressed content where pixel neighbour relationships are already disrupted by quantisation.
                </dd>
              </div>
            </dl>
          </div>
        </details>

        <!-- On-demand: Shadow Consistency -->
        <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
          <summary
            class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
          >
            <span class="flex items-center gap-3">
              <span class="font-medium text-sm text-text-light dark:text-quartz">Shadow Consistency</span>
            </span>
            <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
              <span class="hidden group-open:inline">Close</span>
              <span class="group-open:hidden">Details</span>
            </span>
          </summary>
          <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
            <dl class="space-y-3 text-sm">
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  Whether the implied direction of light is consistent across different regions of the image. In an authentic photograph, shadows and highlights all point away from the same light source. Composite images (where elements were photographed under different lighting conditions) frequently fail this check.
                </dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  Divides the image into regions and computes a gradient-weighted estimate of light direction (expressed as an angle) for each region. Compares estimated light directions across regions. Significant angular disagreement (weighted by the strength of the gradient signal) is treated as evidence of inconsistent lighting.
                </dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  Different parts of the image appear to have been lit from different directions, suggesting elements were photographed or generated separately and composited together.
                </dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  Scenes with multiple artificial light sources (studio setups, concert photography, street scenes at night), reflective surfaces, and images with strong background/foreground separation can legitimately show regional lighting inconsistencies.
                </dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
                <dd class="text-flint-dark dark:text-flint-light">On-demand investigation tool (not part of the automatic pipeline)</dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  Demoted to on-demand in April 2026. The gradient-weighted light direction estimate is noisy on textured scenes and cluttered backgrounds, and the forensic audit concluded it adds scoring noise without reliable discrimination. The canonical shadow-constraint technique (Kee, O'Brien &amp; Farid 2013) requires user-placed shadow/object point pairs and is a better fit as a manual ROI tool, not an automatic detector. Available in Expert View for manual inspection of light direction.
                </dd>
              </div>
            </dl>
          </div>
        </details>

        <!-- On-demand: Splice Boundary -->
        <details class="group rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite">
          <summary
            class="flex items-center justify-between gap-3 px-4 py-3 cursor-pointer list-none
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis rounded"
          >
            <span class="flex items-center gap-3">
              <span class="font-medium text-sm text-text-light dark:text-quartz">Splice Boundary</span>
            </span>
            <span class="flex-shrink-0 text-xs text-flint-dark dark:text-flint-light select-none">
              <span class="hidden group-open:inline">Close</span>
              <span class="group-open:hidden">Details</span>
            </span>
          </summary>
          <div class="px-4 pb-4 pt-3 border-t border-border-light dark:border-border-dark">
            <dl class="space-y-3 text-sm">
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What it measures</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  The physical boundary where one image region ends and another begins: the cut edge produced when elements are composited. Three independent edge signals are combined to localise these boundaries.
                </dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">How it works</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  Analyses three signals simultaneously: (1) JPEG DCT grid discontinuities, abrupt changes in the compression block pattern at potential splice points; (2) noise level changes, sudden shifts in noise grain across a boundary; (3) feathering artefacts, the soft-edge signature left by selection tools and layer masking. Agreement between multiple signals at the same location substantially increases confidence.
                </dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">What a positive finding means</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  One or more signals detected an anomalous edge inconsistent with natural image content. When corroborated by Segmented ELA flagging the same region, this is the highest-confidence composite manipulation signal Jura Trace can produce.
                </dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known false positive triggers</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  Hard vignettes, image borders, intentionally added frames or decorative edges, and embedded watermarks or logos all create artificial boundaries that can trigger this detector. Cropped images may show strong grid discontinuities at the crop boundary.
                </dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Active in modes</dt>
                <dd class="text-flint-dark dark:text-flint-light">On-demand investigation tool (not part of the automatic pipeline)</dd>
              </div>
              <div>
                <dt class="font-medium text-text-light dark:text-quartz mb-0.5">Known Limitations</dt>
                <dd class="text-flint-dark dark:text-flint-light leading-relaxed">
                  Demoted to on-demand in April 2026. The three-signal fusion (JPEG grid alignment, noise asymmetry, feathering) is heuristic stacking without published validation, and the forensic audit found the detector never set suspicious=true in production, contributing noise without adding discriminative value. Available in Expert View for manual inspection. A future replacement using learned splice localisation (TruFor / MVSS-Net) is backlog work.
                </dd>
              </div>
            </dl>
          </div>
        </details>

        </div><!-- /on-demand space-y-2 -->
      </div><!-- /on-demand investigation tools wrapper -->

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

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
      Jura Trace is a forensic aid. Like all forensic tools, it has limits. The
      following limitations apply to every analysis.
    </p>

    <ul class="space-y-5">

      <li class="flex gap-3 text-sm">
        <span
          class="flex-shrink-0 mt-0.5 w-4 text-center font-bold text-cinnabar-dark dark:text-cinnabar-light"
          aria-hidden="true"
        >&#215;</span>
        <div>
          <p class="font-medium text-text-light dark:text-quartz mb-1">Automated analysis cannot prove authenticity.</p>
          <p class="text-flint-dark dark:text-flint-light leading-relaxed">
            A high trust score means no detectors found anomalies. It does not mean the
            content is definitively authentic. It means analysis found nothing to indicate
            otherwise.
          </p>
        </div>
      </li>

      <li class="flex gap-3 text-sm">
        <span
          class="flex-shrink-0 mt-0.5 w-4 text-center font-bold text-cinnabar-dark dark:text-cinnabar-light"
          aria-hidden="true"
        >&#215;</span>
        <div>
          <p class="font-medium text-text-light dark:text-quartz mb-1">A low trust score may reflect legitimate processing.</p>
          <p class="text-flint-dark dark:text-flint-light leading-relaxed">
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
          class="flex-shrink-0 mt-0.5 w-4 text-center font-bold text-cinnabar-dark dark:text-cinnabar-light"
          aria-hidden="true"
        >&#215;</span>
        <div>
          <p class="font-medium text-text-light dark:text-quartz mb-1">Scores should inform human judgement, not replace it.</p>
          <p class="text-flint-dark dark:text-flint-light leading-relaxed">
            No forensic detector has a zero false positive or false negative rate. Results
            must be interpreted by a human, in context, alongside other available evidence.
            The "Know What's Real" tagline reflects an aspiration, not a guarantee that
            analysis will always reach the correct conclusion.
          </p>
        </div>
      </li>

      <li class="flex gap-3 text-sm">
        <span
          class="flex-shrink-0 mt-0.5 w-4 text-center font-bold text-cinnabar-dark dark:text-cinnabar-light"
          aria-hidden="true"
        >&#215;</span>
        <div>
          <p class="font-medium text-text-light dark:text-quartz mb-1">Jura Trace is not a legal authority.</p>
          <p class="text-flint-dark dark:text-flint-light leading-relaxed">
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
          class="flex-shrink-0 mt-0.5 w-4 text-center font-bold text-cinnabar-dark dark:text-cinnabar-light"
          aria-hidden="true"
        >&#215;</span>
        <div>
          <p class="font-medium text-text-light dark:text-quartz mb-1">Detectors are trained on current AI generation techniques.</p>
          <p class="text-flint-dark dark:text-flint-light leading-relaxed">
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

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-6">
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
            <td class="py-2.5 text-flint-dark dark:text-flint-light">Most reliable single manipulation indicator</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Segmented ELA</td>
            <td class="py-2.5 pr-6 tabular-nums font-medium text-text-light dark:text-quartz">1.5</td>
            <td class="py-2.5 text-flint-dark dark:text-flint-light">Regional variant; high specificity for compositing</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Colour Temperature</td>
            <td class="py-2.5 pr-6 tabular-nums font-medium text-text-light dark:text-quartz">1.5</td>
            <td class="py-2.5 text-flint-dark dark:text-flint-light">Strong composite indicator when consistent with other regional signals</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Noise Analysis</td>
            <td class="py-2.5 pr-6 tabular-nums font-medium text-text-light dark:text-quartz">1.0</td>
            <td class="py-2.5 text-flint-dark dark:text-flint-light">Standard weight; useful for both AI detection and compositing</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Copy-Move Detection</td>
            <td class="py-2.5 pr-6 tabular-nums font-medium text-text-light dark:text-quartz">1.0</td>
            <td class="py-2.5 text-flint-dark dark:text-flint-light">Standard weight; specific to clone stamp manipulation</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Shadow Consistency</td>
            <td class="py-2.5 pr-6 tabular-nums text-flint-dark dark:text-flint-light italic">On-demand</td>
            <td class="py-2.5 text-flint-dark dark:text-flint-light">Demoted Sprint 28: on-demand investigation tool only, does not contribute to trust score</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">Splice Boundary</td>
            <td class="py-2.5 pr-6 tabular-nums text-flint-dark dark:text-flint-light italic">On-demand</td>
            <td class="py-2.5 text-flint-dark dark:text-flint-light">Demoted Sprint 28: on-demand investigation tool only, does not contribute to trust score</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-6 text-text-light dark:text-quartz">AI Generation Detection</td>
            <td class="py-2.5 pr-6 text-flint-dark dark:text-flint-light italic">Independent</td>
            <td class="py-2.5 text-flint-dark dark:text-flint-light">Considered via worst-case combination with manipulation score; not pooled into the weighted sum</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Concordance check -->
    <h3 class="font-heading text-lg text-text-light dark:text-quartz mb-3 mt-6 leading-tight">
      Concordance Check
    </h3>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      When ELA and the AI Generation Detection ensemble both return clean results,
      but other signals (noise analysis, copy-move) flag concerns, Jura Trace
      applies a concordance dampening factor. When the two most reliable detectors
      agree that the image is clean, the weight of disagreeing secondary signals
      is reduced.
    </p>

    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
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
        class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none
               focus-visible:ring-2 focus-visible:ring-lapis rounded"
      >
        &#8592; Verify Guide
      </a>
      <a
        href="/help/glossary"
        class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none
               focus-visible:ring-2 focus-visible:ring-lapis rounded"
      >
        Glossary &#8594;
      </a>
    </div>
  </nav>

</article>
