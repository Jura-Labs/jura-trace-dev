<script lang="ts">
  // Static reference page — no reactive state.
  //
  // Source: 2026-04-28 four-agent format-support audit (JTV-105 truth-grid).
  // The truth-grid is the single source of truth for which file formats
  // Jura Trace v1.0 can meaningfully analyse, and what each format gets
  // in the pipeline. This page surfaces the same content to pilots so
  // they can decide before dropping a file rather than after.

  type Coverage = 'full' | 'partial' | 'provenance-only' | 'none';

  type Row = {
    family: string;
    formats: string;
    protect: string;
    verify: string;
    coverage: Coverage;
    note?: string;
  };

  const supportedRows: Row[] = [
    {
      family: 'Photographs (JPEG family)',
      formats: 'JPEG, PNG, TIFF',
      protect:
        'Content Credentials, invisible watermark, perceptual fingerprint, full metadata',
      verify:
        'Full forensic stack — ELA, JPEG Ghost, copy-move, deepfake (GBM v4 + UnivFD v9), CLIP, EXIF anomaly',
      coverage: 'full',
    },
    {
      family: 'Modern lossy codecs',
      formats: 'WebP, AVIF, HEIC',
      protect:
        'Content Credentials, invisible watermark (HEIC excluded), perceptual fingerprint',
      verify:
        'Deepfake, EXIF anomaly, CLIP, segmented ELA. ELA and JPEG Ghost are codec-gated off — they only work on JPEG.',
      coverage: 'partial',
      note: 'iPhone photos arrive as HEIC; Linux builds need pillow-heif (shipped in v1.0).',
    },
    {
      family: 'Camera video',
      formats: 'MP4, MOV',
      protect:
        'Content Credentials (experimental). No watermark, no fingerprint.',
      verify:
        'C2PA content credentials, EXIF metadata, native preview. Deepfake analysis and transcription are planned for v1.0.x — see JTV-139.',
      coverage: 'provenance-only',
      note:
        'v1.0 ships container-level provenance only. Per-frame deepfake, audio-visual sync, and transcription require Global Majority device calibration before re-enabling.',
    },
    {
      family: 'Documents',
      formats: 'PDF',
      protect: 'No Content Credentials, no watermark.',
      verify:
        'PDF Provenance — origin metadata only (signatures, incremental saves, PDF/A). Image manipulation detection is not available for PDF files.',
      coverage: 'provenance-only',
    },
  ];

  const excludedRows: Row[] = [
    {
      family: 'Other video',
      formats: 'WebM, MKV, AVI',
      protect: '—',
      verify: 'No detector path — excluded from the file picker.',
      coverage: 'none',
      note: 'No detector path. Per-frame deepfake is deferred to v1.0.x (JTV-139) for all video formats.',
    },
    {
      family: 'Audio',
      formats: 'WAV, MP3, FLAC, OGG, AAC, M4A',
      protect: '—',
      verify:
        'Audio deepfake model is excluded from v1.0. The current corpus has 2 speakers and 1 TTS engine; an honest detector requires retraining on ASVspoof + WaveFake with held-out speakers.',
      coverage: 'none',
      note:
        'Voice-note evidence is on the v1.1 roadmap (AASIST + ENF + ASVspoof) — see JTV-110.',
    },
    {
      family: 'Office documents',
      formats: 'DOCX, ODT, EPUB, TXT',
      protect: '—',
      verify: 'No detector path — excluded from the file picker.',
      coverage: 'none',
    },
    {
      family: 'Animated images',
      formats: 'GIF',
      protect: '—',
      verify:
        'Animated GIFs have no Content Credentials, watermark, or fingerprint path. Excluded from the file picker for v1.0.',
      coverage: 'none',
    },
    {
      family: '3D models',
      formats: 'STL, OBJ, GLTF, GLB',
      protect: '—',
      verify: 'No detector path — out of scope for v1.0.',
      coverage: 'none',
    },
  ];

  const badgeClass = (c: Coverage): string => {
    switch (c) {
      case 'full':
        return 'bg-malachite/15 text-malachite-dark dark:text-malachite-light border-malachite/30';
      case 'partial':
        return 'bg-amber/15 text-amber-dark dark:text-amber-light border-amber/30';
      case 'provenance-only':
        return 'bg-lapis/15 text-lapis-dark dark:text-lapis-light border-lapis/30';
      case 'none':
        return 'bg-flint/15 text-flint-dark dark:text-flint-light border-flint/30';
    }
  };

  const badgeLabel = (c: Coverage): string => {
    switch (c) {
      case 'full':
        return 'Full pipeline';
      case 'partial':
        return 'Partial';
      case 'provenance-only':
        return 'Provenance only';
      case 'none':
        return 'Not supported';
    }
  };
</script>

