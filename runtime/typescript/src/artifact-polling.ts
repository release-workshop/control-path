/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Compiled artifact refresh and polling helpers (used by the generated SDK).
 */

import { loadFromURL, ArtifactNotModifiedError, type LoadOptions } from './ast-loader';
import { buildFlagNameMapFromArtifact } from './utils';
import type { Artifact, Logger } from './types';

export interface ArtifactRefreshState {
  artifact: Artifact | null;
  flagNameMap: Record<string, number>;
  etag?: string;
}

export type ArtifactRefreshResult =
  | { status: 'updated'; state: ArtifactRefreshState }
  | { status: 'not-modified'; state: ArtifactRefreshState }
  | { status: 'rejected'; state: ArtifactRefreshState; reason: string }
  | { status: 'failed'; state: ArtifactRefreshState };

/** Flag names declared in the compiled artifact (string table indices). */
export function artifactFlagNames(artifact: { flagNames: number[]; strs: string[] }): string[] {
  return artifact.flagNames
    .map((nameIndex) => artifact.strs[nameIndex])
    .filter((name): name is string => typeof name === 'string' && name.length > 0);
}

/**
 * Whether a polled artifact may replace the in-memory copy.
 * Rejects environment mismatch and zero overlap with the generated SDK flag set.
 */
export function validateArtifactPoll(
  artifact: Artifact,
  expectedEnv: string,
  sdkQualifiedNames: ReadonlySet<string>
): { accepted: true } | { accepted: false; reason: string } {
  if (artifact.env !== expectedEnv) {
    return { accepted: false, reason: 'environment mismatch' };
  }

  const overlap = artifactFlagNames(artifact).some((name) => sdkQualifiedNames.has(name));
  if (!overlap) {
    return { accepted: false, reason: 'zero flag-name overlap with SDK' };
  }

  return { accepted: true };
}

/** Throws when {@link validateArtifactPoll} rejects the artifact (init and strict loaders). */
export function assertArtifactAccepted(
  artifact: Artifact,
  expectedEnv: string,
  sdkQualifiedNames: ReadonlySet<string>
): void {
  const validation = validateArtifactPoll(artifact, expectedEnv, sdkQualifiedNames);
  if (!validation.accepted) {
    throw new Error(`Compiled artifact rejected: ${validation.reason}`);
  }
}

/**
 * Expected environment for guardrails: `<env>.ast` path segment, catalog URL match, or `artifact.env`.
 */
export function resolveExpectedArtifactEnv(
  artifactSource: string,
  artifact: Artifact,
  artifactUrls: Readonly<Record<string, string>>
): string {
  const pathMatch = artifactSource.match(/(?:^|[/\\])([a-z][a-z0-9_-]*)\.ast$/i);
  if (pathMatch) {
    const envFromPath = pathMatch[1].toLowerCase();
    if (artifactUrls[envFromPath]) {
      return envFromPath;
    }
  }
  if (artifactUrls[artifact.env]) {
    return artifact.env;
  }
  for (const [env, url] of Object.entries(artifactUrls)) {
    if (artifactSource === url) {
      return env;
    }
  }
  return pathMatch ? pathMatch[1].toLowerCase() : artifact.env;
}

/**
 * Whether init should run poll guardrails for this environment.
 *
 * Returns false when the catalog has no `artifacts.<expectedEnv>.url` — workflows
 * without declared artifact URLs skip strict init validation (and artifact polling).
 */
export function shouldValidateArtifactAtInit(
  artifactUrls: Readonly<Record<string, string>>,
  expectedEnv: string
): boolean {
  return artifactUrls[expectedEnv] !== undefined;
}

/**
 * Fetch a compiled artifact from a URL, retaining prior state on 304 or transient errors.
 *
 * Pass `loadOptions.publicKey` / `requireSignature` when verifying SaaS-signed artifacts
 * (wired in generated SDK with issue 13; local-only catalogs typically omit signing).
 */
export async function refreshArtifactFromUrl(
  url: string,
  expectedEnv: string,
  sdkQualifiedNames: ReadonlySet<string>,
  prior: ArtifactRefreshState,
  timeoutMs?: number,
  logger?: Logger,
  loadOptions?: LoadOptions
): Promise<ArtifactRefreshResult> {
  try {
    const result = await loadFromURL(url, timeoutMs, logger, {
      ...loadOptions,
      etag: prior.etag,
    });

    const validation = validateArtifactPoll(result.artifact, expectedEnv, sdkQualifiedNames);
    if (!validation.accepted) {
      logger?.warn(
        `Artifact refresh rejected for ${url} (${validation.reason}); keeping prior artifact`
      );
      return {
        status: 'rejected',
        state: prior,
        reason: validation.reason,
      };
    }

    const state: ArtifactRefreshState = {
      artifact: result.artifact,
      etag: result.etag,
      flagNameMap: buildFlagNameMapFromArtifact(result.artifact),
    };
    return { status: 'updated', state };
  } catch (error) {
    if (error instanceof ArtifactNotModifiedError) {
      return { status: 'not-modified', state: prior };
    }
    logger?.warn(
      `Artifact refresh failed for ${url}; keeping prior artifact`,
      error instanceof Error ? error : undefined
    );
    return { status: 'failed', state: prior };
  }
}

/**
 * Serializes artifact URL refreshes and only commits state on successful updates.
 */
export class ArtifactRefreshCoordinator {
  private state: ArtifactRefreshState;

  private tail: Promise<void> = Promise.resolve();

  constructor(initial: ArtifactRefreshState = { artifact: null, flagNameMap: {} }) {
    this.state = initial;
  }

  getState(): ArtifactRefreshState {
    return this.state;
  }

  /** Replace in-memory state (e.g. after init from file or URL). */
  seed(initial: ArtifactRefreshState): void {
    this.state = initial;
    this.tail = Promise.resolve();
  }

  reset(initial: ArtifactRefreshState = { artifact: null, flagNameMap: {} }): void {
    this.state = initial;
    this.tail = Promise.resolve();
  }

  refresh(
    url: string,
    expectedEnv: string,
    sdkQualifiedNames: ReadonlySet<string>,
    timeoutMs?: number,
    logger?: Logger,
    loadOptions?: LoadOptions
  ): Promise<ArtifactRefreshResult> {
    const job = this.tail.then(() =>
      this.runRefresh(url, expectedEnv, sdkQualifiedNames, timeoutMs, logger, loadOptions)
    );
    this.tail = job.then(
      () => undefined,
      () => undefined
    );
    return job;
  }

  private async runRefresh(
    url: string,
    expectedEnv: string,
    sdkQualifiedNames: ReadonlySet<string>,
    timeoutMs?: number,
    logger?: Logger,
    loadOptions?: LoadOptions
  ): Promise<ArtifactRefreshResult> {
    const result = await refreshArtifactFromUrl(
      url,
      expectedEnv,
      sdkQualifiedNames,
      this.state,
      timeoutMs,
      logger,
      loadOptions
    );
    if (result.status === 'updated') {
      this.state = result.state;
    }
    if (result.status === 'rejected') {
      return { status: 'rejected', state: this.state, reason: result.reason };
    }
    return { status: result.status, state: this.state };
  }
}
