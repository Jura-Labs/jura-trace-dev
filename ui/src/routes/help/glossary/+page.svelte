<script lang="ts">
  import { onMount } from 'svelte';

  // Letters that have at least one entry — used to render the jump bar.
  const activeLetters = new Set(['A', 'C', 'D', 'E', 'F', 'G', 'I', 'J', 'L', 'M', 'N', 'O', 'P', 'R', 'S', 'T', 'V', 'W']);

  // All 26 letters rendered in the bar; inactive ones are dimmed and non-interactive.
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('');

  // Smooth-scroll to a letter section anchor.
  // scrollIntoView with smooth behaviour is used for compatibility with Tauri's WebView.
  function scrollToLetter(letterId: string) {
    const el = document.getElementById(letterId);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }
</script>

<!-- Page header -->
<header class="mb-6">
  <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
    Glossary
  </h1>
  <p class="text-base text-flint-dark dark:text-flint-light leading-relaxed max-w-2xl">
    Technical terms used throughout Jura Trace, explained in plain language.
  </p>
</header>

<div class="earth-line mb-6" role="separator" aria-hidden="true"></div>

<!--
  Alphabet jump bar — sticky, only active letters are interactive.
  Semi-transparent obsidian background lets content scroll underneath
  while keeping the bar legible.
-->
<nav
  aria-label="Jump to letter"
  class="sticky top-0 z-10 -mx-1 px-1 py-2 mb-8
         bg-surface-light/95 dark:bg-obsidian/95
         backdrop-blur-sm
         border-b border-border-light dark:border-border-dark"
