/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Kill switch file loader for v2 boolean runtime artifacts.
 */

import { readFile } from 'fs/promises';
import { resolve, normalize } from 'path';
import type { KillSwitchFile } from './types';

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

function validateFilePath(filePath: string): string {
  if (!filePath || typeof filePath !== 'string') {
    throw new Error('File path is required');
  }

  if (filePath.includes('\0')) {
    throw new Error('Null byte detected in file path');
  }

  const normalized = normalize(filePath);
  if (normalized.includes('..')) {
    throw new Error('Path traversal detected in file path');
  }

  const resolved = resolve(normalized);
  const pathParts = resolved.split(/[/\\]/);
  for (const part of pathParts) {
    if (part === '..') {
      throw new Error('Path traversal detected in file path');
    }
  }

  return resolved;
}

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
  try {
    new URL(url);
  } catch {
    throw new Error(`Invalid URL: ${url}`);
  }

  if (!url.startsWith('http://') && !url.startsWith('https://')) {
    throw new Error(`Unsupported URL protocol. Only http:// and https:// are allowed: ${url}`);
  }

  const effectiveTimeout = Math.min(timeout, MAX_URL_TIMEOUT);
  let currentUrl = url;
  let redirectCount = 0;
  let response: Response | null = null;

  while (redirectCount <= MAX_REDIRECTS) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), effectiveTimeout);

    try {
      const headers: Record<string, string> = {};
      if (etag) {
        headers['If-None-Match'] = etag;
      }

      response = await fetch(currentUrl, {
        signal: controller.signal,
        redirect: 'manual',
        headers,
      });
      clearTimeout(timeoutId);

      if (response.status === 304) {
        throw new KillSwitchFileNotModifiedError();
      }

      if (response.status >= 300 && response.status < 400) {
        if (redirectCount >= MAX_REDIRECTS) {
          throw new Error(`Too many redirects (max: ${MAX_REDIRECTS})`);
        }

        const location = response.headers.get('location');
        if (!location) {
          throw new Error(`Redirect without location header: ${response.status}`);
        }

        currentUrl = new URL(location, currentUrl).toString();
        redirectCount++;
        continue;
      }

      break;
    } catch (error) {
      clearTimeout(timeoutId);
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

  if (!response) {
    throw new Error(`Failed to load kill switch file from URL ${url}`);
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
