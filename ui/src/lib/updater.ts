// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Auto-updater state machine extracted from settings/+page.svelte.
 *
 * Keeping this logic in a plain TypeScript module makes it testable
 * without a DOM or a running Svelte component. The Svelte page owns
 * the reactive state variable; this module owns the async logic and
 * reports state transitions through a callback.
 */

export type UpdateStatus =
  | { state: 'idle' }
  | { state: 'checking' }
  | { state: 'available'; version: string }
  | { state: 'up-to-date' }
  | { state: 'downloading' }
  | { state: 'installing' }
  | { state: 'error'; message: string };

/**
 * Injected dependencies for checkForUpdate.
 * Defaults wire up the real Tauri plugin and api module; tests supply fakes.
 */
export interface UpdaterDeps {
  isTauri: () => boolean;
  checkPlugin: () => Promise<{
    version: string;
    downloadAndInstall: (
      cb: (event: { event: 'Started' | 'Progress' | 'Finished' }) => void,
    ) => Promise<void>;
  } | null>;
  backup: () => Promise<{ snapshotPath: string; sha256: string }>;
}

/**
 * Real Tauri-backed deps — imported lazily inside the function so the
 * plugin never loads in a browser bundle.
 */
export async function makeTauriDeps(): Promise<UpdaterDeps> {
  const { check } = await import('@tauri-apps/plugin-updater');
  const { autoBackupBeforeUpdate } = await import('$lib/api');
  return {
    isTauri: () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window,
    checkPlugin: check,
    backup: autoBackupBeforeUpdate,
  };
}

/**
 * Run the update state machine, reporting each transition via onStatus.
 *
 * The caller (the Svelte component) binds onStatus to a $state setter so
 * every call here triggers a reactive re-render automatically.
 */
export async function checkForUpdate(
  onStatus: (s: UpdateStatus) => void,
  deps: UpdaterDeps,
): Promise<void> {
  if (!deps.isTauri()) {
    onStatus({
      state: 'error',
      message: 'Update checks are only available in the desktop application.',
    });
    return;
  }

  onStatus({ state: 'checking' });

  try {
    const update = await deps.checkPlugin();

    if (!update) {
      onStatus({ state: 'up-to-date' });
      return;
    }

    onStatus({ state: 'available', version: update.version });

    try {
      const result = await deps.backup();
      console.info(
        `[updater] pre-install backup at ${result.snapshotPath} (${result.sha256.slice(0, 12)}...)`,
      );
    } catch (backupErr) {
      console.warn('[updater] pre-install backup failed; continuing with update:', backupErr);
    }

    onStatus({ state: 'downloading' });

    await update.downloadAndInstall((event) => {
      if (event.event === 'Started' || event.event === 'Progress') {
        onStatus({ state: 'downloading' });
      } else if (event.event === 'Finished') {
        onStatus({ state: 'installing' });
      }
    });

    onStatus({ state: 'installing' });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    onStatus(classifyError(message));
  }
}

/**
 * Map raw plugin-updater errors to user-facing UpdateStatus values.
 *
 * The Tauri plugin emits raw strings that are not safe to surface verbatim,
 * so they are mapped to friendly statuses here.
 *
 * A 404 is NOT "up to date". This function used to treat it as one, and the
 * previous version of this comment asserted that a 404 "means the endpoint
 * is reachable and just has no newer version" — while two lines further on
 * describing a 404 as an infrastructure problem. Both cannot be true, and
 * the second one is right.
 *
 * With the static-JSON updater, "no update available" is decided by
 * comparing versions in a manifest that was successfully fetched. A 404
 * means the manifest was not fetched at all. Reporting that as up-to-date
 * tells the user the opposite of what happened.
 *
 * This is not hypothetical. The declared fallback endpoint returns 404
 * today, and the update manifest was broken from 18 June to 4 September
 * 2026. Anyone who pressed Check for Updates in that window was shown a
 * green "up to date" by this function.
 */
// ── Automatic startup check (JTV / v1.1.0 Tier 1 item T1.2) ────────────────
//
// Until now the only way to discover an update was a menu item that opened
// Settings. Cloudflare shows zero requests to /api/updates in 24 hours, and
// the macOS updater archive has zero downloads in the 79 days since release,
// so in practice nobody has ever checked. Fixing the manifest (BL-REL-002)
// therefore reaches nobody on its own: something has to ask.
//
// Three properties this deliberately has, because an automatic outbound
// call in a local-first product is not a free action:
//
//   1. It NEVER installs. `checkForUpdate` above downloads and installs as
//      soon as it finds a version, which is defensible when a person just
//      pressed a button and is watching. It is not defensible unprompted at
//      launch, where it would spend an 800 MB download and restart the app
//      without anyone asking for it. This path only ever reports.
//   2. It respects Standard (offline) mode. A user who chose fully local
//      operation must not get silent outbound traffic at startup.
//   3. It is throttled and quiet. At most one check per interval, and a
//      failure is reported to the caller but never nagged at the user.

/** How often the startup check may run. */
export const STARTUP_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;

/** Key under which the last successful check timestamp is stored. */
export const LAST_CHECK_STORAGE_KEY = 'jura.updater.lastCheckMs';

