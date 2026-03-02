<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '$lib/api';

  // ── Persistence keys ────────────────────────────────────────────
  const KEY_OLLAMA_URL    = 'jura-ollama-url';
  const KEY_VISION_MODEL  = 'jura-vision-model';
  const KEY_TEXT_MODEL    = 'jura-text-model';

  // ── Defaults ────────────────────────────────────────────────────
  const DEFAULT_OLLAMA_URL   = 'http://localhost:11434';
  const DEFAULT_VISION_MODEL = 'llava:7b';
  const DEFAULT_TEXT_MODEL   = 'qwen2.5:7b-instruct';

  // ── State ───────────────────────────────────────────────────────
  let ollamaUrl    = $state(DEFAULT_OLLAMA_URL);
  let visionModel  = $state(DEFAULT_VISION_MODEL);
  let textModel    = $state(DEFAULT_TEXT_MODEL);
  let appVersion   = $state('0.1.0-dev');

  let saved        = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    ollamaUrl   = localStorage.getItem(KEY_OLLAMA_URL)   ?? DEFAULT_OLLAMA_URL;
    visionModel = localStorage.getItem(KEY_VISION_MODEL) ?? DEFAULT_VISION_MODEL;
    textModel   = localStorage.getItem(KEY_TEXT_MODEL)   ?? DEFAULT_TEXT_MODEL;
    appVersion  = await getVersion();
  });

  function saveSettings() {
    localStorage.setItem(KEY_OLLAMA_URL,   ollamaUrl);
    localStorage.setItem(KEY_VISION_MODEL, visionModel);
    localStorage.setItem(KEY_TEXT_MODEL,   textModel);

    // Show confirmation briefly
    saved = true;
    if (saveTimer !== null) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => { saved = false; }, 2500);
  }
</script>

<div class="space-y-6">
  <h1 class="text-2xl font-heading text-quartz">Settings</h1>

  <!-- Ollama Configuration -->
  <section
    class="bg-graphite rounded-lg border border-graphite-light p-6"
    aria-labelledby="ollama-heading"
  >
    <h2 id="ollama-heading" class="text-lg font-heading text-quartz mb-4">Ollama Configuration</h2>
    <div class="space-y-4">

      <div>
        <label for="ollama-url" class="block text-sm font-medium text-quartz mb-1">
          Ollama URL
        </label>
        <input
          id="ollama-url"
          type="url"
          bind:value={ollamaUrl}
          class="w-full max-w-md px-3 py-2 rounded border border-graphite-light bg-obsidian text-quartz text-sm
                 focus:outline-none focus:ring-2 focus:ring-lapis focus:border-transparent transition-colors"
          placeholder={DEFAULT_OLLAMA_URL}
          autocomplete="off"
          spellcheck={false}
        />
        <p class="text-xs text-flint mt-1">
          Required for auto-cataloguing and claim checking
        </p>
      </div>

      <div>
        <label for="vision-model" class="block text-sm font-medium text-quartz mb-1">
          Vision Model
        </label>
        <input
          id="vision-model"
          type="text"
          bind:value={visionModel}
          class="w-full max-w-md px-3 py-2 rounded border border-graphite-light bg-obsidian text-quartz text-sm
                 focus:outline-none focus:ring-2 focus:ring-lapis focus:border-transparent transition-colors"
          placeholder={DEFAULT_VISION_MODEL}
          autocomplete="off"
          spellcheck={false}
        />
        <p class="text-xs text-flint mt-1">
          Used for image description and visual analysis
        </p>
      </div>

      <div>
        <label for="text-model" class="block text-sm font-medium text-quartz mb-1">
          Text Model
        </label>
        <input
          id="text-model"
          type="text"
          bind:value={textModel}
          class="w-full max-w-md px-3 py-2 rounded border border-graphite-light bg-obsidian text-quartz text-sm
                 focus:outline-none focus:ring-2 focus:ring-lapis focus:border-transparent transition-colors"
          placeholder={DEFAULT_TEXT_MODEL}
          autocomplete="off"
          spellcheck={false}
        />
        <p class="text-xs text-flint mt-1">
          Used for claim verification and metadata summarisation
        </p>
      </div>

      <!-- Save row -->
      <div class="flex items-center gap-4 pt-2">
        <button
          onclick={saveSettings}
          class="px-5 py-2 bg-lapis hover:bg-lapis-light text-white text-sm font-medium rounded transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
        >
          Save Settings
        </button>

        {#if saved}
          <p
            class="text-sm text-malachite-light motion-safe:animate-fade-in"
            role="status"
            aria-live="polite"
          >
            Settings saved.
          </p>
        {/if}
      </div>

    </div>
  </section>

  <!-- Data Storage -->
  <section
    class="bg-graphite rounded-lg border border-graphite-light p-6"
    aria-labelledby="storage-heading"
  >
    <h2 id="storage-heading" class="text-lg font-heading text-quartz mb-4">Data Storage</h2>
    <div>
      <label for="data-dir" class="block text-sm font-medium text-quartz mb-1">
        Data Directory
      </label>
      <div class="flex gap-2 max-w-md">
        <input
          id="data-dir"
          type="text"
          value="./data"
          readonly
          class="flex-1 px-3 py-2 rounded border border-graphite-light bg-obsidian text-quartz text-sm
                 cursor-not-allowed opacity-70"
          aria-readonly="true"
        />
        <button
          class="px-4 py-2 rounded border border-graphite-light text-sm text-quartz
                 hover:border-lapis transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
        >
          Browse
        </button>
      </div>
      <p class="text-xs text-flint mt-1">
        Where assets, fingerprints, and verification data are stored
      </p>
    </div>
  </section>

  <!-- About -->
  <section
    class="bg-graphite rounded-lg border border-graphite-light p-6"
    aria-labelledby="about-heading"
  >
    <h2 id="about-heading" class="text-lg font-heading text-quartz mb-4">About</h2>
    <dl class="grid grid-cols-[max-content_1fr] gap-x-8 gap-y-2 text-sm max-w-md">
      <dt class="text-flint">Version</dt>
      <dd class="text-quartz">{appVersion}</dd>
      <dt class="text-flint">Licence</dt>
      <dd class="text-quartz">PolyForm Noncommercial 1.0.0</dd>
      <dt class="text-flint">Developer</dt>
      <dd class="text-quartz">Juralabs CIC</dd>
    </dl>
  </section>
</div>
