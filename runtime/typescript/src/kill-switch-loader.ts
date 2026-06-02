/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Kill switch file loader for v2 boolean runtime artifacts.
 */

import { readFile } from 'fs/promises';
import type { KillSwitchFile } from './types';
import { buildConditionalGetHeaders, fetchWithRedirects, validateFilePath } from './loader-utils';

/** Thrown when the remote kill switch file has not changed (HTTP 304). */
export class KillSwitchFileNotModifiedError extends Error {
  constructor() {
    super('Kill switch file has not been modified since last request');
    this.name = 'KillSwitchFileNotModifiedError';
  }
}

const MAX_KILL_SWITCH_FILE_SIZE = 1024 * 1024;
const DEFAULT_URL_TIMEOUT = 10000;
const MAX_URL_TIMEOUT = 60 * 1000;
const MAX_REDIRECTS = 5;

/** Load a kill switch file from a local path. */
export async function loadKillSwitchFromFile(filePath: string): Promise<KillSwitchFile> {
  const validatedPath = validateFilePath(filePath);

  let content: string;
  try {
    content = await readFile(validatedPath, 'utf-8');
  } catch (error) {
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
      throw new Error(`Kill switch file not found: ${filePath}`);
    }
    throw error;
  }

  if (content.length > MAX_KILL_SWITCH_FILE_SIZE) {
    throw new Error(
      `Kill switch file too large: ${content.length} bytes (max: ${MAX_KILL_SWITCH_FILE_SIZE} bytes)`
    );
  }

  return parseKillSwitchFile(content);
}

/** Result of loading a kill switch file from a URL. */
export interface KillSwitchLoadResult {
  killSwitchFile: KillSwitchFile;
  etag?: string;
}

/** Load a kill switch file from a URL with ETag support for conditional GET. */
export async function loadKillSwitchFromURL(
  url: string,
  etag?: string,
  timeout = DEFAULT_URL_TIMEOUT,
  logger?: {
    warn: (message: string, error?: Error) => void;
    error: (message: string, error?: Error) => void;
  }
): Promise<KillSwitchLoadResult> {
  const effectiveTimeout = Math.min(timeout, MAX_URL_TIMEOUT);

  try {
    const response = await fetchWithRedirects({
      url,
      effectiveTimeoutMs: effectiveTimeout,
      maxRedirects: MAX_REDIRECTS,
      headers: buildConditionalGetHeaders(etag),
    });

    if (response.status === 304) {
      throw new KillSwitchFileNotModifiedError();
    }

    if (!response.ok) {
      throw new Error(
        `Failed to load kill switch file from URL ${url}: ${response.status} ${response.statusText}`
      );
    }

    const contentType = response.headers.get('content-type');
    if (
      contentType &&
      !contentType.includes('application/json') &&
      !contentType.includes('text/json')
    ) {
      logger?.warn(
        `Unexpected Content-Type for kill switch file: ${contentType}. Expected application/json.`
      );
    }

    const content = await response.text();
    if (content.length > MAX_KILL_SWITCH_FILE_SIZE) {
      throw new Error(
        `Kill switch file too large: ${content.length} bytes (max: ${MAX_KILL_SWITCH_FILE_SIZE} bytes)`
      );
    }

    const killSwitchFile = parseKillSwitchFile(content);
    const responseEtag = response.headers.get('etag') || undefined;

    return {
      killSwitchFile,
      etag: responseEtag,
    };
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      throw new Error(
        `Timeout loading kill switch file from URL ${url} after ${effectiveTimeout}ms`
      );
    }
    if (error instanceof KillSwitchFileNotModifiedError) {
      throw error;
    }
    throw error;
  }
}

function isKillSwitchFile(value: unknown): value is KillSwitchFile {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return false;
  }

  const obj = value as Record<string, unknown>;
  if (typeof obj.version !== 'string') {
    return false;
  }

  if (!obj.flags || typeof obj.flags !== 'object' || Array.isArray(obj.flags)) {
    return false;
  }

  const flags = obj.flags as Record<string, unknown>;
  for (const [flagName, flagValue] of Object.entries(flags)) {
    if (typeof flagName !== 'string' || flagName.length === 0) {
      return false;
    }
    if (typeof flagValue !== 'boolean') {
      return false;
    }
  }

  return true;
}

function parseKillSwitchFile(content: string): KillSwitchFile {
  let parsed: unknown;
  try {
    parsed = JSON.parse(content);
  } catch (error) {
    throw new Error(
      `Failed to parse kill switch file JSON: ${error instanceof Error ? error.message : String(error)}`
    );
  }

  if (!isKillSwitchFile(parsed)) {
    throw new Error('Invalid kill switch file format: structure validation failed');
  }

  return parsed;
}
