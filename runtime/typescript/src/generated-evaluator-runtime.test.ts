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
import { GeneratedEvaluatorRuntime } from './generated-evaluator-runtime';
import { loadFromURL, ArtifactNotModifiedError } from './ast-loader';
import { RuleType } from './types';
import type { Artifact } from './types';

vi.mock('./ast-loader', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./ast-loader')>();
  return {
    ...actual,
    loadFromURL: vi.fn(),
  };
});

const mockedLoadFromURL = vi.mocked(loadFromURL);

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
      await runtime.init();

      expect(runtime.getArtifact()).toEqual(artifact);
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
});
