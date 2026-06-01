/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  refreshArtifactFromUrl,
  ArtifactRefreshCoordinator,
  validateArtifactPoll,
  assertArtifactAccepted,
  resolveExpectedArtifactEnv,
  shouldValidateArtifactAtInit,
  artifactFlagNames,
  type ArtifactRefreshState,
} from './artifact-polling';
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

const mockedLoad = vi.mocked(loadFromURL);

const SDK_FLAGS = new Set(['new_dashboard', 'premium_checkout']);

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

function priorState(artifact: Artifact, etag = '"v0"'): ArtifactRefreshState {
  return {
    artifact,
    etag,
    flagNameMap: { new_dashboard: 0 },
  };
}

describe('resolveExpectedArtifactEnv', () => {
  const urls = {
    production: 'https://flags.example.com/production/rules.ast',
    staging: 'https://flags.example.com/staging/rules.ast',
  };

  it('prefers <env>.ast in the artifact path when that env has a catalog URL', () => {
    const artifact = sampleArtifact({ env: 'staging' });
    expect(
      resolveExpectedArtifactEnv('.controlpath/production.ast', artifact, urls)
    ).toBe('production');
  });

  it('normalizes path env segment to lowercase catalog keys', () => {
    const artifact = sampleArtifact({ env: 'staging' });
    expect(
      resolveExpectedArtifactEnv('.controlpath/Production.ast', artifact, urls)
    ).toBe('production');
  });

  it('matches init URL to catalog artifact URL', () => {
    const artifact = sampleArtifact();
    expect(
      resolveExpectedArtifactEnv(urls.production, artifact, urls)
    ).toBe('production');
  });
});

describe('resolveExpectedArtifactEnv (SaaS CDN contract URLs)', () => {
  const saasUrls = {
    production:
      'https://cdn.controlpath.dev/v2/runtime/projects/acme/checkout/catalogs/acme.checkout-service/environments/production/rules.ast',
    staging:
      'https://cdn.controlpath.dev/v2/runtime/projects/acme/checkout/catalogs/acme.checkout-service/environments/staging/rules.ast',
  };

  it('matches init source to SaaS CDN artifact poll URL', () => {
    const artifact = sampleArtifact({ env: 'production' });
    expect(resolveExpectedArtifactEnv(saasUrls.production, artifact, saasUrls)).toBe(
      'production'
    );
  });

  it('resolves env from bundled path when SaaS CDN URLs are embedded', () => {
    const artifact = sampleArtifact();
    expect(
      resolveExpectedArtifactEnv('.controlpath/production.ast', artifact, saasUrls)
    ).toBe('production');
  });
});

describe('shouldValidateArtifactAtInit', () => {
  it('is true only when artifacts.<env>.url exists', () => {
    expect(shouldValidateArtifactAtInit({ production: 'https://x/rules.ast' }, 'production')).toBe(
      true
    );
    expect(shouldValidateArtifactAtInit({ production: 'https://x/rules.ast' }, 'staging')).toBe(
      false
    );
    expect(shouldValidateArtifactAtInit({}, 'production')).toBe(false);
  });
});

describe('assertArtifactAccepted', () => {
  it('throws with reason when validation fails', () => {
    const artifact = sampleArtifact({ env: 'staging' });
    expect(() => assertArtifactAccepted(artifact, 'production', SDK_FLAGS)).toThrow(
      'Compiled artifact rejected: environment mismatch'
    );
  });
});

describe('validateArtifactPoll', () => {
  it('accepts when env matches and at least one flag overlaps the SDK', () => {
    const artifact = sampleArtifact();
    expect(validateArtifactPoll(artifact, 'production', SDK_FLAGS)).toEqual({ accepted: true });
  });

  it('rejects environment mismatch', () => {
    const artifact = sampleArtifact({ env: 'staging' });
    const result = validateArtifactPoll(artifact, 'production', SDK_FLAGS);
    expect(result).toEqual({ accepted: false, reason: 'environment mismatch' });
  });

  it('rejects zero flag-name overlap with the SDK', () => {
    const artifact = sampleArtifact({
      strs: ['ON', 'OFF', 'unknown_flag'],
      flagNames: [2],
    });
    const result = validateArtifactPoll(artifact, 'production', SDK_FLAGS);
    expect(result).toEqual({ accepted: false, reason: 'zero flag-name overlap with SDK' });
  });

  it('accepts artifacts with extra flags not in the SDK', () => {
    const artifact = sampleArtifact({
      strs: ['ON', 'OFF', 'new_dashboard', 'future_flag'],
      flagNames: [2, 3],
      flags: [[[RuleType.SERVE, undefined, 0]], [[RuleType.SERVE, undefined, 0]]],
    });
    expect(artifactFlagNames(artifact)).toContain('future_flag');
    expect(validateArtifactPoll(artifact, 'production', SDK_FLAGS)).toEqual({ accepted: true });
  });
});

