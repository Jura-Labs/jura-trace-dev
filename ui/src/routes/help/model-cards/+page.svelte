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
    <p class="text-base text-flint dark:text-flint-light leading-relaxed max-w-2xl">
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
    <p class="text-xs text-flint dark:text-flint-light uppercase tracking-nav font-medium mb-3">Contents</p>
    <ol class="space-y-1 text-sm">
      {#each [
        { href: '#gbm-classifier', label: 'GBM Deepfake Classifier' },
        { href: '#univfd-probe',   label: 'UnivFD Linear Probe' },
        { href: '#update-schedule', label: 'Update Schedule' },
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
          <dd class="text-flint dark:text-flint-light">Gradient Boosting Machine (GBM)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Framework</dt>
          <dd class="text-flint dark:text-flint-light">scikit-learn GradientBoostingClassifier</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">File</dt>
          <dd class="text-flint dark:text-flint-light font-mono text-xs">models/deepfake_classifier.joblib</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">File size</dt>
          <dd class="text-flint dark:text-flint-light">~1.2 MB</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Last trained</dt>
          <dd class="text-flint dark:text-flint-light">3 April 2026</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Version</dt>
          <dd class="text-flint dark:text-flint-light">2.0</dd>
        </div>
      </dl>
    </div>

    <!-- Purpose -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Purpose</h3>
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
      The GBM classifier analyses an 80-dimensional feature vector extracted from
      images by the forensic pipeline (ELA statistics, noise patterns, frequency
      domain features, copy-move indicators, and more). It produces a probability
      score indicating how likely an image is to be AI-generated. This score is
      blended with the heuristic detector scores (35% classifier / 65% heuristic)
      to produce the final trust assessment.
    </p>

    <!-- Training data -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Training Data</h3>
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5 mb-6">
      <dl class="space-y-3 text-sm">
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Total images</dt>
          <dd class="text-flint dark:text-flint-light">709 (326 authentic + 383 AI-generated)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Authentic sources</dt>
          <dd class="text-flint dark:text-flint-light">Guardian press photos, COCO validation set, real camera DCIM photos, Wikimedia Commons photographs</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">AI-generated sources</dt>
          <dd class="text-flint dark:text-flint-light">ELSA 1M (Stable Diffusion, DALL-E mix), Gemini Imagen 4, SDXL-Turbo, user-submitted test images</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Feature vector</dt>
          <dd class="text-flint dark:text-flint-light">80 features extracted from: ELA (mean, std, max, entropy), noise analysis, copy-move detection, frequency domain, JPEG ghost, NPR, segmented ELA, shadow consistency, colour temperature, splice boundary</dd>
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
        <tbody class="text-flint dark:text-flint-light">
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">AUC-ROC (5-fold cross-validation)</td>
            <td class="py-2 pr-4 font-mono">1.0000</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Authentic false positive rate</td>
            <td class="py-2 pr-4 font-mono">0%</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">AI detection rate (recall)</td>
            <td class="py-2 pr-4 font-mono">100%</td>
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
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
      <li><strong>Minimum image size:</strong> Images below 128&times;128 pixels produce unreliable results. A guard prevents analysis of very small images.</li>
      <li><strong>Training corpus size:</strong> 709 images is relatively small. Perfect cross-validation scores (AUC 1.0) may indicate the model has learned format-specific artefacts rather than generalisable AI detection features. The format confound (JPEG vs PNG) was addressed in v2.0 by adding authentic PNGs to the training corpus.</li>
      <li><strong>Generator coverage:</strong> May underperform on content from generators released after April 2026 that were not represented in the training set. Quarterly retraining planned.</li>
      <li><strong>Compression sensitivity:</strong> Heavy JPEG compression or multiple re-compression cycles degrade the feature vector quality, reducing reliability.</li>
    </ul>

    <!-- Version history -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Version History</h3>
    <div class="overflow-x-auto">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Version</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Date</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Change</th>
          </tr>
        </thead>
        <tbody class="text-flint dark:text-flint-light">
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4 font-mono">1.0</td>
            <td class="py-2 pr-4">18 March 2026</td>
            <td class="py-2 pr-4">Initial training. 545 images, AUC 0.945, 14% FP rate.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 font-mono">2.0</td>
            <td class="py-2 pr-4">3 April 2026</td>
            <td class="py-2 pr-4">Format confound eliminated (authentic PNGs added). Corpus expanded to 709. AUC 1.000, FP rate 0%.</td>
          </tr>
        </tbody>
      </table>
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
          <dd class="text-flint dark:text-flint-light">Logistic Regression on CLIP embeddings</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Base model</dt>
          <dd class="text-flint dark:text-flint-light">CLIP ViT-B/32 (open_clip, laion2b_s34b_b79k)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Approach</dt>
          <dd class="text-flint dark:text-flint-light">UnivFD (Ojha et al. 2023) — linear probe on frozen CLIP features</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Embedding dimension</dt>
          <dd class="text-flint dark:text-flint-light font-mono">512</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">File</dt>
          <dd class="text-flint dark:text-flint-light font-mono text-xs">models/univfd_probe.joblib</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">File size</dt>
          <dd class="text-flint dark:text-flint-light">4.8 KB</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Last trained</dt>
          <dd class="text-flint dark:text-flint-light">7 April 2026</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Version</dt>
          <dd class="text-flint dark:text-flint-light">6.0</dd>
        </div>
      </dl>
    </div>

    <!-- Purpose -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Purpose</h3>
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
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
          <dt class="font-medium text-text-light dark:text-quartz">Total images</dt>
          <dd class="text-flint dark:text-flint-light">6,009 (2,873 authentic + 3,136 AI-generated)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Authentic sources</dt>
          <dd class="text-flint dark:text-flint-light">COCO validation (1,600), Google Photos (828), ImageNet validation (600), CelebA faces (200), camera DCIM photos (59)</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">AI-generated sources (10+ generator families)</dt>
          <dd class="text-flint dark:text-flint-light">ELSA 1M / Stable Diffusion (500), DiffusionDB (500), DALL-E 3 (500), Civitai SFW (500), SDXL-Turbo (300), ArtBench (200), Midjourney v6 (150), Gemini Imagen 4 (70), user-submitted (38), and others</dd>
        </div>
        <div>
          <dt class="font-medium text-text-light dark:text-quartz">Regularisation</dt>
          <dd class="text-flint dark:text-flint-light">C=1.0 (L2), class_weight=balanced, solver=lbfgs, max_iter=1000</dd>
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
        <tbody class="text-flint dark:text-flint-light">
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">AUC-ROC (5-fold cross-validation)</td>
            <td class="py-2 pr-4 font-mono">0.9929</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Authentic false positive rate</td>
            <td class="py-2 pr-4 font-mono">2.7% (77 / 2,873)</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">AI detection rate (recall)</td>
            <td class="py-2 pr-4 font-mono">95.2% (2,987 / 3,136)</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Overall accuracy</td>
            <td class="py-2 pr-4 font-mono">95.7%</td>
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
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
      <li><strong>CLIP dependency:</strong> Requires the open_clip ViT-B/32 model (~350 MB). When CLIP is not installed, the probe is unavailable and gracefully skipped.</li>
      <li><strong>Non-photographic content:</strong> Paintings, digital illustrations, and stylised artwork can produce false positives. Wikimedia art/illustrations were removed from the training corpus after analysis showed a 38% false positive rate on that source.</li>
      <li><strong>High-end camera photos:</strong> Some images from high-end cameras (DJI drones, Sony DSC series) with very clean noise profiles are occasionally flagged. These represent 18 of the 77 current false positives.</li>
      <li><strong>COCO/ImageNet edge cases:</strong> 38 of 77 false positives come from COCO and ImageNet images, likely images with unusual compositions or post-processing that overlap with AI-generated CLIP embeddings.</li>
      <li><strong>Generator coverage:</strong> Trained on 10+ generator families up to April 2026. New generators may produce outputs that fall outside the learned decision boundary. Quarterly retraining planned.</li>
      <li><strong>Demographic bias:</strong> Not yet audited for demographic performance disparities. A demographic bias evaluation is planned (Sprint 29 of the TRIED compliance roadmap).</li>
    </ul>

    <!-- Improvement history -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Improvement History</h3>
    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
      The probe's false positive rate was reduced from 28.7% to 2.7% through a
      systematic four-step process on 6&ndash;7 April 2026:
    </p>
    <div class="overflow-x-auto mb-6">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Step</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">AUC-ROC</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">FP Rate</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Corpus</th>
          </tr>
        </thead>
        <tbody class="text-flint dark:text-flint-light">
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Baseline (C=0.5)</td>
            <td class="py-2 pr-4 font-mono">0.9650</td>
            <td class="py-2 pr-4 font-mono">9.7%</td>
            <td class="py-2 pr-4 font-mono">3,125</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Regularisation tuned (C=1.0)</td>
            <td class="py-2 pr-4 font-mono">0.9710</td>
            <td class="py-2 pr-4 font-mono">9.2%</td>
            <td class="py-2 pr-4 font-mono">3,125</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Wikimedia art removed</td>
            <td class="py-2 pr-4 font-mono">0.9907</td>
            <td class="py-2 pr-4 font-mono">3.7%</td>
            <td class="py-2 pr-4 font-mono">2,826</td>
          </tr>
          <tr>
            <td class="py-2 pr-4">Full expansion (10+ generators)</td>
            <td class="py-2 pr-4 font-mono">0.9929</td>
            <td class="py-2 pr-4 font-mono">2.7%</td>
            <td class="py-2 pr-4 font-mono">6,009</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Version history -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Version History</h3>
    <div class="overflow-x-auto">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Version</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Date</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Change</th>
          </tr>
        </thead>
        <tbody class="text-flint dark:text-flint-light">
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4 font-mono">1.0</td>
            <td class="py-2 pr-4">3 April 2026</td>
            <td class="py-2 pr-4">Initial training. 834 images, AUC 0.9774, FP rate 28.7%.</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4 font-mono">2.0&ndash;5.0</td>
            <td class="py-2 pr-4">4 April 2026</td>
            <td class="py-2 pr-4">Iterative corpus expansion and threshold tuning. Added Gemini, DCIM photos. FP rate reduced to 0% on limited test set.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 font-mono">6.0</td>
            <td class="py-2 pr-4">7 April 2026</td>
            <td class="py-2 pr-4">Major corpus expansion to 6,009 images. 10+ generator families. Wikimedia art removed. C=1.0. AUC 0.9929, FP 2.7%.</td>
          </tr>
        </tbody>
      </table>
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

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed mb-4">
      AI image generators evolve rapidly. Models trained today may not detect
      outputs from generators released six months from now. Jura Trace commits to
      the following update cadence:
    </p>

    <ul class="list-disc pl-5 space-y-2 text-sm text-flint dark:text-flint-light leading-relaxed mb-6">
      <li><strong>Quarterly retraining:</strong> Both classifiers will be retrained at least every three months with newly sourced AI-generated and authentic images.</li>
      <li><strong>New generator coverage:</strong> When major new generators are released (or existing generators receive significant updates), training data from those generators will be incorporated in the next quarterly cycle.</li>
      <li><strong>Model distribution:</strong> Updated model weights will be distributed via the application's auto-update mechanism. Users will be notified when newer models are available.</li>
      <li><strong>Transparency:</strong> This page will be updated with each retraining cycle to reflect the current training data composition, performance metrics, and known limitations.</li>
    </ul>

    <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
      Next scheduled retraining: <strong>July 2026</strong> (Q3).
    </p>
  </section>

</article>
