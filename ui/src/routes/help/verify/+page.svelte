<script lang="ts">
  // No reactive state needed — this is a static help page.
  // Section IDs are used for deep linking from contextual help buttons.
</script>

<!-- ── Page header ────────────────────────────────────────────────── -->
<header class="mb-8">
  <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
    Verifying Content Authenticity
  </h1>
  <p class="text-base text-flint dark:text-flint-light leading-relaxed max-w-2xl">
    The Verify section examines a file through multiple independent analysis signals and
    returns a verdict alongside a detailed breakdown. Every layer of the investigation —
    from EXIF anomalies to deep-learning AI detection — runs entirely on your device. No
    file data leaves Jura Trace.
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
      <a href="#what-verify-does"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        1. What Verify Does
      </a>
    </li>
    <li>
      <a href="#input-methods"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        2. Input Methods
      </a>
    </li>
    <li>
      <a href="#investigation-modes"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        3. Investigation Modes
      </a>
    </li>
    <li>
      <a href="#trust-score"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        4. Understanding the Trust Score
      </a>
    </li>
    <li>
      <a href="#verdicts"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        5. Understanding Verdicts
      </a>
    </li>
    <li>
      <a href="#reading-results"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        6. Reading the Results
      </a>
    </li>
    <li>
      <a href="#video-and-audio"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        7. Video and Audio
      </a>
    </li>
    <li>
      <a href="#investigate-further"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        8. Investigate Further
      </a>
    </li>
    <li>
      <a href="#exporting-results"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        9. Exporting Results
      </a>
    </li>
    <li>
      <a href="#document-analysis"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        10. Document Analysis
      </a>
    </li>
  </ol>
</nav>

<!-- ══════════════════════════════════════════════════════════════════
     1. What Verify Does
     ══════════════════════════════════════════════════════════════════ -->
<section id="what-verify-does" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    1. What Verify Does
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    When you submit a file for verification, Jura Trace runs it through several distinct
    analysis layers, each looking for different types of evidence. The results are combined
    into a single trust score and a three-way verdict.
  </p>

  <ul class="space-y-3 mb-6 text-sm text-flint dark:text-flint-light leading-relaxed">
    <li class="flex gap-3">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold mt-0.5">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">C2PA credential checking</strong> —
        reads any embedded Content Credentials manifest and verifies the content hash against
        the current file data.
      </span>
    </li>
    <li class="flex gap-3">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold mt-0.5">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">EXIF metadata analysis</strong> —
        checks the metadata embedded in the file for internal inconsistencies and anomalies
        that may indicate manipulation.
      </span>
    </li>
    <li class="flex gap-3">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold mt-0.5">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Image forensic analysis</strong> —
        a suite of detectors (ELA, noise analysis, copy-move detection, NPR, chromatic
        aberration, JPEG ghost, and regional analysis) that look for traces of compositing,
        re-touching, or format manipulation at the pixel level.
      </span>
    </li>
    <li class="flex gap-3">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold mt-0.5">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">AI-generation detection</strong> —
        a trained ensemble classifier that distinguishes characteristics common to
        AI-generated images from those of camera-captured photographs.
      </span>
    </li>
    <li class="flex gap-3">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold mt-0.5">→</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Watermark extraction</strong> —
        checks whether an invisible Jura Trace watermark is present and, if so, which asset
        record it corresponds to.
      </span>
    </li>
  </ul>

  <!-- Callout: local-first -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">All analysis is local.</strong>
      No file data, no metadata, and no analysis results are sent to any external server.
      The forensic pipeline runs on your device using the Analysis Engine and the Rust
      core engine.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     2. Input Methods
     ══════════════════════════════════════════════════════════════════ -->
<section id="input-methods" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    2. Input Methods
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The Verify page offers three ways to submit content for analysis. Select an input method
    using the tabs at the top of the page.
  </p>

  <div class="space-y-5">
    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        File
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Drag a file onto the drop area, or click <strong class="text-text-light dark:text-text-dark">Browse</strong>
        to open the file picker. Jura Trace detects the format from the file's magic bytes —
        not the extension — so a JPEG saved as <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">.png</span>
        will still be handled correctly. Supported formats include JPEG, PNG, TIFF, WebP,
        MP4, MOV, WAV, MP3, and PDF.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Batch
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Select multiple files at once. Jura Trace analyses each file in sequence and shows
        individual results in a list. Useful when reviewing a set of images from a single
        news event or collection.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        URL
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-2">
        Paste a public URL and Jura Trace will fetch the resource and analyse it. The
        request is made from your device — no proxy or cloud service is involved.
      </p>
      <div class="bg-graphite rounded-lg border border-border-dark p-4">
        <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
          <strong class="text-text-light dark:text-text-dark">Security note:</strong>
          Jura Trace will not fetch URLs pointing to your local network, private IP address
          ranges, or loopback addresses (e.g.
          <span class="font-mono text-xs">localhost</span>,
          <span class="font-mono text-xs">192.168.x.x</span>). This prevents
          accidental or malicious requests to internal services.
        </p>
      </div>
    </div>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     3. Investigation Modes
     ══════════════════════════════════════════════════════════════════ -->
