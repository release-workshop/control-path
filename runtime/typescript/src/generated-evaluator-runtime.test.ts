/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { pack } from 'msgpackr';
import { writeFile, mkdtemp, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  GeneratedEvaluatorRuntime,
  DEFAULT_GENERATED_ARTIFACT_INIT_JITTER_MS,
  DEFAULT_GENERATED_ARTIFACT_POLL_JITTER_MS,
  DEFAULT_GENERATED_ARTIFACT_POLL_MS,
  DEFAULT_GENERATED_KILL_SWITCH_INIT_JITTER_MS,
  DEFAULT_GENERATED_KILL_SWITCH_POLL_JITTER_MS,
  DEFAULT_GENERATED_KILL_SWITCH_POLL_MS,
} from './generated-evaluator-runtime';
import { loadFromURL, ArtifactNotModifiedError } from './ast-loader';
import { loadKillSwitchFromURL } from './kill-switch-loader';
import {
  killSwitchInitDelayMs,
  pollInitDelayMs,
  startJitteredPoll,
  startKillSwitchPoll,
} from './kill-switch-polling';
import { RuleType } from './types';
import type { Artifact } from './types';

vi.mock('./ast-loader', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./ast-loader')>();
  return {
    ...actual,
    loadFromURL: vi.fn(),
  };
});

vi.mock('./kill-switch-loader', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./kill-switch-loader')>();
  return {
    ...actual,
    loadKillSwitchFromURL: vi.fn(),
  };
});

vi.mock('./kill-switch-polling', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./kill-switch-polling')>();
  return {
    ...actual,
    startKillSwitchPoll: vi.fn(),
    startJitteredPoll: vi.fn(),
    killSwitchInitDelayMs: vi.fn(),
    pollInitDelayMs: vi.fn(),
  };
});

const mockedLoadFromURL = vi.mocked(loadFromURL);
const mockedLoadKillSwitchFromURL = vi.mocked(loadKillSwitchFromURL);
const mockedStartKillSwitchPoll = vi.mocked(startKillSwitchPoll);
const mockedStartJitteredPoll = vi.mocked(startJitteredPoll);
const mockedKillSwitchInitDelayMs = vi.mocked(killSwitchInitDelayMs);
const mockedPollInitDelayMs = vi.mocked(pollInitDelayMs);

function sampleArtifact(overrides: Partial<Artifact> = {}): Artifact {
  return {
    v: '1.0',
    env: 'production',
    strs: ['ON', 'OFF', 'new_dashboard'],
    flags: [[[RuleType.SERVE, undefined, 0]]],
    flagNames: [2],
    ...overrides,
  };
}

async function writeArtifactFile(artifact: Artifact): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), 'cp-gen-eval-'));
  const path = join(dir, 'production.ast');
  await writeFile(path, pack(artifact));
  return path;
}

