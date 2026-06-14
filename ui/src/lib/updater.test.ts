// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Unit tests for the auto-updater state machine (updater.ts).
 *
 * All external deps (Tauri plugin, autoBackupBeforeUpdate, isTauri) are
 * injected via the UpdaterDeps interface so no Tauri runtime is required.
 *
 * Run with: cd ui && npm test
 */
import { describe, expect, it, vi } from 'vitest';
import { checkForUpdate, classifyError, type UpdateStatus, type UpdaterDeps } from './updater';

// Minimal fake update object returned by the plugin.
function makeFakeUpdate(
  version = '0.9.1',
  installBehaviour: 'success' | 'throw' = 'success',
) {
  return {
    version,
    downloadAndInstall: vi.fn(
      async (cb: (event: { event: 'Started' | 'Progress' | 'Finished' }) => void) => {
        cb({ event: 'Started' });
        cb({ event: 'Progress' });
        cb({ event: 'Finished' });
        if (installBehaviour === 'throw') {
          throw new Error('Install failed unexpectedly');
        }
      },
    ),
  };
}

// Build a deps object with sensible defaults that each test can override.
function makeDeps(overrides: Partial<UpdaterDeps> = {}): UpdaterDeps {
  return {
    isTauri: () => true,
    checkPlugin: vi.fn(async () => null),
    backup: vi.fn(async () => ({ snapshotPath: '/tmp/backup.db', sha256: 'abc123def456' })),
    ...overrides,
  };
}

// Collect all status transitions emitted during a run.
async function run(deps: UpdaterDeps): Promise<UpdateStatus[]> {
  const transitions: UpdateStatus[] = [];
  await checkForUpdate((s) => transitions.push(s), deps);
  return transitions;
}

describe('checkForUpdate — browser fallback', () => {
  it('sets error when not running inside Tauri', async () => {
    const deps = makeDeps({ isTauri: () => false });
    const transitions = await run(deps);

    expect(transitions).toHaveLength(1);
    expect(transitions[0]).toEqual({
      state: 'error',
      message: 'Update checks are only available in the desktop application.',
    });
  });

  it('does not call checkPlugin in browser context', async () => {
    const deps = makeDeps({ isTauri: () => false });
    await run(deps);
    expect(deps.checkPlugin).not.toHaveBeenCalled();
  });
});

describe('checkForUpdate — up to date', () => {
  it('transitions checking -> up-to-date when plugin returns null', async () => {
    const deps = makeDeps({ checkPlugin: vi.fn(async () => null) });
    const transitions = await run(deps);

    expect(transitions[0]).toEqual({ state: 'checking' });
    expect(transitions[transitions.length - 1]).toEqual({ state: 'up-to-date' });
  });

  it('maps "No updates available" error to up-to-date', async () => {
    const deps = makeDeps({
      checkPlugin: vi.fn(async () => {
        throw new Error('No updates available from endpoint');
      }),
    });
    const transitions = await run(deps);

    expect(transitions[transitions.length - 1]).toEqual({ state: 'up-to-date' });
  });

  it('maps "404" in error message to up-to-date', async () => {
    const deps = makeDeps({
      checkPlugin: vi.fn(async () => {
        throw new Error('Request failed with status 404');
      }),
    });
    const transitions = await run(deps);

    expect(transitions[transitions.length - 1]).toEqual({ state: 'up-to-date' });
  });
});

describe('checkForUpdate — successful install', () => {
  it('emits the full state sequence when an update is available', async () => {
    const update = makeFakeUpdate('0.9.1');
    const deps = makeDeps({ checkPlugin: vi.fn(async () => update) });
    const transitions = await run(deps);

    const states = transitions.map((t) => t.state);
    expect(states).toContain('checking');
    expect(states).toContain('available');
    expect(states).toContain('downloading');
    expect(states).toContain('installing');
  });

  it('reports the correct version in the available state', async () => {
    const update = makeFakeUpdate('1.2.3');
    const deps = makeDeps({ checkPlugin: vi.fn(async () => update) });
    const transitions = await run(deps);

    const availableState = transitions.find((t) => t.state === 'available');
    expect(availableState).toBeDefined();
    expect(availableState).toMatchObject({ state: 'available', version: '1.2.3' });
  });

  it('ends in installing after downloadAndInstall resolves', async () => {
    const update = makeFakeUpdate('0.9.1');
    const deps = makeDeps({ checkPlugin: vi.fn(async () => update) });
    const transitions = await run(deps);

    expect(transitions[transitions.length - 1]).toEqual({ state: 'installing' });
  });
});