>
  <ul
    role="list"
    class="flex flex-wrap gap-x-1 gap-y-1"
    aria-label="Alphabet index"
  >
    {#each alphabet as letter}
      {@const active = activeLetters.has(letter)}
      <li>
        {#if active}
          <button
            type="button"
            onclick={() => scrollToLetter(`letter-${letter}`)}
            class="
              inline-flex items-center justify-center w-7 h-7 rounded
              text-sm font-medium
              text-flint-dark dark:text-flint-light
              hover:text-lapis dark:hover:text-lapis dark:text-lapis-light
              hover:bg-lapis/5 dark:hover:bg-lapis/10
              motion-safe:transition-colors motion-safe:duration-150
              focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
              focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
            "
            aria-label="Jump to letter {letter}"
          >{letter}</button>
        {:else}
          <span
            class="inline-flex items-center justify-center w-7 h-7 rounded text-sm
                   text-flint-dark dark:text-flint-light select-none cursor-default"
            aria-hidden="true"
          >{letter}</span>
        {/if}
      </li>
    {/each}
  </ul>
</nav>

<!-- Definition list, grouped by first letter -->
<div class="space-y-10">

  <!-- ══════════ A ══════════ -->
  <section aria-labelledby="letter-A">
    <h2
      id="letter-A"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >A</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-ahash"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >aHash <span class="font-normal text-flint-dark dark:text-flint-light">(Average Hash)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A perceptual hashing algorithm that reduces an image to an 8×8 greyscale grid and
          compares the average brightness of each cell. Fast to compute and effective for
          near-duplicate detection. Jura Trace computes aHash alongside dHash and pHash to
          build a three-signal fingerprint for every registered asset.
        </dd>
      </div>

      <div>
        <dt
          id="term-authentic"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Authentic <span class="font-normal text-flint-dark dark:text-flint-light">(Verdict)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          The verdict assigned when no forensic detectors flag significant anomalies and
          the deepfake ensemble score falls below 0.30. An Authentic verdict does not
          guarantee the content is unmanipulated — it means automated analysis found no
          evidence of manipulation within the limits of current detection methods.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ C ══════════ -->
  <section aria-labelledby="letter-C">
    <h2
      id="letter-C"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >C</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-c2pa"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >C2PA <span class="font-normal text-flint-dark dark:text-flint-light">(Coalition for Content Provenance and Authenticity)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          An open technical standard for embedding cryptographic provenance metadata —
          provenance manifests — directly into digital files. Supported by Adobe, Microsoft,
          and major camera manufacturers. C2PA manifests record who created or edited a
          file, when, and with what tools. Jura Trace can both sign assets with new C2PA
          provenance manifests and verify the integrity of existing manifests.
        </dd>
      </div>

      <div>
        <dt
          id="term-clip"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >CLIP <span class="font-normal text-flint-dark dark:text-flint-light">(Contrastive Language-Image Pre-training)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A neural network trained to understand images and text jointly. Jura Trace uses an
          open-source implementation — open_clip ViT-B/32 — for zero-shot AI/authentic
          classification without requiring task-specific training data. Optional: requires
          a one-time model download of approximately 350 MB.
        </dd>
      </div>

      <div>
        <dt
          id="term-composite"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Composite</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          An image created by combining elements from multiple source images. Composite
          detection is the purpose of Jura Trace's regional forensic detectors: segmented
          ELA, shadow consistency, colour temperature, and splice boundary analysis. When
          two or more regional detectors fire simultaneously, the trust score applies a
          composite amplification adjustment.
        </dd>
      </div>

      <div>
        <dt
          id="term-c2pa-provenance"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >C2PA Provenance Manifest</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          The cryptographic provenance metadata embedded in a file under the open
          <a
            href="https://c2pa.org/"
            target="_blank"
            rel="noopener noreferrer"
            class="text-lapis dark:text-lapis-light underline hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          >C2PA (Coalition for Content Provenance and Authenticity)</a>
          standard. A C2PA provenance manifest contains information about the creator,
          creation date, editing tools, and any subsequent modifications. Each layer of
          editing leaves a verifiable deposit in the file's provenance chain — a
          geological record of the content's history. Jura Trace signs with a C2PA
          provenance manifest and verifies manifests from other signers including Adobe,
          BBC, Canon, Leica, Microsoft, and Nikon.
          <span class="block mt-2 text-xs text-flint-dark dark:text-flint-light">
            See also: <em>Content Credentials</em> — Adobe's branded term for the same
            underlying C2PA standard. The two terms refer to the same technology.
          </span>
        </dd>
      </div>

      <div>
        <dt
          id="term-copy-move"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Copy-Move Detection</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A forensic technique that searches for duplicated regions within a single image.
          Clone-stamp tools and copy-paste manipulation leave regions that are identical —
          or near-identical — to other parts of the same image. Jura Trace detects these
          matches and highlights suspected clone regions in a visualisation overlay.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ D ══════════ -->
  <section aria-labelledby="letter-D">
    <h2
      id="letter-D"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >D</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-deep-mode"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Deep Mode</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          An investigation mode that runs the full forensic pipeline including all regional
          detectors: segmented ELA, shadow consistency, colour temperature, and splice
          boundary. More thorough than Standard mode. Typical analysis time: 30–60 seconds
          depending on file size and whether Ollama is running.
        </dd>
      </div>

      <div>
        <dt
          id="term-deepfake"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Deepfake</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          Synthetic or manipulated media created using artificial intelligence. The term
          originally referred to face-swapped video but now broadly covers AI-generated
          images, video, and audio. Jura Trace's deepfake detection ensemble combines eight
          forensic signals with a trained GBM classifier to produce a continuous score
          from 0 to 1.
        </dd>
      </div>

      <div>
        <dt
          id="term-dhash"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >dHash <span class="font-normal text-flint-dark dark:text-flint-light">(Difference Hash)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A perceptual hashing algorithm that compares adjacent pixel brightness differences
          rather than absolute values. More sensitive to structural changes — such as
          cropping or content removal — than aHash. Used alongside aHash and pHash in
          Jura Trace's fingerprinting system.
        </dd>
      </div>

      <div>
        <dt
          id="term-dwt-dct-svd"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >DWT-DCT-SVD</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          The invisible watermarking technique used by Jura Trace. Embeds a 128-bit UUID
          payload in the frequency domain of an image using three sequential transforms:
          Discrete Wavelet Transform, Discrete Cosine Transform, and Singular Value
          Decomposition. The embedded signal survives JPEG compression at quality 70 or
          above, proportional resizing, and up to 30% cropping. Three strength levels are
          available — Low, Medium, and High — trading imperceptibility against robustness.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ E ══════════ -->
  <section aria-labelledby="letter-E">
    <h2
      id="letter-E"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >E</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-ela"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >ELA <span class="font-normal text-flint-dark dark:text-flint-light">(Error Level Analysis)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A forensic technique that re-compresses an image at a known quality level and
          measures the difference between the re-compressed and original versions. Regions
          with a different compression history — such as content pasted from another source
          image — appear as bright patches in the ELA map because they have not yet reached
          the same error floor as the rest of the image. Jura Trace offers both whole-image
          ELA and segmented ELA for regional analysis.
        </dd>
      </div>

      <div>
        <dt
          id="term-exif"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >EXIF <span class="font-normal text-flint-dark dark:text-flint-light">(Exchangeable Image File Format)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          Metadata embedded in image files by cameras and editing software. Includes camera
          make and model, capture date and time, GPS coordinates, resolution, exposure
          settings, and software modification history. Jura Trace evaluates 12 EXIF anomaly
          rules — looking for inconsistencies such as a software timestamp that predates the
          camera's capture timestamp, or GPS coordinates that conflict with other metadata.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ F ══════════ -->
  <section aria-labelledby="letter-F">
    <h2
      id="letter-F"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >F</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-fingerprint"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Fingerprint</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A compact numerical representation of an image's visual content, computed using
          perceptual hashing. See
          <a href="#term-perceptual-hash" class="text-lapis dark:text-lapis-light underline decoration-lapis/30 hover:decoration-lapis dark:decoration-lapis-light/30 dark:hover:decoration-lapis-light">Perceptual Hash</a>.
          Jura Trace stores three fingerprint variants (aHash, dHash, pHash) per asset to
          support robust near-duplicate matching even after an asset has been resized,
          compressed, or lightly edited.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ G ══════════ -->
  <section aria-labelledby="letter-G">
    <h2
      id="letter-G"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >G</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-gbm"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >GBM <span class="font-normal text-flint-dark dark:text-flint-light">(Gradient Boosted Machine)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          The machine learning classifier at the centre of Jura Trace's AI detection
          ensemble. Current version is <strong>GBM v4</strong>, trained on an
          84-feature vector extracted from the forensic signal pipeline using a
          corpus of 10,709 images (5,724 authentic, 4,985 AI-generated from 14
          generator families). Achieves a cross-validation AUC-ROC of 0.9868 with
          an authentic false-positive rate of 4.54% at the calibrated threshold.
          Combined with the UnivFD v10onnx CLIP probe (AUC-ROC 0.9929) into an ensemble
          score. Degrades gracefully — analysis continues with heuristic scoring
          alone if the model file is not present.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ I ══════════ -->
  <section aria-labelledby="letter-I">
    <h2
      id="letter-I"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >I</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-inconclusive"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Inconclusive <span class="font-normal text-flint-dark dark:text-flint-light">(Verdict)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          The verdict assigned when forensic signals are mixed or insufficient to reach a
          confident classification. Corresponds to a deepfake ensemble score between 0.30
          and 0.65. Inconclusive is the most common outcome for legitimate photographs that
          have been processed, shared, or lightly edited. It means the analysis cannot
          confidently classify the content as authentic or synthetic — not that it suspects
          manipulation.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ J ══════════ -->
  <section aria-labelledby="letter-J">
    <h2
      id="letter-J"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >J</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-jpeg-ghost"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >JPEG Ghost</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A forensic technique that detects double JPEG compression. When a region from one
          JPEG is pasted into another and the combined image is re-saved, the spliced region
          retains artefacts from its original compression quality level. Comparing the image
          against a sweep of re-compression qualities reveals these hidden strata — like
          reading the compressed layers in rock to identify past events.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ L ══════════ -->
  <section aria-labelledby="letter-L">
    <h2
      id="letter-L"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >L</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-lbp"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >LBP <span class="font-normal text-flint-dark dark:text-flint-light">(Local Binary Pattern)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A texture descriptor that encodes the relationship between each pixel and its
          immediate neighbours as a binary string. Natural photographs have characteristic
          LBP distributions; AI-generated images often produce measurably different ones.
          LBP block variance is one of the three highest-weighted features in Jura Trace's
          GBM classifier. LBP drift would also serve as a temporal consistency signal in
          video deepfake analysis — that detector is under evaluation for a future release
          (deferred from v1.0).
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ M ══════════ -->
  <section aria-labelledby="letter-M">
    <h2
      id="letter-M"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >M</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-manifest"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Manifest</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          In C2PA terminology, the cryptographic record embedded in a file that describes
          its provenance: who created it, when, with what tools, and what edits were made.
          A manifest is cryptographically signed so that any subsequent modification to the
          file invalidates the signature. Jura Trace displays manifest details — including
          the claim generator, signing timestamp, and individual assertions — in the C2PA
          panel on the Verify page.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ N ══════════ -->
  <section aria-labelledby="letter-N">
    <h2
      id="letter-N"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >N</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-noise-analysis"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Noise Analysis</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          Examination of the random grain pattern in an image, produced by a camera's image
          sensor. Natural photographs have a consistent, spatially uniform noise signature.
          AI-generated images often exhibit unnaturally smooth regions, inconsistent noise
          across the frame, or noise patterns that differ from natural sensor characteristics.
          Jura Trace maps per-block noise variance and flags anomalous blocks in a heatmap
          overlay.
        </dd>
      </div>

      <div>
        <dt
          id="term-npr"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >NPR <span class="font-normal text-flint-dark dark:text-flint-light">(Neighbouring Pixel Relationships)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A forensic technique that analyses the statistical relationships between adjacent
          pixels. Natural photographs captured through a lens and sensor have characteristic
          NPR patterns — spatial correlations that AI generation tends to disrupt. Jura Trace
          computes horizontal/vertical correlation, differential variance ratio, and
          high-frequency energy ratio to produce a single NPR anomaly score.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ O ══════════ -->
  <section aria-labelledby="letter-O">
    <h2
      id="letter-O"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >O</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-ollama"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Ollama</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A local large language model runtime used optionally by Jura Trace. When Ollama
          is running, it enables two additional capabilities: image description via the
          LLaVA vision model, and factual claim verification via the Qwen2.5 text model
          (RAG claim checker). All inference runs entirely on your device — no data leaves
          your machine. Jura Trace operates normally when Ollama is unavailable; these
          features are simply skipped.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ P ══════════ -->
  <section aria-labelledby="letter-P">
    <h2
      id="letter-P"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >P</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-perceptual-hash"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Perceptual Hash <span class="font-normal text-flint-dark dark:text-flint-light">(pHash)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A fingerprinting technique that creates a compact numerical representation of an
          image's visual content. Unlike cryptographic hashes — where a single changed pixel
          produces a completely different hash — perceptual hashes remain similar when an
          image is resized, compressed, colour-corrected, or lightly edited. Jura Trace
          computes three perceptual hash variants per asset: aHash, dHash, and pHash.
          Comparing hashes between assets enables near-duplicate matching across your
          collection.
        </dd>
      </div>

      <div>
        <dt
          id="term-provenance"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Provenance</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          The documented history of a piece of content — who created it, how it has been
          modified, and where it has been. Provenance is the bedrock of trust in digital
          media. C2PA provenance manifests are the primary provenance mechanism in Jura Trace,
          providing a tamper-evident chain from original capture through every subsequent
          edit.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ R ══════════ -->
  <section aria-labelledby="letter-R">
    <h2
      id="letter-R"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >R</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-rag"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >RAG <span class="font-normal text-flint-dark dark:text-flint-light">(Retrieval-Augmented Generation)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A technique that combines a large language model with a searchable knowledge base
          to verify factual claims. Jura Trace uses RAG with the Ollama Qwen2.5 model to
          check claims extracted from transcribed audio and video content. All retrieval
          and inference runs locally using TF-IDF search over Jura Trace's local knowledge
          base — no web requests are made. Requires Ollama to be running.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ S ══════════ -->
  <section aria-labelledby="letter-S">
    <h2
      id="letter-S"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >S</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-segmented-ela"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Segmented ELA</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          An enhanced version of Error Level Analysis that divides the image into an 8×8
          grid and analyses each region independently. Localised manipulation that a
          whole-image ELA might average out — for example, a small pasted element in a
          scene — becomes visible when the grid approach isolates it. One of four regional
          forensic detectors that contribute to composite image detection in Deep mode.
        </dd>
      </div>

      <div>
        <dt
          id="term-shadow-consistency"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Shadow Consistency</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A forensic technique that analyses light direction across different image regions
          using gradient-weighted calculations. In a genuine photograph, shadows and
          highlights point toward the same light source throughout the frame. Composited
          images often contain elements from different source photographs, so shadows in
          different regions can point in conflicting directions.
        </dd>
      </div>

      <div>
        <dt
          id="term-sidecar"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Analysis Engine</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          The forensic analysis service that runs alongside the Jura Trace desktop
          application on a local loopback port chosen at startup. Handles computationally
          intensive forensic analysis — ELA, noise, copy-move, deepfake detection
          (GBM v4 + UnivFD v10onnx ensemble), CLIP classification, JPEG Ghost, segmented
          ELA, colour temperature, watermarking — plus the on-demand investigation tools
          (NPR, shadow consistency, splice boundary) available in Expert View. Video
          deepfake and audio analysis are deferred from v1.0 and under evaluation for
          a future release. Jura Trace operates normally when the Analysis Engine is
          not running; forensic analysis results are simply omitted from the report.
        </dd>
      </div>

      <div>
        <dt
          id="term-splice-boundary"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Splice Boundary</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A forensic technique that examines edges within an image for three distinct
          manipulation signals: JPEG compression grid discontinuities, noise level changes
          at boundaries, and feathering artefacts consistent with soft-selection pasting.
          Genuine photographs show smooth, continuous edge characteristics; spliced images
          often have abrupt transitions in one or more of these signals.
        </dd>
      </div>

      <div>
        <dt
          id="term-standard-mode"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Standard Mode</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          The default investigation mode. Runs EXIF anomaly analysis, C2PA manifest
          verification, whole-image ELA, noise analysis, copy-move detection, and the
          deepfake ensemble. Typically completes in 10–20 seconds. Regional detectors
          (segmented ELA, shadow consistency, colour temperature, splice boundary) are
          not included — use Deep mode when investigating possible composite images.
        </dd>
      </div>

      <div>
        <dt
          id="term-synthetic"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Synthetic <span class="font-normal text-flint-dark dark:text-flint-light">(Verdict)</span></dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          The verdict assigned when multiple forensic detectors flag strong AI generation
          or manipulation signals and the deepfake ensemble score exceeds 0.65. Indicates
          high confidence that the content was artificially generated or significantly
          altered. A Synthetic verdict does not identify the specific tool used — it means
          the combined forensic evidence exceeds the confidence threshold for flagging.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ T ══════════ -->
  <section aria-labelledby="letter-T">
    <h2
      id="letter-T"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >T</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-temporal-consistency"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Temporal Consistency</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          In video deepfake analysis, a measure of how forensic signals change between
          frames over time. Genuine video has consistent noise patterns, spectral
          characteristics, and LBP texture distributions across frames. Deepfake video —
          where frames are synthesised independently — often shows frame-to-frame drift in
          these signals. Jura Trace computes temporal consistency from three drift metrics:
          noise drift, spectral drift, and LBP drift.
        </dd>
      </div>

      <div>
        <dt
          id="term-trust-score"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Trust Score</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A percentage from 0% to 100% reflecting how many forensic signals indicate
          authentic content. Computed from two weighted components: EXIF metadata analysis
          (40% weight) and the forensic signal pipeline (60% weight). A high trust score
          means few anomalies were detected; a low score means multiple detectors raised
          concerns. The trust score is a forensic confidence indicator — not a guarantee
          of authenticity. The
          <a href="#term-verdict" class="text-lapis dark:text-lapis-light underline decoration-lapis/30 hover:decoration-lapis dark:decoration-lapis-light/30 dark:hover:decoration-lapis-light">Verdict</a>
          is the primary classification output.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ V ══════════ -->
  <section aria-labelledby="letter-V">
    <h2
      id="letter-V"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >V</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-verdict"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Verdict</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          The three-way classification assigned to verified content:
          <a href="#term-authentic" class="text-lapis dark:text-lapis-light underline decoration-lapis/30 hover:decoration-lapis dark:decoration-lapis-light/30 dark:hover:decoration-lapis-light">Authentic</a>,
          <a href="#term-inconclusive" class="text-lapis dark:text-lapis-light underline decoration-lapis/30 hover:decoration-lapis dark:decoration-lapis-light/30 dark:hover:decoration-lapis-light">Inconclusive</a>,
          or
          <a href="#term-synthetic" class="text-lapis dark:text-lapis-light underline decoration-lapis/30 hover:decoration-lapis dark:decoration-lapis-light/30 dark:hover:decoration-lapis-light">Synthetic</a>.
          Determined by the deepfake detection ensemble score, not the trust score alone.
          The verdict and the trust score can disagree — for example, an image with clean
          EXIF metadata (high trust score) but strong AI generation signals (Synthetic
          verdict). Always consider both values together.
        </dd>
      </div>

    </dl>
  </section>

  <!-- ══════════ W ══════════ -->
  <section aria-labelledby="letter-W">
    <h2
      id="letter-W"
      class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-4 scroll-mt-16"
    >W</h2>
    <div class="earth-line mb-5" role="separator" aria-hidden="true"></div>
    <dl class="space-y-5">

      <div>
        <dt
          id="term-watermark"
          class="font-medium text-base text-text-light dark:text-text-dark scroll-mt-20"
        >Watermark (Invisible)</dt>
        <dd class="mt-1 text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-prose">
          A signal embedded in the frequency domain of an image using the
          <a href="#term-dwt-dct-svd" class="text-lapis dark:text-lapis-light underline decoration-lapis/30 hover:decoration-lapis dark:decoration-lapis-light/30 dark:hover:decoration-lapis-light">DWT-DCT-SVD</a>
          technique. Invisible to the human eye, the watermark carries a 128-bit UUID that
          identifies the asset and the institution that protected it. It survives JPEG
          compression at quality 70 or above, proportional resizing, and up to 30% cropping.
          Jura Trace can both embed invisible watermarks during the Protect workflow and
          extract them during the Verify workflow — providing a secondary provenance layer
          that does not depend on file metadata.
        </dd>
      </div>

    </dl>
  </section>

</div>

<!-- Footer -->
<div class="earth-line mt-10 mb-6" role="separator" aria-hidden="true"></div>
<p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed">
  Jura Trace processes all analysis on-device. No content, metadata, or fingerprints are
  transmitted to external services. See
  <a href="/help/methodology" class="text-lapis dark:text-lapis-light underline decoration-lapis/30 hover:decoration-lapis dark:decoration-lapis-light/30 dark:hover:decoration-lapis-light">How Analysis Works</a>
  for a full account of each detector's methodology and known limitations.
</p>