<section id="investigation-modes" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    3. Investigation Modes
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Choose a mode before submitting a file. Each mode runs a different set of detectors
    and trades speed for depth. The selector is shown below the input area.
  </p>

  <div class="overflow-x-auto mb-6">
    <table class="w-full text-sm border-collapse">
      <thead>
        <tr class="border-b border-border-light dark:border-border-dark">
          <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Mode</th>
          <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Typical time</th>
          <th class="text-left py-2 text-text-light dark:text-text-dark font-semibold">Detectors included</th>
        </tr>
      </thead>
      <tbody class="text-flint dark:text-flint-light">
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-3 pr-4 font-semibold text-text-light dark:text-text-dark align-top">Standard</td>
          <td class="py-3 pr-4 align-top whitespace-nowrap">~15 s</td>
          <td class="py-3">
            EXIF anomaly analysis, C2PA credential check, ELA (Error Level Analysis),
            AI-generation detection, watermark extraction, noise analysis, copy-move
            detection, JPEG ghost, NPR, chromatic aberration
          </td>
        </tr>
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-3 pr-4 font-semibold text-text-light dark:text-text-dark align-top">Deep</td>
          <td class="py-3 pr-4 align-top whitespace-nowrap">~60 s</td>
          <td class="py-3">
            All Standard detectors, plus: segmented ELA (8 × 8 regional grid), shadow
            consistency analysis, colour temperature analysis, splice boundary detection.
            Recommended for suspected composites or content where Standard returns an
            inconclusive result.
          </td>
        </tr>
        <tr>
          <td class="py-3 pr-4 font-semibold text-text-light dark:text-text-dark align-top">Archival</td>
          <td class="py-3 pr-4 align-top whitespace-nowrap">~90 s</td>
          <td class="py-3">
            All Deep detectors with scanner-calibrated tolerances tuned for digitised
            material. Adds full chain-of-custody audit logging. Use this mode for legal
            evidence handling, institutional acquisition, or any context requiring a
            complete evidence record.
          </td>
        </tr>
      </tbody>
    </table>
  </div>

  <!-- Tip: which mode to use -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Not sure which to choose?</strong>
      Start with Standard. If the result is inconclusive and the content is high-stakes,
      re-run in Deep mode. Reserve Archival mode for situations where you need a
      court-admissible or formally audited evidence record.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     4. Understanding the Trust Score
     ══════════════════════════════════════════════════════════════════ -->
