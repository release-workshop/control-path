/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Shared kill switch refresh and polling helpers (used by the generated SDK).
 */

import { loadKillSwitchFromURL, KillSwitchFileNotModifiedError } from './kill-switch-loader';
import type { KillSwitchFile, Logger } from './types';

export interface KillSwitchRefreshState {
  file: KillSwitchFile | null;
  etag?: string;
}

export type KillSwitchRefreshResult =
  | { status: 'updated'; state: KillSwitchRefreshState }
  | { status: 'not-modified'; state: KillSwitchRefreshState }
  | { status: 'failed'; state: KillSwitchRefreshState };

/**
 * Fetch kill switch JSON from a URL, retaining prior state on 304 or transient errors.
 *
 * Callers that run overlapping refreshes should use {@link KillSwitchRefreshCoordinator}
 * instead of assigning `result.state` directly (failed/not-modified results carry the
 * `prior` snapshot from call start and can race with a newer in-flight refresh).
 */
export async function refreshKillSwitchFromUrl(
  url: string,
  prior: KillSwitchRefreshState,
  timeoutMs?: number,
  logger?: Logger
): Promise<KillSwitchRefreshResult> {
  try {
    const result = await loadKillSwitchFromURL(url, prior.etag, timeoutMs, logger);
    const state: KillSwitchRefreshState = {
      file: result.killSwitchFile,
      etag: result.etag,
    };
    return { status: 'updated', state };
  } catch (error) {
    if (error instanceof KillSwitchFileNotModifiedError) {
      return { status: 'not-modified', state: prior };
    }
    logger?.warn(
      `Kill switch refresh failed for ${url}; keeping prior values`,
      error instanceof Error ? error : undefined
    );
    return { status: 'failed', state: prior };
  }
}

/**
 * Serializes kill switch URL refreshes and only commits state on successful updates.
 */
export class KillSwitchRefreshCoordinator {
  private state: KillSwitchRefreshState;

  private tail: Promise<void> = Promise.resolve();

  constructor(initial: KillSwitchRefreshState = { file: null }) {
    this.state = initial;
  }

  getState(): KillSwitchRefreshState {
    return this.state;
  }

  /** Replace in-memory state (e.g. when re-initializing the evaluator). */
  reset(initial: KillSwitchRefreshState = { file: null }): void {
    this.state = initial;
    this.tail = Promise.resolve();
  }

  refresh(url: string, timeoutMs?: number, logger?: Logger): Promise<KillSwitchRefreshResult> {
    const job = this.tail.then(() => this.runRefresh(url, timeoutMs, logger));
    this.tail = job.then(
      () => undefined,
      () => undefined
    );
    return job;
  }

  private async runRefresh(
    url: string,
    timeoutMs?: number,
    logger?: Logger
  ): Promise<KillSwitchRefreshResult> {
    const result = await refreshKillSwitchFromUrl(url, this.state, timeoutMs, logger);
    if (result.status === 'updated') {
      this.state = result.state;
    }
    return {
      status: result.status,
      state: this.state,
    };
  }
}

/** Options for {@link startKillSwitchPoll}. */
export interface KillSwitchPollOptions {
  /**
   * Maximum extra random delay (ms) added to each poll interval.
   * Actual delay is `intervalMs + floor(random() * (jitterMs + 1))`.
   * Default: `min(floor(intervalMs * 0.2), 15_000)`.
   */
  jitterMs?: number;
}

function defaultPollJitterMs(intervalMs: number): number {
  return Math.min(Math.floor(intervalMs * 0.2), 15_000);
}

/**
 * Schedule background kill switch refresh with jittered intervals (avoids thundering herd on CDN).
 *
 * Uses chained `setTimeout` rather than `setInterval` so each tick drifts independently.
 * Pair with an immediate or init-jittered first fetch separately if needed.
 */
export function startKillSwitchPoll(
  refresh: () => void | Promise<void>,
  intervalMs: number,
  options?: KillSwitchPollOptions
): () => void {
  const jitterMs = options?.jitterMs ?? defaultPollJitterMs(intervalMs);
  let cancelled = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const scheduleNext = (): void => {
    if (cancelled) {
      return;
    }
    const delay = intervalMs + Math.floor(Math.random() * (jitterMs + 1));
    timer = setTimeout(() => {
      void Promise.resolve(refresh()).finally(() => {
        if (!cancelled) {
          scheduleNext();
        }
      });
    }, delay);
  };

  scheduleNext();

  return () => {
    cancelled = true;
    if (timer !== undefined) {
      clearTimeout(timer);
    }
  };
}

/**
 * Random delay (ms) in `[0, maxMs)` for staggering the first kill switch fetch after deploy.
 */
export function killSwitchInitDelayMs(maxMs: number): number {
  if (maxMs <= 0) {
    return 0;
  }
  return Math.floor(Math.random() * maxMs);
}

/** Generic alias for {@link startKillSwitchPoll} (artifact and kill switch timers share the helper). */
export const startJitteredPoll = startKillSwitchPoll;

/** Generic alias for {@link killSwitchInitDelayMs}. */
export const pollInitDelayMs = killSwitchInitDelayMs;

/** Generic alias for {@link KillSwitchPollOptions}. */
export type JitteredPollOptions = KillSwitchPollOptions;
