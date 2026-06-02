/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { normalize, resolve } from 'path';

/**
 * Validate and normalize a local file path while preventing path traversal.
 */
export function validateFilePath(filePath: string, allowedDirectory?: string): string {
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

  const resolvedPath = resolve(normalized);
  const pathParts = resolvedPath.split(/[/\\]/);
  for (const part of pathParts) {
    if (part === '..') {
      throw new Error('Path traversal detected in file path');
    }
  }

  if (allowedDirectory) {
    const allowedPath = resolve(allowedDirectory);
    const resolvedNormalized = resolvedPath.replace(/[/\\]+/g, '/');
    const allowedNormalized = allowedPath.replace(/[/\\]+/g, '/');
    if (
      !resolvedNormalized.startsWith(allowedNormalized + '/') &&
      resolvedNormalized !== allowedNormalized
    ) {
      throw new Error('File path outside allowed directory');
    }
  }

  return resolvedPath;
}

/**
 * Build request headers for conditional GET (If-None-Match).
 */
export function buildConditionalGetHeaders(etag?: string): Record<string, string> {
  if (!etag) {
    return {};
  }
  return { 'If-None-Match': etag };
}

/**
 * Validate that a URL uses an allowed protocol.
 */
export function validateHttpUrl(url: string): void {
  try {
    new URL(url);
  } catch {
    throw new Error(`Invalid URL: ${url}`);
  }

  if (!url.startsWith('http://') && !url.startsWith('https://')) {
    throw new Error(`Unsupported URL protocol. Only http:// and https:// are allowed: ${url}`);
  }
}

interface FetchWithRedirectsOptions {
  url: string;
  effectiveTimeoutMs: number;
  maxRedirects: number;
  headers?: Record<string, string>;
}

/**
 * Fetch URL with manual redirect handling and timeout.
 */
export async function fetchWithRedirects(options: FetchWithRedirectsOptions): Promise<Response> {
  validateHttpUrl(options.url);

  let currentUrl = options.url;
  let redirectCount = 0;

  while (redirectCount <= options.maxRedirects) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), options.effectiveTimeoutMs);

    try {
      const response = await fetch(currentUrl, {
        signal: controller.signal,
        redirect: 'manual',
        headers: options.headers,
      });

      if (response.status >= 300 && response.status < 400 && response.status !== 304) {
        if (redirectCount >= options.maxRedirects) {
          throw new Error(`Too many redirects (max: ${options.maxRedirects})`);
        }

        const location = response.headers.get('location');
        if (!location) {
          throw new Error(`Redirect without location header: ${response.status}`);
        }

        try {
          currentUrl = new URL(location, currentUrl).toString();
        } catch {
          throw new Error(`Invalid redirect URL: ${location}`);
        }

        validateHttpUrl(currentUrl);

        redirectCount++;
        continue;
      }

      return response;
    } finally {
      clearTimeout(timeoutId);
    }
  }

  throw new Error(`Failed to fetch URL: ${options.url}`);
}