<section id="trust-score" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    4. Understanding the Trust Score
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The trust score is a percentage (0–100%) that expresses how much provenance evidence
    Jura Trace found in favour of the content being authentic. It is displayed as a
    coloured bar and a number at the top of the results panel.
  </p>

  <div class="overflow-x-auto mb-6">
    <table class="w-full text-sm border-collapse">
      <thead>
        <tr class="border-b border-border-light dark:border-border-dark">
          <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Band</th>
          <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Score</th>
          <th class="text-left py-2 text-text-light dark:text-text-dark font-semibold">Interpretation</th>
        </tr>
      </thead>
      <tbody class="text-flint dark:text-flint-light">
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4 font-semibold text-malachite dark:text-malachite-light">High Trust</td>
          <td class="py-2 pr-4">71–100%</td>
          <td class="py-2">Strong provenance evidence. Multiple signals are consistent with authenticity.</td>
        </tr>
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4 font-semibold text-amber dark:text-amber-light">Moderate</td>
          <td class="py-2 pr-4">41–70%</td>
          <td class="py-2">Mixed or limited evidence. Some signals are normal; others are flagged. Use with caution.</td>
        </tr>
        <tr>
          <td class="py-2 pr-4 font-semibold text-cinnabar dark:text-cinnabar-light">Low Trust</td>
          <td class="py-2 pr-4">0–40%</td>
          <td class="py-2">Significant anomalies detected. Multiple signals suggest manipulation or synthetic origin.</td>
        </tr>
      </tbody>
    </table>
  </div>

  <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    How the score is calculated
  </h3>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
    The trust score combines two independent strands of evidence:
  </p>

  <ul class="space-y-2 mb-4 text-sm text-flint dark:text-flint-light leading-relaxed">
    <li class="flex gap-2">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold">40%</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">EXIF metadata analysis</strong> —
        checks camera make and model, GPS coordinates, software fields, and timestamp
        consistency.
      </span>
    </li>
    <li class="flex gap-2">
      <span class="text-lapis dark:text-lapis-light flex-none font-semibold">60%</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Forensic signal analysis</strong> —
        combines the outputs from all active detectors (ELA, noise, copy-move, and so on),
        weighted by confidence.
      </span>
    </li>
  </ul>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    When two or more regional detectors fire together (for example, segmented ELA and shadow
    inconsistency in the same image), the forensic contribution is amplified to reflect the
    compounding evidence — up to a maximum cap. This prevents a single marginal signal from
    dominating the score whilst still surfacing genuine multi-signal anomalies.
  </p>

  <!-- Important caveat -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">The trust score is not a guarantee.</strong>
      A score of 85% does not mean the content is definitely authentic. A score of 20% does
      not mean it is definitely synthetic. The score reflects the weight of available
      evidence, which is always incomplete. Use it alongside your own professional judgement
      and additional investigation where appropriate.
    </p>
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mt-2">
      For a full explanation of every detector and its weighting, see
      <a href="/help/methodology" class="text-lapis dark:text-lapis-light hover:underline">
        How Analysis Works
      </a>.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     5. Understanding Verdicts
     ══════════════════════════════════════════════════════════════════ -->
<section id="verdicts" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    5. Understanding Verdicts
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    Jura Trace returns one of three verdicts, shown in the verdict badge at the top of the
    results panel. The verdict is determined by the AI-detection ensemble — specifically by
    the deepfake score — not by the trust score. The two figures are related but independent.
  </p>

  <div class="space-y-4 mb-6">
    <div class="bg-graphite rounded-lg border border-border-dark p-4">
      <div class="flex items-center gap-3 mb-2">
        <span class="inline-block px-2 py-0.5 rounded text-xs font-semibold bg-malachite/10 text-malachite dark:text-malachite-light border border-malachite/20">
          Authentic
        </span>
      </div>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        The AI-detection ensemble found no significant indicators of synthetic generation.
        The deepfake score is below 0.30. This verdict means the detectors are consistent
        with a camera-captured or otherwise non-AI-generated image — it does not rule out
        human editing or compositing.
      </p>
    </div>

    <div class="bg-graphite rounded-lg border border-border-dark p-4">
      <div class="flex items-center gap-3 mb-2">
        <span class="inline-block px-2 py-0.5 rounded text-xs font-semibold bg-amber/10 text-amber dark:text-amber-light border border-amber/20">
          Inconclusive
        </span>
      </div>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        The deepfake score falls between 0.30 and 0.65. Evidence is present but not
        sufficient to make a confident determination. <strong class="text-text-light dark:text-text-dark">Inconclusive
        is the most common and expected outcome for real-world content.</strong> Many authentic
        photographs — particularly those that have been edited, cropped, or heavily
        compressed — will return an inconclusive result. Treat it as a signal to investigate
        further, not as evidence of manipulation.
      </p>
    </div>

    <div class="bg-graphite rounded-lg border border-border-dark p-4">
      <div class="flex items-center gap-3 mb-2">
        <span class="inline-block px-2 py-0.5 rounded text-xs font-semibold bg-cinnabar/10 text-cinnabar dark:text-cinnabar-light border border-cinnabar/20">
          Synthetic
        </span>
      </div>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        The deepfake score exceeds 0.65, or an AI-generation watermark was detected in the
        file metadata. The detectors found strong indicators consistent with AI-generated
        content. This verdict warrants careful review and corroboration before acting on it.
      </p>
    </div>
  </div>

  <!-- Tip: verdicts vs trust score -->
  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      <strong class="text-text-light dark:text-text-dark">Verdict and trust score can diverge.</strong>
      An image may return an <em>Authentic</em> verdict (low deepfake score) but a low trust
      score (many EXIF anomalies). Conversely, a <em>Synthetic</em> verdict with a moderate
      trust score may indicate AI-assisted editing rather than full AI generation. Read both
      together for the most complete picture.
    </p>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     6. Reading the Results
     ══════════════════════════════════════════════════════════════════ -->
