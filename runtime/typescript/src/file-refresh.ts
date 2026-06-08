/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Shared filesystem refresh helpers (mtime + size poll, last-good on failure).
 */

import { stat } from 'fs/promises';
import type { Logger } from './types';

export interface FileFingerprint {
  mtimeMs: number;
  size: number;
}

export type FileRefreshResult<T> =
  | { status: 'updated'; value: T; fingerprint: FileFingerprint }
  | { status: 'not-modified' }
  | { status: 'failed'; value: T | undefined };

function fingerprintsMatch(prior: FileFingerprint | undefined, current: FileFingerprint): boolean {
  return prior !== undefined && prior.mtimeMs === current.mtimeMs && prior.size === current.size;
}

/**
 * Poll a file by mtime/size; load only when changed. Keeps `priorValue` on missing file or load error.
 */
export async function refreshFromFile<T>(
  filePath: string,
  priorFingerprint: FileFingerprint | undefined,
  load: (path: string) => Promise<T>,
  logger?: Logger,
  priorValue?: T
): Promise<FileRefreshResult<T>> {
  let fingerprint: FileFingerprint;
  try {
    const stats = await stat(filePath);
    fingerprint = {
      mtimeMs: stats.mtimeMs,
      size: stats.size,
    };
  } catch (error) {
    logger?.warn(
      `File refresh failed for ${filePath}; keeping prior values`,
      error instanceof Error ? error : undefined
    );
    return { status: 'failed', value: priorValue };
  }

  if (fingerprintsMatch(priorFingerprint, fingerprint)) {
    return { status: 'not-modified' };
  }

  try {
    const value = await load(filePath);
    return { status: 'updated', value, fingerprint };
  } catch (error) {
    logger?.warn(
      `File refresh failed for ${filePath}; keeping prior values`,
      error instanceof Error ? error : undefined
    );
    return { status: 'failed', value: priorValue };
  }
}
