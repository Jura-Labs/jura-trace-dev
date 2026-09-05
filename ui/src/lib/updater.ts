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
