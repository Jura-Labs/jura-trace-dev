<script lang="ts">
  import type { InputQualityAssessment } from '$lib/types';

  let { quality }: { quality: InputQualityAssessment } = $props();

  // Build list of applicable warnings based on quality assessment conditions
  let warnings = $derived.by(() => {
    const w: { id: string; text: string }[] = [];

    if (quality.isJpeg && quality.jpegQualityEstimate != null && quality.jpegQualityEstimate < 40) {
      w.push({
        id: 'low-quality',
        text: `This image has been heavily compressed (estimated quality: ${quality.jpegQualityEstimate}%). Compression reduces the reliability of ELA, noise analysis, and JPEG ghost detection.`,
      });
    }

    if (quality.resolutionCategory === 'low') {
      w.push({
        id: 'low-res',
        text: `This is a low-resolution image (${quality.width}\u00d7${quality.height}). Deepfake detection, copy-move analysis, and regional forensics are less reliable at this resolution.`,
      });
    }

    if (quality.isScreenshotLikely) {
      w.push({
        id: 'screenshot',
        text: 'This appears to be a screenshot. Screenshots lose compression artefacts and metadata, reducing the effectiveness of EXIF analysis, JPEG forensics, and frequency analysis.',
      });
    }

    if (!quality.isJpeg && quality.resolutionCategory !== 'n/a') {
      w.push({
        id: 'non-jpeg',
        text: 'JPEG ghost analysis is not applicable to this file format.',
      });
    }

    if (!quality.hasGps || !quality.hasTimestamp) {
      w.push({
        id: 'no-geo',
        text: 'No GPS coordinates or timestamp found in metadata. Sun position, weather cross-reference, and shadow time estimation are unavailable.',
      });
    }

    return w;
  });
</script>

{#if warnings.length > 0}
  <div class="space-y-2 mb-6" role="region" aria-label="Analysis limitations">
    {#each warnings as warning (warning.id)}
      <div
        class="flex gap-3 px-4 py-3 rounded-lg border border-amber-500/30 bg-amber-50 dark:bg-amber-900/10 text-sm"
        role="status"
      >
        <span class="flex-shrink-0 text-amber-600 dark:text-amber-400 mt-0.5" aria-hidden="true">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
            <path fill-rule="evenodd" d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.17 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625L8.485 2.495zM10 6a.75.75 0 01.75.75v3.5a.75.75 0 01-1.5 0v-3.5A.75.75 0 0110 6zm0 9a1 1 0 100-2 1 1 0 000 2z" clip-rule="evenodd" />
          </svg>
        </span>
        <p class="text-amber-800 dark:text-amber-300 leading-relaxed">{warning.text}</p>
      </div>
    {/each}
  </div>
{/if}
