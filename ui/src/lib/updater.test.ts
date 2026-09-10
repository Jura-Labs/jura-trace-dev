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
import {
  checkForUpdate,
  classifyError,
  makeStartupCheckDeps,
  runStartupUpdateCheck,
  shouldCheckNow,
  STARTUP_CHECK_INTERVAL_MS,
  type StartupCheckDeps,
  type UpdateStatus,
  type UpdaterDeps,
} from './updater';

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
    relaunch: vi.fn(async () => undefined),
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

  // This test previously asserted that a 404 maps to up-to-date, which
  // certified the bug rather than catching it. A 404 means the manifest was
  // never fetched, so the honest answer is that we do not know whether an
  // update exists. Telling the user they are current is the one answer that
  // is definitely wrong.
  it('reports a 404 as an error, never as up-to-date', async () => {
    const deps = makeDeps({
      checkPlugin: vi.fn(async () => {
        throw new Error('Request failed with status 404');
      }),
    });
    const transitions = await run(deps);
    const final = transitions[transitions.length - 1];

    expect(final.state).toBe('error');
    expect(final).not.toEqual({ state: 'up-to-date' });
    if (final.state === 'error') {
      expect(final.message).toMatch(/could not reach the update service/i);
    }
  });

  // The declared fallback endpoint returns 404 in production, and the
  // manifest itself was broken for eleven weeks. Both surface here.
  it('reports an unreachable update service as an error', async () => {
    for (const raw of [
      'Could not fetch a valid release JSON from the remote',
      'Failed to fetch',
      'Network request failed',
    ]) {
      const deps = makeDeps({
        checkPlugin: vi.fn(async () => {
          throw new Error(raw);
        }),
      });
      const transitions = await run(deps);
      expect(transitions[transitions.length - 1].state).toBe('error');
    }
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

// BL-REL-004: "Installing, restarting shortly..." was shown while nothing
// restarted the app. The first update ever applied to a real install
// (9 September 2026) sat on that spinner until the user quit by hand.
describe('checkForUpdate — relaunch after install', () => {
  it('calls relaunch exactly once, after downloadAndInstall has resolved', async () => {
    const order: string[] = [];
    const update = {
      version: '1.1.0',
      downloadAndInstall: vi.fn(async () => {
        order.push('install');
      }),
    };
    const relaunch = vi.fn(async () => {
      order.push('relaunch');
    });
    const deps = makeDeps({ checkPlugin: vi.fn(async () => update), relaunch });
    const transitions = await run(deps);

    expect(relaunch).toHaveBeenCalledTimes(1);
    expect(order).toEqual(['install', 'relaunch']);
    expect(transitions[transitions.length - 1]).toEqual({ state: 'installing' });
  });

  it('does not relaunch when the install throws', async () => {
    const update = makeFakeUpdate('0.9.1', 'throw');
    const relaunch = vi.fn(async () => undefined);
    const deps = makeDeps({ checkPlugin: vi.fn(async () => update), relaunch });
    await run(deps);

    expect(relaunch).not.toHaveBeenCalled();
  });

  it('does not relaunch when there is nothing to install', async () => {
    const relaunch = vi.fn(async () => undefined);
    const deps = makeDeps({ checkPlugin: vi.fn(async () => null), relaunch });
    await run(deps);

    expect(relaunch).not.toHaveBeenCalled();
  });

  it('tells the user to quit and reopen when relaunch fails', async () => {
    const update = makeFakeUpdate('0.9.1');
    const relaunch = vi.fn(async () => {
      throw new Error('plugin:process|restart not allowed');
    });
    const deps = makeDeps({ checkPlugin: vi.fn(async () => update), relaunch });
    const transitions = await run(deps);

    expect(transitions[transitions.length - 1]).toEqual({
      state: 'error',
      message: 'The update is installed. Quit Jura Trace and open it again to finish.',
    });
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
      expect(last.message).toContain('Could not reach the update service');
      expect(last.message).toContain('juralabs.org/download.');
      expect(last.message).not.toContain('JSON');
    }
  });

  it('maps "Failed to fetch" network error to friendly error', () => {
    const result = classifyError('Failed to fetch');
    expect(result.state).toBe('error');
    if (result.state === 'error') {
      expect(result.message).toContain('Could not reach the update service');
    }
  });

  it('maps "Network request failed" to friendly error', () => {
    const result = classifyError('Network request failed: connection reset');
    expect(result.state).toBe('error');
    if (result.state === 'error') {
      expect(result.message).toContain('Could not reach the update service');
    }
  });

  it('maps only "No updates available" to up-to-date, never a 404', () => {
    // "No updates available" is the plugin telling us it fetched a manifest
    // and compared versions. A 404 is the manifest never arriving. Only the
    // first is evidence that the user is current.
    expect(classifyError('No updates available from endpoint').state).toBe('up-to-date');
    expect(classifyError('Request failed with status 404').state).toBe('error');
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

// ── Automatic startup check (T1.2) ─────────────────────────────────────────

function makeStartupDeps(over: Partial<StartupCheckDeps> = {}): StartupCheckDeps {
  return {
    isTauri: () => true,
    getNetworkMode: async () => 'enhanced',
    checkPlugin: async () => null,
    now: () => 1_000_000_000,
    readLastCheck: () => null,
    writeLastCheck: () => {},
    ...over,
  };
}

describe('shouldCheckNow', () => {
  it('allows a check when there is no recorded previous check', () => {
    expect(shouldCheckNow(1000, null)).toBe(true);
  });

  it('allows a check when the recorded value is not a finite number', () => {
    expect(shouldCheckNow(1000, NaN)).toBe(true);
  });

  it('blocks a check inside the interval', () => {
    const now = 1_000_000_000;
    expect(shouldCheckNow(now, now - 60_000)).toBe(false);
  });

  it('allows a check exactly on the interval boundary', () => {
    const now = 1_000_000_000;
    expect(shouldCheckNow(now, now - STARTUP_CHECK_INTERVAL_MS)).toBe(true);
  });

  it('treats a future timestamp as stale rather than blocking indefinitely', () => {
    const now = 1_000_000_000;
    // A clock change must not lock the user out of update checks until the
    // real time catches up with the bogus stored value.
    expect(shouldCheckNow(now, now + STARTUP_CHECK_INTERVAL_MS * 10)).toBe(true);
  });
});

describe('runStartupUpdateCheck', () => {
  it('skips outside the desktop app', async () => {
    const checkPlugin = vi.fn();
    const r = await runStartupUpdateCheck(makeStartupDeps({ isTauri: () => false, checkPlugin }));
    expect(r).toEqual({ kind: 'skipped', reason: 'not-desktop' });
    expect(checkPlugin).not.toHaveBeenCalled();
  });

  it('makes NO outbound call in Standard (offline) mode', async () => {
    const checkPlugin = vi.fn();
    const r = await runStartupUpdateCheck(
      makeStartupDeps({ getNetworkMode: async () => 'standard', checkPlugin }),
    );
    expect(r).toEqual({ kind: 'skipped', reason: 'offline-mode' });
    // The point of the gate: a user who chose local-only operation gets no
    // silent network traffic at launch.
    expect(checkPlugin).not.toHaveBeenCalled();
  });

  it('skips when a check already ran inside the interval', async () => {
    const checkPlugin = vi.fn();
    const now = 1_000_000_000;
    const r = await runStartupUpdateCheck(
      makeStartupDeps({ now: () => now, readLastCheck: () => now - 1000, checkPlugin }),
    );
    expect(r).toEqual({ kind: 'skipped', reason: 'throttled' });
    expect(checkPlugin).not.toHaveBeenCalled();
  });

  it('reports an available update with its version', async () => {
    const r = await runStartupUpdateCheck(
      makeStartupDeps({ checkPlugin: async () => ({ version: '1.1.0' }) }),
    );
    expect(r).toEqual({ kind: 'available', version: '1.1.0' });
  });

  it('reports up-to-date when the plugin returns no update', async () => {
    const r = await runStartupUpdateCheck(makeStartupDeps({ checkPlugin: async () => null }));
    expect(r).toEqual({ kind: 'up-to-date' });
  });

  it('records the check timestamp only after the request completes', async () => {
    const writeLastCheck = vi.fn();
    const now = 42;
    await runStartupUpdateCheck(
      makeStartupDeps({ now: () => now, writeLastCheck, checkPlugin: async () => null }),
    );
    expect(writeLastCheck).toHaveBeenCalledWith(now);
  });

  it('does NOT record a timestamp when the check fails, so it retries next launch', async () => {
    const writeLastCheck = vi.fn();
    const r = await runStartupUpdateCheck(
      makeStartupDeps({
        writeLastCheck,
        checkPlugin: async () => {
          throw new Error('Network request failed');
        },
      }),
    );
    expect(r.kind).toBe('failed');
    expect(writeLastCheck).not.toHaveBeenCalled();
  });

  it('reports a 404 as a failure, never as up-to-date', async () => {
    const r = await runStartupUpdateCheck(
      makeStartupDeps({
        checkPlugin: async () => {
          throw new Error('Request failed with status 404');
        },
      }),
    );
    // The whole point of BL-REL-002's fourth fault: a missing manifest is not
    // evidence that the user is current.
    expect(r.kind).toBe('failed');
  });

  it('treats the plugin’s explicit no-update error as up-to-date', async () => {
    const r = await runStartupUpdateCheck(
      makeStartupDeps({
        checkPlugin: async () => {
          throw new Error('No updates available');
        },
      }),
    );
    expect(r).toEqual({ kind: 'up-to-date' });
  });

  it('cannot install: the dependency interface exposes no way to do so', async () => {
    // Structural guarantee rather than a behavioural one. StartupCheckDeps
    // has no `backup` and the object returned by checkPlugin carries only a
    // version, so there is no downloadAndInstall to call even by mistake.
    const update = await makeStartupDeps({
      checkPlugin: async () => ({ version: '1.1.0' }),
    }).checkPlugin();
    expect(update).not.toBeNull();
    expect(Object.keys(update ?? {})).toEqual(['version']);
    expect('downloadAndInstall' in (update ?? {})).toBe(false);

    const deps = makeStartupDeps();
    expect('backup' in deps).toBe(false);
  });

  it('exports a real dependency factory', () => {
    expect(typeof makeStartupCheckDeps).toBe('function');
  });
});
