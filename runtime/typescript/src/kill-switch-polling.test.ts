/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { writeFileSync, mkdirSync, rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  refreshKillSwitchFromUrl,
  refreshKillSwitchFromPath,
  KillSwitchRefreshCoordinator,
  startKillSwitchPoll,
  killSwitchInitDelayMs,
  type KillSwitchRefreshState,
} from './kill-switch-polling';
import { loadKillSwitchFromURL, KillSwitchFileNotModifiedError } from './kill-switch-loader';

vi.mock('./kill-switch-loader', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./kill-switch-loader')>();
  return {
    ...actual,
    loadKillSwitchFromURL: vi.fn(),
  };
});

const mockedLoad = vi.mocked(loadKillSwitchFromURL);

describe('refreshKillSwitchFromPath', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = join(
      tmpdir(),
      'cp-ks-path-poll-test',
      `${Date.now()}-${Math.random().toString(36).substring(7)}`
    );
    mkdirSync(testDir, { recursive: true });
  });

  afterEach(() => {
    try {
      rmSync(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors.
    }
  });

  const prior: KillSwitchRefreshState = {
    file: { version: '2.0', flags: { old_flag: true } },
    fileFingerprint: { mtimeMs: 0, size: 1 },
  };

  it('updates state when file changes', async () => {
    const filePath = join(testDir, 'kill.json');
    writeFileSync(filePath, JSON.stringify({ version: '2.0', flags: { new_dashboard: false } }));

    const result = await refreshKillSwitchFromPath(filePath, { file: null });

    expect(result.status).toBe('updated');
    expect(result.state.file?.flags.new_dashboard).toBe(false);
    expect(result.state.fileFingerprint).toBeDefined();
  });

  it('retains prior state when file is unchanged', async () => {
    const filePath = join(testDir, 'kill.json');
    writeFileSync(filePath, JSON.stringify({ version: '2.0', flags: { old_flag: true } }));

    const loaded = await refreshKillSwitchFromPath(filePath, { file: null });
    expect(loaded.status).toBe('updated');

    const again = await refreshKillSwitchFromPath(filePath, loaded.state);
    expect(again.status).toBe('not-modified');
    expect(again.state).toBe(loaded.state);
  });

  it('retains prior state when file is missing', async () => {
    const filePath = join(testDir, 'missing.json');
    const warn = vi.fn();

    const result = await refreshKillSwitchFromPath(filePath, prior, {
      debug: vi.fn(),
      info: vi.fn(),
      warn,
      error: vi.fn(),
    });

    expect(result.status).toBe('failed');
    expect(result.state).toBe(prior);
    expect(warn).toHaveBeenCalled();
  });

  it('retains prior state when file bytes are invalid', async () => {
    const filePath = join(testDir, 'bad.json');
    writeFileSync(filePath, '{ invalid');

    const result = await refreshKillSwitchFromPath(filePath, prior);

    expect(result.status).toBe('failed');
    expect(result.state.file?.flags.old_flag).toBe(true);
  });
});

describe('refreshKillSwitchFromUrl', () => {
  const prior: KillSwitchRefreshState = {
    file: { version: '2.0', flags: { old_flag: true } },
    etag: '"old"',
  };

  beforeEach(() => {
    mockedLoad.mockReset();
  });

  it('updates state and returns updated on success', async () => {
    mockedLoad.mockResolvedValue({
      killSwitchFile: { version: '2.0', flags: { new_dashboard: false } },
      etag: '"new"',
    });

    const result = await refreshKillSwitchFromUrl('https://example.com/kill.json', prior);

    expect(result.status).toBe('updated');
    expect(result.state.file?.flags.new_dashboard).toBe(false);
    expect(result.state.etag).toBe('"new"');
    expect(mockedLoad).toHaveBeenCalledWith(
      'https://example.com/kill.json',
      '"old"',
      undefined,
      undefined
    );
  });

  it('retains prior state on 304 not modified', async () => {
    mockedLoad.mockRejectedValue(new KillSwitchFileNotModifiedError());

    const result = await refreshKillSwitchFromUrl('https://example.com/kill.json', prior);

    expect(result.status).toBe('not-modified');
    expect(result.state).toBe(prior);
  });

  it('retains prior state on network failure', async () => {
    mockedLoad.mockRejectedValue(new Error('network down'));

    const result = await refreshKillSwitchFromUrl('https://example.com/kill.json', prior);

    expect(result.status).toBe('failed');
    expect(result.state).toBe(prior);
  });

  it('logs a warning when refresh fails and a logger is provided', async () => {
    mockedLoad.mockRejectedValue(new Error('network down'));
    const warn = vi.fn();

    await refreshKillSwitchFromUrl('https://example.com/kill.json', prior, undefined, {
      debug: vi.fn(),
      info: vi.fn(),
      warn,
      error: vi.fn(),
    });

    expect(warn).toHaveBeenCalledWith(
      'Kill switch refresh failed for https://example.com/kill.json; keeping prior values',
      expect.any(Error)
    );
  });
});

