<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion, checkSidecarHealth, getDbPath, setDbPath, getLicenceTier, setLicenceTier, getAiDescriptionEnabled, setAiDescriptionEnabled, getPowerSaverMode, setPowerSaverMode, createApiKey, listApiKeys, revokeApiKey, getSigningMode, setSigningMode, getConformantCertInfo, importConformantCertificate, clearConformantCert, getNetworkMode, setNetworkMode } from '$lib/api';
  import type { ApiKeyInfo, CreateKeyResult } from '$lib/api';
  import type { ConformantCertificateInfo, LicenceTier, NetworkMode, SidecarHealth, SigningMode, TierInfo } from '$lib/types';
  import ContextualHelpLink from '$lib/components/ContextualHelpLink.svelte';
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

  // JTV-138 (2026-05-02): capabilities deferred to v1.0.x. The /health
  // response still reports them as `false` for v1.0.x re-add detection
  // (Setup Wizard previews ffprobe etc.) but they should not render as
  // greyed pills in the Settings UI — that reads as "broken" rather
  // than "intentionally out of scope". JTV-139 v1.0.x re-add will
  // remove these entries from the set when restoring the routes.
  const V1_DEFERRED_CAPABILITIES = new Set([
    'videoMetadata',
    'audioMetadata',
    'videoFrames',
    'videoDeepfake',
    'transcription',
    'audioDeepfake',
  ]);

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

  // ── Database location state ───────────────────────────────────────
  let currentDbPath   = $state('');
  let dbPathChanging  = $state(false);
  let dbPathFeedback  = $state<{ ok: boolean; message: string } | null>(null);
  let dbFeedbackTimer: ReturnType<typeof setTimeout> | null = null;

  async function handleChangeDbLocation() {
    if (!isTauri()) return;
    dbPathChanging = true;
    dbPathFeedback = null;

    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: true,
        title: 'Choose Database Location',
      });
      if (!selected || typeof selected !== 'string') {
        dbPathChanging = false;
        return;
      }

      // Append the filename to the chosen directory using the platform-aware path API
      const { join } = await import('@tauri-apps/api/path');
      const newPath = await join(selected, 'jura_trace.db');

      const result = await setDbPath(newPath);
      currentDbPath = result;
      dbPathFeedback = { ok: true, message: 'Database location updated. Restart the application for the change to take full effect.' };
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      dbPathFeedback = { ok: false, message: `Failed to move database: ${msg}` };
    } finally {
      dbPathChanging = false;
      if (dbFeedbackTimer !== null) clearTimeout(dbFeedbackTimer);
      dbFeedbackTimer = setTimeout(() => { dbPathFeedback = null; }, 8000);
    }
  }

  function isTauri(): boolean {
    return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  }

  // ── Backup & Restore state (JTV-130) ──────────────────────────
  let backupBusy        = $state(false);
  let backupSuccess     = $state<import('$lib/api').BackupResult | null>(null);
  let backupError       = $state<string | null>(null);

  let restoreBusy       = $state(false);
  let restoreFeedback   = $state<{ ok: boolean; message: string } | null>(null);
  let pendingRestore    = $state<{ path: string; preflight: import('$lib/api').RestoreResult } | null>(null);

  let csvImportBusy     = $state(false);
  let csvImportResult   = $state<import('$lib/api').CsvImportResult | null>(null);
  let csvImportError    = $state<string | null>(null);

  async function handleBackupDatabase() {
    if (!isTauri()) return;
    backupBusy = true;
    backupSuccess = null;
    backupError = null;
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const { backupDatabase } = await import('$lib/api');
      const selected = await open({
        directory: true,
        title: 'Choose backup destination',
      });
      if (!selected || typeof selected !== 'string') {
        backupBusy = false;
        return;
      }
      backupSuccess = await backupDatabase(selected);
    } catch (err: unknown) {
      backupError = err instanceof Error ? err.message : String(err);
    } finally {
      backupBusy = false;
    }
  }

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      // Clipboard API may be unavailable in some Tauri webview contexts;
      // fall back to a hidden textarea + execCommand as a last resort.
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.position = 'fixed';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.select();
      try { document.execCommand('copy'); } catch { /* ignore */ }
      document.body.removeChild(ta);
    }
  }

  async function handleRestorePreflight() {
    if (!isTauri()) return;
    restoreBusy = true;
    restoreFeedback = null;
    pendingRestore = null;
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const { restoreDatabase } = await import('$lib/api');
      const selected = await open({
        title: 'Select snapshot to restore',
        filters: [{ name: 'SQLite snapshot', extensions: ['sqlite', 'db', 'sqlite3'] }],
      });
      if (!selected || typeof selected !== 'string') {
        restoreBusy = false;
        return;
      }
      const preflight = await restoreDatabase(selected, false);
      pendingRestore = { path: selected, preflight };
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      restoreFeedback = { ok: false, message: `Cannot restore: ${msg}` };
    } finally {
      restoreBusy = false;
    }
  }

  async function handleRestoreConfirmed() {
    if (!pendingRestore || !isTauri()) return;
    restoreBusy = true;
    try {
      const { restoreDatabase } = await import('$lib/api');
      const result = await restoreDatabase(pendingRestore.path, true);
      restoreFeedback = { ok: result.success, message: result.message };
      pendingRestore = null;
      // Reload to re-initialise the in-memory app state against the new DB.
      setTimeout(() => window.location.reload(), 1500);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      restoreFeedback = { ok: false, message: `Restore failed: ${msg}` };
      pendingRestore = null;
    } finally {
      restoreBusy = false;
    }
  }

  function handleRestoreCancel() {
    pendingRestore = null;
  }

  async function handleImportCsv() {
    if (!isTauri()) return;
    csvImportBusy = true;
    csvImportResult = null;
    csvImportError = null;
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const { importAssetsCsv } = await import('$lib/api');
      const selected = await open({
        title: 'Select CSV catalogue file',
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });
      if (!selected || typeof selected !== 'string') {
        csvImportBusy = false;
        return;
      }
      csvImportResult = await importAssetsCsv(selected);
    } catch (err: unknown) {
      csvImportError = err instanceof Error ? err.message : String(err);
    } finally {
      csvImportBusy = false;
    }
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
      defaultVerifyMode: 'standard',
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
    currentDbPath = await getDbPath();
    currentTier = await getLicenceTier();
    aiDescPref = await getAiDescriptionEnabled();
    powerSaverMode = await getPowerSaverMode();
    await loadApiKeys();
    // Load signing mode + conformant cert (BYOC)
    try {
      signingMode = await getSigningMode();
    } catch {
      signingMode = 'bedrock';
    }
    try {
      conformantCert = await getConformantCertInfo();
    } catch {
      conformantCert = null;
    }
    // Load network access mode
    networkMode = await getNetworkMode();
  });

  function handleRerunWizard() {
    localStorage.removeItem('jura-setup-complete');
    // Navigate to root layout where the wizard is mounted
    window.location.href = '/';
  }

  function saveSettings() {
    localStorage.setItem(KEY_OLLAMA_URL,   ollamaUrl);
    localStorage.setItem(KEY_VISION_MODEL, visionModel);
    localStorage.setItem(KEY_TEXT_MODEL,   textModel);

    // Show confirmation briefly
    saved = true;
    if (saveTimer !== null) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => { saved = false; }, 2500);
  }

  // ── Auto-updater ──────────────────────────────────────────────────────
  // Uses @tauri-apps/plugin-updater which is only available inside a Tauri
  // build.  In browser preview / Playwright tests we show a graceful notice
  // instead of throwing.

  type UpdateStatus =
    | { state: 'idle' }
    | { state: 'checking' }
    | { state: 'available'; version: string }
    | { state: 'up-to-date' }
    | { state: 'downloading' }
    | { state: 'installing' }
    | { state: 'error'; message: string };

  let updateStatus = $state<UpdateStatus>({ state: 'idle' });

  async function checkForUpdate() {
    if (!isTauri()) {
      updateStatus = { state: 'error', message: 'Update checks are only available in the desktop application.' };
      return;
    }

    updateStatus = { state: 'checking' };
    try {
      // Dynamic import keeps the plugin out of the browser bundle entirely.
      const { check } = await import('@tauri-apps/plugin-updater');
      const update = await check();

      if (!update) {
        updateStatus = { state: 'up-to-date' };
        return;
      }

      updateStatus = { state: 'available', version: update.version };

      // Download and install immediately — the plugin shows a restart prompt.
      updateStatus = { state: 'downloading' };
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          updateStatus = { state: 'downloading' };
        } else if (event.event === 'Progress') {
          // Progress events carry { chunkLength, contentLength } — we use
          // them only to stay in the "downloading" state and could render a
          // progress bar here in a future iteration.
          updateStatus = { state: 'downloading' };
        } else if (event.event === 'Finished') {
          updateStatus = { state: 'installing' };
        }
      });
      // After downloadAndInstall resolves the app will restart automatically.
      updateStatus = { state: 'installing' };
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      // "No updates available" surfaces as an error from the plugin when the
      // pubkey is empty and the endpoint returns a 404 or no newer version.
      // Surface a user-friendly message rather than a raw error string.
      if (message.includes('No updates available') || message.includes('404')) {
        updateStatus = { state: 'up-to-date' };
      } else {
        updateStatus = { state: 'error', message };
      }
    }
  }

  // ── Licence Tier ──────────────────────────────────────────────────────────
  // Pilot-phase tier indicator. Allows demonstration of tier value propositions
  // without a licence server. The tier is stored in config.json and persists
  // across restarts. In production this will be replaced by signed JWT enforcement.

  const TIER_INFO: Record<LicenceTier, TierInfo> = {
    community: {
      tier: 'community',
      name: 'Community',
      codename: 'Flint',
      description: 'Free, non-commercial use. Full verification pipeline, all 17+ detectors, batch processing, PDF reports, and MONITOR Layer 1. Community support via GitHub Issues.',
      badgeClass: 'bg-flint/15 border border-flint/30',
      badgeTextClass: 'text-flint-dark dark:text-flint-light',
    },
    professional: {
      tier: 'professional',
      name: 'Professional',
      codename: 'Stratum',
      description: 'Individual commercial licence. Adds report customisation (your name, organisation, case reference), full methodology versioning, comparative analysis, MONITOR Layer 2 (reverse image search), Conformant C2PA signing, REST API access (port 8300), extended audit log retention (24 months), and best-effort email support.',
      badgeClass: 'bg-lapis/15 border border-lapis/30',
      badgeTextClass: 'text-lapis dark:text-lapis-light',
    },
    enterprise: {
      tier: 'enterprise',
      name: 'Enterprise',
      codename: 'Bedrock',
      description: 'Unlimited-seat commercial licence. Adds silent installer with MDM/ADMX templates, central TOML configuration, bulk C2PA watched-folder signing, custom RAG knowledge base, auto-update policy control, compliance documentation pack, priority access to generator detection updates, and a quarterly check-in call.',
      badgeClass: 'bg-amber/15 border border-amber/30',
      badgeTextClass: 'text-amber-dark dark:text-amber-light',
    },
  };

  let currentTier = $state<LicenceTier>('community');
  let tierChanging = $state(false);
  let tierFeedback = $state<{ ok: boolean; message: string } | null>(null);
  let tierFeedbackTimer: ReturnType<typeof setTimeout> | null = null;

  const currentTierInfo = $derived(TIER_INFO[currentTier]);

  async function handleTierChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    const newTier = select.value as LicenceTier;
    if (newTier === currentTier) return;

    tierChanging = true;
    tierFeedback = null;

    try {
      await setLicenceTier(newTier);
      currentTier = newTier;
      tierFeedback = { ok: true, message: `Plan updated to ${TIER_INFO[newTier].name}.` };
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      tierFeedback = { ok: false, message: `Failed to update plan: ${msg}` };
    } finally {
      tierChanging = false;
      if (tierFeedbackTimer !== null) clearTimeout(tierFeedbackTimer);
      tierFeedbackTimer = setTimeout(() => { tierFeedback = null; }, 5000);
    }
  }

  // ── AI image description preference ────────────────────────────────────
  // LLaVA-backed image descriptions run as the final stage of verify on
  // image content. They depend on Ollama being installed and reachable, and
  // add 5–30 seconds to every image verify. Gated behind an explicit user
  // opt-in so verify stays fast by default. The tri-state value distinguishes
  // "never decided" (null) from "explicitly off" (false).
  let aiDescPref = $state<boolean | null>(null);
  let aiDescChanging = $state(false);
  let aiDescFeedback = $state<{ ok: boolean; message: string } | null>(null);
  let aiDescFeedbackTimer: ReturnType<typeof setTimeout> | null = null;

  /** Effective state shown in the UI: undecided defaults to "off". */
  const aiDescActive = $derived(aiDescPref === true);

  /** True when Ollama appears reachable from the sidecar health check. */
  const ollamaDetected = $derived(
    sidecarHealth?.ollama != null && sidecarHealth.ollama.length > 0,
  );

  // ── JTV-132: Ollama install + model download (2026-05-02) ──────────────
  // Bundling Ollama in v1.0 was ruled out by the four-agent evaluation
  // (rag-ollama-engineer + persona-testing): ROOTED port-conflict, 8GB-RAM
  // exclusion of pilot personas (Tom IT-manager, Aisha DRRF), 1Mbps-bandwidth
  // exclusion of field deployment, and Apple notarisation incompatibility
  // with re-signing third-party binaries are hard blockers. The optional
  // path therefore needs frictionless install UX in Settings — moved here
  // from the SetupWizard 2026-05-02 (commit cb87998) so first launch is not
  // gated on a 9 GB model decision.
  function detectPlatform(): 'mac' | 'windows' | 'linux' {
    if (typeof navigator === 'undefined') return 'linux';
    const p = navigator.userAgent.toLowerCase();
    if (p.includes('win')) return 'windows';
    if (p.includes('mac')) return 'mac';
    return 'linux';
  }
  const platform = detectPlatform();

  let ollamaInstalling = $state(false);
  let ollamaInstallError = $state<string | null>(null);
  let pullingModel = $state<string | null>(null);
  let pullProgress = $state<string | null>(null);
  let pullPercent = $state<number | null>(null);
  let pullError = $state<string | null>(null);

  /** Model presence flags driven by the live `/health` ollamaModels list. */
  const ollamaModelsList = $derived(sidecarHealth?.ollamaModels ?? []);
  const llavaInstalled = $derived(ollamaModelsList.some((m) => m.startsWith('llava')));
  const qwenInstalled = $derived(ollamaModelsList.some((m) => m.startsWith('qwen2.5')));

  /**
   * Three-state Ollama summary for the Service Status card.
   * 'not-installed' — no Ollama process reachable on configured URL
   * 'installed-no-models' — reachable, but neither llava nor qwen2.5 are pulled
   * 'partial' — one model present, one missing
   * 'ready' — both models present
   */
  const ollamaState = $derived<'not-installed' | 'installed-no-models' | 'partial' | 'ready'>(
    !ollamaOnline
      ? 'not-installed'
      : llavaInstalled && qwenInstalled
        ? 'ready'
        : !llavaInstalled && !qwenInstalled
          ? 'installed-no-models'
          : 'partial',
  );

  /**
   * Attempt to install Ollama via the platform package manager.
   * macOS: brew (arm64 first, then Intel fallback).
   * Windows: winget Ollama.Ollama.
   * Linux: opens the official curl install instructions in a browser.
   *
   * Mirrors the FFmpeg auto-install pattern that previously lived in
   * SetupWizard.svelte (removed in commit cb87998). Reuses the shell
   * capability scopes already declared in src-tauri/capabilities/default.json.
   */
  async function installOllama() {
    ollamaInstalling = true;
    ollamaInstallError = null;
    try {
      if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
        throw new Error('Auto-install is only available in the desktop application.');
      }
      const { Command } = await import('@tauri-apps/plugin-shell');

      if (platform === 'windows') {
        try {
          const probe = await Command.create('winget', ['--version']).execute();
          if (probe.code !== 0) throw new Error('winget not found');
        } catch {
          throw new Error(
            'winget is not available on this machine. Open ollama.com to download the installer manually.',
          );
        }
        const r = await Command.create('winget', [
          'install', 'Ollama.Ollama',
          '--accept-package-agreements',
          '--accept-source-agreements',
          '--silent',
        ]).execute();
        if (r.code !== 0) {
          const detail = r.stderr?.trim() || `Exit code ${r.code}`;
          throw new Error(`winget install failed: ${detail}`);
        }
      } else if (platform === 'mac') {
        async function tryBrew(scope: 'brew-arm' | 'brew-intel') {
          try {
            const probe = await Command.create(scope, ['--version']).execute();
            if (probe.code !== 0) return { ok: false as const, stderr: probe.stderr?.trim() };
            const r = await Command.create(scope, ['install', 'ollama']).execute();
            if (r.code !== 0) return { ok: false as const, stderr: r.stderr?.trim() || `Exit code ${r.code}` };
            return { ok: true as const };
          } catch (e) {
            return { ok: false as const, err: e };
          }
        }
        let outcome = await tryBrew('brew-arm');
        if (!outcome.ok) outcome = await tryBrew('brew-intel');
        if (!outcome.ok) {
          throw new Error(
            'Homebrew not reachable at /opt/homebrew/bin/brew or /usr/local/bin/brew. ' +
            'Install Homebrew from brew.sh, or download Ollama from ollama.com.',
          );
        }
      } else {
        // Linux: no package-manager auto-install path — open the official
        // curl-install one-liner in the user's browser.
        const { open } = await import('@tauri-apps/plugin-shell');
        await open('https://ollama.com/download/linux');
      }
      // Re-check health so the green checkmark appears immediately on success.
      await refreshHealth();
    } catch (e) {
      ollamaInstallError = e instanceof Error ? e.message : String(e);
    } finally {
      ollamaInstalling = false;
    }
  }

  /** Open ollama.com in the user's default browser (manual-install fallback). */
  async function openOllamaDownload() {
    try {
      if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
        const { open } = await import('@tauri-apps/plugin-shell');
        await open('https://ollama.com');
      } else {
        window.open('https://ollama.com', '_blank', 'noopener,noreferrer');
      }
    } catch {
      window.open('https://ollama.com', '_blank', 'noopener,noreferrer');
    }
  }

  /**
   * Stream a model pull through the sidecar's `/ollama/pull` proxy.
   * The CSP restricts connect-src to 127.0.0.1:8200, so the Tauri webview
   * cannot speak to Ollama directly — the sidecar forwards each request to
   * whatever JURA_OLLAMA_BASE_URL is configured there. SSE chunks emit
   * `{status, completed, total}` progress that we render as a percentage.
   */
  async function pullOllamaModel(modelName: string) {
    pullingModel = modelName;
    pullError = null;
    pullProgress = 'Connecting…';
    pullPercent = null;
    try {
      const resp = await fetch('http://127.0.0.1:8200/ollama/pull', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: modelName, stream: true }),
      });
      if (!resp.ok) {
        let detail = `HTTP ${resp.status}`;
        try {
          const body = await resp.json();
          if (body?.message) detail = body.message;
        } catch { /* ignore */ }
        throw new Error(detail);
      }
      const reader = resp.body?.getReader();
      const decoder = new TextDecoder();
      if (reader) {
        let buffer = '';
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          buffer += decoder.decode(value, { stream: true });
          const lines = buffer.split('\n');
          buffer = lines.pop() || '';
          for (const line of lines) {
            if (!line.startsWith('data: ')) continue;
            try {
              const data = JSON.parse(line.slice(6));
              if (data.error) throw new Error(data.message || data.error);
              if (data.status === 'success') {
                pullProgress = 'Complete';
                pullPercent = 100;
              } else if (data.completed && data.total) {
                pullPercent = Math.round((data.completed / data.total) * 100);
                const mb = (data.completed / 1_000_000).toFixed(0);
                const totalMb = (data.total / 1_000_000).toFixed(0);
                pullProgress = `${data.status || 'Downloading'} — ${mb} / ${totalMb} MB`;
              } else if (data.status) {
                pullProgress = data.status;
              }
            } catch (parseErr) {
              if (parseErr instanceof Error && parseErr.message !== line.slice(6)) throw parseErr;
            }
          }
        }
      }
      await refreshHealth();
    } catch (e) {
      pullError = `Failed to download ${modelName}: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      pullingModel = null;
      pullProgress = null;
      pullPercent = null;
    }
  }

  async function handleAiDescToggle(event: Event) {
    const checkbox = event.currentTarget as HTMLInputElement;
    const next = checkbox.checked;
    aiDescChanging = true;
    aiDescFeedback = null;
    try {
      await setAiDescriptionEnabled(next);
      aiDescPref = next;
      aiDescFeedback = {
        ok: true,
        message: next
          ? 'AI image descriptions enabled. Verify will take longer on images.'
          : 'AI image descriptions disabled. Verify will run faster.',
      };
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      aiDescFeedback = { ok: false, message: `Failed to update preference: ${msg}` };
      // Roll the checkbox back to the last known state so the UI stays honest.
      checkbox.checked = aiDescActive;
    } finally {
      aiDescChanging = false;
      if (aiDescFeedbackTimer !== null) clearTimeout(aiDescFeedbackTimer);
      aiDescFeedbackTimer = setTimeout(() => { aiDescFeedback = null; }, 5000);
    }
  }

  // ── Power-saver mode ────────────────────────────────────────────────────
  let powerSaverMode = $state(false);
  let powerSaverChanging = $state(false);
  let powerSaverFeedback = $state<{ ok: boolean; message: string } | null>(null);
  let powerSaverFeedbackTimer: ReturnType<typeof setTimeout> | null = null;

  async function handlePowerSaverToggle(event: Event) {
    const checkbox = event.currentTarget as HTMLInputElement;
    const next = checkbox.checked;
    powerSaverChanging = true;
    powerSaverFeedback = null;
    try {
      await setPowerSaverMode(next);
      powerSaverMode = next;
      powerSaverFeedback = {
        ok: true,
        message: next
          ? 'Power-saver mode enabled. The analysis engine will stop after five minutes of inactivity.'
          : 'Power-saver mode disabled. The analysis engine stays running.',
      };
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      powerSaverFeedback = { ok: false, message: `Failed to update preference: ${msg}` };
      checkbox.checked = powerSaverMode;
    } finally {
      powerSaverChanging = false;
      if (powerSaverFeedbackTimer !== null) clearTimeout(powerSaverFeedbackTimer);
      powerSaverFeedbackTimer = setTimeout(() => { powerSaverFeedback = null; }, 5000);
    }
  }

  // ── API Key Management ──────────────────────────────────────────────────
  // Available on Professional and Enterprise tiers (Team retired 2026-05-04).
  // Keys authenticate against the local REST API on port 8300.

  let apiKeys = $state<ApiKeyInfo[]>([]);
  let apiKeysLoading = $state(false);
  let newKeyName = $state('');
  let newKeyRateLimit = $state(100);
  let creatingKey = $state(false);
  let newlyCreatedKey = $state<CreateKeyResult | null>(null);
  let apiKeyFeedback = $state<{ ok: boolean; message: string } | null>(null);
  let apiKeyFeedbackTimer: ReturnType<typeof setTimeout> | null = null;
  let pendingRevokeId = $state<string | null>(null);

  const apiKeysAvailable = $derived(currentTier === 'professional' || currentTier === 'enterprise');
  const activeKeyCount = $derived(apiKeys.filter(k => !k.revoked).length);

  async function loadApiKeys() {
    apiKeysLoading = true;
    apiKeys = await listApiKeys();
    apiKeysLoading = false;
  }

  async function handleCreateKey() {
    if (!newKeyName.trim()) return;
    creatingKey = true;
    apiKeyFeedback = null;
    newlyCreatedKey = null;

    try {
      const result = await createApiKey(newKeyName.trim(), newKeyRateLimit);
      newlyCreatedKey = result;
      newKeyName = '';
      newKeyRateLimit = 100;
      await loadApiKeys();
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      apiKeyFeedback = { ok: false, message: `Failed to create key: ${msg}` };
      if (apiKeyFeedbackTimer !== null) clearTimeout(apiKeyFeedbackTimer);
      apiKeyFeedbackTimer = setTimeout(() => { apiKeyFeedback = null; }, 8000);
    } finally {
      creatingKey = false;
    }
  }

  function handleRequestRevoke(keyId: string) {
    pendingRevokeId = keyId;
  }

  async function handleConfirmRevoke() {
    if (!pendingRevokeId) return;
    try {
      await revokeApiKey(pendingRevokeId);
      pendingRevokeId = null;
      await loadApiKeys();
      apiKeyFeedback = { ok: true, message: 'API key revoked.' };
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      apiKeyFeedback = { ok: false, message: `Failed to revoke key: ${msg}` };
    }
    if (apiKeyFeedbackTimer !== null) clearTimeout(apiKeyFeedbackTimer);
    apiKeyFeedbackTimer = setTimeout(() => { apiKeyFeedback = null; }, 5000);
  }

  function handleCancelRevoke() {
    pendingRevokeId = null;
  }

  function handleCopyKey() {
    if (newlyCreatedKey) {
      navigator.clipboard.writeText(newlyCreatedKey.key);
    }
  }

  function handleDismissNewKey() {
    newlyCreatedKey = null;
  }

  // ── Signing Mode (BYOC) ───────────────────────────────────────────────────
  // Bedrock = per-install local CA (default, offline-first).
  // Conformant = user-imported C2PA-trust-list cert for cross-tool interoperability.

  let signingMode = $state<SigningMode>('bedrock');
  let conformantCert = $state<ConformantCertificateInfo | null>(null);
  let signingModeLoading = $state(false);

  // Cert import flow
  let showCertImport = $state(false);
  let certPath = $state('');
  let keyPath = $state('');
  let importLoading = $state(false);
  let importError = $state<string | null>(null);

  // Post-import mode-switch offer
  let offerModeSwitch = $state(false);
  let modeSwitchLoading = $state(false);
  let modeSwitchError = $state<string | null>(null);

  // Clear cert confirmation
  let pendingClearCert = $state(false);
  let clearCertLoading = $state(false);

  // Local Signing certificate details toggle
  let showLocalCertDetails = $state(false);

  // Fingerprint copy feedback
  let fingerprintCopied = $state(false);
  let fingerprintCopyTimer: ReturnType<typeof setTimeout> | null = null;

  const canImport = $derived(certPath.trim().length > 0 && keyPath.trim().length > 0);

  async function handlePickCertFile() {
    if (!isTauri()) return;
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      title: 'Select Certificate Chain (PEM)',
      filters: [{ name: 'Certificate', extensions: ['pem', 'crt', 'cer'] }],
    });
    if (selected && typeof selected === 'string') {
      certPath = selected;
    }
  }

  async function handlePickKeyFile() {
    if (!isTauri()) return;
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      title: 'Select Private Key (PEM)',
      filters: [{ name: 'Private key', extensions: ['pem', 'key'] }],
    });
    if (selected && typeof selected === 'string') {
      keyPath = selected;
    }
  }

  async function handleImportCert() {
    if (!canImport) return;
    importLoading = true;
    importError = null;
    try {
      const info = await importConformantCertificate(certPath.trim(), keyPath.trim());
      conformantCert = info;
      showCertImport = false;
      certPath = '';
      keyPath = '';
      // Offer to switch mode if currently on Bedrock
      if (signingMode === 'bedrock') {
        offerModeSwitch = true;
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      // AppError objects from Tauri come as { code, message }
      if (err !== null && typeof err === 'object' && 'message' in err) {
        importError = (err as { message: string }).message;
      } else {
        importError = msg;
      }
    } finally {
      importLoading = false;
    }
  }

  async function handleSwitchToConformant() {
    modeSwitchLoading = true;
    modeSwitchError = null;
    try {
      await setSigningMode('conformant');
      signingMode = 'conformant';
      offerModeSwitch = false;
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      modeSwitchError = msg;
    } finally {
      modeSwitchLoading = false;
    }
  }

  async function handleSwitchToBedrock() {
    signingModeLoading = true;
    try {
      await setSigningMode('bedrock');
      signingMode = 'bedrock';
    } catch {
      // Switching to Bedrock cannot fail (no cert required)
    } finally {
      signingModeLoading = false;
    }
  }

  function handleRequestClearCert() {
    pendingClearCert = true;
  }

  function handleCancelClearCert() {
    pendingClearCert = false;
  }

  async function handleConfirmClearCert() {
    clearCertLoading = true;
    try {
      await clearConformantCert();
      conformantCert = null;
      signingMode = 'bedrock';
      pendingClearCert = false;
    } catch {
      // Idempotent — treat any error as a no-op
      pendingClearCert = false;
    } finally {
      clearCertLoading = false;
    }
  }

  async function handleCopyFingerprint() {
    if (!conformantCert) return;
    await navigator.clipboard.writeText(conformantCert.fingerprintSha256);
    fingerprintCopied = true;
    if (fingerprintCopyTimer !== null) clearTimeout(fingerprintCopyTimer);
    fingerprintCopyTimer = setTimeout(() => { fingerprintCopied = false; }, 2000);
  }

  function handleCancelImport() {
    showCertImport = false;
    certPath = '';
    keyPath = '';
    importError = null;
  }

  /** Format an ISO 8601 date as a short absolute date. */
  function formatAbsoluteDate(iso: string): string {
    try {
      return new Date(iso).toLocaleDateString('en-GB', {
        day: 'numeric', month: 'short', year: 'numeric',
      });
    } catch {
      return iso;
    }
  }

  /** Return a human-friendly relative distance from now. */
  function humaniseDistance(iso: string): string {
    try {
      const ms = new Date(iso).getTime() - Date.now();
      const abs = Math.abs(ms);
      const past = ms < 0;
      const days = Math.round(abs / 86_400_000);
      const months = Math.round(days / 30.4);
      const years = Math.round(days / 365);
      let label: string;
      if (days < 1) label = 'today';
      else if (days < 2) label = '1 day';
      else if (days < 60) label = `${days} days`;
      else if (months < 24) label = `${months} month${months === 1 ? '' : 's'}`;
      else label = `${years} year${years === 1 ? '' : 's'}`;
      if (label === 'today') return past ? 'expired today' : 'valid until today';
      return past ? `expired ${label} ago` : `expires in ${label}`;
    } catch {
      return '';
    }
  }

  /** Format an ISO date as "X days/months ago" (for importedAt). */
  function humaniseAgo(iso: string): string {
    try {
      const ms = Date.now() - new Date(iso).getTime();
      const days = Math.round(ms / 86_400_000);
      if (days < 1) return 'today';
      if (days < 2) return '1 day ago';
      if (days < 60) return `${days} days ago`;
      const months = Math.round(days / 30.4);
      return `${months} month${months === 1 ? '' : 's'} ago`;
    } catch {
      return '';
    }
  }

  // ── Network Access Mode ──────────────────────────────────────────────────
  // Controls whether Jura Trace makes outbound network connections during
  // verification. Standard = fully local (default). Enhanced = OCSP/CRL checks
  // and remote Content Credentials retrieval during verification.

  let networkMode = $state<NetworkMode>('standard');
  let networkModeChanging = $state(false);
  let networkModeFeedback = $state<{ ok: boolean; message: string } | null>(null);
  let networkModeFeedbackTimer: ReturnType<typeof setTimeout> | null = null;

  async function handleNetworkModeChange(mode: NetworkMode) {
    if (mode === networkMode) return;
    networkModeChanging = true;
    networkModeFeedback = null;
    try {
      const resolved = await setNetworkMode(mode);
      networkMode = resolved;
      networkModeFeedback = {
        ok: true,
        message: mode === 'standard'
          ? 'Network mode set to Standard. No outbound connections will be made.'
          : 'Network mode set to Enhanced. Online verification features are now active.',
      };
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      networkModeFeedback = { ok: false, message: `Failed to update network mode: ${msg}` };
    } finally {
      networkModeChanging = false;
      if (networkModeFeedbackTimer !== null) clearTimeout(networkModeFeedbackTimer);
      networkModeFeedbackTimer = setTimeout(() => { networkModeFeedback = null; }, 5000);
    }
  }
</script>

<div class="space-y-6">
  <h1 class="text-2xl font-heading text-text-light dark:text-quartz">Settings</h1>

  <!-- Ollama Configuration -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="ollama-heading"
  >
    <h2 id="ollama-heading" class="text-lg font-heading text-text-light dark:text-quartz mb-4">AI Assistant (Ollama)</h2>
    <div class="space-y-4">

      <div>
        <label for="ollama-url" class="block text-sm font-medium text-text-light dark:text-quartz mb-1">
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
        <p class="text-xs text-flint-dark dark:text-flint-light mt-1">
          Ollama runs AI models locally on your computer for two optional features: (1) reading text
          visible in images such as screenshots or memes, and (2) checking factual claims in
          transcribed speech against a knowledge base. Neither feature is required — Jura Trace works
          fully without Ollama.
        </p>
      </div>

      <div>
        <label for="vision-model" class="block text-sm font-medium text-text-light dark:text-quartz mb-1">
          Image description model
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
        <p class="text-xs text-flint-dark dark:text-flint-light mt-1">
          Used for image description and visual analysis
        </p>
      </div>

      <div>
        <label for="text-model" class="block text-sm font-medium text-text-light dark:text-quartz mb-1">
          Claim checking model
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
        <p class="text-xs text-flint-dark dark:text-flint-light mt-1">
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
            class="text-sm text-malachite-dark dark:text-malachite-light motion-safe:animate-fade-in"
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
        <h2 id="profiles-heading" class="text-lg font-heading text-text-light dark:text-quartz">Deployment Profiles</h2>
        <p class="text-xs text-flint-dark dark:text-flint-light mt-0.5">
          Save your current AI settings as a named profile so you can quickly switch between different configurations.
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
                   ? 'border-graphite-light text-flint-dark dark:text-flint-light cursor-not-allowed opacity-50'
                   : 'border-lapis/60 text-lapis dark:text-lapis-light hover:bg-lapis/10 hover:border-lapis'}"
        >
          Save current settings as profile
        </button>
      {/if}
    </div>

    <!-- Max profiles warning -->
    {#if atProfileLimit}
      <p
        id="profile-limit-notice"
        class="text-xs text-amber-dark dark:text-amber-light mb-4 px-3 py-2 rounded border border-amber/20 bg-amber/5"
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
            <p id="profile-name-error" class="text-xs text-cinnabar-dark dark:text-cinnabar-light" role="alert">
              {nameError}
            </p>
          {:else}
            <p id="profile-name-hint" class="text-xs text-flint-dark dark:text-flint-light">
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
              class="px-4 py-1.5 text-sm font-medium rounded border border-graphite-light text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz hover:border-lapis/50 transition-colors
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
      <p class="text-sm text-flint-dark dark:text-flint-light py-4 text-center border border-dashed border-border-light dark:border-border-dark rounded-lg">
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
                <p class="text-xs text-flint-dark dark:text-flint-light mt-0.5 truncate">
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
                    class="px-3 py-1.5 text-xs font-medium rounded border border-lapis/50 text-lapis dark:text-lapis-light hover:bg-lapis/10 hover:border-lapis transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
                  >
                    Load
                  </button>
                {/if}

                {#if pendingDeleteId !== profile.id}
                  <button
                    onclick={() => handleRequestDelete(profile.id)}
                    aria-label="Delete profile {profile.name}"
                    class="px-3 py-1.5 text-xs font-medium rounded border border-cinnabar/30 text-cinnabar-dark dark:text-cinnabar-light hover:bg-cinnabar/10 hover:border-cinnabar/60 transition-colors
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
                <p class="text-xs text-cinnabar-dark dark:text-cinnabar-light flex-1">
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
                  class="px-3 py-1.5 text-xs font-medium rounded border border-graphite-light text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz hover:border-lapis/50 transition-colors
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
      <div class="flex items-center gap-1.5">
        <h2 id="status-heading" class="text-lg font-heading text-text-light dark:text-quartz">Service Status</h2>
        <ContextualHelpLink href="/help/settings#service-status" label="Learn about service status indicators" />
      </div>
      <button
        class="text-xs px-3 py-2.5 min-h-[44px] rounded border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-text-light dark:hover:text-quartz hover:border-lapis/50
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
      <!-- Analysis Engine card -->
      <div class="rounded-lg border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40 p-4">
        <div class="flex items-center justify-between mb-3">
          <span class="text-sm font-medium text-text-light dark:text-quartz">Analysis Engine</span>
          <span
            class="text-xs px-2 py-0.5 rounded-full
                   {sidecarOnline
                     ? 'bg-malachite/15 text-malachite-light border border-malachite/20'
                     : 'bg-cinnabar/10 text-cinnabar-dark dark:text-cinnabar-light border border-cinnabar/20'}"
          >
            {sidecarOnline ? 'Online' : 'Offline'}
          </span>
        </div>
        <!-- P0-7: raw URL removed — meaningless to non-technical pilots -->

        {#if sidecarOnline && sidecarHealth}
          <p class="text-xs text-flint-dark dark:text-flint-light mb-2">Version: <span class="text-text-light dark:text-quartz">{sidecarHealth.version}</span></p>
          <div class="flex flex-wrap gap-1.5">
            {#each Object.entries(sidecarHealth.capabilities).filter(([cap]) => !V1_DEFERRED_CAPABILITIES.has(cap)) as [cap, enabled]}
              <span
                class="text-xs px-2 py-0.5 rounded
                       {enabled
                         ? 'bg-malachite/10 text-malachite-light border border-malachite/20'
                         : 'bg-gray-100 dark:bg-graphite-light text-flint-dark dark:text-flint-light border border-border-light dark:border-graphite-light'}"
                title={cap === 'clipDetect' && !enabled
                  ? 'CLIP ONNX models missing or onnxruntime unavailable — UnivFD v9 probe and zero-shot AI cross-check are both offline. GBM v4 base classifier still runs but recall on diffusion models is degraded.'
                  : ''}
              >
                {cap === 'clipDetect' ? 'AI cross-check (CLIP)'
                  : cap === 'rag' ? 'Knowledge base'
                  : cap}
              </span>
            {/each}
          </div>
        {:else}
          <div class="mt-2 space-y-2">
            <p class="text-xs font-medium text-cinnabar-dark dark:text-cinnabar-light leading-relaxed">
              Analysis Engine — Offline
            </p>
            <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed">
              Core checks (provenance and metadata) work without it. For full
              forensic analysis including AI detection, restart Jura Trace. If
              the engine remains offline after restarting, visit the Help
              section or contact support.
            </p>
          </div>
        {/if}
      </div>

      <!-- Ollama card — JTV-132: three-state status + one-click install + model download -->
      <div class="rounded-lg border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40 p-4">
        <div class="flex items-center justify-between mb-3">
          <span class="text-sm font-medium text-text-light dark:text-quartz">Ollama <span class="text-xs font-normal text-flint-dark dark:text-flint-light">(optional)</span></span>
          <span
            class="text-xs px-2 py-0.5 rounded-full
                   {ollamaState === 'ready'
                     ? 'bg-malachite/15 text-malachite-light border border-malachite/20'
                     : ollamaState === 'partial' || ollamaState === 'installed-no-models'
                       ? 'bg-amber/15 text-amber-dark dark:text-amber-light border border-amber/30'
                       : 'bg-gray-100 dark:bg-graphite-light text-flint-dark dark:text-flint-light border border-border-light dark:border-graphite-light'}"
            aria-live="polite"
          >
            {ollamaState === 'ready'
              ? 'Ready'
              : ollamaState === 'partial'
                ? 'Models incomplete'
                : ollamaState === 'installed-no-models'
                  ? 'Installed — no models'
                  : 'Not installed'}
          </span>
        </div>

        <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mb-3">
          Optional enrichment: AI image descriptions (LLaVA) and claim verification (Qwen2.5).
          Core verification, forensic analysis, and AI deepfake detection all work without Ollama.
          {#if ollamaUrl !== DEFAULT_OLLAMA_URL}
            <span class="block mt-1">URL: <code class="font-mono text-[11px]">{ollamaUrl}</code></span>
          {/if}
        </p>

        {#if ollamaState === 'not-installed'}
          <!-- Install path -->
          <div class="flex flex-wrap items-center gap-2 mb-2">
            {#if platform === 'windows' || platform === 'mac'}
              <button
                onclick={installOllama}
                disabled={ollamaInstalling || healthLoading}
                class="min-h-[36px] inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium
                       bg-lapis hover:bg-lapis-dark text-white transition-colors duration-150
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-graphite
                       disabled:opacity-50 disabled:cursor-not-allowed"
                aria-label="Install Ollama automatically using {platform === 'windows' ? 'winget' : 'Homebrew'}"
              >
                {#if ollamaInstalling}
                  <svg class="w-3.5 h-3.5 motion-safe:animate-spin" fill="none" viewBox="0 0 16 16" aria-hidden="true">
                    <circle cx="8" cy="8" r="5" stroke="currentColor" stroke-width="2" stroke-dasharray="14 14" />
                  </svg>
                  Installing…
                {:else}
                  Install Ollama automatically
                {/if}
              </button>
            {/if}
            <button
              onclick={openOllamaDownload}
              class="min-h-[36px] inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium
                     border border-lapis/40 text-lapis dark:text-lapis-light hover:bg-lapis/10 transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
            >
              {platform === 'linux' ? 'View install instructions' : 'Download from ollama.com'}
            </button>
            <button
              onclick={refreshHealth}
              disabled={healthLoading}
              class="min-h-[36px] px-3 py-1.5 rounded-md text-xs text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-graphite
                     disabled:opacity-50"
            >
              {healthLoading ? 'Checking…' : 'Re-check'}
            </button>
          </div>

          {#if ollamaInstallError}
            <div role="alert" class="rounded-md px-3 py-2 mb-2 bg-cinnabar/10 border border-cinnabar/25 text-xs text-cinnabar-dark dark:text-cinnabar-light">
              {ollamaInstallError}
            </div>
          {/if}

          <p class="text-[11px] text-flint-dark dark:text-flint-light leading-relaxed">
            <a
              href="/help/ollama"
              class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
            >Read the Ollama install guide →</a>
          </p>

        {:else}
          <!-- Reachable: show model checklist + pull buttons -->
          <ul class="space-y-2 mb-2" aria-label="Ollama models">
            {#each [{ name: 'llava:7b', label: 'Image descriptions', size: '~4.7 GB', installed: llavaInstalled }, { name: 'qwen2.5:7b-instruct', label: 'Claim verification', size: '~4.4 GB', installed: qwenInstalled }] as model (model.name)}
              <li class="flex items-center gap-3 rounded px-3 py-2 border {model.installed ? 'bg-malachite/5 border-malachite/20' : 'bg-gray-100 dark:bg-graphite-light/30 border-border-light dark:border-border-dark'}">
                <span class="flex-shrink-0 w-2 h-2 rounded-full {model.installed ? 'bg-malachite-light' : 'bg-flint-light'}" aria-hidden="true"></span>
                <div class="flex-1 min-w-0">
                  <p class="text-xs font-medium {model.installed ? 'text-malachite-dark dark:text-malachite-light' : 'text-text-light dark:text-quartz'}">
                    <code class="font-mono">{model.name}</code>
                    <span class="ml-1.5 font-normal text-flint-dark dark:text-flint-light">{model.size}</span>
                  </p>
                  <p class="text-[11px] text-flint-dark dark:text-flint-light mt-0.5">
                    {model.installed
                      ? `${model.label} — ready`
                      : pullingModel === model.name
                        ? (pullProgress ?? 'Downloading…') + (pullPercent !== null ? ` (${pullPercent}%)` : '')
                        : `${model.label} — not downloaded`}
                  </p>
                  {#if pullingModel === model.name && pullPercent !== null}
                    <div class="mt-1 h-1 rounded-full overflow-hidden bg-lapis/15" role="progressbar" aria-valuenow={pullPercent} aria-valuemin={0} aria-valuemax={100}>
                      <div class="h-full bg-lapis-light transition-all duration-300" style="width: {pullPercent}%;"></div>
                    </div>
                  {/if}
                </div>
                {#if !model.installed && pullingModel !== model.name}
                  <button
                    onclick={() => pullOllamaModel(model.name)}
                    disabled={pullingModel !== null}
                    class="min-h-[32px] px-2.5 py-1 rounded text-xs font-medium border border-lapis/40 text-lapis dark:text-lapis-light hover:bg-lapis/10 transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-graphite
                           disabled:opacity-50 disabled:cursor-not-allowed"
                    aria-label="Download {model.name} ({model.size})"
                  >
                    Download
                  </button>
                {/if}
              </li>
            {/each}
          </ul>

          {#if pullError}
            <div role="alert" class="rounded-md px-3 py-2 bg-cinnabar/10 border border-cinnabar/25 text-xs text-cinnabar-dark dark:text-cinnabar-light">
              {pullError}
            </div>
          {/if}

          <p class="text-[11px] text-flint-dark dark:text-flint-light leading-relaxed">
            If you also use ROOTED these models are shared — you only need to download them once.
            <a
              href="/help/ollama"
              class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
            >Install guide →</a>
          </p>
        {/if}
      </div>
    </div>
  </section>

  <!-- Database Location -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="db-location-heading"
  >
    <div class="flex items-center gap-1.5 mb-1">
      <h2 id="db-location-heading" class="text-lg font-heading text-text-light dark:text-quartz">Database Location</h2>
      <ContextualHelpLink href="/help/settings#database" label="Learn about database storage and location settings" />
    </div>
    <p class="text-xs text-flint-dark dark:text-flint-light mb-4">
      Where assets, fingerprints, and verification records are stored. Useful for institutional deployments where data must reside on a shared or managed drive.
    </p>

    <div>
      <label for="db-path" class="block text-sm font-medium text-text-light dark:text-quartz mb-1">
        Current database file
      </label>
      <div class="flex gap-2 max-w-xl">
        <input
          id="db-path"
          type="text"
          value={currentDbPath || 'Loading…'}
          readonly
          class="flex-1 px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian text-text-light dark:text-quartz text-sm font-mono
                 cursor-not-allowed opacity-70"
          aria-readonly="true"
          aria-describedby="db-path-hint"
        />
        <button
          onclick={handleChangeDbLocation}
          disabled={dbPathChanging || !isTauri()}
          class="shrink-0 px-4 py-2.5 min-h-[44px] rounded border text-sm font-medium transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                 {dbPathChanging || !isTauri()
                   ? 'border-graphite-light text-flint-dark dark:text-flint-light cursor-not-allowed opacity-50'
                   : 'border-lapis/60 text-lapis dark:text-lapis-light hover:bg-lapis/10 hover:border-lapis'}"
          aria-busy={dbPathChanging}
        >
          {#if dbPathChanging}
            <span class="flex items-center gap-1.5">
              <span
                class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
                aria-hidden="true"
              ></span>
              Moving…
            </span>
          {:else}
            Change Location…
          {/if}
        </button>
      </div>

      <p id="db-path-hint" class="text-xs text-flint-dark dark:text-flint-light mt-1">
        The database will be copied atomically to the new location. The original file is not deleted until the move is verified.
      </p>

      {#if dbPathFeedback !== null}
        <p
          class="mt-3 text-sm px-3 py-2 rounded border
                 {dbPathFeedback.ok
                   ? 'text-malachite-dark dark:text-malachite-light border-malachite/20 bg-malachite/5'
                   : 'text-cinnabar-dark dark:text-cinnabar-light border-cinnabar/20 bg-cinnabar/5'}"
          role="status"
          aria-live="polite"
        >
          {dbPathFeedback.message}
        </p>
      {/if}
    </div>
  </section>

  <!-- Data Management (JTV-130) -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="data-mgmt-heading"
  >
    <div class="flex items-center gap-1.5 mb-1">
      <h2 id="data-mgmt-heading" class="text-lg font-heading text-text-light dark:text-quartz">Data Management</h2>
    </div>
    <p class="text-xs text-flint-dark dark:text-flint-light mb-5">
      Disaster-recovery tools for institutional deployments. Snapshots include the full asset catalogue, fingerprints, audit log, and verification history.
    </p>

    <!-- Backup -->
    <div class="mb-6">
      <h3 class="text-sm font-medium text-text-light dark:text-quartz mb-1">Backup</h3>
      <p class="text-xs text-flint-dark dark:text-flint-light mb-2">
        Write a defragmented snapshot of the current database to a folder of your choice. A JSON manifest sidecar records the schema version and a SHA-256 checksum for later verification.
      </p>
      <button
        onclick={handleBackupDatabase}
        disabled={backupBusy || !isTauri()}
        class="px-4 py-2.5 min-h-[44px] rounded border text-sm font-medium transition-colors
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
               {backupBusy || !isTauri()
                 ? 'border-graphite-light text-flint-dark dark:text-flint-light cursor-not-allowed opacity-50'
                 : 'border-lapis/60 text-lapis dark:text-lapis-light hover:bg-lapis/10 hover:border-lapis'}"
        aria-busy={backupBusy}
      >
        {#if backupBusy}
          <span class="flex items-center gap-1.5">
            <span class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin" aria-hidden="true"></span>
            Writing snapshot…
          </span>
        {:else}
          Create Snapshot…
        {/if}
      </button>
      {#if backupSuccess !== null}
        <div
          class="mt-3 text-sm px-3 py-2 rounded border text-malachite-dark dark:text-malachite-light border-malachite/20 bg-malachite/5"
          role="status"
          aria-live="polite"
        >
          <p class="font-medium mb-1">Snapshot written.</p>
          <dl class="grid grid-cols-[max-content_1fr] gap-x-3 gap-y-1 text-xs">
            <dt class="text-flint-dark dark:text-flint-light">File</dt>
            <dd class="break-all font-mono select-all">{backupSuccess.snapshotPath}</dd>
            <dt class="text-flint-dark dark:text-flint-light">Manifest</dt>
            <dd class="break-all font-mono select-all">{backupSuccess.manifestPath}</dd>
            <dt class="text-flint-dark dark:text-flint-light">Schema</dt>
            <dd>v{backupSuccess.schemaVersion}</dd>
            <dt class="text-flint-dark dark:text-flint-light">SHA-256</dt>
            <dd class="flex items-start gap-2 min-w-0">
              <code class="break-all font-mono text-[11px] select-all flex-1">{backupSuccess.sha256}</code>
              <button
                type="button"
                onclick={() => backupSuccess && copyToClipboard(backupSuccess.sha256)}
                class="shrink-0 px-2 py-0.5 text-[11px] rounded border border-border-light dark:border-border-dark text-lapis dark:text-lapis-light hover:bg-lapis/10
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis"
                aria-label="Copy SHA-256 to clipboard"
              >Copy</button>
            </dd>
          </dl>
        </div>
      {/if}
      {#if backupError !== null}
        <p
          class="mt-3 text-sm px-3 py-2 rounded border text-cinnabar-dark dark:text-cinnabar-light border-cinnabar/20 bg-cinnabar/5"
          role="status"
          aria-live="polite"
        >
          Backup failed: {backupError}
        </p>
      {/if}
    </div>

    <!-- Restore -->
    <div class="mb-6 pt-5 border-t border-border-light dark:border-border-dark">
      <h3 class="text-sm font-medium text-text-light dark:text-quartz mb-1">Restore</h3>
      <p class="text-xs text-flint-dark dark:text-flint-light mb-2">
        Replace the current database with a previously-saved snapshot. The audit trail in the snapshot is verified before any destructive change is made. This cannot be undone.
      </p>
      <button
        onclick={handleRestorePreflight}
        disabled={restoreBusy || !isTauri()}
        class="px-4 py-2.5 min-h-[44px] rounded border text-sm font-medium transition-colors
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
               {restoreBusy || !isTauri()
                 ? 'border-graphite-light text-flint-dark dark:text-flint-light cursor-not-allowed opacity-50'
                 : 'border-cinnabar/60 text-cinnabar dark:text-cinnabar-light hover:bg-cinnabar/10 hover:border-cinnabar'}"
        aria-busy={restoreBusy}
      >
        {#if restoreBusy}
          <span class="flex items-center gap-1.5">
            <span class="w-3 h-3 border-2 border-cinnabar border-t-transparent rounded-full motion-safe:animate-spin" aria-hidden="true"></span>
            Working…
          </span>
        {:else}
          Restore from Snapshot…
        {/if}
      </button>
      {#if restoreFeedback !== null}
        <p
          class="mt-3 text-sm px-3 py-2 rounded border
                 {restoreFeedback.ok
                   ? 'text-malachite-dark dark:text-malachite-light border-malachite/20 bg-malachite/5'
                   : 'text-cinnabar-dark dark:text-cinnabar-light border-cinnabar/20 bg-cinnabar/5'}"
          role="status"
          aria-live="polite"
        >
          {restoreFeedback.message}
        </p>
      {/if}
    </div>

    <!-- Import CSV catalogue -->
    <div class="pt-5 border-t border-border-light dark:border-border-dark">
      <h3 class="text-sm font-medium text-text-light dark:text-quartz mb-1">Import CSV catalogue</h3>
      <p class="text-xs text-flint-dark dark:text-flint-light mb-2">
        Bulk-add assets to the catalogue from a CSV file. The CSV must have a <code class="font-mono text-[11px]">file_path</code> column. Rows whose SHA-256 already exists in the catalogue are skipped silently.
      </p>
      <button
        onclick={handleImportCsv}
        disabled={csvImportBusy || !isTauri()}
        class="px-4 py-2.5 min-h-[44px] rounded border text-sm font-medium transition-colors
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
               {csvImportBusy || !isTauri()
                 ? 'border-graphite-light text-flint-dark dark:text-flint-light cursor-not-allowed opacity-50'
                 : 'border-lapis/60 text-lapis dark:text-lapis-light hover:bg-lapis/10 hover:border-lapis'}"
        aria-busy={csvImportBusy}
      >
        {#if csvImportBusy}
          <span class="flex items-center gap-1.5">
            <span class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin" aria-hidden="true"></span>
            Importing…
          </span>
        {:else}
          Import from CSV…
        {/if}
      </button>
      {#if csvImportResult !== null}
        <div
          class="mt-3 text-sm px-3 py-2 rounded border text-text-light dark:text-quartz border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40"
          role="status"
          aria-live="polite"
        >
          <p>
            {csvImportResult.imported} imported,
            {csvImportResult.skippedDuplicates} skipped (already in catalogue),
            {csvImportResult.failed} failed.
          </p>
          {#if csvImportResult.errors.length > 0}
            <details class="mt-2">
              <summary class="cursor-pointer text-xs text-flint-dark dark:text-flint-light">
                Show {csvImportResult.errors.length} row error{csvImportResult.errors.length === 1 ? '' : 's'}
              </summary>
              <ul class="mt-1 text-xs text-cinnabar-dark dark:text-cinnabar-light list-disc list-inside space-y-0.5">
                {#each csvImportResult.errors as err}
                  <li>{err}</li>
                {/each}
              </ul>
            </details>
          {/if}
        </div>
      {/if}
      {#if csvImportError !== null}
        <p
          class="mt-3 text-sm px-3 py-2 rounded border text-cinnabar-dark dark:text-cinnabar-light border-cinnabar/20 bg-cinnabar/5"
          role="status"
          aria-live="polite"
        >
          Import failed: {csvImportError}
        </p>
      {/if}
    </div>
  </section>

  <!-- Restore confirmation modal (JTV-130) -->
  {#if pendingRestore !== null}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 px-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="restore-modal-heading"
    >
      <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark max-w-lg w-full p-6">
        <h2 id="restore-modal-heading" class="text-lg font-heading text-text-light dark:text-quartz mb-3">
          Replace database with snapshot?
        </h2>
        <p class="text-sm text-text-light dark:text-quartz mb-3">
          The current database will be replaced with the snapshot you selected. <strong>This cannot be undone.</strong>
        </p>
        <dl class="grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1 text-xs mb-4">
          <dt class="text-flint-dark dark:text-flint-light">Snapshot path</dt>
          <dd class="text-text-light dark:text-quartz break-all font-mono">{pendingRestore.path}</dd>
          <dt class="text-flint-dark dark:text-flint-light">Asset count</dt>
          <dd class="text-text-light dark:text-quartz">{pendingRestore.preflight.assetCount}</dd>
          <dt class="text-flint-dark dark:text-flint-light">Schema version</dt>
          <dd class="text-text-light dark:text-quartz">v{pendingRestore.preflight.snapshotSchemaVersion}{pendingRestore.preflight.snapshotSchemaVersion !== pendingRestore.preflight.currentSchemaVersion ? ` → v${pendingRestore.preflight.currentSchemaVersion} (forward migration on open)` : ''}</dd>
          <dt class="text-flint-dark dark:text-flint-light">Audit chain</dt>
          <dd class="text-text-light dark:text-quartz">{pendingRestore.preflight.auditChainValid ? 'Verified' : 'Invalid'}</dd>
        </dl>
        <p class="text-xs text-flint-dark dark:text-flint-light mb-3">
          The application will reload after the restore completes.
        </p>
        <!-- P1-2: Offer to back up the current database first.
             Niamh Gallagher persona flagged the destructive replace
             without an undo path. -->
        {#if !backupSuccess}
          <p class="text-xs text-flint-dark dark:text-flint-light mb-5">
            <button
              type="button"
              onclick={handleBackupDatabase}
              disabled={backupBusy || restoreBusy}
              class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:text-obsidian dark:hover:text-quartz
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded
                     {backupBusy || restoreBusy ? 'opacity-50 cursor-not-allowed' : ''}"
            >
              {backupBusy ? 'Writing snapshot of current database…' : 'Back up the current database first →'}
            </button>
          </p>
        {:else}
          <p class="text-xs text-malachite-dark dark:text-malachite-light mb-5">
            Current database backed up to <span class="font-mono break-all select-all">{backupSuccess.snapshotPath}</span>.
          </p>
        {/if}
        <div class="flex gap-3 justify-end">
          <button
            onclick={handleRestoreCancel}
            disabled={restoreBusy}
            class="px-4 py-2.5 min-h-[44px] rounded border border-border-light dark:border-border-dark text-sm text-text-light dark:text-quartz hover:bg-gray-50 dark:hover:bg-obsidian/50
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          >
            Cancel
          </button>
          <button
            onclick={handleRestoreConfirmed}
            disabled={restoreBusy || !pendingRestore.preflight.auditChainValid}
            class="px-4 py-2.5 min-h-[44px] rounded border text-sm font-medium transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                   {restoreBusy || !pendingRestore.preflight.auditChainValid
                     ? 'border-graphite-light text-flint-dark dark:text-flint-light cursor-not-allowed opacity-50'
                     : 'border-cinnabar bg-cinnabar text-white hover:bg-cinnabar-dark'}"
            aria-busy={restoreBusy}
          >
            {#if restoreBusy}
              Restoring…
            {:else}
              Restore — I understand this will replace all current data
            {/if}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- About -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="about-heading"
  >
    <h2 id="about-heading" class="text-lg font-heading text-text-light dark:text-quartz mb-4">About</h2>
    <dl class="grid grid-cols-[max-content_1fr] gap-x-8 gap-y-2 text-sm max-w-md">
      <dt class="text-flint-dark dark:text-flint-light">Version</dt>
      <dd class="text-text-light dark:text-quartz">{appVersion}</dd>
      <dt class="text-flint-dark dark:text-flint-light">Licence</dt>
      <dd class="text-text-light dark:text-quartz">AGPL-3.0-or-later</dd>
      <dt class="text-flint-dark dark:text-flint-light">Developer</dt>
      <dd class="text-text-light dark:text-quartz">
        <a
          href="https://juralabs.org"
          target="_blank"
          rel="noopener noreferrer"
          class="text-lapis dark:text-lapis-light hover:text-lapis-dark dark:hover:text-lapis dark:text-lapis-light underline underline-offset-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
        >Jura Labs CIC<span class="sr-only"> (opens in new tab)</span></a>
      </dd>
    </dl>

    <!-- Software updates -->
    <div class="mt-6 pt-5 border-t border-border-light dark:border-border-dark">
      <div class="flex items-center gap-4 flex-wrap">
        <button
          onclick={checkForUpdate}
          disabled={updateStatus.state === 'checking' || updateStatus.state === 'downloading' || updateStatus.state === 'installing'}
          class="px-5 py-2.5 min-h-[44px] rounded border text-sm font-medium transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                 {updateStatus.state === 'checking' || updateStatus.state === 'downloading' || updateStatus.state === 'installing'
                   ? 'border-graphite-light text-flint-dark dark:text-flint-light cursor-not-allowed opacity-50'
                   : 'border-lapis/60 text-lapis dark:text-lapis-light hover:bg-lapis/10 hover:border-lapis'}"
          aria-busy={updateStatus.state === 'checking' || updateStatus.state === 'downloading'}
        >
          {#if updateStatus.state === 'checking'}
            <span class="flex items-center gap-1.5">
              <span
                class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
                aria-hidden="true"
              ></span>
              Checking for updates...
            </span>
          {:else if updateStatus.state === 'downloading'}
            <span class="flex items-center gap-1.5">
              <span
                class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
                aria-hidden="true"
              ></span>
              Downloading update...
            </span>
          {:else if updateStatus.state === 'installing'}
            <span class="flex items-center gap-1.5">
              <span
                class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
                aria-hidden="true"
              ></span>
              Installing — restarting shortly...
            </span>
          {:else}
            Check for Updates
          {/if}
        </button>

        <!-- Inline status feedback -->
        {#if updateStatus.state === 'up-to-date'}
          <p
            class="text-sm text-malachite-dark dark:text-malachite-light"
            role="status"
            aria-live="polite"
          >
            Jura Trace is up to date.
          </p>
        {:else if updateStatus.state === 'available'}
          <p
            class="text-sm text-lapis dark:text-lapis-light"
            role="status"
            aria-live="polite"
          >
            Version {updateStatus.version} is available — downloading...
          </p>
        {:else if updateStatus.state === 'error'}
          <p
            class="text-sm text-cinnabar-dark dark:text-cinnabar-light"
            role="alert"
            aria-live="assertive"
          >
            {updateStatus.message}
          </p>
        {/if}
      </div>

      <p class="text-xs text-flint-dark dark:text-flint-light mt-2">
        Updates are downloaded and applied locally. No telemetry is sent.
      </p>
    </div>
  </section>

  <!-- Setup Wizard -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="setup-wizard-heading"
  >
    <div class="flex items-center gap-1.5 mb-1">
      <h2 id="setup-wizard-heading" class="text-lg font-heading text-text-light dark:text-quartz">Setup Wizard</h2>
      <ContextualHelpLink href="/help/settings#setup-wizard" label="Learn about the setup wizard" />
    </div>
    <p class="text-xs text-flint-dark dark:text-flint-light mb-4">
      Re-run the first-launch setup wizard to check the Analysis Engine, FFmpeg, and Ollama configuration.
      Useful after reinstalling or upgrading Jura Trace.
    </p>

    <button
      onclick={handleRerunWizard}
      class="px-5 py-2.5 min-h-[44px] rounded border text-sm font-medium transition-colors
             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
             border-lapis/60 text-lapis dark:text-lapis-light hover:bg-lapis/10 hover:border-lapis"
    >
      Re-run Setup Wizard
    </button>
  </section>

  <!-- Your Plan -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="plan-heading"
  >
    <div class="flex items-center gap-1.5 mb-1">
      <h2 id="plan-heading" class="text-lg font-heading text-text-light dark:text-quartz">Your Plan</h2>
      <ContextualHelpLink href="/help/settings#your-plan" label="Learn about licence plans and features" />
    </div>
    <p class="text-xs text-flint-dark dark:text-flint-light mb-4">
      During the pilot, you can explore different plans by selecting them here. In the full release, your plan will reflect your licence agreement.
    </p>

    <!-- Current tier badge + description -->
    <div class="flex items-start gap-3 mb-5 p-4 rounded-lg border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40">
      <span
        class="shrink-0 inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {currentTierInfo.badgeClass} {currentTierInfo.badgeTextClass}"
        aria-label="Current plan: {currentTierInfo.name}"
      >
        {currentTierInfo.name}
      </span>
      <div class="min-w-0">
        <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed">
          {currentTierInfo.description}
        </p>
      </div>
    </div>

    <!-- Change plan dropdown -->
    <div class="flex flex-col gap-2 max-w-xs">
      <label for="tier-select" class="block text-sm font-medium text-text-light dark:text-quartz">
        Change plan
      </label>
      <select
        id="tier-select"
        value={currentTier}
        onchange={handleTierChange}
        disabled={tierChanging}
        class="px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian text-text-light dark:text-quartz text-sm transition-colors
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent
               disabled:opacity-50 disabled:cursor-not-allowed"
        aria-describedby="tier-select-hint"
      >
        <option value="community">Community (non-commercial)</option>
        <option value="professional">Professional (individual commercial)</option>
        <option value="enterprise">Enterprise (unlimited seats)</option>
      </select>
      <p id="tier-select-hint" class="text-xs text-flint-dark dark:text-flint-light">
        Pilot mode: tier changes are saved to your local config and persist across restarts.
        Visit <a
          href="https://juralabs.org"
          target="_blank"
          rel="noopener noreferrer"
          class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
        >juralabs.org<span class="sr-only"> (opens in new tab)</span></a> to purchase a licence.
      </p>
    </div>

    {#if tierFeedback !== null}
      <p
        class="mt-3 text-sm px-3 py-2 rounded border
               {tierFeedback.ok
                 ? 'text-malachite-dark dark:text-malachite-light border-malachite/20 bg-malachite/5'
                 : 'text-cinnabar-dark dark:text-cinnabar-light border-cinnabar/20 bg-cinnabar/5'}"
        role="status"
        aria-live="polite"
      >
        {tierFeedback.message}
      </p>
    {/if}
  </section>

  <!-- Analysis preferences -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="analysis-heading"
  >
    <div class="flex items-center gap-1.5 mb-1">
      <h2 id="analysis-heading" class="text-lg font-heading text-text-light dark:text-quartz">Analysis preferences</h2>
    </div>
    <p class="text-xs text-flint-dark dark:text-flint-light mb-4">
      Control which optional analysis stages run during verify. Turning stages off makes verify faster.
    </p>

    <div class="flex items-start justify-between gap-4 p-4 rounded-lg border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40">
      <div class="min-w-0 flex-1">
        <label for="ai-desc-toggle" class="block text-sm font-medium text-text-light dark:text-quartz">
          AI image descriptions
        </label>
        <p class="text-xs text-flint-dark dark:text-flint-light mt-1 leading-relaxed">
          Uses Ollama LLaVA to generate a plain-English description of each verified image.
          Adds roughly 5–30 seconds per image. Requires Ollama with a vision model installed.
        </p>
        <p class="text-xs mt-2">
          {#if ollamaDetected}
            <span class="text-malachite-dark dark:text-malachite-light">Ollama detected</span>
            {#if sidecarHealth?.ollama}<span class="text-flint-dark dark:text-flint-light"> — version {sidecarHealth.ollama}</span>{/if}
          {:else}
            <span class="text-flint-dark dark:text-flint-light">
              Ollama not detected. Enabling this has no effect until Ollama is installed and a vision model (e.g. <code class="font-mono text-[11px]">llava</code>) is pulled.
            </span>
          {/if}
        </p>
        {#if aiDescPref === null}
          <p class="text-xs text-flint-dark dark:text-flint-light mt-2 italic">
            Not yet set — currently off. Enable to opt in.
          </p>
        {/if}
      </div>
      <label class="relative inline-flex items-center cursor-pointer shrink-0 mt-1">
        <input
          id="ai-desc-toggle"
          type="checkbox"
          class="sr-only peer"
          checked={aiDescActive}
          disabled={aiDescChanging}
          onchange={handleAiDescToggle}
          aria-describedby="ai-desc-hint"
        />
        <span
          class="w-11 h-6 bg-gray-300 dark:bg-flint/40 rounded-full peer peer-checked:bg-lapis dark:peer-checked:bg-lapis-light
                 peer-focus-visible:ring-2 peer-focus-visible:ring-lapis peer-focus-visible:ring-offset-2
                 dark:peer-focus-visible:ring-offset-graphite
                 peer-disabled:opacity-50 peer-disabled:cursor-not-allowed
                 after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full
                 after:h-5 after:w-5 after:transition-transform peer-checked:after:translate-x-5"
          aria-hidden="true"
        ></span>
      </label>
    </div>

    {#if aiDescFeedback !== null}
      <p
        class="mt-3 text-sm px-3 py-2 rounded border
               {aiDescFeedback.ok
                 ? 'text-malachite-dark dark:text-malachite-light border-malachite/20 bg-malachite/5'
                 : 'text-cinnabar-dark dark:text-cinnabar-light border-cinnabar/20 bg-cinnabar/5'}"
        role="status"
        aria-live="polite"
      >
        {aiDescFeedback.message}
      </p>
    {/if}

    <!-- Power-saver mode -->
    <div class="flex items-start justify-between gap-4 p-4 rounded-lg border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40 mt-4">
      <div class="min-w-0 flex-1">
        <label for="power-saver-toggle" class="block text-sm font-medium text-text-light dark:text-quartz">
          Power-saver mode
        </label>
        <p id="power-saver-hint" class="text-xs text-flint-dark dark:text-flint-light mt-1 leading-relaxed">
          Stops the analysis engine after five minutes of inactivity to free approximately 300–500 MB of RAM. The first verification afterwards takes 30–90 seconds longer while the engine reloads. Recommended only on machines with under 16 GB of RAM.
        </p>
      </div>
      <label class="relative inline-flex items-center cursor-pointer shrink-0 mt-1">
        <input
          id="power-saver-toggle"
          type="checkbox"
          class="sr-only peer"
          checked={powerSaverMode}
          disabled={powerSaverChanging}
          onchange={handlePowerSaverToggle}
          aria-describedby="power-saver-hint"
        />
        <span
          class="w-11 h-6 bg-gray-300 dark:bg-flint/40 rounded-full peer peer-checked:bg-lapis dark:peer-checked:bg-lapis-light
                 peer-focus-visible:ring-2 peer-focus-visible:ring-lapis peer-focus-visible:ring-offset-2
                 dark:peer-focus-visible:ring-offset-graphite
                 peer-disabled:opacity-50 peer-disabled:cursor-not-allowed
                 after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full
                 after:h-5 after:w-5 after:transition-transform peer-checked:after:translate-x-5"
          aria-hidden="true"
        ></span>
      </label>
    </div>

    {#if powerSaverFeedback !== null}
      <p
        class="mt-3 text-sm px-3 py-2 rounded border
               {powerSaverFeedback.ok
                 ? 'text-malachite-dark dark:text-malachite-light border-malachite/20 bg-malachite/5'
                 : 'text-cinnabar-dark dark:text-cinnabar-light border-cinnabar/20 bg-cinnabar/5'}"
        role="status"
        aria-live="polite"
      >
        {powerSaverFeedback.message}
      </p>
    {/if}
  </section>

  <!-- Signing Mode (BYOC) -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="signing-mode-heading"
  >
    <div class="flex items-center gap-1.5 mb-1">
      <h2 id="signing-mode-heading" class="text-lg font-heading text-text-light dark:text-quartz">Signing Mode</h2>
      <ContextualHelpLink href="/help/bedrock-signing" label="Learn about Local and Conformant signing modes" />
    </div>
    <p class="text-xs text-flint-dark dark:text-flint-light mb-5">
      Controls which certificate Jura Trace uses when embedding C2PA manifests into protected assets.
    </p>

    <!-- Mode cards -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-5" role="group" aria-label="Signing mode selection">

      <!-- Bedrock card -->
      <div
        class="relative flex flex-col rounded-lg border-2 p-5 transition-colors
               {signingMode === 'bedrock'
                 ? 'border-lapis bg-lapis/5 dark:bg-lapis/5'
                 : 'border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40'}"
        aria-current={signingMode === 'bedrock' ? 'true' : undefined}
      >
        {#if signingMode === 'bedrock'}
          <span
            class="absolute top-3 right-3 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-lapis/15 text-lapis dark:text-lapis-light border border-lapis/30"
            aria-label="Currently active"
          >
            Active
          </span>
        {/if}

        <div class="mb-3">
          <p class="text-sm font-semibold text-text-light dark:text-quartz">Local Signing</p>
          <p class="text-xs text-flint-dark dark:text-flint-light mt-0.5">Offline-first default</p>
        </div>

        <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mb-3">
          Uses a per-install certificate authority generated on first launch. Fully offline —
          no account, no external connections, no dependency on external services. Produces
          fully valid C2PA v2.x manifests readable by any C2PA-capable tool worldwide.
        </p>

        <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed pt-3 border-t border-border-light dark:border-border-dark">
          Third-party tools will confirm this file's integrity. Your identity as signer will show
          as unverified in external validators — this is expected in Local Signing mode and does
          not affect the validity of the manifest.
        </p>

        <!-- Certificate Details expandable -->
        <div class="mt-3">
          <button
            onclick={() => showLocalCertDetails = !showLocalCertDetails}
            class="text-xs text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline underline-offset-2 transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded flex items-center gap-1"
            aria-expanded={showLocalCertDetails}
            aria-controls="local-cert-details"
          >
            <svg
              class="w-3 h-3 transition-transform {showLocalCertDetails ? 'rotate-90' : ''}"
              fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
            </svg>
            Certificate details
          </button>
          {#if showLocalCertDetails}
            <div
              id="local-cert-details"
              class="mt-2 p-3 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian/30 text-xs text-flint-dark dark:text-flint-light leading-relaxed space-y-2"
            >
              <div>
                <span class="font-medium text-text-light dark:text-quartz">Algorithm:</span>
                ECDSA P-256 (industry-standard elliptic curve)
              </div>
              <div>
                <span class="font-medium text-text-light dark:text-quartz">Key storage:</span>
                Private key stored only on this device, in the application data directory with restricted file permissions. Never transmitted.
              </div>
              <div>
                <span class="font-medium text-text-light dark:text-quartz">Scope:</span>
                Unique to this installation. Each device generates its own certificate authority on first launch.
              </div>
              <div>
                <span class="font-medium text-text-light dark:text-quartz">If reinstalled:</span>
                A new certificate authority is generated. Files signed previously remain fully valid C2PA manifests — the signature and assertions are intact regardless of whether the original certificate still exists.
              </div>
              <div>
                <span class="font-medium text-text-light dark:text-quartz">Backup:</span>
                Not required. The certificate proves <em>which device</em> signed a file, not <em>whether</em> the file is authentic. Verification works without the original signing device.
              </div>
            </div>
          {/if}
        </div>

        {#if signingMode === 'conformant'}
          <button
            onclick={handleSwitchToBedrock}
            disabled={signingModeLoading}
            class="mt-4 self-start px-4 py-2 min-h-[44px] text-sm font-medium rounded border border-lapis/60 text-lapis dark:text-lapis-light
                   hover:bg-lapis/10 hover:border-lapis transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite
                   disabled:opacity-50 disabled:cursor-not-allowed"
            aria-busy={signingModeLoading}
          >
            {#if signingModeLoading}
              <span class="flex items-center gap-1.5">
                <span class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin" aria-hidden="true"></span>
                Switching...
              </span>
            {:else}
              Switch to Local Signing
            {/if}
          </button>
        {/if}
      </div>

      <!-- Conformant card -->
      <div
        class="relative flex flex-col rounded-lg border-2 p-5 transition-colors
               {signingMode === 'conformant'
                 ? 'border-lapis bg-lapis/5 dark:bg-lapis/5'
                 : 'border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40'}"
        aria-current={signingMode === 'conformant' ? 'true' : undefined}
      >
        {#if signingMode === 'conformant'}
          <span
            class="absolute top-3 right-3 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-lapis/15 text-lapis dark:text-lapis-light border border-lapis/30"
            aria-label="Currently active"
          >
            Active
          </span>
        {/if}

        <div class="mb-3">
          <p class="text-sm font-semibold text-text-light dark:text-quartz">Conformant Signing</p>
          <p class="text-xs text-flint-dark dark:text-flint-light mt-0.5">Trust-list certificate (optional)</p>
        </div>

        <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mb-3">
          Uses an institution-supplied certificate from a C2PA-approved certificate authority.
          Manifests signed with this certificate validate cleanly in any conformant C2PA tool,
          including Adobe Inspect and enterprise procurement gates. Requires an annual certificate
          from a C2PA-approved CA (typically £200–£1,500).
        </p>

        {#if conformantCert === null}
          <!-- No cert — offer import -->
          <button
            onclick={() => { showCertImport = true; offerModeSwitch = false; }}
            class="mt-auto self-start px-4 py-2.5 min-h-[44px] text-sm font-medium rounded bg-lapis text-white
                   hover:bg-lapis-dark dark:hover:bg-lapis-light transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
          >
            Import certificate
          </button>
        {:else}
          <!-- Cert present — show summary -->
          <div class="mt-auto pt-3 border-t border-border-light dark:border-border-dark space-y-2">

            <!-- Validity warning -->
            {#if !conformantCert.isCurrentlyValid}
              <p
                class="text-xs px-3 py-2 rounded border border-cinnabar/30 bg-cinnabar/5 text-cinnabar-dark dark:text-cinnabar-light"
                role="alert"
              >
                This certificate is expired or not yet valid. Signing with it will fail.
              </p>
            {/if}

            <p class="text-sm font-medium text-text-light dark:text-quartz truncate" title={conformantCert.subjectCn}>
              {conformantCert.subjectCn}
            </p>
            <p class="text-xs text-flint-dark dark:text-flint-light">
              {formatAbsoluteDate(conformantCert.notBefore)} — {formatAbsoluteDate(conformantCert.notAfter)}
              <span
                class="ml-1 {conformantCert.isCurrentlyValid ? 'text-malachite-dark dark:text-malachite-light' : 'text-cinnabar-dark dark:text-cinnabar-light'}"
              >
                ({humaniseDistance(conformantCert.notAfter)})
              </span>
            </p>
            <p class="text-xs text-flint-dark dark:text-flint-light">Imported {humaniseAgo(conformantCert.importedAt)}</p>

            <!-- Action row -->
            <div class="flex flex-wrap items-center gap-2 pt-1">
              {#if signingMode !== 'conformant'}
                <button
                  onclick={handleSwitchToConformant}
                  disabled={modeSwitchLoading || !conformantCert.isCurrentlyValid}
                  class="px-3 py-1.5 text-xs font-medium rounded bg-lapis text-white hover:bg-lapis-dark dark:hover:bg-lapis-light
                         transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                         disabled:opacity-50 disabled:cursor-not-allowed"
                  aria-busy={modeSwitchLoading}
                >
                  {#if modeSwitchLoading}
                    <span class="flex items-center gap-1">
                      <span class="w-2.5 h-2.5 border-2 border-white border-t-transparent rounded-full motion-safe:animate-spin" aria-hidden="true"></span>
                      Activating...
                    </span>
                  {:else}
                    Activate
                  {/if}
                </button>
              {/if}
              <button
                onclick={() => { showCertImport = true; importError = null; }}
                class="px-3 py-1.5 text-xs font-medium rounded border border-lapis/50 text-lapis dark:text-lapis-light
                       hover:bg-lapis/10 hover:border-lapis transition-colors
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis"
              >
                Replace
              </button>
              {#if !pendingClearCert}
                <button
                  onclick={handleRequestClearCert}
                  class="px-3 py-1.5 text-xs font-medium rounded border border-cinnabar/30 text-cinnabar-dark dark:text-cinnabar-light
                         hover:bg-cinnabar/10 hover:border-cinnabar/60 transition-colors
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar"
                >
                  Remove
                </button>
              {:else}
                <span class="flex items-center gap-2 text-xs text-cinnabar-dark dark:text-cinnabar-light">
                  Remove certificate?
                  <button
                    onclick={handleConfirmClearCert}
                    disabled={clearCertLoading}
                    class="font-medium underline underline-offset-2 disabled:opacity-50
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar rounded"
                    aria-busy={clearCertLoading}
                  >
                    {clearCertLoading ? 'Removing...' : 'Confirm'}
                  </button>
                  <button
                    onclick={handleCancelClearCert}
                    class="text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                  >
                    Cancel
                  </button>
                </span>
              {/if}
            </div>

            {#if modeSwitchError !== null}
              <p class="text-xs text-cinnabar-dark dark:text-cinnabar-light" role="alert" aria-live="assertive">
                {modeSwitchError}
              </p>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <!-- Post-import mode-switch offer -->
    {#if offerModeSwitch}
      <div
        class="mb-4 p-4 rounded-lg border border-lapis/30 bg-lapis/5"
        role="status"
        aria-live="polite"
      >
        <p class="text-sm text-text-light dark:text-quartz mb-3">
          Certificate imported. Switch active signing mode to Conformant now?
        </p>
        <div class="flex items-center gap-3">
          <button
            onclick={handleSwitchToConformant}
            disabled={modeSwitchLoading}
            class="px-4 py-2 min-h-[44px] text-sm font-medium rounded bg-lapis text-white hover:bg-lapis-dark dark:hover:bg-lapis-light
                   transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite
                   disabled:opacity-50 disabled:cursor-not-allowed"
            aria-busy={modeSwitchLoading}
          >
            {modeSwitchLoading ? 'Switching...' : 'Yes, switch now'}
          </button>
          <button
            onclick={() => { offerModeSwitch = false; }}
            class="px-4 py-2 min-h-[44px] text-sm font-medium rounded border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light
                   hover:text-text-light dark:hover:text-quartz hover:border-lapis/50 transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
          >
            Not now
          </button>
        </div>
        {#if modeSwitchError !== null}
          <p class="mt-2 text-xs text-cinnabar-dark dark:text-cinnabar-light" role="alert" aria-live="assertive">
            {modeSwitchError}
          </p>
        {/if}
      </div>
    {/if}

    <!-- Cert import form -->
    {#if showCertImport}
      <div
        class="p-5 rounded-lg border border-lapis/30 bg-gray-50 dark:bg-obsidian/40"
        role="region"
        aria-label="Import conformant certificate"
      >
        <h3 class="text-sm font-medium text-text-light dark:text-quartz mb-4">Import certificate</h3>

        <!-- Certificate chain picker -->
        <div class="mb-4">
          <label class="block text-sm font-medium text-text-light dark:text-quartz mb-1" for="cert-path-display">
            Certificate chain (PEM)
          </label>
          <div class="flex items-center gap-2">
            <input
              id="cert-path-display"
              type="text"
              readonly
              value={certPath || 'No file selected'}
              aria-label="Selected certificate chain path"
              class="flex-1 min-w-0 px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian
                     text-text-light dark:text-quartz text-sm font-mono truncate cursor-not-allowed opacity-80"
            />
            <button
              type="button"
              onclick={handlePickCertFile}
              class="shrink-0 px-4 py-2 min-h-[44px] text-sm font-medium rounded border border-lapis/60 text-lapis dark:text-lapis-light
                     hover:bg-lapis/10 hover:border-lapis transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                     focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
              aria-label="Browse for certificate chain file"
            >
              Browse…
            </button>
          </div>
          <p class="text-xs text-flint-dark dark:text-flint-light mt-1">Accepted formats: .pem, .crt, .cer</p>
        </div>

        <!-- Private key picker -->
        <div class="mb-4">
          <label class="block text-sm font-medium text-text-light dark:text-quartz mb-1" for="key-path-display">
            Private key (PEM)
          </label>
          <div class="flex items-center gap-2">
            <input
              id="key-path-display"
              type="text"
              readonly
              value={keyPath || 'No file selected'}
              aria-label="Selected private key path"
              class="flex-1 min-w-0 px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian
                     text-text-light dark:text-quartz text-sm font-mono truncate cursor-not-allowed opacity-80"
            />
            <button
              type="button"
              onclick={handlePickKeyFile}
              class="shrink-0 px-4 py-2 min-h-[44px] text-sm font-medium rounded border border-lapis/60 text-lapis dark:text-lapis-light
                     hover:bg-lapis/10 hover:border-lapis transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                     focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
              aria-label="Browse for private key file"
            >
              Browse…
            </button>
          </div>
          <p class="text-xs text-flint-dark dark:text-flint-light mt-1">Accepted formats: .pem, .key</p>
        </div>

        <!-- Help text -->
        <div class="mb-4 p-3 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian/30 text-xs text-flint-dark dark:text-flint-light leading-relaxed space-y-1.5">
          <p>
            The <strong class="text-text-light dark:text-quartz">certificate chain</strong> is a PEM file containing your end-entity certificate followed by any intermediate CA certificates. Your institution's IT security team or the CA that issued the certificate will have provided this file.
          </p>
          <p>
            The <strong class="text-text-light dark:text-quartz">private key</strong> is the PEM file generated alongside the certificate signing request (CSR). It never leaves this device — Jura Trace stores it in the application data directory with restricted permissions.
          </p>
          <p>
            Certificates must be issued by a C2PA-approved certificate authority and carry the correct key usage and extended key usage extensions.
            <a
              href="/help/bedrock-signing"
              class="text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline underline-offset-2
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
            >
              Learn more about signing modes
            </a>.
          </p>
        </div>

        <!-- Import error -->
        {#if importError !== null}
          <div
            class="mb-4 px-3 py-2 rounded border border-cinnabar/30 bg-cinnabar/5"
            role="alert"
            aria-live="assertive"
          >
            <p class="text-xs text-cinnabar-dark dark:text-cinnabar-light">{importError}</p>
          </div>
        {/if}

        <!-- Action row -->
        <div class="flex items-center gap-3">
          <button
            type="button"
            onclick={handleImportCert}
            disabled={!canImport || importLoading}
            class="px-5 py-2.5 min-h-[44px] text-sm font-medium rounded bg-lapis text-white
                   hover:bg-lapis-dark dark:hover:bg-lapis-light transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                   disabled:opacity-50 disabled:cursor-not-allowed"
            aria-busy={importLoading}
          >
            {#if importLoading}
              <span class="flex items-center gap-1.5">
                <span class="w-3 h-3 border-2 border-white border-t-transparent rounded-full motion-safe:animate-spin" aria-hidden="true"></span>
                Importing...
              </span>
            {:else}
              Import
            {/if}
          </button>
          <button
            type="button"
            onclick={handleCancelImport}
            disabled={importLoading}
            class="px-4 py-2.5 min-h-[44px] text-sm font-medium rounded border border-border-light dark:border-border-dark
                   text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz hover:border-lapis/50
                   transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          >
            Cancel
          </button>
        </div>
      </div>
    {/if}

    <!-- Cert detail panel (when cert present and import form closed) -->
    {#if conformantCert !== null && !showCertImport}
      <details class="group mt-4">
        <summary
          class="list-none flex items-center gap-2 cursor-pointer text-xs text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          aria-label="Certificate details"
        >
          <svg
            class="w-3.5 h-3.5 motion-safe:group-open:rotate-90 transition-transform duration-200"
            viewBox="0 0 16 16"
            fill="currentColor"
            aria-hidden="true"
          >
            <path d="M6 3.5L11 8l-5 4.5V3.5z"/>
          </svg>
          Certificate details
        </summary>

        <div class="mt-3 p-4 rounded-lg border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40 space-y-3">

          <!-- Fingerprint -->
          <div>
            <p class="text-xs font-medium text-text-light dark:text-quartz mb-1">SHA-256 fingerprint</p>
            <div class="flex items-center gap-2">
              <code
                class="flex-1 min-w-0 px-2 py-1.5 rounded bg-white dark:bg-obsidian border border-border-light dark:border-border-dark
                       text-[11px] font-mono text-text-light dark:text-quartz break-all select-all"
                title="Full fingerprint — click to select all"
              >
                {conformantCert.fingerprintSha256.slice(0, 48)}…
              </code>
              <button
                onclick={handleCopyFingerprint}
                class="shrink-0 px-2.5 py-1.5 text-xs font-medium rounded border border-lapis/50 text-lapis dark:text-lapis-light
                       hover:bg-lapis/10 transition-colors
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis"
                aria-label="Copy full SHA-256 fingerprint"
              >
                {fingerprintCopied ? 'Copied' : 'Copy'}
              </button>
            </div>
          </div>

          <!-- Issuer CN. The Rust side extracts this via x509-parser from
               the tbsCertificate.issuer field. Hidden when the extraction
               fails and returns an empty string, or when an older install
               returns the legacy "(see certificate chain)" placeholder. -->
          {#if conformantCert.issuerCn && !conformantCert.issuerCn.startsWith('(see')}
            <div>
              <p class="text-xs font-medium text-text-light dark:text-quartz mb-0.5">Issuer</p>
              <p class="text-xs text-flint-dark dark:text-flint-light">{conformantCert.issuerCn}</p>
            </div>
          {/if}

          <!-- Algorithm -->
          <div>
            <p class="text-xs font-medium text-text-light dark:text-quartz mb-0.5">Algorithm</p>
            <p class="text-xs text-flint-dark dark:text-flint-light font-mono">{conformantCert.signingAlgorithm}</p>
          </div>

          <!-- Key usage chips -->
          {#if conformantCert.keyUsage.length > 0}
            <div>
              <p class="text-xs font-medium text-text-light dark:text-quartz mb-1.5">Key usage</p>
              <div class="flex flex-wrap gap-1.5" aria-label="Key usage flags">
                {#each conformantCert.keyUsage as usage}
                  <span class="px-2 py-0.5 rounded text-[11px] bg-gray-100 dark:bg-graphite-light text-flint-dark dark:text-flint-light border border-border-light dark:border-graphite-light">
                    {usage}
                  </span>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Extended key usage chips -->
          {#if conformantCert.extendedKeyUsage.length > 0}
            <div>
              <p class="text-xs font-medium text-text-light dark:text-quartz mb-1.5">Extended key usage</p>
              <div class="flex flex-wrap gap-1.5" aria-label="Extended key usage flags">
                {#each conformantCert.extendedKeyUsage as eku}
                  <span class="px-2 py-0.5 rounded text-[11px] bg-gray-100 dark:bg-graphite-light text-flint-dark dark:text-flint-light border border-border-light dark:border-graphite-light">
                    {eku}
                  </span>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </details>
    {/if}
  </section>

  <!-- API Key Management -->
  <section
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="api-keys-heading"
  >
    <div class="flex items-center gap-1.5 mb-1">
      <h2 id="api-keys-heading" class="text-lg font-heading text-text-light dark:text-quartz">API Keys</h2>
      <ContextualHelpLink href="/help/settings#api-keys" label="Learn about API key management" />
    </div>
    <p class="text-xs text-flint-dark dark:text-flint-light mb-4">
      Manage authentication keys for the local REST API on port 8300. Keys allow external tools (CI pipelines, n8n workflows, custom scripts) to call the Jura Trace verification engine programmatically.
    </p>

    {#if !apiKeysAvailable}
      <div class="p-4 rounded-lg border border-lapis/20 bg-lapis/5">
        <p class="text-sm text-flint-dark dark:text-flint-light">
          API access is available on <strong class="text-text-light dark:text-quartz">Professional</strong> and <strong class="text-text-light dark:text-quartz">Enterprise</strong> plans. Upgrade your plan above to manage API keys.
        </p>
      </div>
    {:else}
      <!-- Newly created key banner (shown once, dismissed by user) -->
      {#if newlyCreatedKey}
        <div
          class="mb-4 p-4 rounded-lg border border-malachite/30 bg-malachite/5"
          role="alert"
          aria-live="polite"
        >
          <p class="text-sm font-medium text-malachite-dark dark:text-malachite-light mb-2">
            API key created — copy it now. It will not be shown again.
          </p>
          <div class="flex items-center gap-2 mb-3">
            <code class="flex-1 px-3 py-2 rounded bg-white dark:bg-obsidian border border-border-light dark:border-border-dark text-xs font-mono text-text-light dark:text-quartz break-all select-all">
              {newlyCreatedKey.key}
            </code>
            <button
              onclick={handleCopyKey}
              class="shrink-0 px-3 py-2 text-xs font-medium rounded border border-lapis/50 text-lapis dark:text-lapis-light hover:bg-lapis/10 transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis"
              aria-label="Copy API key to clipboard"
            >
              Copy
            </button>
          </div>
          <div class="flex items-center justify-between text-xs text-flint-dark dark:text-flint-light">
            <span>Name: <strong>{newlyCreatedKey.name}</strong> &middot; Rate limit: {newlyCreatedKey.rateLimit} requests per minute</span>
            <button
              onclick={handleDismissNewKey}
              class="text-xs text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
            >
              Dismiss
            </button>
          </div>
        </div>
      {/if}

      <!-- Create new key form -->
      <div class="mb-5 p-4 rounded-lg border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40">
        <h3 class="text-sm font-medium text-text-light dark:text-quartz mb-3">Create new key</h3>
        <div class="flex flex-col sm:flex-row gap-3">
          <div class="flex-1">
            <label for="api-key-name" class="sr-only">Key name</label>
            <input
              id="api-key-name"
              type="text"
              bind:value={newKeyName}
              placeholder="Key name (e.g. CI pipeline)"
              maxlength="100"
              class="w-full px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian text-text-light dark:text-quartz text-sm
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent transition-colors"
            />
          </div>
          <div class="w-28">
            <label for="api-key-rate" class="sr-only">Rate limit per minute</label>
            <input
              id="api-key-rate"
              type="number"
              bind:value={newKeyRateLimit}
              min="1"
              max="10000"
              class="w-full px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian text-text-light dark:text-quartz text-sm
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent transition-colors"
              aria-label="Rate limit per minute"
              title="Requests per minute"
            />
          </div>
          <button
            onclick={handleCreateKey}
            disabled={creatingKey || !newKeyName.trim()}
            class="shrink-0 px-4 py-2 min-h-[44px] text-sm font-medium rounded bg-lapis text-white hover:bg-lapis-dark dark:hover:bg-lapis-light transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                   disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {#if creatingKey}
              <span class="inline-flex items-center gap-1.5">
                <span class="w-3 h-3 border-2 border-white border-t-transparent rounded-full motion-safe:animate-spin" aria-hidden="true"></span>
                Creating...
              </span>
            {:else}
              Create Key
            {/if}
          </button>
        </div>
        <p class="mt-2 text-xs text-flint-dark dark:text-flint-light">
          Rate limit: {newKeyRateLimit} requests/min. Keys use <code class="text-xs">Authorization: Bearer jt_...</code> header format.
        </p>
      </div>

      <!-- Feedback banner -->
      {#if apiKeyFeedback}
        <p
          class="mb-4 text-sm px-3 py-2 rounded border
                 {apiKeyFeedback.ok
                   ? 'text-malachite-dark dark:text-malachite-light border-malachite/20 bg-malachite/5'
                   : 'text-cinnabar-dark dark:text-cinnabar-light border-cinnabar/20 bg-cinnabar/5'}"
          role="status"
          aria-live="polite"
        >
          {apiKeyFeedback.message}
        </p>
      {/if}

      <!-- Key list -->
      {#if apiKeysLoading}
        <div class="flex items-center gap-2 text-sm text-flint-dark dark:text-flint-light py-4">
          <span class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin" aria-hidden="true"></span>
          Loading keys...
        </div>
      {:else if apiKeys.length === 0}
        <p class="text-sm text-flint-dark dark:text-flint-light py-4">
          No API keys created yet. Create one above to get started.
        </p>
      {:else}
        <div class="overflow-x-auto">
          <table class="w-full text-sm" aria-label="API keys">
            <thead>
              <tr class="border-b border-border-light dark:border-border-dark text-left">
                <th class="py-2 pr-4 font-medium text-flint-dark dark:text-flint-light">Name</th>
                <th class="py-2 pr-4 font-medium text-flint-dark dark:text-flint-light">Key ID</th>
                <th class="py-2 pr-4 font-medium text-flint-dark dark:text-flint-light">Rate Limit</th>
                <th class="py-2 pr-4 font-medium text-flint-dark dark:text-flint-light">Status</th>
                <th class="py-2 pr-4 font-medium text-flint-dark dark:text-flint-light">Created</th>
                <th class="py-2 font-medium text-flint-dark dark:text-flint-light"><span class="sr-only">Actions</span></th>
              </tr>
            </thead>
            <tbody>
              {#each apiKeys as key (key.keyId)}
                <tr class="border-b border-border-light/50 dark:border-border-dark/50 {key.revoked ? 'opacity-50' : ''}">
                  <td class="py-2.5 pr-4 text-text-light dark:text-quartz">{key.name}</td>
                  <td class="py-2.5 pr-4">
                    <code class="text-xs text-flint-dark dark:text-flint-light">{key.keyId.slice(0, 8)}...</code>
                  </td>
                  <td class="py-2.5 pr-4 text-flint-dark dark:text-flint-light">{key.rateLimit} requests per minute</td>
                  <td class="py-2.5 pr-4">
                    {#if key.revoked}
                      <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-cinnabar/10 text-cinnabar-dark dark:text-cinnabar-light border border-cinnabar/20">
                        Revoked
                      </span>
                    {:else}
                      <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-malachite/10 text-malachite-dark dark:text-malachite-light border border-malachite/20">
                        Active
                      </span>
                    {/if}
                  </td>
                  <td class="py-2.5 pr-4 text-xs text-flint-dark dark:text-flint-light">
                    {new Date(key.createdAt).toLocaleDateString()}
                  </td>
                  <td class="py-2.5 text-right">
                    {#if !key.revoked}
                      {#if pendingRevokeId === key.keyId}
                        <span class="inline-flex items-center gap-2">
                          <button
                            onclick={handleConfirmRevoke}
                            class="text-xs font-medium text-cinnabar-dark hover:text-cinnabar-dark dark:text-cinnabar-light transition-colors
                                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar rounded"
                          >
                            Confirm revoke
                          </button>
                          <button
                            onclick={handleCancelRevoke}
                            class="text-xs text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                          >
                            Cancel
                          </button>
                        </span>
                      {:else}
                        <button
                          onclick={() => handleRequestRevoke(key.keyId)}
                          class="text-xs font-medium text-cinnabar-dark/70 hover:text-cinnabar-dark dark:text-cinnabar-light/70 dark:hover:text-cinnabar-light transition-colors
                                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar rounded"
                        >
                          Revoke
                        </button>
                      {/if}
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <p class="mt-3 text-xs text-flint-dark dark:text-flint-light">
          {activeKeyCount} active key{activeKeyCount === 1 ? '' : 's'} &middot; {apiKeys.length} total
        </p>
      {/if}
    {/if}
  </section>

  <!-- Network Access -->
  <section
    id="network-access"
    class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-6"
    aria-labelledby="network-access-heading"
  >
    <h2 id="network-access-heading" class="text-lg font-heading text-text-light dark:text-quartz mb-1">Network Access</h2>
    <p class="text-xs text-flint-dark dark:text-flint-light mb-5">
      Controls whether Jura Trace makes outbound network connections. Standard mode is fully local with no external calls. Enhanced mode enables online certificate verification and remote Content Credentials retrieval.
    </p>

    <!-- Mode cards -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4" role="group" aria-label="Network access mode selection">

      <!-- Standard card -->
      <div
        class="relative flex flex-col rounded-lg border-2 p-5 transition-colors
               {networkMode === 'standard'
                 ? 'border-lapis bg-lapis/5 dark:bg-lapis/5'
                 : 'border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40'}"
        aria-current={networkMode === 'standard' ? 'true' : undefined}
      >
        {#if networkMode === 'standard'}
          <span
            class="absolute top-3 right-3 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-lapis/15 text-lapis dark:text-lapis-light border border-lapis/30"
            aria-label="Currently active"
          >
            Active
          </span>
        {/if}

        <h3 class="text-sm font-semibold text-text-light dark:text-quartz mb-1 pr-14">Standard</h3>
        <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mb-4">
          Fully local — no outbound network connections. Certificate revocation checks are skipped. Recommended for air-gapped environments.
        </p>

        {#if networkMode !== 'standard'}
          <button
            type="button"
            onclick={() => handleNetworkModeChange('standard')}
            disabled={networkModeChanging}
            class="mt-auto self-start px-4 py-2 min-h-[44px] text-sm font-medium rounded border border-lapis/60 text-lapis dark:text-lapis-light
                   hover:bg-lapis/10 hover:border-lapis transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite
                   disabled:opacity-50 disabled:cursor-not-allowed"
            aria-busy={networkModeChanging}
          >
            {#if networkModeChanging}
              <span class="flex items-center gap-1.5">
                <span class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin" aria-hidden="true"></span>
                Switching...
              </span>
            {:else}
              Switch to Standard
            {/if}
          </button>
        {/if}
      </div>

      <!-- Enhanced card -->
      <div
        class="relative flex flex-col rounded-lg border-2 p-5 transition-colors
               {networkMode === 'enhanced'
                 ? 'border-amber bg-amber/5 dark:bg-amber/5'
                 : 'border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40'}"
        aria-current={networkMode === 'enhanced' ? 'true' : undefined}
      >
        {#if networkMode === 'enhanced'}
          <span
            class="absolute top-3 right-3 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-amber/15 text-amber-dark dark:text-amber-light border border-amber/30"
            aria-label="Currently active"
          >
            Active
          </span>
        {/if}

        <h3 class="text-sm font-semibold text-text-light dark:text-quartz mb-1 pr-14">Enhanced</h3>
        <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mb-4">
          Enables online verification features including certificate revocation checks (OCSP/CRL) and remote Content Credentials retrieval. Network calls are made only during verification.
        </p>

        {#if networkMode !== 'enhanced'}
          <button
            type="button"
            onclick={() => handleNetworkModeChange('enhanced')}
            disabled={networkModeChanging}
            class="mt-auto self-start px-4 py-2 min-h-[44px] text-sm font-medium rounded bg-lapis text-white
                   hover:bg-lapis-dark transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite
                   disabled:opacity-50 disabled:cursor-not-allowed"
            aria-busy={networkModeChanging}
          >
            {#if networkModeChanging}
              <span class="flex items-center gap-1.5">
                <span class="w-3 h-3 border-2 border-amber border-t-transparent rounded-full motion-safe:animate-spin" aria-hidden="true"></span>
                Switching...
              </span>
            {:else}
              Switch to Enhanced
            {/if}
          </button>
        {/if}
      </div>
    </div>

    <!-- Current mode note -->
    <p
      class="text-xs px-3 py-2 rounded border inline-flex items-center gap-1.5
             {networkMode === 'standard'
               ? 'text-lapis dark:text-lapis-light border-lapis/20 bg-lapis/5'
               : 'text-amber-dark dark:text-amber-light border-amber/20 bg-amber/5'}"
      aria-live="polite"
    >
      <span class="font-medium">Current mode:</span>
      {networkMode === 'standard' ? 'Standard' : 'Enhanced'}
    </p>

    {#if networkModeFeedback !== null}
      <p
        class="mt-3 text-sm px-3 py-2 rounded border
               {networkModeFeedback.ok
                 ? 'text-malachite-dark dark:text-malachite-light border-malachite/20 bg-malachite/5'
                 : 'text-cinnabar-dark dark:text-cinnabar-light border-cinnabar/20 bg-cinnabar/5'}"
        role="status"
        aria-live="polite"
      >
        {networkModeFeedback.message}
      </p>
    {/if}
  </section>
</div>