describe('checkForUpdate — backup failure is non-blocking', () => {
  it('still reaches installing when backup throws', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const update = makeFakeUpdate('0.9.1');
    const deps = makeDeps({
      checkPlugin: vi.fn(async () => update),
      backup: vi.fn(async () => {
        throw new Error('Disk full');
      }),
    });

    const transitions = await run(deps);

    expect(transitions[transitions.length - 1]).toEqual({ state: 'installing' });
    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining('[updater] pre-install backup failed'),
      expect.anything(),
    );

    warnSpy.mockRestore();
  });

  it('does not surface a backup failure as an error state', async () => {
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    const update = makeFakeUpdate('0.9.1');
    const deps = makeDeps({
      checkPlugin: vi.fn(async () => update),
      backup: vi.fn(async () => {
        throw new Error('Permission denied');
      }),
    });

    const transitions = await run(deps);
    const errorStates = transitions.filter((t) => t.state === 'error');
    expect(errorStates).toHaveLength(0);

    vi.restoreAllMocks();
  });
});

describe('checkForUpdate — generic error handling', () => {
  it('surfaces unknown errors as error state with the message', async () => {
    const deps = makeDeps({
      checkPlugin: vi.fn(async () => {
        throw new Error('Network unreachable');
      }),
    });
    const transitions = await run(deps);

    expect(transitions[transitions.length - 1]).toEqual({
      state: 'error',
      message: 'Network unreachable',
    });
  });

  it('coerces non-Error throws to a string message', async () => {
    const deps = makeDeps({
      checkPlugin: vi.fn(async () => {
        // eslint-disable-next-line @typescript-eslint/only-throw-error
        throw 'something went wrong';
      }),
    });
    const transitions = await run(deps);

    const last = transitions[transitions.length - 1];
    expect(last.state).toBe('error');
    if (last.state === 'error') {
      expect(last.message).toBe('something went wrong');
    }
  });
});

describe('classifyError — channel-unavailable normalisation', () => {
  it('maps "Could not fetch a valid release JSON" to friendly error', async () => {
    const deps = makeDeps({
      checkPlugin: vi.fn(async () => {
        throw new Error('Could not fetch a valid release JSON from the remote');
      }),
    });
    const transitions = await run(deps);

    const last = transitions[transitions.length - 1];
    expect(last.state).toBe('error');
    if (last.state === 'error') {
      expect(last.message).toContain('Update channel temporarily unavailable');
      expect(last.message).toContain('juralabs.org/download.');
      expect(last.message).not.toContain('JSON');
    }
  });

  it('maps "Failed to fetch" network error to friendly error', () => {
    const result = classifyError('Failed to fetch');
    expect(result.state).toBe('error');
    if (result.state === 'error') {
      expect(result.message).toContain('Update channel temporarily unavailable');
    }
  });

  it('maps "Network request failed" to friendly error', () => {
    const result = classifyError('Network request failed: connection reset');
    expect(result.state).toBe('error');
    if (result.state === 'error') {
      expect(result.message).toContain('Update channel temporarily unavailable');
    }
  });

  it('preserves "404" and "No updates available" up-to-date mapping', () => {
    expect(classifyError('Request failed with status 404').state).toBe('up-to-date');
    expect(classifyError('No updates available from endpoint').state).toBe('up-to-date');
  });

  it('falls through to raw error for unknown messages', () => {
    const result = classifyError('Signature verification failed');
    expect(result).toEqual({ state: 'error', message: 'Signature verification failed' });
  });

  it('friendly error message contains no em-dash and no emoji', () => {
    const result = classifyError('Could not fetch a valid release JSON from the remote');
    if (result.state === 'error') {
      expect(result.message).not.toContain('—');
      expect(result.message).not.toMatch(/[^\x00-\x7F]/);
    }
  });
});

describe('checkForUpdate — copy contract', () => {
  it('browser-mode error uses exact user-facing copy', async () => {
    const deps = makeDeps({ isTauri: () => false });
    const transitions = await run(deps);

    const errorState = transitions[0];
    expect(errorState.state).toBe('error');
    if (errorState.state === 'error') {
      expect(errorState.message).toBe(
        'Update checks are only available in the desktop application.',
      );
      // No em-dash
      expect(errorState.message).not.toContain('—');
      // No emoji
      expect(errorState.message).not.toMatch(/[^\x00-\x7F]/);
    }
  });
});