describe('refreshArtifactFromUrl', () => {
  const prior = priorState(sampleArtifact());

  beforeEach(() => {
    mockedLoad.mockReset();
  });

  it('updates state and returns updated on success', async () => {
    const next = sampleArtifact({ strs: ['ON', 'OFF', 'new_dashboard'] });
    mockedLoad.mockResolvedValue({ artifact: next, etag: '"v1"' });

    const result = await refreshArtifactFromUrl(
      'https://example.com/rules.ast',
      'production',
      SDK_FLAGS,
      prior
    );

    expect(result.status).toBe('updated');
    expect(result.state.etag).toBe('"v1"');
    expect(result.state.flagNameMap.new_dashboard).toBe(0);
    expect(mockedLoad).toHaveBeenCalledWith(
      'https://example.com/rules.ast',
      undefined,
      undefined,
      expect.objectContaining({ etag: '"v0"' })
    );
  });

  it('retains prior state on 304 not modified', async () => {
    mockedLoad.mockRejectedValue(new ArtifactNotModifiedError());

    const result = await refreshArtifactFromUrl(
      'https://example.com/rules.ast',
      'production',
      SDK_FLAGS,
      prior
    );

    expect(result.status).toBe('not-modified');
    expect(result.state).toBe(prior);
  });

  it('retains prior state on network failure', async () => {
    mockedLoad.mockRejectedValue(new Error('network down'));

    const result = await refreshArtifactFromUrl(
      'https://example.com/rules.ast',
      'production',
      SDK_FLAGS,
      prior
    );

    expect(result.status).toBe('failed');
    expect(result.state).toBe(prior);
  });

  it('rejects poll when env mismatches but keeps prior artifact', async () => {
    mockedLoad.mockResolvedValue({
      artifact: sampleArtifact({ env: 'staging' }),
      etag: '"bad"',
    });

    const result = await refreshArtifactFromUrl(
      'https://example.com/rules.ast',
      'production',
      SDK_FLAGS,
      prior
    );

    expect(result.status).toBe('rejected');
    expect(result.state).toBe(prior);
    if (result.status === 'rejected') {
      expect(result.reason).toBe('environment mismatch');
    }
  });

  it('rejects poll on zero flag-name overlap with the SDK but keeps prior artifact', async () => {
    mockedLoad.mockResolvedValue({
      artifact: sampleArtifact({
        strs: ['ON', 'OFF', 'unknown_flag'],
        flagNames: [2],
      }),
      etag: '"wrong-object"',
    });

    const result = await refreshArtifactFromUrl(
      'https://example.com/rules.ast',
      'production',
      SDK_FLAGS,
      prior
    );

    expect(result.status).toBe('rejected');
    expect(result.state).toBe(prior);
    if (result.status === 'rejected') {
      expect(result.reason).toBe('zero flag-name overlap with SDK');
    }
  });
});

describe('ArtifactRefreshCoordinator', () => {
  beforeEach(() => {
    mockedLoad.mockReset();
  });

  it('only commits state on successful refresh', async () => {
    const coordinator = new ArtifactRefreshCoordinator(priorState(sampleArtifact()));
    mockedLoad.mockRejectedValue(new Error('cdn down'));

    const result = await coordinator.refresh(
      'https://example.com/rules.ast',
      'production',
      SDK_FLAGS
    );

    expect(result.status).toBe('failed');
    expect(coordinator.getState().artifact?.env).toBe('production');
  });

  it('serializes overlapping refreshes so a stale failure cannot overwrite a newer success', async () => {
    const coordinator = new ArtifactRefreshCoordinator();
    let releaseSlow: () => void;
    const slowGate = new Promise<void>((resolve) => {
      releaseSlow = resolve;
    });

    mockedLoad.mockImplementationOnce(async () => {
      await slowGate;
      throw new Error('slow failure');
    });
    mockedLoad.mockResolvedValueOnce({
      artifact: sampleArtifact(),
      etag: '"fast"',
    });

    const slow = coordinator.refresh(
      'https://example.com/rules.ast',
      'production',
      SDK_FLAGS
    );
    const fast = coordinator.refresh(
      'https://example.com/rules.ast',
      'production',
      SDK_FLAGS
    );

    releaseSlow!();
    await Promise.all([slow, fast]);

    expect(coordinator.getState().etag).toBe('"fast"');
    expect(mockedLoad).toHaveBeenCalledTimes(2);
  });

  it('hot-swaps evaluation after a successful refresh', async () => {
    const { resolveBooleanFlag } = await import('./resolve-flag');
    const initial = sampleArtifact({
      flags: [[[RuleType.SERVE, undefined, 1]]],
    });
    const coordinator = new ArtifactRefreshCoordinator(priorState(initial));

    mockedLoad.mockResolvedValue({
      artifact: sampleArtifact({
        flags: [[[RuleType.SERVE, undefined, 0]]],
      }),
      etag: '"v2"',
    });

    await coordinator.refresh('https://example.com/rules.ast', 'production', SDK_FLAGS);

    const value = resolveBooleanFlag({
      qualifiedName: 'new_dashboard',
      flagIndex: coordinator.getState().flagNameMap.new_dashboard,
      artifact: coordinator.getState().artifact!,
      catalogDefault: false,
      attributes: { id: 'user1' },
    });

    expect(value).toBe(true);
  });
});