describe('GeneratedEvaluatorRuntime', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedStartKillSwitchPoll.mockReturnValue(vi.fn());
    mockedStartJitteredPoll.mockReturnValue(vi.fn());
    mockedKillSwitchInitDelayMs.mockReturnValue(0);
    mockedPollInitDelayMs.mockReturnValue(0);
  });

  it('loads a local artifact on init and evaluates a flag with attributes', async () => {
    const artifact = sampleArtifact();
    const artifactPath = await writeArtifactFile(artifact);
    const runtime = new GeneratedEvaluatorRuntime({
      killSwitchUrls: {},
      artifactUrls: {},
      sdkQualifiedFlagNames: new Set(['new_dashboard']),
    });

    try {
      await runtime.init({ artifact: artifactPath });

      expect(runtime.getArtifact()?.env).toBe('production');
      expect(
        runtime.evaluateBooleanFlag({
          qualifiedName: 'new_dashboard',
          catalogDefault: false,
          attributes: { id: 'user-1' },
        })
      ).toBe(true);
    } finally {
      await rm(join(artifactPath, '..'), { recursive: true, force: true });
    }
  });

  it('rejects init when artifact env does not match declared artifact URL env', async () => {
    const artifact = sampleArtifact({ env: 'staging' });
    const artifactPath = await writeArtifactFile(artifact);
    const runtime = new GeneratedEvaluatorRuntime({
      killSwitchUrls: {},
      artifactUrls: { production: 'https://flags.example.com/production/rules.ast' },
      sdkQualifiedFlagNames: new Set(['new_dashboard']),
    });

    try {
      await expect(runtime.init({ artifact: artifactPath })).rejects.toThrow(
        'Compiled artifact rejected: environment mismatch'
      );
    } finally {
      await rm(join(artifactPath, '..'), { recursive: true, force: true });
    }
  });

  it('sends prior artifact ETag on refresh after no-artifact re-init', async () => {
    const artifact = sampleArtifact();
    const artifactUrl = 'https://flags.example.com/production/rules.ast';
    mockedLoadFromURL
      .mockResolvedValueOnce({ artifact, etag: '"v1"' })
      .mockRejectedValueOnce(new ArtifactNotModifiedError());

    const runtime = new GeneratedEvaluatorRuntime({
      killSwitchUrls: {},
      artifactUrls: { production: artifactUrl },
      sdkQualifiedFlagNames: new Set(['new_dashboard']),
    });

    await runtime.init({ artifact: artifactUrl });
    await runtime.init();
    await runtime.refreshArtifact();

    expect(runtime.getArtifact()).toEqual(artifact);
    expect(mockedLoadFromURL).toHaveBeenLastCalledWith(
      artifactUrl,
      undefined,
      undefined,
      expect.objectContaining({ etag: '"v1"' })
    );
  });

  it('keeps the loaded artifact when a background artifact refresh gets HTTP 304', async () => {
    const artifact = sampleArtifact();
    const artifactUrl = 'https://flags.example.com/production/rules.ast';
    mockedLoadFromURL
      .mockResolvedValueOnce({ artifact, etag: '"v1"' })
      .mockRejectedValueOnce(new ArtifactNotModifiedError());

    const runtime = new GeneratedEvaluatorRuntime({
      killSwitchUrls: {},
      artifactUrls: { production: artifactUrl },
      sdkQualifiedFlagNames: new Set(['new_dashboard']),
    });

    await runtime.init({ artifact: artifactUrl });
    await runtime.refreshArtifact();

    expect(runtime.getArtifact()).toEqual(artifact);
    expect(mockedLoadFromURL).toHaveBeenLastCalledWith(
      artifactUrl,
      undefined,
      undefined,
      expect.objectContaining({ etag: '"v1"' })
    );
  });

  it('starts independent kill-switch and artifact poll loops with ADR defaults', async () => {
    const artifact = sampleArtifact();
    const artifactPath = await writeArtifactFile(artifact);
    const runtime = new GeneratedEvaluatorRuntime({
      killSwitchUrls: { production: 'https://flags.example.com/production/kill-switches.json' },
      artifactUrls: { production: 'https://flags.example.com/production/rules.ast' },
      sdkQualifiedFlagNames: new Set(['new_dashboard']),
    });

    try {
      await runtime.init({ artifact: artifactPath });

      expect(mockedStartKillSwitchPoll).toHaveBeenCalledWith(
        expect.any(Function),
        DEFAULT_GENERATED_KILL_SWITCH_POLL_MS,
        { jitterMs: DEFAULT_GENERATED_KILL_SWITCH_POLL_JITTER_MS }
      );
      expect(mockedStartJitteredPoll).toHaveBeenCalledWith(
        expect.any(Function),
        DEFAULT_GENERATED_ARTIFACT_POLL_MS,
        { jitterMs: DEFAULT_GENERATED_ARTIFACT_POLL_JITTER_MS }
      );
      expect(mockedKillSwitchInitDelayMs).toHaveBeenCalledWith(
        DEFAULT_GENERATED_KILL_SWITCH_INIT_JITTER_MS
      );
      expect(mockedPollInitDelayMs).toHaveBeenCalledWith(
        DEFAULT_GENERATED_ARTIFACT_INIT_JITTER_MS
      );
    } finally {
      await rm(join(artifactPath, '..'), { recursive: true, force: true });
    }
  });

  it('does not stop another instance poll loop when one instance stops polling', async () => {
    const artifact = sampleArtifact();
    const artifactPath = await writeArtifactFile(artifact);
    const stopKillSwitchPollFns: Array<ReturnType<typeof vi.fn>> = [];
    mockedStartKillSwitchPoll.mockImplementation(() => {
      const stop = vi.fn();
      stopKillSwitchPollFns.push(stop);
      return stop;
    });

    const runtimeA = new GeneratedEvaluatorRuntime({
      killSwitchUrls: { production: 'https://flags.example.com/production/kill-switches.json' },
      artifactUrls: {},
      sdkQualifiedFlagNames: new Set(['new_dashboard']),
    });
    const runtimeB = new GeneratedEvaluatorRuntime({
      killSwitchUrls: { production: 'https://flags.example.com/production/kill-switches.json' },
      artifactUrls: {},
      sdkQualifiedFlagNames: new Set(['new_dashboard']),
    });

    try {
      await runtimeA.init({ artifact: artifactPath });
      await runtimeB.init({ artifact: artifactPath });

      runtimeA.stopKillSwitchPolling();

      expect(stopKillSwitchPollFns[0]).toHaveBeenCalled();
      expect(stopKillSwitchPollFns[1]).not.toHaveBeenCalled();
    } finally {
      await rm(join(artifactPath, '..'), { recursive: true, force: true });
    }
  });

  it('keeps loaded artifact when init is called without artifact', async () => {
    const artifact = sampleArtifact();
    const artifactPath = await writeArtifactFile(artifact);
    const runtime = new GeneratedEvaluatorRuntime({
      killSwitchUrls: {},
      artifactUrls: {},
      sdkQualifiedFlagNames: new Set(['new_dashboard']),
    });

    try {
      await runtime.init({ artifact: artifactPath });
      const killSwitchPollCallsBefore = mockedStartKillSwitchPoll.mock.calls.length;
      await runtime.init();

      expect(runtime.getArtifact()).toEqual(artifact);
      expect(
        runtime.evaluateBooleanFlag({
          qualifiedName: 'new_dashboard',
          catalogDefault: false,
          attributes: { id: 'user-1' },
        })
      ).toBe(true);
      expect(mockedStartKillSwitchPoll.mock.calls.length).toBeGreaterThan(
        killSwitchPollCallsBefore
      );
    } finally {
      await rm(join(artifactPath, '..'), { recursive: true, force: true });
    }
  });

  it('restores prior state when init with artifact fails to load', async () => {
    const artifact = sampleArtifact();
    const artifactPath = await writeArtifactFile(artifact);
    const killSwitchUrl = 'https://flags.example.com/production/kill-switches.json';
    mockedLoadKillSwitchFromURL.mockResolvedValue({
      killSwitchFile: { version: '2.0', flags: { new_dashboard: false } },
      etag: '"ks1"',
    });

    const runtime = new GeneratedEvaluatorRuntime({
      killSwitchUrls: { production: killSwitchUrl },
      artifactUrls: {},
      sdkQualifiedFlagNames: new Set(['new_dashboard']),
    });

    try {
      await runtime.init({ artifact: artifactPath });
      await runtime.refreshKillSwitch();
      expect(
        runtime.evaluateBooleanFlag({
          qualifiedName: 'new_dashboard',
          catalogDefault: false,
          attributes: { id: 'user-1' },
        })
      ).toBe(false);

      await expect(runtime.init({ artifact: join(tmpdir(), 'missing-rules.ast') })).rejects.toThrow();

      expect(runtime.getArtifact()).toEqual(artifact);
      expect(
        runtime.evaluateBooleanFlag({
          qualifiedName: 'new_dashboard',
          catalogDefault: false,
          attributes: { id: 'user-1' },
        })
      ).toBe(false);
    } finally {
      await rm(join(artifactPath, '..'), { recursive: true, force: true });
    }
  });

  it('keeps kill-switch overrides when init is called without artifact', async () => {
    const artifact = sampleArtifact();
    const artifactPath = await writeArtifactFile(artifact);
    const killSwitchUrl = 'https://flags.example.com/production/kill-switches.json';
    mockedLoadKillSwitchFromURL.mockResolvedValue({
      killSwitchFile: { version: '2.0', flags: { new_dashboard: false } },
      etag: '"ks1"',
    });

    const runtime = new GeneratedEvaluatorRuntime({
      killSwitchUrls: { production: killSwitchUrl },
      artifactUrls: {},
      sdkQualifiedFlagNames: new Set(['new_dashboard']),
    });

    try {
      await runtime.init({ artifact: artifactPath });
      await runtime.refreshKillSwitch();
      expect(
        runtime.evaluateBooleanFlag({
          qualifiedName: 'new_dashboard',
          catalogDefault: false,
          attributes: { id: 'user-1' },
        })
      ).toBe(false);

      mockedLoadKillSwitchFromURL.mockClear();
      await runtime.init();

      expect(
        runtime.evaluateBooleanFlag({
          qualifiedName: 'new_dashboard',
          catalogDefault: false,
          attributes: { id: 'user-1' },
        })
      ).toBe(false);
      expect(mockedLoadKillSwitchFromURL).not.toHaveBeenCalled();
    } finally {
      await rm(join(artifactPath, '..'), { recursive: true, force: true });
    }
  });
});