export type StartupCheckOutcome =
  | { kind: 'skipped'; reason: 'not-desktop' | 'offline-mode' | 'throttled' }
  | { kind: 'up-to-date' }
  | { kind: 'available'; version: string }
  | { kind: 'failed'; message: string };

/**
 * Dependencies for the startup check.
 *
 * Deliberately does NOT include `backup` or anything that can install: the
 * type makes the "never installs" property structural rather than a promise
 * in a comment. There is no way to install through this interface.
 */
export interface StartupCheckDeps {
  isTauri: () => boolean;
  getNetworkMode: () => Promise<'standard' | 'enhanced'>;
  checkPlugin: () => Promise<{ version: string } | null>;
  now: () => number;
  readLastCheck: () => number | null;
  writeLastCheck: (ms: number) => void;
}

/**
 * Whether enough time has passed to check again.
 *
 * Pure, so the throttle is testable without clocks or storage. A missing or
 * unparseable previous timestamp means "never checked", which allows a
 * check — the safe direction, since the cost of an extra check is one small
 * request and the cost of wrongly suppressing is a user who never learns an
 * update exists.
 */
export function shouldCheckNow(
  nowMs: number,
  lastCheckMs: number | null,
  intervalMs: number = STARTUP_CHECK_INTERVAL_MS,
): boolean {
  if (lastCheckMs === null || !Number.isFinite(lastCheckMs)) return true;
  // A timestamp in the future means a clock change; treat it as stale rather
  // than blocking checks until the future catches up.
  if (lastCheckMs > nowMs) return true;
  return nowMs - lastCheckMs >= intervalMs;
}

/**
 * Check whether an update exists, without installing anything.
 *
 * Returns an outcome rather than driving UI state, so the caller decides
 * whether to surface it. Errors are classified with the same rules as the
 * manual path, which means a 404 is reported as a failure and never as
 * "up to date".
 *
 * On the network-mode gate: this is a frontend check, and JTV-183 recorded
 * that a frontend `networkMode === 'enhanced'` test is bypassable from the
 * browser console, which is why the weather lookup's gate was moved into
 * Rust. The distinction here is that this gate exists to honour a user's own
 * preference, not to contain a hostile caller, and the thing it withholds is
 * a version number rather than the GPS coordinates of evidence. It is a
 * preference, enforced where the call is made. Moving the updater behind a
 * Rust command would make it a boundary as well; that is worth doing if the
 * updater ever carries anything identifying, and is noted in the backlog
 * rather than done here.
 */
export async function runStartupUpdateCheck(
  deps: StartupCheckDeps,
): Promise<StartupCheckOutcome> {
  if (!deps.isTauri()) {
    return { kind: 'skipped', reason: 'not-desktop' };
  }

  if ((await deps.getNetworkMode()) !== 'enhanced') {
    return { kind: 'skipped', reason: 'offline-mode' };
  }

  const now = deps.now();
  if (!shouldCheckNow(now, deps.readLastCheck())) {
    return { kind: 'skipped', reason: 'throttled' };
  }

  try {
    const update = await deps.checkPlugin();
    // Recorded only after the request completes, so a failed check retries on
    // the next launch instead of being suppressed for a day.
    deps.writeLastCheck(now);
    if (!update) return { kind: 'up-to-date' };
    return { kind: 'available', version: update.version };
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    const classified = classifyError(message);
    if (classified.state === 'up-to-date') return { kind: 'up-to-date' };
    return {
      kind: 'failed',
      message: classified.state === 'error' ? classified.message : message,
    };
  }
}

/**
 * Real dependencies for the startup check, wired to Tauri and localStorage.
 *
 * Storage access is wrapped because it throws outright in some privacy
 * configurations, and a browser that refuses to remember the last check
 * should still get a working check rather than an exception at launch.
 */
export async function makeStartupCheckDeps(): Promise<StartupCheckDeps> {
  const { check } = await import('@tauri-apps/plugin-updater');
  const { getNetworkMode } = await import('$lib/api');
  return {
    isTauri: () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window,
    getNetworkMode,
    checkPlugin: check,
    now: () => Date.now(),
    readLastCheck: () => {
      try {
        const raw = localStorage.getItem(LAST_CHECK_STORAGE_KEY);
        if (raw === null) return null;
        const n = Number(raw);
        return Number.isFinite(n) ? n : null;
      } catch {
        return null;
      }
    },
    writeLastCheck: (ms: number) => {
      try {
        localStorage.setItem(LAST_CHECK_STORAGE_KEY, String(ms));
      } catch {
        // Storage unavailable; the check simply runs again next launch.
      }
    },
  };
}

export function classifyError(message: string): UpdateStatus {
  if (message.includes('No updates available')) {
    return { state: 'up-to-date' };
  }
  if (
    message.includes('404') ||
    message.includes('Could not fetch a valid release JSON') ||
    message.includes('Failed to fetch') ||
    message.includes('Network request failed')
  ) {
    return {
      state: 'error',
      message:
        'Could not reach the update service, so we cannot tell whether an update is available. Please try again later. If the problem persists, download the latest installer from juralabs.org/download.',
    };
  }
  return { state: 'error', message };
}
