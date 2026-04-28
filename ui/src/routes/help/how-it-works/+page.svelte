<script lang="ts">
  // No reactive state required — this is a static help page.
  // Section IDs are used for deep linking from the Verify page.
</script>

<!--
  Help — How It Works (Beginner's Guide)
  =======================================
  Plain-English overview for non-technical users: museum curators, journalists,
  fact-checkers. Covers what Jura Trace checks, why two AI checks appear,
  what "Experimental" means, how the trust score is built, and when to dig deeper.

  Audience: non-technical pilot users.
  Style: Sanctuary theme, Georgia serif headings, Tailwind utility classes.
  British spelling throughout. No emojis.
-->

<article aria-labelledby="how-it-works-heading">

  <!-- ── Page heading ──────────────────────────────────────────────────── -->
  <header class="mb-10">
    <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-2">
      Start here
    </p>
    <h1
      id="how-it-works-heading"
      class="font-heading text-3xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      How It Works
    </h1>
    <p class="text-base text-flint-dark dark:text-flint-light leading-relaxed max-w-2xl">
      Jura Trace examines a file from several different angles at once — checking
      its credentials, its pixel-level structure, and whether it bears the
      hallmarks of AI generation. This page explains each layer of that analysis
      in plain English, so you can read results with confidence. No technical
      background is required.
    </p>
    <div class="earth-line mt-6" aria-hidden="true"></div>
  </header>

  <!-- ── Table of contents ─────────────────────────────────────────────── -->
  <nav aria-label="Page contents" class="mb-10">
    <p class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-nav font-medium mb-3">
      On this page
    </p>
    <ol class="space-y-1 text-sm">
      {#each [
        { href: '#what-jura-trace-checks', label: '1. What Jura Trace Checks' },
        { href: '#two-ai-checks',          label: '2. Why You See Two AI Checks' },
        { href: '#experimental',           label: '3. What "Experimental" Means' },
        { href: '#trust-score',            label: '4. How the Trust Score Is Built' },
        { href: '#when-to-dig-deeper',     label: '5. When to Dig Deeper' },
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


  <!-- ══════════════════════════════════════════════════════════════════
       1. What Jura Trace Checks
       ══════════════════════════════════════════════════════════════════ -->
  <section id="what-jura-trace-checks" aria-labelledby="what-checks-heading" class="mb-12">

    <h2
      id="what-checks-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      1. What Jura Trace Checks
    </h2>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      When you drop a file into Jura Trace, it runs four broad categories of
      analysis simultaneously. Each one looks at a different layer of the file —
      from its signed credentials to the arrangement of individual pixels — so
      that no single test has to carry the full burden of proof. Think of it as
      four independent witnesses reading the same evidence.
    </p>

    <ul class="space-y-5 mb-6 list-none pl-0">

      <li class="flex gap-4">
        <span class="flex-none w-5 mt-0.5 text-lapis dark:text-lapis-light font-semibold text-sm" aria-hidden="true">→</span>
        <div>
          <h3 class="text-sm font-medium text-text-light dark:text-quartz mb-1">
            Provenance credentials
          </h3>
          <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
            Some files carry a signed C2PA manifest — a tamper-evident record of
            who created the file, when, and with what tool. Jura Trace reads that
            manifest and checks the cryptographic signature. If the file has been
            altered since signing, the signature breaks. If no manifest is present,
            that absence is noted — it does not automatically mean the file is
            inauthentic, but it does mean one layer of the provenance chain is
            missing.
          </p>
        </div>
      </li>

      <li class="flex gap-4">
        <span class="flex-none w-5 mt-0.5 text-lapis dark:text-lapis-light font-semibold text-sm" aria-hidden="true">→</span>
        <div>
          <h3 class="text-sm font-medium text-text-light dark:text-quartz mb-1">
            Forensic signals
          </h3>
          <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
            Genuine photographs have predictable noise patterns, consistent
            compression artefacts, and coherent lighting physics. Editing tools
            leave traces — inconsistent JPEG compression in a spliced region,
            copy-moved patches with repeated texture, or shadows that do not
            match the light source. Jura Trace runs several detectors that look
            for these microscopic inconsistencies in the image data itself.
          </p>
        </div>
      </li>

      <li class="flex gap-4">
        <span class="flex-none w-5 mt-0.5 text-lapis dark:text-lapis-light font-semibold text-sm" aria-hidden="true">→</span>
        <div>
          <h3 class="text-sm font-medium text-text-light dark:text-quartz mb-1">
            AI-generation detection
          </h3>
          <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
            AI image generators produce images in subtly different ways from
            cameras: characteristic frequency patterns in fine detail, distinctive
            colour statistics, and semantic compositions that differ from real
            photographs. Jura Trace runs two independent models that look for
            these patterns — more on why two appear in the next section.
          </p>
        </div>
      </li>

      <li class="flex gap-4">
        <span class="flex-none w-5 mt-0.5 text-lapis dark:text-lapis-light font-semibold text-sm" aria-hidden="true">→</span>
        <div>
          <h3 class="text-sm font-medium text-text-light dark:text-quartz mb-1">
            Camera authenticity
          </h3>
          <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
            Digital files carry embedded metadata (EXIF) that records the camera
            model, lens, GPS co-ordinates, and capture time. Jura Trace checks
            this metadata for internal contradictions — for example, a file
            claiming to be from an iPhone that lacks the colour profile all
            iPhones write, or GPS co-ordinates rounded to suspicious integer
            degrees that suggest they were typed rather than captured. These
            checks are about consistency, not absolute proof.
          </p>
        </div>
      </li>

    </ul>

    <div class="bg-lapis/5 dark:bg-lapis/10 border border-lapis/20 dark:border-lapis/25 rounded-lg px-5 py-4">
      <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
        All of this analysis happens entirely on your device. No file data is sent
        to any server. Jura Trace connects to the internet only if you explicitly
        request it for optional certificate verification.
      </p>
    </div>

  </section>

  <div class="earth-line mb-10" aria-hidden="true"></div>


  <!-- ══════════════════════════════════════════════════════════════════
       2. Why You See Two AI Checks
       ══════════════════════════════════════════════════════════════════ -->
  <section id="two-ai-checks" aria-labelledby="two-ai-heading" class="mb-12">

    <h2
      id="two-ai-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      2. Why You See Two AI Checks
    </h2>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-4">
      In the results panel, you will see two separate entries under the
      AI-generation heading: <strong class="text-text-light dark:text-quartz">AI Generation (GBM Deepfake)</strong> and
      <strong class="text-text-light dark:text-quartz">CLIP / UnivFD Probe</strong>. This is intentional. The two
      models are not duplicates — they examine the image in fundamentally
      different ways, and running both gives stronger evidence than either
      alone could provide.
    </p>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-4">
      Consider the analogy of a second medical opinion. If you consult one
      doctor who specialises in blood chemistry and another who specialises in
      imaging scans, and both reach the same conclusion, you can be more confident
      in that conclusion than if only one had reviewed the case. If they disagree,
      that disagreement is itself useful information — it tells you the picture
      is genuinely ambiguous and deserves closer review. The same logic applies
      here.
    </p>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-8">
      When both models agree that an image is authentic, the combined signal is
      stronger than either reading alone. When one flags concern and the other
      does not, the inconclusive result reflects a genuine uncertainty — not a
      mistake in the software. The trust score accounts for that disagreement
      by treating the combined evidence more cautiously.
    </p>

    <!-- Side-by-side comparison table -->
    <h3 class="font-heading text-lg text-text-light dark:text-quartz mb-4 leading-tight tracking-heading">
      What Each Model Looks For
    </h3>

    <div class="overflow-x-auto mb-6">
      <table
        class="w-full text-sm border-collapse"
        aria-label="Comparison of GBM Deepfake and CLIP UnivFD Probe detectors"
      >
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="text-left py-3 pr-6 font-medium text-text-light dark:text-quartz w-1/2">
              AI Generation (GBM Deepfake)
            </th>
            <th class="text-left py-3 font-medium text-text-light dark:text-quartz w-1/2">
              CLIP / UnivFD Probe
            </th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border-light dark:divide-border-dark">
          <tr>
            <td class="py-3 pr-6 align-top">
              <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-1">What it looks for</p>
              <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
                Low-level image artefacts — JPEG compression patterns, noise
                distribution, colour channel statistics, and 84 other measurable
                signals extracted from the pixel data.
              </p>
            </td>
            <td class="py-3 align-top">
              <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-1">What it looks for</p>
              <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
                High-level semantic patterns — what the image depicts and whether
                the overall composition, textures, and spatial relationships match
                the signature of AI-generated imagery rather than real-world
                photography.
              </p>
            </td>
          </tr>
          <tr>
            <td class="py-3 pr-6 align-top">
              <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-1">When it helps most</p>
              <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
                Detecting AI images that have been re-saved, re-compressed, or
                shared through social media — the pixel-level traces survive
                moderate processing.
              </p>
            </td>
            <td class="py-3 align-top">
              <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-1">When it helps most</p>
              <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
                Detecting AI images where pixel-level artefacts have been
                smoothed away — the semantic pattern remains even when individual
                pixel statistics have been normalised.
              </p>
            </td>
          </tr>
          <tr>
            <td class="py-3 pr-6 align-top">
              <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-1">Known limitations</p>
              <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
                May be less reliable on images from very recent AI generators not
                yet represented in the training data, or on heavily cropped images
                where the pixel-feature sample is small.
              </p>
            </td>
            <td class="py-3 align-top">
              <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-1">Known limitations</p>
              <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
                May score some older diffusion-model images (e.g. early Stable
                Diffusion) with lower confidence, and can be uncertain on
                content types far from its training distribution (e.g. scanned
                documents, technical diagrams).
              </p>
            </td>
          </tr>
          <tr>
            <td class="py-3 pr-6 align-top">
              <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-1">Accuracy on photographs</p>
              <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
                <span class="tabular-nums">4.5%</span> false-positive rate
                (real photographs incorrectly flagged as AI),
                <span class="tabular-nums">92.5%</span> recall on AI-generated
                content. Validated on 10,709 images.
                <a
                  href="/help/model-cards"
                  class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                >Model card</a>.
              </p>
            </td>
            <td class="py-3 align-top">
              <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-1">Accuracy on photographs</p>
              <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
                <span class="tabular-nums">4.1%</span> false-positive rate
                (real photographs incorrectly flagged as AI),
                <span class="tabular-nums">95.7%</span> recall on AI-generated
                content. Validated on 39,016 images including
                platform-forwarded re-encodes.
                <a
                  href="/help/model-cards"
                  class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                >Model card</a>.
              </p>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      For a full account of each model's training data, accuracy figures, and
      version history, see the
      <a
        href="/help/model-cards"
        class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
      >Model Cards</a> page.
    </p>

  </section>

  <div class="earth-line mb-10" aria-hidden="true"></div>


  <!-- ══════════════════════════════════════════════════════════════════
       3. What "Experimental" Means
       ══════════════════════════════════════════════════════════════════ -->
  <section id="experimental" aria-labelledby="experimental-heading" class="mb-12">

    <h2
      id="experimental-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      3. What "Experimental" Means
    </h2>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-4">
      Some detectors in the results panel carry a small
      <span class="inline-flex items-center gap-1 align-middle">
        <span class="text-xs px-1.5 py-0.5 rounded border border-amber/40 dark:border-amber/50 text-amber-dark dark:text-amber-light bg-amber/5 dark:bg-amber/10 font-medium">Experimental</span>
      </span>
      label. This does not mean the detector is broken or unreliable — it means
      that we are still gathering evidence about its accuracy across a wider range
      of image types, and we have chosen to count its contribution to the trust
      score at half weight while that calibration work continues.
    </p>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-4">
      Think of it this way: a detective who has worked a hundred cases similar to
      yours gets a full vote in the team discussion. A detective who is new to
      this type of case — but still sharp and genuinely helpful — gets half a
      vote until their track record on similar cases is established. Their
      observation still matters; it simply carries less weight until confidence
      in their judgement on this type of evidence is better calibrated.
    </p>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      In practice, this means an Experimental detector's finding can still shift
      the trust score and is always worth reading. Treat it as supporting
      evidence rather than a final determination. If an Experimental detector
      flags concern, check whether the other detectors corroborate it. If they
      do, take the signal seriously. If the non-Experimental detectors are all
      clear, the Experimental flag warrants curiosity but not alarm.
    </p>

    <div class="bg-amber/5 dark:bg-amber/10 border border-amber/20 dark:border-amber/25 rounded-lg px-5 py-4">
      <p class="text-sm font-medium text-text-light dark:text-quartz mb-1">What to do with an Experimental result</p>
      <ul class="space-y-1.5 text-sm text-flint-dark dark:text-flint-light">
        <li class="flex gap-2">
          <span class="text-amber-dark dark:text-amber-light flex-none mt-0.5" aria-hidden="true">→</span>
          Read the finding alongside all the other detector results, not in isolation.
        </li>
        <li class="flex gap-2">
          <span class="text-amber-dark dark:text-amber-light flex-none mt-0.5" aria-hidden="true">→</span>
          If multiple detectors agree, that agreement is meaningful regardless of which
          ones are Experimental.
        </li>
        <li class="flex gap-2">
          <span class="text-amber-dark dark:text-amber-light flex-none mt-0.5" aria-hidden="true">→</span>
          If the overall trust score is inconclusive (40–70%) and an Experimental detector
          is contributing to that uncertainty, consider running a deeper investigation or
          consulting the on-demand tools.
        </li>
      </ul>
    </div>

  </section>

  <div class="earth-line mb-10" aria-hidden="true"></div>


  <!-- ══════════════════════════════════════════════════════════════════
       4. How the Trust Score Is Built
       ══════════════════════════════════════════════════════════════════ -->
  <section id="trust-score" aria-labelledby="trust-score-heading" class="mb-12">

    <h2
      id="trust-score-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      4. How the Trust Score Is Built
    </h2>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-4">
      Every detector that runs on a file casts a weighted vote. Detectors with
      a strong accuracy record on diverse content contribute more. Detectors still
      being calibrated (marked Experimental) contribute less. The votes are
      combined and clipped to a final score between 0% and 100%, where a higher
      score means the evidence is more consistent with authentic, unmanipulated
      content.
    </p>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-4">
      A score above 71% is classified as
      <strong class="text-malachite-dark dark:text-malachite-light">Authentic</strong>.
      Between 40% and 70% it is
      <strong class="text-amber-dark dark:text-amber-light">Inconclusive</strong> — the
      evidence is mixed and human review is recommended. Below 40%, the balance
      of evidence raises
      <strong class="text-cinnabar-dark dark:text-cinnabar-light">Concerns</strong> and
      closer examination is warranted.
    </p>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      The trust score is a starting point for your judgement, not a verdict.
      Automated analysis can be wrong — high-quality AI images can pass, and
      unusual authentic photographs can fail. The score is designed to direct
      your attention, not to replace it. Jura Trace is built on the principle
      that humans stay in the loop.
    </p>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      For the exact weight assigned to each detector and the full scoring
      formula, see the
      <a
        href="/help/methodology#signal-weighting"
        class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
      >Signal Weighting</a> section of the
      <a
        href="/help/methodology"
        class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
      >How Analysis Works</a> page.
    </p>

  </section>

  <div class="earth-line mb-10" aria-hidden="true"></div>


  <!-- ══════════════════════════════════════════════════════════════════
       5. When to Dig Deeper
       ══════════════════════════════════════════════════════════════════ -->
  <section id="when-to-dig-deeper" aria-labelledby="dig-deeper-heading" class="mb-12">

    <h2
      id="dig-deeper-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      5. When to Dig Deeper
    </h2>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
      The top-level result gives you a quick orientation. In some situations,
      clicking through to the Level 3 detail panel — or using the on-demand
      investigation tools — is the right next step. Consider doing so when any
      of the following apply.
    </p>

    <ul class="space-y-4 mb-6" aria-label="Situations that warrant deeper investigation">

      <li class="flex gap-3">
        <span class="flex-none w-5 mt-0.5 text-amber-dark dark:text-amber-light font-semibold text-sm" aria-hidden="true">→</span>
        <div>
          <p class="text-sm font-medium text-text-light dark:text-quartz mb-0.5">
            Trust score is between 40% and 70%
          </p>
          <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
            An inconclusive score means the detectors are not in strong agreement.
            The Level 3 panel breaks down each detector's individual finding so you
            can see which signals are pulling the score in opposite directions.
          </p>
        </div>
      </li>

      <li class="flex gap-3">
        <span class="flex-none w-5 mt-0.5 text-amber-dark dark:text-amber-light font-semibold text-sm" aria-hidden="true">→</span>
        <div>
          <p class="text-sm font-medium text-text-light dark:text-quartz mb-0.5">
            Any detector shows a "Suspicious" label
          </p>
          <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
            A Suspicious finding from a single detector is worth reading even if
            the overall score is high. One strong signal can indicate a targeted
            manipulation that other detectors are not sensitive to.
          </p>
        </div>
      </li>

      <li class="flex gap-3">
        <span class="flex-none w-5 mt-0.5 text-amber-dark dark:text-amber-light font-semibold text-sm" aria-hidden="true">→</span>
        <div>
          <p class="text-sm font-medium text-text-light dark:text-quartz mb-0.5">
            Provenance chain is broken or absent
          </p>
          <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
            If the file carries a C2PA manifest but the signature has failed
            verification, or if no manifest is present on a file that claims
            a credentialled source, the forensic results take on greater
            importance. Check whether the pixel-level detectors and camera
            authenticity checks provide corroborating evidence.
          </p>
        </div>
      </li>

      <li class="flex gap-3">
        <span class="flex-none w-5 mt-0.5 text-amber-dark dark:text-amber-light font-semibold text-sm" aria-hidden="true">→</span>
        <div>
          <p class="text-sm font-medium text-text-light dark:text-quartz mb-0.5">
            Camera authenticity fails
          </p>
          <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
            EXIF anomaly findings can indicate metadata that has been manually
            written rather than captured by a device. Read the individual
            anomaly entries — each one describes exactly which inconsistency
            was found — and consider whether the explanation is innocent
            (e.g. a file exported from editing software) or potentially
            significant.
          </p>
        </div>
      </li>

      <li class="flex gap-3">
        <span class="flex-none w-5 mt-0.5 text-amber-dark dark:text-amber-light font-semibold text-sm" aria-hidden="true">→</span>
        <div>
          <p class="text-sm font-medium text-text-light dark:text-quartz mb-0.5">
            The two AI checks disagree
          </p>
          <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
            When the GBM Deepfake model and the CLIP / UnivFD Probe reach
            different conclusions, that disagreement is itself a signal. It does
            not mean one model is wrong — it means the image sits in an
            ambiguous region for at least one of them. The Level 3 panel and
            the on-demand investigation tools (noise analysis, shadow
            consistency, splice boundary) can help resolve the uncertainty.
          </p>
        </div>
      </li>

    </ul>

    <div class="bg-lapis/5 dark:bg-lapis/10 border border-lapis/20 dark:border-lapis/25 rounded-lg px-5 py-4">
      <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
        The on-demand investigation tools — Noise Pattern Analysis, Shadow
        Consistency, and Splice Boundary — do not run automatically and do not
        affect the trust score. They are available in the Level 3 panel for
        situations where the primary results are inconclusive and you need
        additional evidence to inform your judgement.
      </p>
    </div>

  </section>


  <!-- ── Next steps ─────────────────────────────────────────────────── -->
  <section aria-labelledby="next-steps-heading" class="mb-4">
    <div class="earth-line mb-8" aria-hidden="true"></div>
    <h2
      id="next-steps-heading"
      class="font-heading text-xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      Next Steps
    </h2>
    <ul class="space-y-2 text-sm text-flint-dark dark:text-flint-light">
      <li class="flex gap-2">
        <span class="text-lapis dark:text-lapis-light flex-none" aria-hidden="true">→</span>
        <span><a href="/help/verify" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">Verifying Content Authenticity</a> — step-by-step guide to running an analysis.</span>
      </li>
      <li class="flex gap-2">
        <span class="text-lapis dark:text-lapis-light flex-none" aria-hidden="true">→</span>
        <span><a href="/help/methodology" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">How Analysis Works</a> — full methodology, detector weights, and known limitations.</span>
      </li>
      <li class="flex gap-2">
        <span class="text-lapis dark:text-lapis-light flex-none" aria-hidden="true">→</span>
        <span><a href="/help/model-cards" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">Model Cards</a> — training data, performance metrics, and version history for the AI detection models.</span>
      </li>
      <li class="flex gap-2">
        <span class="text-lapis dark:text-lapis-light flex-none" aria-hidden="true">→</span>
        <span><a href="/help/glossary" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">Glossary</a> — definitions of terms such as C2PA, EXIF, ELA, and perceptual hashing.</span>
      </li>
    </ul>
  </section>

</article>
