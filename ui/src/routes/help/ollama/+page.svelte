<script lang="ts">
  // Static reference page — no reactive state.
  // Ollama-dependent features are not available in v1.0. This page
  // replaces the previous setup guide pending the v1.0.1 model decision.
</script>

<article class="mx-auto max-w-3xl">

  <!-- Page heading -->
  <header class="mb-8">
    <h1 class="text-3xl font-heading text-text-light dark:text-text-dark tracking-heading mb-3">
      Ollama
    </h1>
    <p class="text-base text-text-light dark:text-quartz leading-relaxed mb-4">
      AI-assisted image descriptions and claim verification are not available in this
      release. They are planned for a later release, where a single multimodal model
      will handle both capabilities in one optional download.
    </p>
  </header>

  <div class="earth-line mb-8" role="separator" aria-hidden="true"></div>

  <section class="mb-10" aria-labelledby="v10-heading">
    <h2 id="v10-heading" class="font-heading text-xl text-text-light dark:text-text-dark mb-3 tracking-heading">
      What is available in v1.0
    </h2>
    <p class="text-sm text-text-light dark:text-quartz leading-relaxed mb-4">
      All 13 forensic detectors, C2PA Validator-Conformant verification, EXIF anomaly
      analysis, perceptual fingerprinting, and Local Signing run entirely without Ollama.
      No LLM download is required to use Jura Trace.
    </p>
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-4">
      <p class="text-sm text-text-light dark:text-quartz leading-relaxed">
        The Service Status panel in
        <a href="/help/settings#service-status" class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded">Settings</a>
        shows an Ollama indicator. A status of Unavailable here has no effect on any v1.0
        feature — it is informational only.
      </p>
    </div>
  </section>

</article>
