<script lang="ts">
  // No reactive state required — this is a static reference page.
  // Section IDs are deep-link targets from the Verify page's
  // "Is the content intact?" card; per-row `?` icons in `verify/+page.svelte`
  // navigate here with the relevant fragment.
</script>

<!--
  Help — Forensic Detectors (Detector Reference)
  ===============================================
  Plain-English explanation of every detector that appears on the
  "Is the content intact?" card in the Verify page.  Authored from the
  three-agent review session 2026-04-28 (persona-testing for the copy,
  content-authenticity-expert for the technical caveats, ux-frontend-designer
  for the IA pattern).

  Audience: non-technical pilot users (museum archivists, journalists,
  policy advisers) who can see a percentage and a row title but lack the
  forensic background to interpret either.

  Each detector follows the same three-paragraph structure:
    • What it does (algorithm in plain language)
    • What a high score means (in trust terms — defensible claim)
    • Known limitation (the cross-examination caveat)

  Seven detectors carry an additional "Standalone caveat" call-out
  flagging that the signal alone is not strong enough to ground a
  forensic-report finding.  Sourced from the content-authenticity-expert
  audit; reflects published literature (Krawetz, Farid, Fridrich, Tan).

  Style: Sanctuary theme, Georgia serif headings, mineral palette,
  British spelling, no emojis.
-->