<section id="reading-results" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    6. Reading the Results
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
    Once analysis completes, the results panel unfolds below the input area. It is organised
    into sections you can work through from top to bottom.
  </p>

  <div class="space-y-6">

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Verdict badge and trust score bar
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        The verdict badge (Authentic / Inconclusive / Synthetic) and the trust score bar
        appear at the very top of the results. A confidence indicator shows how many
        detectors contributed to the verdict — a higher count means more evidence was
        available.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Metadata section
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Shows the file's EXIF data and any anomalies detected. Each anomaly is listed with a
        brief explanation of why it is considered unusual. A higher number of anomalies
        contributes to a lower EXIF trust contribution.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        C2PA credential panel
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        If a C2PA manifest is present, this panel shows the claim generator, signing
        timestamp, and whether the content hash is valid. A hash mismatch is shown in red
        and means the file has been modified after signing.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Forensic details (expandable)
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-2">
        Each forensic detector appears as an expandable row. Click any row to see the full
        signal detail — including visualisation heatmaps for ELA and noise analysis, matched
        clone regions for copy-move detection, and the feature breakdown for AI detection.
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        The Signal Agreement panel summarises which detectors agree and which disagree.
        When multiple independent detectors flag the same concern, the agreement is stronger
        evidence than any single signal alone.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Region analysis (Deep and Archival modes only)
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        When you run Deep or Archival mode, a region analysis section appears showing
        results from the four regional detectors: segmented ELA, shadow consistency, colour
        temperature, and splice boundary. These detectors divide the image into a grid and
        look for inconsistencies between regions — a key signal for composite images where
        different sections originate from different sources.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Watermark panel
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Shows whether an invisible Jura Trace watermark was detected. If a watermark is
        found, the panel displays the institution name and confidence level. A match against
        your local asset catalogue confirms the file's origin.
      </p>
    </div>

  </div>

  [Screenshot: Full results panel showing the verdict badge set to "Inconclusive", a trust score bar at 61%, the EXIF metadata section with two anomalies highlighted, and an expanded ELA forensic row with a heatmap visualisation]
</section>

<!-- ══════════════════════════════════════════════════════════════════
     7. Video and Audio
     ══════════════════════════════════════════════════════════════════ -->
<section id="video-and-audio" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    7. Video and Audio
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
    Jura Trace handles video and audio files with dedicated analysis pipelines. Some features
    require FFmpeg to be installed on your system — the Settings page shows whether FFmpeg
    is available.
  </p>

  <div class="space-y-6">

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Video metadata
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        For MP4 and MOV files, the metadata section shows codec, resolution, frame rate,
        duration, and audio track information extracted via FFprobe. Anomalies in these
        fields — such as a codec that does not match the container, or a resolution
        inconsistent with the declared camera model — are flagged in the same way as EXIF
        anomalies.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Frame timeline and per-frame deepfake analysis
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
        Jura Trace extracts evenly-spaced frames from a video and runs the image deepfake
        pipeline on each one. The results appear as a timeline strip — each frame is shown
        as a thumbnail with a coloured score badge.
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-3">
        Click any frame in the timeline to expand it. The expanded view shows the per-frame
        classifier score, individual signal values, and a heatmap visualisation. This lets
        you identify specific moments in a video that flag as suspicious.
      </p>
      <div class="overflow-x-auto mb-2">
        <table class="w-full text-sm border-collapse">
          <thead>
            <tr class="border-b border-border-light dark:border-border-dark">
              <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Mode</th>
              <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Frames analysed</th>
              <th class="text-left py-2 text-text-light dark:text-text-dark font-semibold">Estimated time</th>
            </tr>
          </thead>
          <tbody class="text-flint dark:text-flint-light">
            <tr class="border-b border-border-light/50 dark:border-border-dark/50">
              <td class="py-2 pr-4">Standard</td>
              <td class="py-2 pr-4">6</td>
              <td class="py-2">~12 s</td>
            </tr>
            <tr class="border-b border-border-light/50 dark:border-border-dark/50">
              <td class="py-2 pr-4">Deep</td>
              <td class="py-2 pr-4">20</td>
              <td class="py-2">~40 s</td>
            </tr>
            <tr>
              <td class="py-2 pr-4">Archival</td>
              <td class="py-2 pr-4">40</td>
              <td class="py-2">~80 s</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        The aggregate video verdict combines the per-frame scores using the formula
        0.5 × mean score + 0.3 × maximum score + 0.2 × temporal consistency score.
        The temporal consistency signals — noise drift, spectral drift, and LBP drift —
        detect discontinuities between frames that can indicate stitching or deepfake
        artefacts.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Audio metadata
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        For WAV and MP3 files, the metadata panel shows codec, sample rate, channels,
        bitrate, and duration. C2PA signing applies to both audio formats.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Transcription
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        When the <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">faster-whisper</span>
        package is installed, Jura Trace automatically transcribes audio and video files.
        The transcript appears in a collapsible panel below the metadata section. If Ollama
        is running, the transcript is also passed to the RAG claim checker, which looks for
        verifiable factual claims in the content.
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mt-2">
        Transcription is optional. If
        <span class="font-mono text-xs bg-graphite/60 dark:bg-graphite-light/20 px-1 py-0.5 rounded">faster-whisper</span>
        is not installed, the analysis pipeline completes normally without a transcript.
        The first use downloads the speech model automatically — approximately 500 MB.
      </p>
    </div>

  </div>

  [Screenshot: Video verify results showing a frame timeline strip with six thumbnail frames, two of which have amber score badges, and one expanded frame showing the per-frame deepfake heatmap]
