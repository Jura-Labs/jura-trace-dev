<script lang="ts">
  // No reactive state required — static transparency page.
</script>

<!--
  Help — Model Cards
  ==================
  TRIED Pillar 2: "factsheets, similar to model cards and datasheets"
  Documents the GBM deepfake classifier and UnivFD linear probe with
  training data, performance metrics, known failure modes, and version history.

  Sanctuary theme. British spelling. No emojis. WCAG 2.2 AA.
-->

<article aria-labelledby="model-cards-heading">

  <!-- ── Page heading ──────────────────────────────────────────────────── -->
  <header class="mb-10">
    <h1
      id="model-cards-heading"
      class="font-heading text-3xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      Model Cards
    </h1>
    <p class="text-base text-flint-dark dark:text-flint-light leading-relaxed max-w-2xl">
      Jura Trace uses two machine learning classifiers to assess whether content
      is AI-generated. This page documents their training data, performance,
      known limitations, and update history — following the
      <a href="https://arxiv.org/abs/1810.03993" target="_blank" rel="noopener noreferrer"
         class="text-lapis dark:text-lapis-light underline hover:no-underline">model card</a>
      framework for transparent ML documentation.
    </p>
    <div class="earth-line mt-6" aria-hidden="true"></div>
  </header>

  <!-- ── Table of contents ─────────────────────────────────────────────── -->
  <nav aria-label="Page contents" class="mb-10">
    <p class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-nav font-medium mb-3">Contents</p>
    <ol class="space-y-1 text-sm">
      {#each [
        { href: '#gbm-classifier', label: 'GBM Deepfake Classifier' },
        { href: '#univfd-probe',   label: 'UnivFD Linear Probe' },
        { href: '#update-schedule', label: 'Update Schedule' },
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

  <!-- ══════════════════════════════════════════════════════════════════ -->
  <!-- GBM Deepfake Classifier                                          -->
  <!-- ══════════════════════════════════════════════════════════════════ -->
  <section aria-labelledby="gbm-heading" class="mb-14" id="gbm-classifier">
    <h2
      id="gbm-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-6 leading-tight tracking-heading"
    >
      GBM Deepfake Classifier
    </h2>

    <!-- Overview -->
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6 mb-6">
      <dl class="grid grid-cols-1 sm:grid-cols-2 gap-x-8 gap-y-4 text-sm">
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Model type</dt>
          <dd class="text-flint-dark dark:text-flint-light">Gradient Boosting Machine (GBM)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Framework</dt>
          <dd class="text-flint-dark dark:text-flint-light">scikit-learn GradientBoostingClassifier</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">File</dt>
          <dd class="text-flint-dark dark:text-flint-light font-mono text-xs">models/deepfake_classifier.joblib</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">File size</dt>
          <dd class="text-flint-dark dark:text-flint-light">~1.2 MB</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Shipped in</dt>
          <dd class="text-flint-dark dark:text-flint-light">v1.0 (22 June 2026)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Model build</dt>
          <dd class="text-flint-dark dark:text-flint-light">v1.0 launch build</dd>
        </div>
      </dl>
      <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mt-4 italic">
        Performance metrics and training-corpus figures below describe the v1.0 launch
        build retrained in the run-up to release. Earlier development-time builds
        are not documented here; the figures on this page are the build that ships.
      </p>
    </div>

    <!-- Purpose -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Purpose</h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      The GBM classifier analyses an 84-dimensional feature vector extracted from
      images by the forensic pipeline (ELA statistics, noise patterns, frequency
      domain features, copy-move indicators, and more). It produces a probability
      score indicating how likely an image is to be AI-generated. This score runs
      as one head of a two-head AI-detection ensemble alongside the UnivFD probe;
      the combined output feeds the final trust assessment.
    </p>

    <!-- Training data -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Training Data</h3>
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5 mb-6">
      <dl class="space-y-3 text-sm">
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Total images</dt>
          <dd class="text-flint-dark dark:text-flint-light">over 10,000 (balanced authentic and AI-generated)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Authentic sources</dt>
          <dd class="text-flint-dark dark:text-flint-light">Guardian press photos, COCO (train + validation), Flickr30k, Flickr8k, real camera DCIM photos, Wikimedia Commons photographs (curated, non-art)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">AI-generated sources (14 generator families)</dt>
          <dd class="text-flint-dark dark:text-flint-light">ELSA 1M (Stable Diffusion, DALL-E mix), DiffusionDB, DALL-E 3, Civitai SFW, SDXL-Turbo, Midjourney v6, Gemini Imagen 4, Grok Aurora, ArtBench, HuggingFace AI, and others</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Feature vector</dt>
          <dd class="text-flint-dark dark:text-flint-light">84 features extracted from:
            <ul class="list-disc pl-4 mt-1 space-y-0.5">
              <li>ELA (mean, std, max, entropy)</li>
              <li>Noise analysis</li>
              <li>Copy-move detection</li>
              <li>Frequency domain</li>
              <li>JPEG ghost</li>
              <li>NPR (neighbouring pixel relationship)</li>
              <li>Segmented ELA</li>
              <li>Shadow consistency</li>
              <li>Colour temperature</li>
              <li>Splice boundary</li>
              <li>Camera discrimination: demosaic peak count, inter-channel coherence, blocking strength variance, MakerNote authenticity</li>
            </ul>
          </dd>
        </div>
      </dl>
    </div>

    <!-- Performance -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Performance Metrics</h3>
    <div class="overflow-x-auto mb-6">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Metric</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Value</th>
          </tr>
        </thead>
        <tbody class="text-flint-dark dark:text-flint-light">
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">AUC-ROC (5-fold cross-validation)</td>
            <td class="py-2 pr-4 font-mono">0.9868</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Authentic false positive rate</td>
            <td class="py-2 pr-4 font-mono">4.54%</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">AI detection rate (recall)</td>
            <td class="py-2 pr-4 font-mono">92.52%</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Calibrated threshold</td>
            <td class="py-2 pr-4 font-mono">0.49 (FP 4.79%, recall 92.68%)</td>
          </tr>
          <tr>
            <td class="py-2 pr-4">Cross-validation folds</td>
            <td class="py-2 pr-4 font-mono">5 (stratified)</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Known limitations -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Known Limitations</h3>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      <li><strong>Minimum image size:</strong> Images below 128&times;128 pixels produce unreliable results. A guard prevents analysis of very small images.</li>
      <li><strong>Wildlife and macro photography:</strong> The <code>wikimedia_photos</code> category shows a 24.80% FP rate (n=254), driven largely by wildlife and insect macro photography. Additional iNaturalist photographs are being added to the training corpus to address this.</li>
      <li><strong>High-end camera photos:</strong> Images from DJI drones and Sony DSC cameras show a 10.32% FP rate. MakerNote EXIF data provides a partial mitigation at inference time.</li>
      <li><strong>Generator coverage:</strong> May underperform on content from generators released after April 2026 that were not represented in the training set. Quarterly retraining planned.</li>
      <li><strong>Compression sensitivity:</strong> Heavy JPEG compression or multiple re-compression cycles degrade the feature vector quality, reducing reliability.</li>
    </ul>

    <!-- Release history -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Release History</h3>
    <div class="overflow-x-auto">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Release</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Date</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Notes</th>
          </tr>
        </thead>
        <tbody class="text-flint-dark dark:text-flint-light">
          <tr>
            <td class="py-2 pr-4 font-mono">v1.0 launch build</td>
            <td class="py-2 pr-4">22 June 2026</td>
            <td class="py-2 pr-4">First public release. Corpus of over 10,000 images across 14 AI-generator families with the metrics shown above.</td>
          </tr>
        </tbody>
      </table>
      <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mt-3 italic">
        Development-time iterations between March and June 2026 are not documented here.
        The next scheduled retrain is described in the Update Schedule section below.
      </p>
    </div>
  </section>

  <div class="earth-line mb-14" aria-hidden="true"></div>

  <!-- ══════════════════════════════════════════════════════════════════ -->
  <!-- UnivFD Linear Probe                                               -->
  <!-- ══════════════════════════════════════════════════════════════════ -->
  <section aria-labelledby="univfd-heading" class="mb-14" id="univfd-probe">
    <h2
      id="univfd-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-6 leading-tight tracking-heading"
    >
      UnivFD Linear Probe
    </h2>

    <!-- Overview -->
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6 mb-6">
      <dl class="grid grid-cols-1 sm:grid-cols-2 gap-x-8 gap-y-4 text-sm">
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Model type</dt>
          <dd class="text-flint-dark dark:text-flint-light">Logistic Regression on CLIP embeddings</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Base model</dt>
          <dd class="text-flint-dark dark:text-flint-light">CLIP ViT-B/32 (open_clip, laion2b_s34b_b79k)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Approach</dt>
          <dd class="text-flint-dark dark:text-flint-light">UnivFD (Ojha et al. 2023) — linear probe on frozen CLIP features</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Embedding dimension</dt>
          <dd class="text-flint-dark dark:text-flint-light font-mono">512</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">File</dt>
          <dd class="text-flint-dark dark:text-flint-light font-mono text-xs">models/univfd_probe.joblib</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">File size</dt>
          <dd class="text-flint-dark dark:text-flint-light">4.8 KB</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Shipped in</dt>
          <dd class="text-flint-dark dark:text-flint-light">v1.0 (22 June 2026)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Model build</dt>
          <dd class="text-flint-dark dark:text-flint-light">v1.0 launch build (multi-format: JPEG / PNG / TIFF / WebP / HEIC)</dd>
        </div>
      </dl>
      <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mt-4 italic">
        Performance metrics and training-corpus figures below describe the v1.0 launch
        build retrained in the run-up to release. Earlier development-time builds
        are not documented here; the figures on this page are the build that ships.
      </p>
    </div>

    <!-- Purpose -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Purpose</h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      The UnivFD probe is a lightweight classifier that leverages CLIP's visual
      understanding to detect AI-generated images. CLIP was trained on hundreds of
      millions of image-text pairs and captures high-level semantic features that
      differ between authentic photographs and AI-generated content. The probe adds
      a single linear layer on top of frozen CLIP embeddings — requiring only 4.8 KB
      of weights while benefiting from CLIP's broad visual knowledge. This approach,
      introduced by Ojha et al. (2023), provides strong cross-generator generalisation.
    </p>

    <!-- Training data -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Training Data</h3>
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5 mb-6">
      <dl class="space-y-3 text-sm">
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Total samples</dt>
          <dd class="text-flint-dark dark:text-flint-light">over 50,000 (training corpus includes platform-forwarded re-saves at Q=75/85/2× plus multi-format augmentation across PNG, TIFF, WebP and HEIC)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Authentic sources</dt>
          <dd class="text-flint-dark dark:text-flint-light">COCO (train + validation), Flickr30k, Flickr8k, Google Photos, ImageNet validation, CelebA faces, camera DCIM photos, Wikimedia Commons photographs (curated, non-art)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">AI-generated sources (14 generator families)</dt>
          <dd class="text-flint-dark dark:text-flint-light">ELSA 1M / Stable Diffusion, DiffusionDB, DALL-E 3, Civitai SFW, SDXL-Turbo, ArtBench, Midjourney v6, Gemini Imagen 4, Grok Aurora, HuggingFace AI, and others</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Regularisation</dt>
          <dd class="text-flint-dark dark:text-flint-light">C=0.5 (L2), class_weight=balanced, solver=lbfgs, max_iter=1000</dd>
        </div>
      </dl>
    </div>

    <!-- Performance -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Performance Metrics</h3>
    <div class="overflow-x-auto mb-6">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Metric</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Value</th>
          </tr>
        </thead>
        <tbody class="text-flint-dark dark:text-flint-light">
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">AUC-ROC (5-fold cross-validation)</td>
            <td class="py-2 pr-4 font-mono">0.9933</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Authentic false positive rate</td>
            <td class="py-2 pr-4 font-mono">4.12%</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">AI detection rate (recall)</td>
            <td class="py-2 pr-4 font-mono">95.70%</td>
          </tr>
          <tr>
            <td class="py-2 pr-4">Cross-validation folds</td>
            <td class="py-2 pr-4 font-mono">5 (stratified)</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Known limitations -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Known Limitations</h3>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      <li><strong>CLIP dependency:</strong> Requires the open_clip ViT-B/32 model (~350 MB). When CLIP is not installed, the probe is unavailable and gracefully skipped.</li>
      <li><strong>Non-photographic content:</strong> Paintings, digital illustrations, and stylised artwork can produce false positives. Wikimedia art/illustrations were removed from the training corpus after analysis showed a high false positive rate on that source.</li>
      <li><strong>Wildlife and macro photography:</strong> The <code>wikimedia_photos</code> subset (wildlife, insect macro) is the top FP source. iNaturalist photographs are being added to the authentic corpus to address this.</li>
      <li><strong>High-end camera photos:</strong> Images from DJI drones and Sony DSC cameras with very clean noise profiles are occasionally flagged. MakerNote EXIF data provides a partial mitigation at inference time.</li>
      <li><strong>Generator coverage:</strong> Trained on 14 generator families up to April 2026. New generators may produce outputs that fall outside the learned decision boundary. Quarterly retraining planned.</li>
      <li><strong>Demographic bias:</strong> CLIP-proxy demographic audit completed April 2026. Dark-skin proxy group FP rate 7.8% vs 4.1% overall (1.9&times; ratio &mdash; below the 2&times; failure threshold but notable). Light-skin FP 5.0%. No-people FP 3.1%. Full results in the fairness documentation. Audit uses CLIP text-image similarity as a computational proxy, not human-annotated ground truth.</li>
    </ul>

    <!-- Release history -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Release History</h3>
    <div class="overflow-x-auto">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Release</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Date</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Notes</th>
          </tr>
        </thead>
        <tbody class="text-flint-dark dark:text-flint-light">
          <tr>
            <td class="py-2 pr-4 font-mono">v1.0 launch build</td>
            <td class="py-2 pr-4">22 June 2026</td>
            <td class="py-2 pr-4">First public release. Multi-format training across JPEG / PNG / TIFF / WebP / HEIC with the metrics shown above.</td>
          </tr>
        </tbody>
      </table>
      <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mt-3 italic">
        Development-time iterations between March and June 2026 are not documented here.
        The next scheduled retrain is described in the Update Schedule section below.
      </p>
    </div>
  </section>

  <div class="earth-line mb-14" aria-hidden="true"></div>

  <!-- ══════════════════════════════════════════════════════════════════ -->
  <!-- Update Schedule                                                   -->
  <!-- ══════════════════════════════════════════════════════════════════ -->
  <section aria-labelledby="update-heading" class="mb-14" id="update-schedule">
    <h2
      id="update-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-6 leading-tight tracking-heading"
    >
      Update Schedule
    </h2>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-4">
      AI image generators evolve rapidly. Models trained today may not detect
      outputs from generators released six months from now. Jura Trace's
      approach to keeping the classifiers current:
    </p>

    <ul class="list-disc pl-5 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      <li><strong>Targeted retraining:</strong> Both classifiers are retrained when meaningful new training data has accumulated — typically a combination of newly released AI generators, expanded Global Majority device coverage, and corrected false-positive cases reported by users. We do not retrain on a fixed quarterly clock; the decision is driven by data availability and signal drift.</li>
      <li><strong>New generator coverage:</strong> When major new generators are released or existing generators receive significant updates, training samples from those generators are incorporated in the next retraining cycle.</li>
      <li><strong>Model distribution:</strong> Updated model weights are distributed via the application's auto-update mechanism. Users are notified when newer models are available.</li>
      <li><strong>Transparency:</strong> This page is updated with each retraining cycle to reflect the current training data composition, performance metrics, and known limitations.</li>
    </ul>

    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed">
      Next scheduled retraining: <strong>September&ndash;October 2026</strong>
      (approximately four months after the 22 June 2026 launch).
    </p>
  </section>

</article>