<article aria-labelledby="forensic-detectors-heading">

  <!-- ── Page heading ──────────────────────────────────────────────────── -->
  <header class="mb-10">
    <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-2">
      Reference
    </p>
    <h1
      id="forensic-detectors-heading"
      class="font-heading text-3xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      Detector Reference
    </h1>
    <p class="text-base text-text-light dark:text-quartz leading-relaxed max-w-2xl">
      Every detector that appears on the &quot;Is the content intact?&quot; card,
      explained in plain English. Each entry tells you what the algorithm looks
      for, what a high score actually means, and the most common reason a
      legitimate image might trigger a false alarm. Use the in-app help icons
      to jump straight to the relevant section.
    </p>
    <div class="earth-line mt-6" aria-hidden="true"></div>
  </header>

  <!-- ── How to read this page ───────────────────────────────────────────── -->
  <section aria-labelledby="how-to-read-heading" class="mb-10">
    <h2
      id="how-to-read-heading"
      class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading"
    >
      How to read this page
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-3">
      Each detector has the same three-line structure: what it does, what a
      high score means, and a known limitation. When you see this lapis-tinted
      block under a detector, treat it as a methodology caveat: the signal is
      useful as part of a wider picture but should not be quoted on its own in
      a formal report.
    </p>
    <div
      class="rounded-xl border border-lapis/30 bg-lapis/5 p-4 text-sm text-text-light dark:text-quartz leading-relaxed"
      role="note"
      aria-label="Standalone caveat example"
    >
      <p class="text-xs uppercase tracking-widest text-lapis dark:text-lapis-light font-semibold mb-1">Standalone caveat</p>
      <p>This detector should not be the sole basis for a forensic conclusion. Quote it only alongside corroborating signals.</p>
    </div>
  </section>

  <!-- ── Table of contents ────────────────────────────────────────────────── -->
  <nav aria-label="Detectors on this page" class="mb-12">
    <p class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-nav font-medium mb-3">
      On this page
    </p>
    <ol class="space-y-1 text-sm columns-1 sm:columns-2 gap-x-8">
      {#each [
        { href: '#ela',                  label: 'Error Level Analysis (ELA)' },
        { href: '#noise-pattern',        label: 'Noise Pattern Analysis' },
        { href: '#prnu',                 label: 'PRNU Sensor Pattern Analysis' },
        { href: '#copy-move',            label: 'Copy-Move Detection' },
        { href: '#jpeg-ghost',           label: 'JPEG Ghost' },
        { href: '#segmented-ela',        label: 'Segmented ELA' },
        { href: '#colour-temperature',   label: 'Colour Temperature' },
        { href: '#shadow-consistency',   label: 'Shadow Consistency' },
        { href: '#splice-boundary',      label: 'Splice Boundary' },
        { href: '#npr',                  label: 'Neighbouring Pixel Relationships (NPR)' },
        { href: '#dct-analysis',         label: 'DCT Analysis' },
        { href: '#fourier-analysis',     label: 'Fourier Analysis' },
      ] as item}
        <li class="break-inside-avoid">
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

  <!-- ══════════════════════════════════════════════════════════════════════
       ELA
       ══════════════════════════════════════════════════════════════════════ -->
  <section id="ela" aria-labelledby="ela-heading" class="mb-12 scroll-mt-24">
    <h2 id="ela-heading" class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading">
      Error Level Analysis (ELA)
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What it does:</strong>
      Saves the image at a known compression level and measures how much each
      area changed. Parts that have been edited tend to absorb compression
      differently and show up brighter in the resulting map.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What a high score means:</strong>
      One or more regions were edited, pasted in, or saved separately before
      the final file was assembled.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-3">
      <strong class="text-text-light dark:text-quartz">Limitation:</strong>
      Any legitimate post-processing (cropping, contrast adjustment, or
      converting between formats) can produce elevated ELA readings on an
      untouched image.
    </p>
    <div
      class="rounded-xl border border-lapis/30 bg-lapis/5 p-4 text-sm text-text-light dark:text-quartz leading-relaxed"
      role="note"
    >
      <p class="text-xs uppercase tracking-widest text-lapis dark:text-lapis-light font-semibold mb-1">Standalone caveat</p>
      <p>Single-pass ELA at Q=90 measures global compression homogeneity; on its own it cannot distinguish an edited image from one that has been re-saved or contains contrasty content (Krawetz, Black Hat 2007).</p>
    </div>
  </section>

  <!-- ══════════════════════════════════════════════════════════════════════
       Noise Pattern Analysis
       ══════════════════════════════════════════════════════════════════════ -->
  <section id="noise-pattern" aria-labelledby="noise-pattern-heading" class="mb-12 scroll-mt-24">
    <h2 id="noise-pattern-heading" class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading">
      Noise Pattern Analysis
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What it does:</strong>
      Examines the fine-grain randomness that every camera sensor naturally
      introduces, looking for patches where that randomness is statistically
      too uniform or too erratic compared with the rest of the image.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What a high score means:</strong>
      Portions of the image have a different noise fingerprint from the rest,
      which can indicate regions were composited from another source or
      generated synthetically.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
      <strong class="text-text-light dark:text-quartz">Limitation:</strong>
      Heavy JPEG compression, portrait-mode software blur, or high-ISO
      correction applied in-camera can produce noise irregularities that are
      entirely authentic.
    </p>
  </section>

  <!-- ══════════════════════════════════════════════════════════════════════
       PRNU Sensor Pattern Analysis
       ══════════════════════════════════════════════════════════════════════ -->
  <section id="prnu" aria-labelledby="prnu-heading" class="mb-12 scroll-mt-24">
    <h2 id="prnu-heading" class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading">
      PRNU Sensor Pattern Analysis
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What it does:</strong>
      Photo Response Non-Uniformity (PRNU) is the noise fingerprint left by
      tiny manufacturing imperfections in a camera sensor's photodiodes.
      Real camera sensors produce a roughly <em>symmetric</em> horizontal /
      vertical autocorrelation pattern in the noise residual; diffusion-model
      and other AI-generated outputs typically produce strongly
      <em>asymmetric</em> patterns because they lack the underlying physical
      sensor. Jura Trace computes the spatial autocorrelation of the wavelet
      noise residual and tests for that symmetry as part of its AI-detection
      ensemble (Lukáš, Fridrich &amp; Goljan 2006 framework).
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">Where it surfaces:</strong>
      The "PRNU sensor pattern symmetry" named indicator inside the AI
      Generation panel's detector list. Contributes to the GBM v4 classifier's
      84-feature vector alongside noise, frequency, LBP/GLCM texture, and
      demosaic features. Not displayed as a standalone score. The signal
      feeds into the ensemble.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What a high score means:</strong>
      Asymmetric noise correlation between horizontal and vertical axes,
      inconsistent with how a real camera sensor would record an image.
      This is one of multiple AI-generation signals; agreement across the
      ensemble matters more than any single indicator.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
      <strong class="text-text-light dark:text-quartz">Limitation:</strong>
      Modern camera ISPs apply aggressive in-camera denoising on phone cameras
      (Pixel Night Sight, iPhone Deep Fusion), drones (DJI Mavic-class), and
      compact cameras (Sony RX-class). This attenuates the underlying PRNU
      signal. The symmetry test is more reliable on raw-pipeline JPEGs than
      on computational-photography output. A future release will add a PRNU
      residual energy feature to extend the analysis (tracked as JTV-156).
      A full reference-fingerprint approach (Lukáš/Fridrich/Goljan 2006 §3)
      where individual camera sensors are pre-enrolled is appropriate for
      institutional workflows (museum collections, news-agency staff
      equipment) and is filed as a separate Custom Engineering deliverable
      (JTV-188) rather than a default capability.
    </p>
  </section>

  <!-- ══════════════════════════════════════════════════════════════════════
       Copy-Move Detection
       ══════════════════════════════════════════════════════════════════════ -->
  <section id="copy-move" aria-labelledby="copy-move-heading" class="mb-12 scroll-mt-24">
    <h2 id="copy-move-heading" class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading">
      Copy-Move Detection
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What it does:</strong>
      Searches the image for duplicated regions: areas that have been copied
      from one part of the photo and pasted elsewhere, a common way to
      conceal or repeat content within an otherwise genuine image.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What a high score means:</strong>
      The detector found matching blocks that appear to have been cloned from
      within the same image, suggesting internal duplication rather than
      external splicing.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">Limitation:</strong>
      Repeating textures (brickwork, fabric, foliage) and highly symmetrical
      subjects can produce false matches on images that have never been
      touched.
    </p>
    <p class="text-xs text-flint-dark dark:text-flint-light italic mt-2">
      Visualisation: when the detector flags duplicates, the in-app preview shows
      matched coloured pairs joining the cloned regions. A 0% score means no
      cloned regions were found, and the preview is just the original image.
    </p>
  </section>

  <!-- ══════════════════════════════════════════════════════════════════════
       JPEG Ghost
       ══════════════════════════════════════════════════════════════════════ -->
  <section id="jpeg-ghost" aria-labelledby="jpeg-ghost-heading" class="mb-12 scroll-mt-24">
    <h2 id="jpeg-ghost-heading" class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading">
      JPEG Ghost
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What it does:</strong>
      Re-saves the image repeatedly at different compression levels and looks
      for regions that behave as though they were originally compressed at a
      different quality setting than the rest of the file: a tell-tale sign
      of pasted content.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What a high score means:</strong>
      One or more areas carry a different JPEG compression history from the
      surrounding image, consistent with a region cut from a different photo.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-3">
      <strong class="text-text-light dark:text-quartz">Limitation:</strong>
      The analysis applies only to JPEG files; PNG images score zero by
      design, and heavy post-processing workflows that involve multiple save
      cycles can produce ghost patterns on legitimate images.
    </p>
    <div
      class="rounded-xl border border-lapis/30 bg-lapis/5 p-4 text-sm text-text-light dark:text-quartz leading-relaxed"
      role="note"
    >
      <p class="text-xs uppercase tracking-widest text-lapis dark:text-lapis-light font-semibold mb-1">Standalone caveat</p>
      <p>Farid&apos;s JPEG-ghost residual at multiple quality factors (IEEE TIFS 2009) is suggestive but not dispositive; on this build it is weighted at 0.5 in the trust score and calibration on a commercially-licensed splice corpus is outstanding.</p>
    </div>
  </section>

  <!-- ══════════════════════════════════════════════════════════════════════
       Segmented ELA
       ══════════════════════════════════════════════════════════════════════ -->
  <section id="segmented-ela" aria-labelledby="segmented-ela-heading" class="mb-12 scroll-mt-24">
    <h2 id="segmented-ela-heading" class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading">
      Segmented ELA
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What it does:</strong>
      Divides the image into a grid of 64 equal cells and measures the
      compression response of each cell independently, flagging any cell that
      behaves significantly differently from its neighbours.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What a high score means:</strong>
      A cluster of three or more adjacent flagged cells suggests a localised
      region that was edited, replaced, or composited at a different time or
      quality setting.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
      <strong class="text-text-light dark:text-quartz">Limitation:</strong>
      Text overlays, logos, watermarks, and sharp graphic elements embedded
      in a photographic image will routinely flag cells even if no
      manipulation has occurred.
    </p>
  </section>

  <!-- ══════════════════════════════════════════════════════════════════════
       Colour Temperature
       ══════════════════════════════════════════════════════════════════════ -->
  <section id="colour-temperature" aria-labelledby="colour-temperature-heading" class="mb-12 scroll-mt-24">
    <h2 id="colour-temperature-heading" class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading">
      Colour Temperature
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What it does:</strong>
      Measures the warm-to-cool colour balance across a 4&times;4 grid of
      regions and checks whether any section has a markedly different white
      balance from the rest of the scene.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What a high score means:</strong>
      The image may be a composite where different source photographs were
      taken under different lighting conditions, producing a visible
      colour-temperature mismatch at the join.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
      <strong class="text-text-light dark:text-quartz">Limitation:</strong>
      Natural scenes with mixed lighting (a subject inside near a window, or
      a landscape at sunrise) can show genuine colour temperature variation
      that is not evidence of manipulation.
    </p>
  </section>

  <!-- ══════════════════════════════════════════════════════════════════════
       Shadow Consistency
       ══════════════════════════════════════════════════════════════════════ -->
  <section id="shadow-consistency" aria-labelledby="shadow-consistency-heading" class="mb-12 scroll-mt-24">
    <h2 id="shadow-consistency-heading" class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading">
      Shadow Consistency
      <span class="ml-2 text-[10px] uppercase tracking-widest font-semibold px-2 py-0.5 rounded-full bg-amber/10 text-amber-dark dark:text-amber-light border border-amber/30 align-middle">On-demand</span>
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What it does:</strong>
      Estimates the dominant direction of light falling on each foreground
      element and checks whether shadows across the image are all consistent
      with a single light source.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What a high score means:</strong>
      Two or more regions have shadow directions that differ by more than
      80 degrees, suggesting subjects or objects were photographed under
      different lighting and combined.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-3">
      <strong class="text-text-light dark:text-quartz">Limitation:</strong>
      Multiple light sources, overcast diffuse lighting, and reflective
      surfaces can produce genuine shadow direction conflicts in photographs
      that were never altered.
    </p>
    <div
      class="rounded-xl border border-lapis/30 bg-lapis/5 p-4 text-sm text-text-light dark:text-quartz leading-relaxed"
      role="note"
    >
      <p class="text-xs uppercase tracking-widest text-lapis dark:text-lapis-light font-semibold mb-1">Standalone caveat</p>
      <p>This is a gradient-orientation heterogeneity heuristic, not a cast-shadow geometry analysis (Kee &amp; Farid 2010); it should not be cited as evidence of inconsistent illumination in a forensic report.</p>
    </div>
  </section>

  <!-- ══════════════════════════════════════════════════════════════════════
       Splice Boundary
       ══════════════════════════════════════════════════════════════════════ -->
  <section id="splice-boundary" aria-labelledby="splice-boundary-heading" class="mb-12 scroll-mt-24">
    <h2 id="splice-boundary-heading" class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading">
      Splice Boundary
      <span class="ml-2 text-[10px] uppercase tracking-widest font-semibold px-2 py-0.5 rounded-full bg-amber/10 text-amber-dark dark:text-amber-light border border-amber/30 align-middle">On-demand</span>
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What it does:</strong>
      Examines edges within the image for three simultaneous signs of a
      cut-and-paste join: alignment with JPEG 8&times;8 block boundaries, an
      abrupt noise-level change across the edge, and unnaturally smooth
      gradient feathering.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What a high score means:</strong>
      The detector found edges where all three signals coincide, which is
      consistent with a boundary where one image fragment was pasted onto
      another and blended.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-3">
      <strong class="text-text-light dark:text-quartz">Limitation:</strong>
      Sharp natural edges between high-contrast regions (a subject against a
      bright sky, or a sign on a plain wall) can satisfy one or two of the
      three signals without any manipulation having taken place.
    </p>
    <div
      class="rounded-xl border border-lapis/30 bg-lapis/5 p-4 text-sm text-text-light dark:text-quartz leading-relaxed"
      role="note"
    >
      <p class="text-xs uppercase tracking-widest text-lapis dark:text-lapis-light font-semibold mb-1">Standalone caveat</p>
      <p>Three-signal contour analysis surfaces candidate splice boundaries for analyst review; it is an investigative pointer, not a finding.</p>
    </div>
  </section>

  <!-- ══════════════════════════════════════════════════════════════════════
       NPR
       ══════════════════════════════════════════════════════════════════════ -->
  <section id="npr" aria-labelledby="npr-heading" class="mb-12 scroll-mt-24">
    <h2 id="npr-heading" class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading">
      Neighbouring Pixel Relationships (NPR)
      <span class="ml-2 text-[10px] uppercase tracking-widest font-semibold px-2 py-0.5 rounded-full bg-amber/10 text-amber-dark dark:text-amber-light border border-amber/30 align-middle">On-demand</span>
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What it does:</strong>
      Measures the statistical relationship between every pixel and its
      immediate neighbours. Real camera sensors produce a characteristic
      pattern of pixel-to-pixel correlation that AI generators tend not to
      replicate accurately.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What a high score means:</strong>
      The pixel-level statistics of the image more closely resemble output
      from a diffusion model or GAN than a photograph taken with a camera.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-3">
      <strong class="text-text-light dark:text-quartz">Limitation:</strong>
      Aggressive resizing, heavy sharpening, and extreme noise-reduction
      processing can alter the pixel neighbourhood statistics of a genuine
      photograph enough to produce a false positive.
    </p>
    <div
      class="rounded-xl border border-lapis/30 bg-lapis/5 p-4 text-sm text-text-light dark:text-quartz leading-relaxed"
      role="note"
    >
      <p class="text-xs uppercase tracking-widest text-lapis dark:text-lapis-light font-semibold mb-1">Standalone caveat</p>
      <p>NPR (Tan et al., AAAI 2024) is an AI-synthesis indicator, not a manipulation indicator; thresholds are research-paper defaults and have not been recalibrated for this corpus.</p>
    </div>
  </section>

  <!-- ══════════════════════════════════════════════════════════════════════
       DCT Analysis
       ══════════════════════════════════════════════════════════════════════ -->
  <section id="dct-analysis" aria-labelledby="dct-analysis-heading" class="mb-12 scroll-mt-24">
    <h2 id="dct-analysis-heading" class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading">
      DCT Analysis
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What it does:</strong>
      Divides the image into 8&times;8 pixel blocks (the same unit JPEG
      compression operates on) and measures whether the energy distribution
      across those blocks is uniform, or whether some blocks carry a
      noticeably different compression fingerprint.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What a high score means:</strong>
      The variation in compression energy across blocks is unusually high,
      consistent with blocks that originated from JPEG files saved at
      different quality settings and were then combined.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-3">
      <strong class="text-text-light dark:text-quartz">Limitation:</strong>
      Images that have been legitimately cropped and re-saved, or assembled
      from multiple tiles by a content management system, can show elevated
      DCT variation without any deceptive intent.
    </p>
    <div
      class="rounded-xl border border-lapis/30 bg-lapis/5 p-4 text-sm text-text-light dark:text-quartz leading-relaxed"
      role="note"
    >
      <p class="text-xs uppercase tracking-widest text-lapis dark:text-lapis-light font-semibold mb-1">Standalone caveat</p>
      <p>Per-block 8&times;8 DCT AC-energy variance is suggestive of mixed compression; the operative threshold is heuristic and uncalibrated against a labelled splice corpus.</p>
    </div>
  </section>

  <!-- ══════════════════════════════════════════════════════════════════════
       Fourier Analysis
       ══════════════════════════════════════════════════════════════════════ -->
  <section id="fourier-analysis" aria-labelledby="fourier-analysis-heading" class="mb-12 scroll-mt-24">
    <h2 id="fourier-analysis-heading" class="font-heading text-xl text-text-light dark:text-quartz mb-3 leading-tight tracking-heading">
      Fourier Analysis
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What it does:</strong>
      Applies a mathematical frequency transform to the whole image and looks
      for repeating periodic patterns in the result. Real photographs have
      smooth, broadly spread frequency spectra, while AI-generated images and
      some compositing tools leave distinctive repeating marks.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-2">
      <strong class="text-text-light dark:text-quartz">What a high score means:</strong>
      The detector found more than the expected number of sharp frequency
      peaks, suggesting the image contains periodic patterns that are more
      consistent with algorithmic generation or upscaling than with a camera.
    </p>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-3">
      <strong class="text-text-light dark:text-quartz">Limitation:</strong>
      Scanned images, images captured through patterned glass or fabric, and
      screenshots of content with regular grid layouts (tables, tile
      interfaces) can all produce elevated peak counts on authentic content.
    </p>
    <div
      class="rounded-xl border border-lapis/30 bg-lapis/5 p-4 text-sm text-text-light dark:text-quartz leading-relaxed"
      role="note"
    >
      <p class="text-xs uppercase tracking-widest text-lapis dark:text-lapis-light font-semibold mb-1">Standalone caveat</p>
      <p>FFT peak counting (Frank et al., ICML 2020 motif) flags periodic spectral structure; it cannot attribute the periodicity to synthesis without corroboration.</p>
    </div>
  </section>

</article>
