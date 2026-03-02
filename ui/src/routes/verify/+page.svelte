<script lang="ts">
  // VERIFY workspace — forensics, deepfake detection, claim checking
</script>

<div class="space-y-6">
  <h1 class="text-2xl font-heading">Verify</h1>
  <p class="text-flint dark:text-flint-light">
    Check the authenticity of images, documents, and claims. All analysis happens locally.
  </p>

  <!-- Input methods -->
  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
    <!-- File upload -->
    <div
      class="border-2 border-dashed border-border-light dark:border-border-dark rounded-lg p-8 text-center hover:border-lapis dark:hover:border-lapis-light transition-colors cursor-pointer"
      role="button"
      tabindex="0"
      aria-label="Upload a file to verify"
    >
      <p class="font-medium mb-1">Upload File</p>
      <p class="text-sm text-flint dark:text-flint-light">Image, document, or video</p>
    </div>

    <!-- URL input -->
    <div class="border border-border-light dark:border-border-dark rounded-lg p-8 text-center">
      <p class="font-medium mb-3">Check URL</p>
      <input
        type="url"
        placeholder="https://example.com/image.jpg"
        class="w-full px-3 py-2 rounded border border-border-light dark:border-border-dark bg-surface-light dark:bg-obsidian text-text-light dark:text-text-dark text-sm"
        aria-label="URL to verify"
      />
    </div>

    <!-- Claim text -->
    <div class="border border-border-light dark:border-border-dark rounded-lg p-8 text-center">
      <p class="font-medium mb-3">Check Claim</p>
      <textarea
        placeholder="Is this claim true?"
        rows="2"
        class="w-full px-3 py-2 rounded border border-border-light dark:border-border-dark bg-surface-light dark:bg-obsidian text-text-light dark:text-text-dark text-sm resize-none"
        aria-label="Claim text to verify"
      ></textarea>
    </div>
  </div>

  <!-- Results placeholder -->
  <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-8 text-center">
    <p class="text-flint dark:text-flint-light">Upload a file, paste a URL, or type a claim to begin verification.</p>
  </div>
</div>
