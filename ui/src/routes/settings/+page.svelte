<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion, checkSidecarHealth } from '$lib/api';
  import type { SidecarHealth } from '$lib/types';
  import {
    type DeploymentProfile,
    MAX_PROFILES,
    loadProfiles,
    saveProfile,
    deleteProfile,
    applyProfile,
    generateProfileId,
    validateProfileName,
    formatProfileDate,
  } from '$lib/stores/profile';

  // ── Persistence keys ────────────────────────────────────────────
  const KEY_OLLAMA_URL    = 'jura-ollama-url';
  const KEY_VISION_MODEL  = 'jura-vision-model';
  const KEY_TEXT_MODEL    = 'jura-text-model';

  // ── Defaults ────────────────────────────────────────────────────
  const DEFAULT_OLLAMA_URL   = 'http://localhost:11434';
  const DEFAULT_VISION_MODEL = 'llava:7b';
  const DEFAULT_TEXT_MODEL   = 'qwen2.5:7b-instruct';

  // ── Settings state ──────────────────────────────────────────────
  let ollamaUrl    = $state(DEFAULT_OLLAMA_URL);
  let visionModel  = $state(DEFAULT_VISION_MODEL);
  let textModel    = $state(DEFAULT_TEXT_MODEL);
  let appVersion   = $state('0.2.0-dev');

  let saved        = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  // ── Service health ────────────────────────────────────────────
  let sidecarHealth = $state<SidecarHealth | null>(null);
  let healthLoading = $state(false);

  const sidecarOnline = $derived(sidecarHealth?.status === 'ok');
  const ollamaOnline = $derived(
    sidecarHealth?.ollama != null &&
    sidecarHealth.ollama !== 'unavailable'
  );

  async function refreshHealth() {
    healthLoading = true;
    sidecarHealth = await checkSidecarHealth();
    healthLoading = false;
  }

  // ── Deployment profiles state ─────────────────────────────────
  let profiles = $state<DeploymentProfile[]>([]);

  // Save-as-profile form
  let showSaveForm    = $state(false);
  let newProfileName  = $state('');
  let nameError       = $state<string | null>(null);

  // Per-profile delete confirmation: stores the id pending deletion, or null
  let pendingDeleteId = $state<string | null>(null);

  // Feedback after loading a profile
  let loadedProfileId = $state<string | null>(null);
  let loadTimer: ReturnType<typeof setTimeout> | null = null;

  const atProfileLimit = $derived(profiles.length >= MAX_PROFILES);

  function reloadProfiles() {
    profiles = loadProfiles();
  }

  function handleSaveProfile() {
    const error = validateProfileName(newProfileName);
    if (error) {
      nameError = error;
      return;
    }
    nameError = null;

    const profile: DeploymentProfile = {
      id: generateProfileId(),
      name: newProfileName.trim(),
      ollamaUrl,
      visionModel,
      textModel,
      defaultVerifyMode: 'fast',
      createdAt: new Date().toISOString(),
    };
    saveProfile(profile);
    reloadProfiles();
    newProfileName = '';
    showSaveForm = false;
  }

  function handleCancelSave() {
    showSaveForm = false;
    newProfileName = '';
    nameError = null;
  }

  function handleLoadProfile(profile: DeploymentProfile) {
    applyProfile(profile);
    // Update reactive state to match loaded values
    ollamaUrl   = profile.ollamaUrl;
    visionModel = profile.visionModel;
    textModel   = profile.textModel;

    // Brief visual confirmation
    loadedProfileId = profile.id;
    if (loadTimer !== null) clearTimeout(loadTimer);
    loadTimer = setTimeout(() => { loadedProfileId = null; }, 2500);
  }

  function handleRequestDelete(id: string) {
    pendingDeleteId = id;
  }

  function handleConfirmDelete() {
    if (pendingDeleteId === null) return;
    deleteProfile(pendingDeleteId);
    pendingDeleteId = null;
    reloadProfiles();
  }

  function handleCancelDelete() {
    pendingDeleteId = null;
  }

  function handleNameInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    newProfileName = input.value;
    if (nameError) nameError = validateProfileName(newProfileName);
  }

  onMount(async () => {
    ollamaUrl   = localStorage.getItem(KEY_OLLAMA_URL)   ?? DEFAULT_OLLAMA_URL;
    visionModel = localStorage.getItem(KEY_VISION_MODEL) ?? DEFAULT_VISION_MODEL;
    textModel   = localStorage.getItem(KEY_TEXT_MODEL)   ?? DEFAULT_TEXT_MODEL;
    appVersion  = await getVersion();
    sidecarHealth = await checkSidecarHealth();
    reloadProfiles();
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
  <h1 class="text-2xl font-heading text-text-light dark:text-quartz" style="font-family: Georgia, 'Times New Roman', serif;">Settings</h1>

  <!-- Ollama Configuration -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="ollama-heading"
  >
    <h2 id="ollama-heading" class="text-lg font-heading text-text-light dark:text-quartz mb-4" style="font-family: Georgia, 'Times New Roman', serif;">Ollama Configuration</h2>
    <div class="space-y-4">

      <div>
        <label for="ollama-url" class="block text-sm font-medium text-quartz mb-1">
          Ollama URL
        </label>
        <input
          id="ollama-url"
          type="url"
          bind:value={ollamaUrl}
          class="w-full max-w-md px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian text-text-light dark:text-quartz text-sm
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent transition-colors"
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
          class="w-full max-w-md px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian text-text-light dark:text-quartz text-sm
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent transition-colors"
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
          class="w-full max-w-md px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian text-text-light dark:text-quartz text-sm
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent transition-colors"
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
          class="px-5 py-2.5 min-h-[44px] bg-lapis hover:bg-lapis-dark dark:hover:bg-lapis-light text-white text-sm font-medium rounded transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
        >
          Save Settings
        </button>

        {#if saved}
          <p
            class="text-sm text-malachite dark:text-malachite-light motion-safe:animate-fade-in"
            role="status"
            aria-live="polite"
          >
            Settings saved.
          </p>
        {/if}
      </div>

    </div>
  </section>

  <!-- Deployment Profiles -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="profiles-heading"
  >
    <div class="flex items-center justify-between mb-4">
      <div>
        <h2 id="profiles-heading" class="text-lg font-heading text-text-light dark:text-quartz" style="font-family: Georgia, 'Times New Roman', serif;">Deployment Profiles</h2>
        <p class="text-xs text-flint mt-0.5">
          Save the current Ollama settings as a named profile to switch between environments quickly.
        </p>
      </div>

      {#if !showSaveForm}
        <button
          onclick={() => { showSaveForm = true; }}
          disabled={atProfileLimit}
          aria-describedby={atProfileLimit ? 'profile-limit-notice' : undefined}
          class="shrink-0 ml-4 px-4 py-2 text-sm font-medium rounded border transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-graphite
                 {atProfileLimit
                   ? 'border-graphite-light text-flint cursor-not-allowed opacity-50'
                   : 'border-lapis/60 text-lapis hover:bg-lapis/10 hover:border-lapis'}"
        >
          Save current settings as profile
        </button>
      {/if}
    </div>

    <!-- Max profiles warning -->
    {#if atProfileLimit}
      <p
        id="profile-limit-notice"
        class="text-xs text-amber mb-4 px-3 py-2 rounded border border-amber/20 bg-amber/5"
        role="note"
      >
        Maximum of {MAX_PROFILES} profiles reached. Delete an existing profile to save a new one.
      </p>
    {/if}

    <!-- Inline save form -->
    {#if showSaveForm}
      <div
        class="mb-4 p-4 rounded-lg border border-lapis/30 bg-gray-50 dark:bg-obsidian/40"
        role="region"
        aria-label="Save profile form"
      >
        <p class="text-sm font-medium text-text-light dark:text-quartz mb-3">Save current settings as profile</p>
        <div class="flex flex-col gap-2 max-w-sm">
          <label for="new-profile-name" class="text-sm text-quartz sr-only">
            Profile name
          </label>
          <input
            id="new-profile-name"
            type="text"
            value={newProfileName}
            oninput={handleNameInput}
            maxlength={50}
            placeholder="e.g. Local development"
            autocomplete="off"
            spellcheck={false}
            aria-required="true"
            aria-invalid={nameError !== null}
            aria-describedby={nameError ? 'profile-name-error' : 'profile-name-hint'}
            class="px-3 py-2 rounded border text-sm bg-white dark:bg-obsidian text-text-light dark:text-quartz transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent
                   {nameError ? 'border-cinnabar' : 'border-border-light dark:border-border-dark'}"
          />
          {#if nameError}
            <p id="profile-name-error" class="text-xs text-cinnabar-light" role="alert">
              {nameError}
            </p>
          {:else}
            <p id="profile-name-hint" class="text-xs text-flint">
              1–50 characters. Captures current Ollama URL, vision model, and text model.
            </p>
          {/if}
          <div class="flex gap-2 mt-1">
            <button
              onclick={handleSaveProfile}
              class="px-4 py-1.5 text-sm font-medium rounded bg-lapis text-white hover:bg-lapis-light transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
            >
              Confirm
            </button>
            <button
              onclick={handleCancelSave}
              class="px-4 py-1.5 text-sm font-medium rounded border border-graphite-light text-flint hover:text-quartz hover:border-lapis/50 transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    {/if}

    <!-- Profile list -->
    {#if profiles.length === 0}
      <p class="text-sm text-flint py-4 text-center border border-dashed border-border-light dark:border-border-dark rounded-lg">
        No profiles saved yet.
      </p>
    {:else}
      <ul
        class="space-y-2"
        aria-label="Saved deployment profiles"
        role="list"
      >
        {#each profiles as profile (profile.id)}
          <li
            class="rounded-lg border transition-colors
                   {pendingDeleteId === profile.id
                     ? 'border-cinnabar/30 bg-cinnabar/5'
                     : loadedProfileId === profile.id
                       ? 'border-malachite/30 bg-malachite/5'
                       : 'border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40 hover:border-lapis/30'}"
          >
            <!-- Profile row -->
            <div class="flex items-center justify-between gap-3 px-4 py-3">
              <div class="min-w-0">
                <p class="text-sm font-medium text-text-light dark:text-quartz truncate">{profile.name}</p>
                <p class="text-xs text-flint mt-0.5 truncate">
                  Created {formatProfileDate(profile.createdAt)}
                  &mdash; {profile.ollamaUrl}
                </p>
              </div>

              <div class="flex items-center gap-2 shrink-0">
                {#if loadedProfileId === profile.id}
                  <span
                    class="text-xs text-malachite-light"
                    role="status"
                    aria-live="polite"
                  >
                    Loaded
                  </span>
                {:else}
                  <button
                    onclick={() => handleLoadProfile(profile)}
                    aria-label="Load profile {profile.name}"
                    class="px-3 py-1.5 text-xs font-medium rounded border border-lapis/50 text-lapis hover:bg-lapis/10 hover:border-lapis transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
                  >
                    Load
                  </button>
                {/if}

                {#if pendingDeleteId !== profile.id}
                  <button
                    onclick={() => handleRequestDelete(profile.id)}
                    aria-label="Delete profile {profile.name}"
                    class="px-3 py-1.5 text-xs font-medium rounded border border-cinnabar/30 text-cinnabar-light hover:bg-cinnabar/10 hover:border-cinnabar/60 transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
                  >
                    Delete
                  </button>
                {/if}
              </div>
            </div>

            <!-- Inline delete confirmation -->
            {#if pendingDeleteId === profile.id}
              <div
                class="flex items-center gap-3 px-4 pb-3"
                role="region"
                aria-label="Confirm deletion of {profile.name}"
              >
                <p class="text-xs text-cinnabar-light flex-1">
                  Delete "{profile.name}"? This cannot be undone.
                </p>
                <button
                  onclick={handleConfirmDelete}
                  aria-label="Confirm deletion of {profile.name}"
                  class="px-3 py-1.5 text-xs font-medium rounded bg-cinnabar text-white hover:bg-cinnabar-light transition-colors
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
                >
                  Delete
                </button>
                <button
                  onclick={handleCancelDelete}
                  class="px-3 py-1.5 text-xs font-medium rounded border border-graphite-light text-flint hover:text-quartz hover:border-lapis/50 transition-colors
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
                >
                  Cancel
                </button>
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}

    <!-- Live region for screen reader announcements -->
    <div aria-live="polite" aria-atomic="true" class="sr-only">
      {#if loadedProfileId !== null}
        {(() => {
          const p = profiles.find((x) => x.id === loadedProfileId);
          return p ? `Profile "${p.name}" loaded.` : '';
        })()}
      {/if}
    </div>
  </section>

  <!-- Service Status -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="status-heading"
  >
    <div class="flex items-center justify-between mb-4">
      <h2 id="status-heading" class="text-lg font-heading text-text-light dark:text-quartz" style="font-family: Georgia, 'Times New Roman', serif;">Service Status</h2>
      <button
        class="text-xs px-3 py-2.5 min-h-[44px] rounded border border-border-light dark:border-border-dark text-flint hover:text-text-light dark:hover:text-quartz hover:border-lapis/50
               transition-colors disabled:opacity-50
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
        onclick={refreshHealth}
        disabled={healthLoading}
      >
        {#if healthLoading}
          <span class="flex items-center gap-1.5">
            <span
              class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
              aria-hidden="true"
            ></span>
            Checking...
          </span>
        {:else}
          Refresh
        {/if}
      </button>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <!-- ML Sidecar card -->
      <div class="rounded-lg border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40 p-4">
        <div class="flex items-center justify-between mb-3">
          <span class="text-sm font-medium text-text-light dark:text-quartz">Analysis services</span>
          <span
            class="text-xs px-2 py-0.5 rounded-full
                   {sidecarOnline
                     ? 'bg-malachite/15 text-malachite-light border border-malachite/20'
                     : 'bg-cinnabar/10 text-cinnabar-light border border-cinnabar/20'}"
          >
            {sidecarOnline ? 'Online' : 'Offline'}
          </span>
        </div>
        <p class="text-xs text-flint mb-2">http://127.0.0.1:8200</p>

        {#if sidecarOnline && sidecarHealth}
          <p class="text-xs text-flint mb-2">Version: <span class="text-quartz">{sidecarHealth.version}</span></p>
          <div class="flex flex-wrap gap-1.5">
            {#each Object.entries(sidecarHealth.capabilities) as [cap, enabled]}
              <span
                class="text-xs px-2 py-0.5 rounded
                       {enabled
                         ? 'bg-malachite/10 text-malachite-light border border-malachite/20'
                         : 'bg-graphite-light text-flint border border-graphite-light'}"
              >
                {cap}
              </span>
            {/each}
          </div>
        {:else}
          <div class="mt-2 text-xs text-flint">
            <p class="mb-1">Start the analysis services with:</p>
            <code class="block font-mono text-xs bg-gray-100 dark:bg-obsidian px-2 py-1 rounded text-text-light dark:text-quartz">
              cd sidecar && uvicorn main:app --host 127.0.0.1 --port 8200
            </code>
          </div>
        {/if}
      </div>

      <!-- Ollama card -->
      <div class="rounded-lg border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40 p-4">
        <div class="flex items-center justify-between mb-3">
          <span class="text-sm font-medium text-text-light dark:text-quartz">Ollama</span>
          <span
            class="text-xs px-2 py-0.5 rounded-full
                   {ollamaOnline
                     ? 'bg-malachite/15 text-malachite-light border border-malachite/20'
                     : 'bg-graphite-light text-flint border border-graphite-light'}"
          >
            {ollamaOnline ? 'Connected' : 'Offline'}
          </span>
        </div>
        <p class="text-xs text-flint mb-2">{ollamaUrl}</p>

        {#if ollamaOnline && sidecarHealth?.ollama}
          <p class="text-xs text-flint">
            Status: <span class="text-quartz">{sidecarHealth.ollama}</span>
          </p>
        {:else}
          <p class="text-xs text-flint">
            Required for auto-cataloguing and claim checking. Install from
            <a
              href="https://ollama.com"
              target="_blank"
              rel="noopener noreferrer"
              class="text-lapis hover:text-lapis-dark dark:hover:text-lapis-light underline underline-offset-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
            >ollama.com</a>.
          </p>
        {/if}
      </div>
    </div>
  </section>

  <!-- Data Storage -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="storage-heading"
  >
    <h2 id="storage-heading" class="text-lg font-heading text-text-light dark:text-quartz mb-4" style="font-family: Georgia, 'Times New Roman', serif;">Data Storage</h2>
    <div>
      <label for="data-dir" class="block text-sm font-medium text-text-light dark:text-quartz mb-1">
        Data Directory
      </label>
      <div class="flex gap-2 max-w-md">
        <input
          id="data-dir"
          type="text"
          value="./data"
          readonly
          class="flex-1 px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian text-text-light dark:text-quartz text-sm
                 cursor-not-allowed opacity-70"
          aria-readonly="true"
        />
        <button
          class="px-4 py-2.5 min-h-[44px] rounded border border-border-light dark:border-border-dark text-sm text-text-light dark:text-quartz
                 hover:border-lapis transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
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
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="about-heading"
  >
    <h2 id="about-heading" class="text-lg font-heading text-text-light dark:text-quartz mb-4" style="font-family: Georgia, 'Times New Roman', serif;">About</h2>
    <dl class="grid grid-cols-[max-content_1fr] gap-x-8 gap-y-2 text-sm max-w-md">
      <dt class="text-flint">Version</dt>
      <dd class="text-text-light dark:text-quartz">{appVersion}</dd>
      <dt class="text-flint">Licence</dt>
      <dd class="text-text-light dark:text-quartz">PolyForm Noncommercial 1.0.0</dd>
      <dt class="text-flint">Developer</dt>
      <dd class="text-text-light dark:text-quartz">
        <a
          href="https://juralabs.org"
          target="_blank"
          rel="noopener noreferrer"
          class="text-lapis hover:text-lapis-dark dark:hover:text-lapis-light underline underline-offset-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
        >Juralabs CIC</a>
      </dd>
    </dl>
  </section>
</div>