</section>

<!-- ══════════════════════════════════════════════════════════════════
     8. Investigate Further
     ══════════════════════════════════════════════════════════════════ -->
<section id="investigate-further" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    8. Investigate Further
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    An inconclusive result is not the end of the investigation. Jura Trace provides tools
    and guidance to help you dig deeper.
  </p>

  <div class="space-y-5">

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Reverse image search
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-2">
        The results panel includes links to external reverse image search services. These
        can help you find earlier versions of an image, identify its origin, or find other
        contexts in which it has been published.
      </p>
      <div class="bg-graphite rounded-lg border border-border-dark p-4">
        <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
          <strong class="text-text-light dark:text-text-dark">Privacy note:</strong>
          Using reverse image search requires uploading or sharing the image with an external
          service. If the content is sensitive, unpublished, or subject to confidentiality,
          do not use these links. The links are provided for convenience — Jura Trace does
          not send any data to these services on your behalf.
        </p>
      </div>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Manual visual inspection
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        The inspection checklist (available via the Checklist button in the results panel)
        guides you through eight manual checks — looking for shadow inconsistencies,
        unnatural skin texture, misaligned text, and other artefacts that automated
        detectors can miss. The checklist is designed for trained reviewers but is accessible
        to any careful observer.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Re-run in a deeper mode
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        If Standard mode returned inconclusive and you want the full set of regional
        detectors, use the mode selector to switch to Deep or Archival and run the analysis
        again. You do not need to re-import the file.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        Report a false positive
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        If you believe a result is incorrect — for example, a known authentic photograph
        returned a Synthetic verdict — use the "Report False Positive" button. This records
        the case locally (no data is shared externally) and contributes to improving the
        classifier over time.
      </p>
    </div>

  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     9. Exporting Results
     ══════════════════════════════════════════════════════════════════ -->
<section id="exporting-results" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    9. Exporting Results
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
    Once analysis is complete, you can export the results in two formats. The export buttons
    appear at the bottom of the results panel.
  </p>

  <div class="space-y-5 mb-6">

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        PDF trust report
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-2">
        A formatted document containing the verdict, trust score, investigation mode, a
        summary of each detector's findings, EXIF metadata, and C2PA manifest details where
        present. The PDF also records the Jura Trace version and the date and time of
        analysis.
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Use the PDF report when you need a human-readable record to share with a colleague,
        attach to a story, or submit to an editorial standards process.
      </p>
    </div>

    <div>
      <h3 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-2">
        ZIP case export
      </h3>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-2">
        A compressed archive containing the original file, the PDF trust report, all
        forensic visualisation heatmaps, and a JSON file with the full machine-readable
        result data. This gives a complete, self-contained evidence bundle.
      </p>
      <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
        Use the ZIP export when you are archiving a case, passing evidence to a legal team,
        or sharing a full investigation record with another analyst.
      </p>
    </div>

  </div>

  <div class="overflow-x-auto">
    <table class="w-full text-sm border-collapse">
      <thead>
        <tr class="border-b border-border-light dark:border-border-dark">
          <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Export format</th>
          <th class="text-left py-2 pr-4 text-text-light dark:text-text-dark font-semibold">Best for</th>
          <th class="text-left py-2 text-text-light dark:text-text-dark font-semibold">Contains</th>
        </tr>
      </thead>
      <tbody class="text-flint dark:text-flint-light">
        <tr class="border-b border-border-light/50 dark:border-border-dark/50">
          <td class="py-2 pr-4 font-semibold text-text-light dark:text-text-dark align-top">PDF report</td>
          <td class="py-2 pr-4 align-top">Sharing with non-technical colleagues, editorial review</td>
          <td class="py-2 align-top">Verdict, score, detector summaries, metadata, C2PA details</td>
        </tr>
        <tr>
          <td class="py-2 pr-4 font-semibold text-text-light dark:text-text-dark align-top">ZIP export</td>
          <td class="py-2 pr-4 align-top">Legal evidence, archival, full investigation handover</td>
          <td class="py-2 align-top">Original file + PDF + heatmaps + full JSON data</td>
        </tr>
      </tbody>
    </table>
  </div>