describe('KillSwitchRefreshCoordinator', () => {
  beforeEach(() => {
    mockedLoad.mockReset();
  });

  it('only commits state on successful refresh', async () => {
    const coordinator = new KillSwitchRefreshCoordinator({
      file: { version: '2.0', flags: { kept: true } },
      etag: '"v0"',
    });

    mockedLoad.mockRejectedValue(new Error('cdn down'));
    const result = await coordinator.refresh('https://example.com/kill.json');

    expect(result.status).toBe('failed');
    expect(coordinator.getState().file?.flags.kept).toBe(true);
  });

  it('serializes overlapping refreshes so a stale failure cannot overwrite a newer success', async () => {
    const coordinator = new KillSwitchRefreshCoordinator();
    let releaseSlow: () => void;
    const slowGate = new Promise<void>((resolve) => {
      releaseSlow = resolve;
    });

    mockedLoad.mockImplementationOnce(async () => {
      await slowGate;
      throw new Error('slow failure');
    });
    mockedLoad.mockResolvedValueOnce({
      killSwitchFile: { version: '2.0', flags: { from_fast: true } },
      etag: '"fast"',
    });

    const slow = coordinator.refresh('https://example.com/kill.json');
    const fast = coordinator.refresh('https://example.com/kill.json');

    releaseSlow!();
    await Promise.all([slow, fast]);

    expect(coordinator.getState().file?.flags.from_fast).toBe(true);
    expect(mockedLoad).toHaveBeenCalledTimes(2);
  });

  it('reset clears state and pending queue tail', async () => {
    const coordinator = new KillSwitchRefreshCoordinator({
      file: { version: '2.0', flags: { old: true } },
    });
    coordinator.reset();
    expect(coordinator.getState().file).toBeNull();
  });

  it('refreshFromPath only commits state on successful file read', async () => {
    let testDir: string;
    testDir = join(
      tmpdir(),
      'cp-ks-coord-path',
      `${Date.now()}-${Math.random().toString(36).substring(7)}`
    );
    mkdirSync(testDir, { recursive: true });
    const filePath = join(testDir, 'kill.json');
    writeFileSync(filePath, JSON.stringify({ version: '2.0', flags: { kept: true } }));

    const coordinator = new KillSwitchRefreshCoordinator({
      file: { version: '2.0', flags: { prior: true } },
    });

    const result = await coordinator.refreshFromPath(filePath);
    expect(result.status).toBe('updated');
    expect(coordinator.getState().file?.flags.kept).toBe(true);

    rmSync(testDir, { recursive: true, force: true });
  });
});

describe('startKillSwitchPoll', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.spyOn(Math, 'random').mockReturnValue(0.5);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('invokes refresh after interval plus jitter until stopped', async () => {
    const refresh = vi.fn().mockResolvedValue(undefined);
    const stop = startKillSwitchPoll(refresh, 1000, { jitterMs: 400 });

    expect(refresh).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1200);
    expect(refresh).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1200);
    expect(refresh).toHaveBeenCalledTimes(2);

    stop();
    await vi.advanceTimersByTimeAsync(5000);
    expect(refresh).toHaveBeenCalledTimes(2);
  });

  it('defaults jitter to 20% of interval capped at 15s', () => {
    expect(Math.min(Math.floor(30_000 * 0.2), 15_000)).toBe(6000);
    expect(Math.min(Math.floor(120_000 * 0.2), 15_000)).toBe(15_000);
  });
});

describe('killSwitchInitDelayMs', () => {
  it('returns a value in [0, maxMs)', () => {
    vi.spyOn(Math, 'random').mockReturnValue(0.99);
    expect(killSwitchInitDelayMs(5000)).toBe(4950);
    vi.spyOn(Math, 'random').mockReturnValue(0);
    expect(killSwitchInitDelayMs(5000)).toBe(0);
  });
});

describe('kill switch wins over AST (integration pattern)', () => {
  it('uses qualified import namespace keys from refreshed kill switch file', async () => {
    mockedLoad.mockResolvedValue({
      killSwitchFile: {
        version: '2.0',
        flags: { 'platform.emergency_kill_switch': false },
      },
      etag: '"v1"',
    });

    const refresh = await refreshKillSwitchFromUrl('https://example.com/kill.json', {
      file: null,
      etag: undefined,
    });

    const { resolveBooleanFlag } = await import('./resolve-flag');
    const { RuleType } = await import('./types');

    const artifact = {
      v: '1.0',
      env: 'production',
      strs: ['ON', 'OFF', 'platform.emergency_kill_switch'],
      flags: [[[RuleType.SERVE, undefined, 0]]],
      flagNames: [2],
    };

    const value = resolveBooleanFlag({
      qualifiedName: 'platform.emergency_kill_switch',
      flagIndex: 0,
      artifact,
      catalogDefault: true,
      killSwitchFile: refresh.state.file,
      attributes: { id: 'user1', role: 'admin' },
    });

    expect(value).toBe(false);
  });
});
