<script lang="ts">
  // Settings page — Ollama config, data directory, appearance
</script>

<div class="space-y-6">
  <h1 class="text-2xl font-heading">Settings</h1>

  <!-- Ollama Configuration -->
  <section class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6">
    <h2 class="text-lg font-heading mb-4">Ollama Configuration</h2>
    <div class="space-y-4">
      <div>
        <label for="ollama-url" class="block text-sm font-medium mb-1">Ollama URL</label>
        <input
          id="ollama-url"
          type="url"
          value="http://localhost:11434"
          class="w-full max-w-md px-3 py-2 rounded border border-border-light dark:border-border-dark bg-surface-light dark:bg-obsidian text-text-light dark:text-text-dark text-sm"
        />
        <p class="text-xs text-flint dark:text-flint-light mt-1">Required for auto-cataloguing and claim checking</p>
      </div>
      <div>
        <label for="vision-model" class="block text-sm font-medium mb-1">Vision Model</label>
        <input
          id="vision-model"
          type="text"
          value="llava:7b"
          class="w-full max-w-md px-3 py-2 rounded border border-border-light dark:border-border-dark bg-surface-light dark:bg-obsidian text-text-light dark:text-text-dark text-sm"
        />
      </div>
      <div>
        <label for="text-model" class="block text-sm font-medium mb-1">Text Model</label>
        <input
          id="text-model"
          type="text"
          value="qwen2.5:7b-instruct"
          class="w-full max-w-md px-3 py-2 rounded border border-border-light dark:border-border-dark bg-surface-light dark:bg-obsidian text-text-light dark:text-text-dark text-sm"
        />
      </div>
    </div>
  </section>

  <!-- Data Directory -->
  <section class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6">
    <h2 class="text-lg font-heading mb-4">Data Storage</h2>
    <div>
      <label for="data-dir" class="block text-sm font-medium mb-1">Data Directory</label>
      <div class="flex gap-2 max-w-md">
        <input
          id="data-dir"
          type="text"
          value="./data"
          readonly
          class="flex-1 px-3 py-2 rounded border border-border-light dark:border-border-dark bg-surface-light dark:bg-obsidian text-text-light dark:text-text-dark text-sm"
        />
        <button
          class="px-4 py-2 rounded border border-border-light dark:border-border-dark text-sm hover:border-lapis dark:hover:border-lapis-light transition-colors"
        >
          Browse
        </button>
      </div>
      <p class="text-xs text-flint dark:text-flint-light mt-1">Where assets, fingerprints, and verification data are stored</p>
    </div>
  </section>

  <!-- About -->
  <section class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6">
    <h2 class="text-lg font-heading mb-4">About</h2>
    <dl class="grid grid-cols-2 gap-2 text-sm max-w-md">
      <dt class="text-flint dark:text-flint-light">Version</dt>
      <dd>0.1.0-dev</dd>
      <dt class="text-flint dark:text-flint-light">Licence</dt>
      <dd>PolyForm Noncommercial 1.0.0</dd>
      <dt class="text-flint dark:text-flint-light">Developer</dt>
      <dd>Juralabs CIC</dd>
    </dl>
  </section>
</div>