</section>

<!-- ══════════════════════════════════════════════════════════════════
     10. Document Analysis
     ══════════════════════════════════════════════════════════════════ -->
<section id="document-analysis" class="mb-12">
  <h2 class="text-xl font-heading text-text-light dark:text-text-dark tracking-heading mb-4">
    10. Document Analysis
  </h2>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    PDF documents are handled differently from image and video files. The image forensic
    pipeline — ELA, noise analysis, deepfake detection, and the regional detectors — does
    not apply to PDFs, because these analyses are designed for raster image data.
  </p>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    For a PDF, Jura Trace performs:
  </p>

  <ul class="space-y-2 mb-6 text-sm text-flint dark:text-flint-light leading-relaxed">
    <li class="flex gap-2">
      <span class="text-malachite dark:text-malachite-light flex-none">✓</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">C2PA credential check</strong> —
        reads and verifies any embedded Content Credentials manifest.
      </span>
    </li>
    <li class="flex gap-2">
      <span class="text-malachite dark:text-malachite-light flex-none">✓</span>
      <span>
        <strong class="text-text-light dark:text-text-dark">Metadata extraction</strong> —
        reads document author, creation date, modification date, and producing application.
      </span>
    </li>
    <li class="flex gap-2">
      <span class="text-cinnabar dark:text-cinnabar-light flex-none">✗</span>
      <span>Image forensic analysis (ELA, noise, copy-move, deepfake, regional detectors)</span>
    </li>
    <li class="flex gap-2">
      <span class="text-cinnabar dark:text-cinnabar-light flex-none">✗</span>
      <span>Watermark embedding and extraction</span>
    </li>
  </ul>

  <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
    The trust score for a PDF is based entirely on provenance evidence: whether a valid C2PA
    manifest is present, whether the content hash matches, and whether the document metadata
    is internally consistent.
  </p>

  <div class="bg-graphite rounded-lg border border-border-dark p-4">
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      A high trust score on a PDF confirms provenance integrity — that the document has not
      been modified since it was signed and that the signing identity is declared. It does not
      indicate anything about the truthfulness of the document's contents.
    </p>
  </div>
</section>

<!-- ── Next steps ─────────────────────────────────────────────────── -->
<div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

<section aria-label="Next steps" class="mb-4">
  <h2 class="text-base font-heading font-semibold text-text-light dark:text-text-dark tracking-heading mb-3">
    Next steps
  </h2>
  <ul class="space-y-2 text-sm">
    <li>
      <a href="/help/methodology"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        How Analysis Works
      </a>
      <span class="text-flint dark:text-flint-light"> — the full technical methodology behind every detector and trust score component</span>
    </li>
    <li>
      <a href="/help/glossary"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        Glossary
      </a>
      <span class="text-flint dark:text-flint-light"> — definitions of ELA, NPR, pHash, C2PA, and all other technical terms</span>
    </li>
    <li>
      <a href="/help/personas"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        Usage Guides
      </a>
      <span class="text-flint dark:text-flint-light"> — practical workflows for museum curators, journalists, and fact-checkers</span>
    </li>
    <li>
      <a href="/help/protect"
         class="text-lapis dark:text-lapis-light hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">
        Protecting Your Content
      </a>
      <span class="text-flint dark:text-flint-light"> — apply C2PA credentials and invisible watermarks to your own assets</span>
    </li>
  </ul>
</section>
