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
        { href: '#kb-retrieval',   label: 'Knowledge Base Retrieval (preliminary)' },
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
          <dd class="text-flint-dark dark:text-flint-light">10,709 (5,724 authentic + 4,985 AI-generated)</dd>
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
            <td class="py-2 pr-4">First public release. Corpus of 10,709 images across 14 AI-generator families with the metrics shown above.</td>
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
          <dt class="font-medium text-text-light dark:text-quartz">Total images</dt>
          <dd class="text-flint-dark dark:text-flint-light">39,016 (10,712 original + 32,142 platform-forwarded augmentation via Q=75/85/2× re-saves)</dd>
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
  <!-- Knowledge Base Retrieval (preliminary investigative aid)          -->
  <!-- ══════════════════════════════════════════════════════════════════ -->
  <section aria-labelledby="kb-retrieval-heading" class="mb-14" id="kb-retrieval">
    <h2
      id="kb-retrieval-heading"
      class="font-heading text-2xl text-text-light dark:text-quartz mb-2 leading-tight tracking-heading"
    >
      Knowledge Base Retrieval
    </h2>
    <p class="text-sm font-medium text-amber-dark dark:text-amber-light mb-4">
      Preliminary investigative aid &mdash; not a fact-checker
    </p>

    <div
      class="rounded-lg border border-amber/30 bg-amber/5 dark:bg-amber/10 px-5 py-4 mb-6"
      role="note"
      aria-label="Non-warranty notice"
    >
      <p class="text-sm font-semibold text-amber-dark dark:text-amber-light mb-2">
        Non-warranty notice
      </p>
      <p class="text-sm text-text-light/90 dark:text-quartz/90 leading-relaxed mb-2">
        The knowledge base retrieval tool matches analyst-entered claims against a
        small preliminary reference corpus and surfaces the most related passages.
        It is shipped as an <strong>investigative aid only</strong>. It is:
      </p>
      <ul class="list-disc pl-5 text-sm text-text-light/90 dark:text-quartz/90 leading-relaxed space-y-1">
        <li><strong>not</strong> a fact-checker, and does not produce verdicts about the truth or falsity of any claim, person, organisation, or event;</li>
        <li><strong>not</strong> formally evaluated for accuracy;</li>
        <li><strong>not</strong> validated for legal, medical, financial, or political claim verification;</li>
        <li><strong>not</strong> a source of authority and must not be cited as such in any published or evidentiary context.</li>
      </ul>
      <p class="text-sm text-text-light/90 dark:text-quartz/90 leading-relaxed mt-2">
        A full curated corpus of 5,000+ passages with multilingual coverage is
        under evaluation for a future release. The current preliminary corpus is
        documented below so that users understand exactly what the tool can and
        cannot see.
      </p>
    </div>

    <!-- Intended use -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Intended Use</h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-4">
      When an analyst enters a textual claim during verification (for example
      "this image was taken during the Myanmar protests of 2021"), the tool
      retrieves related passages from its preliminary reference corpus and
      returns them alongside a grounded match assessment. The intended use is to
      surface reference material the analyst may want to consult, not to
      adjudicate the claim.
    </p>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      The tool does <em>not</em> contribute to the forensic verdict or the numeric
      trust score. It is a distinct investigative aid displayed alongside the
      forensic detectors, not one of the 11 forensic signals that make up the
      detection ensemble.
    </p>

    <!-- Architecture -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Architecture</h3>
    <ul class="list-disc pl-5 space-y-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      <li><strong>Retrieval:</strong> TF-IDF over local markdown reference files (no network calls).</li>
      <li><strong>Grounded assessment:</strong> Local Qwen2.5 model via Ollama (optional dependency). Temperature 0.1.</li>
      <li><strong>Output vocabulary:</strong> <code class="font-mono text-xs">consistent_with_kb</code>, <code class="font-mono text-xs">inconsistent_with_kb</code>, or <code class="font-mono text-xs">insufficient_context_in_kb</code>. No "verdicts" are emitted.</li>
      <li><strong>Fail-closed behaviour:</strong> when no passages are retrieved above the relevance threshold, the tool returns <code class="font-mono text-xs">insufficient_context_in_kb</code> without invoking the language model. This is deliberate &mdash; it prevents the model from reasoning from parametric memory about claims the corpus cannot support.</li>
      <li><strong>All processing is local:</strong> no cloud calls, no telemetry, no data leaves the device.</li>
    </ul>

    <!-- Preliminary corpus composition -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Preliminary Corpus Composition</h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-3">
      The current knowledge base is a small, hand-curated set of reference passages
      across six domain documents. This is a <strong>preliminary</strong> corpus &mdash; it
      is approximately two orders of magnitude smaller than a production fact-checking
      corpus. It is published here so that users can assess for themselves whether the
      tool has coverage of the claim they are investigating.
    </p>
    <div class="overflow-x-auto mb-6">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark">
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Domain document</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Approx. passages</th>
            <th class="py-2 pr-4 text-left font-medium text-text-light dark:text-quartz">Notes</th>
          </tr>
        </thead>
        <tbody class="text-flint-dark dark:text-flint-light">
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">C2PA standards reference</td>
            <td class="py-2 pr-4 tabular-nums">~40</td>
            <td class="py-2 pr-4">Assertion types, claim generator field, manifest structure.</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Image forensic methodology</td>
            <td class="py-2 pr-4 tabular-nums">~30</td>
            <td class="py-2 pr-4">ELA, noise, splice, compression artefacts.</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Video forensics</td>
            <td class="py-2 pr-4 tabular-nums">~25</td>
            <td class="py-2 pr-4">Per-frame analysis, temporal consistency, face-swap limitations.</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Digital rights &amp; cultural heritage</td>
            <td class="py-2 pr-4 tabular-nums">~25</td>
            <td class="py-2 pr-4">Object ID, provenance, institutional use.</td>
          </tr>
          <tr class="border-b border-border-light/50 dark:border-border-dark/50">
            <td class="py-2 pr-4">Deepfake detection references</td>
            <td class="py-2 pr-4 tabular-nums">~20</td>
            <td class="py-2 pr-4">Classifier architecture, training corpora, benchmark context.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4">Misinformation research notes</td>
            <td class="py-2 pr-4 tabular-nums">~10</td>
            <td class="py-2 pr-4">Open source investigation references.</td>
          </tr>
        </tbody>
      </table>
    </div>
    <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      Approximate total: 150 passages across 314 lines of source material. The
      published corpus is available in the source repository under
      <code class="font-mono">sidecar/knowledge_base/</code> and is covered by the
      same licence as the rest of the application.
    </p>

    <!-- Known limitations and failure modes -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Known Limitations and Failure Modes</h3>
    <ul class="list-disc pl-5 space-y-2 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      <li><strong>Coverage gap:</strong> the corpus contains no references to specific events, individuals, locations, or dates. Any claim about a real-world event, a named person, or a specific place will return <code class="font-mono text-xs">insufficient_context_in_kb</code>. This is by design &mdash; the tool is not a fact-checker.</li>
      <li><strong>No accuracy metrics:</strong> the tool has not been formally evaluated against a held-out test set. No precision, recall, or F1 numbers are published because none have been measured. This is a limitation, not a secret.</li>
      <li><strong>Subjective and future claims:</strong> the tool cannot assess claims that are subjective ("this is the best photograph of X"), predictive ("this event will happen"), or otherwise outside its reference material.</li>
      <li><strong>Adversarial paraphrase:</strong> the TF-IDF retrieval layer is vulnerable to paraphrase attacks. A claim phrased differently from the corpus wording may fail to retrieve relevant passages even when coverage exists.</li>
      <li><strong>Monolingual corpus:</strong> all current reference passages are in English. Non-English claims will be transcribed and processed, but the retrieval will only match against English references. Multilingual corpus coverage is under evaluation for a future release.</li>
      <li><strong>No public figure heuristics:</strong> the current corpus does not include information about public figures, and the curation process for future expansion will exclude such references entirely to avoid defamation exposure.</li>
    </ul>

    <!-- Out-of-scope use -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Out-of-Scope Use</h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-2">
      This tool must not be used for:
    </p>
    <ul class="list-disc pl-5 space-y-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      <li>Fact-checking claims about named individuals, organisations, or events for publication.</li>
      <li>Any evidentiary, legal, medical, financial, or political claim verification.</li>
      <li>Automated decision-making of any kind.</li>
      <li>Moderation, content classification, or policy enforcement.</li>
      <li>Any use that would attribute forensic authority to its output.</li>
    </ul>

    <!-- Planned expansion -->
    <h3 class="font-medium text-base text-text-light dark:text-quartz mb-2">Planned Expansion</h3>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-2">
      A curated corpus expansion is under evaluation for a future release.
      The intended scope includes:
    </p>
    <ul class="list-disc pl-5 space-y-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      <li>Expansion to approximately 5,000&ndash;10,000 passages across C2PA specification, Berkeley Protocol, ICC Office of the Prosecutor guidelines, ENFSI image forensic guidelines, cultural heritage provenance standards, synthetic media taxonomy references, deepfake detection methodology literature, and misinformation research standards.</li>
      <li>Multilingual coverage in at least five languages (Arabic, Spanish, French, Swahili, Burmese) matching the TRIED case study languages.</li>
      <li>A held-out test set of 150&ndash;300 hand-curated (claim, expected outcome, expected passage) triples for formal accuracy evaluation.</li>
      <li>Per-passage provenance metadata, licence-compatibility audit, and published evaluation metrics.</li>
      <li>A corpus review cycle aligned with the classifier retraining cadence.</li>
      <li>Corpus publication as a separate data artefact on Zenodo with a DOI, aligned with the Continuity Promise.</li>
    </ul>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-6">
      This expansion is conditional on funding outcomes and corpus curation
      feasibility. Until it lands, the current tool remains a preliminary
      investigative aid as documented above.
    </p>

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
            <td class="py-2 pr-4 font-mono">v1.0 launch corpus</td>
            <td class="py-2 pr-4">22 June 2026</td>
            <td class="py-2 pr-4">Six-document preliminary corpus shipped as an investigative aid. ~150 passages across the domains documented above. Output vocabulary: <code class="font-mono text-xs">consistent_with_kb / inconsistent_with_kb / insufficient_context_in_kb</code>. Not a fact-checker.</td>
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