<article aria-labelledby="format-support-heading">
  <header class="mb-10">
    <p class="text-xs font-semibold uppercase tracking-widest text-flint-dark dark:text-flint-light mb-2">
      Reference
    </p>
    <h1
      id="format-support-heading"
      class="font-heading text-3xl text-text-light dark:text-quartz mb-4 leading-tight tracking-heading"
    >
      Format Support
    </h1>
    <p class="text-base text-flint-dark dark:text-flint-light leading-relaxed max-w-2xl">
      Jura Trace v1.0 is honest about what it can and cannot meaningfully
      analyse. The grid below is the single source of truth — drawn from
      the 28 April 2026 four-agent audit that compared marketed format
      support against what the pipeline actually does. If your file type
      isn&rsquo;t listed, it doesn&rsquo;t have a working detector path
      yet.
    </p>
    <div class="earth-line mt-6" aria-hidden="true"></div>
  </header>

  <section aria-labelledby="supported-heading" class="mb-12">
    <h2
      id="supported-heading"
      class="font-heading text-xl text-text-light dark:text-quartz mb-4 tracking-heading"
    >
      Supported formats
    </h2>
    <div class="overflow-x-auto -mx-4 sm:mx-0">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark text-left">
            <th class="px-4 py-3 font-semibold text-text-light dark:text-text-dark">Format family</th>
            <th class="px-4 py-3 font-semibold text-text-light dark:text-text-dark">Coverage</th>
            <th class="px-4 py-3 font-semibold text-text-light dark:text-text-dark">Protect</th>
            <th class="px-4 py-3 font-semibold text-text-light dark:text-text-dark">Verify</th>
          </tr>
        </thead>
        <tbody>
          {#each supportedRows as row}
            <tr class="border-b border-border-light/60 dark:border-border-dark/40 align-top">
              <td class="px-4 py-4">
                <p class="font-medium text-obsidian dark:text-quartz">{row.family}</p>
                <p class="text-xs text-flint-dark dark:text-flint-light mt-0.5">{row.formats}</p>
              </td>
              <td class="px-4 py-4 whitespace-nowrap">
                <span class="text-[10px] px-1.5 py-0.5 rounded-full font-medium border {badgeClass(row.coverage)}">
                  {badgeLabel(row.coverage)}
                </span>
              </td>
              <td class="px-4 py-4 text-flint-dark dark:text-flint-light leading-relaxed">
                {row.protect}
              </td>
              <td class="px-4 py-4 text-flint-dark dark:text-flint-light leading-relaxed">
                {row.verify}
                {#if row.note}
                  <p class="text-[11px] italic mt-1 text-flint-dark dark:text-flint-light/80">{row.note}</p>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>

  <section aria-labelledby="excluded-heading" class="mb-12">
    <h2
      id="excluded-heading"
      class="font-heading text-xl text-text-light dark:text-quartz mb-4 tracking-heading"
    >
      Out of scope for v1.0
    </h2>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-2xl mb-4">
      These formats are excluded from the file picker because they have no
      working detector path today. We would rather refuse the file than
      return a 0.50 trust score with no underlying signal — the pilot
      tester audit on 28 April 2026 confirmed this is the more honest
      design.
    </p>
    <div class="overflow-x-auto -mx-4 sm:mx-0">
      <table class="w-full text-sm border-collapse">
        <thead>
          <tr class="border-b border-border-light dark:border-border-dark text-left">
            <th class="px-4 py-3 font-semibold text-text-light dark:text-text-dark">Format family</th>
            <th class="px-4 py-3 font-semibold text-text-light dark:text-text-dark">Status</th>
            <th class="px-4 py-3 font-semibold text-text-light dark:text-text-dark">Reason</th>
          </tr>
        </thead>
        <tbody>
          {#each excludedRows as row}
            <tr class="border-b border-border-light/60 dark:border-border-dark/40 align-top">
              <td class="px-4 py-4">
                <p class="font-medium text-obsidian dark:text-quartz">{row.family}</p>
                <p class="text-xs text-flint-dark dark:text-flint-light mt-0.5">{row.formats}</p>
              </td>
              <td class="px-4 py-4 whitespace-nowrap">
                <span class="text-[10px] px-1.5 py-0.5 rounded-full font-medium border {badgeClass(row.coverage)}">
                  {badgeLabel(row.coverage)}
                </span>
              </td>
              <td class="px-4 py-4 text-flint-dark dark:text-flint-light leading-relaxed">
                {row.verify}
                {#if row.note}
                  <p class="text-[11px] italic mt-1 text-flint-dark dark:text-flint-light/80">{row.note}</p>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>

  <div class="earth-line my-10" aria-hidden="true"></div>

  <section aria-labelledby="why-heading">
    <h2
      id="why-heading"
      class="font-heading text-lg text-text-light dark:text-quartz mb-3 tracking-heading"
    >
      Why we exclude formats rather than report &ldquo;unknown&rdquo;
    </h2>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-2xl mb-3">
      A trust score that does not reflect a working detector is a worse
      outcome than no score at all. For example, a DOCX file dropped onto
      the pipeline previously returned 0.50 trust with zero underlying
      analysis &mdash; pilots interpreted that as &ldquo;clean&rdquo; when
      the system had not actually checked anything. The same principle
      applies to audio and video deepfake: the existing models trained on
      narrow corpora cannot defensibly assess voice notes or video clips
      submitted as evidence, so it is better to defer the feature than to
      ship false confidence. Video deepfake re-enters scope in v1.0.x once
      Global Majority device calibration and per-generator recall data are
      published &mdash; see <span class="font-mono text-xs">JTV-139</span>.
    </p>
    <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed max-w-2xl">
      The full audit transcript and remediation tickets are tracked under
      <span class="font-mono text-xs">JTV-105</span> and
      <span class="font-mono text-xs">JTV-138</span> in the project tracker.
    </p>
  </section>
</article>
