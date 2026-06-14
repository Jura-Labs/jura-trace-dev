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
 * The Tauri plugin emits raw strings that are not safe to surface verbatim:
 * "No updates available" / "404" mean the endpoint is reachable and just
 * has no newer version; "Could not fetch a valid release JSON from the
 * remote" means the endpoint returned 404 or invalid JSON (typically a
 * temporary infrastructure issue). Both branches return a friendly status
 * rather than the raw plugin string.
 */
export function classifyError(message: string): UpdateStatus {
  if (message.includes('No updates available') || message.includes('404')) {
    return { state: 'up-to-date' };
  }
  if (
    message.includes('Could not fetch a valid release JSON') ||
    message.includes('Failed to fetch') ||
    message.includes('Network request failed')
  ) {
    return {
      state: 'error',
      message:
        'Update channel temporarily unavailable. Please try again later. If the problem persists, download the latest installer from juralabs.org/download.',
    };
  }
  return { state: 'error', message };
}
