/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Override loader module for loading override files from URL or file system.
 * Supports HTTP/HTTPS URLs with ETag support for efficient polling, and local file system access.
 */

import { readFile } from 'fs/promises';
import { resolve, normalize } from 'path';
import type { OverrideFile } from './types';

/**
 * Custom error class for when override file has not been modified (304 Not Modified)
 */
export class OverrideFileNotModifiedError extends Error {
  constructor() {
    super('Override file has not been modified since last request');
    this.name = 'OverrideFileNotModifiedError';
  }
}

/**
 * Maximum size for override files (1MB - override files should be small)
 */
const MAX_OVERRIDE_FILE_SIZE = 1024 * 1024;

/**
 * Default timeout for URL loading (10 seconds - override files should load quickly)
 */
const DEFAULT_URL_TIMEOUT = 10000;

/**
 * Maximum allowed timeout for URL loading (1 minute)
 */
const MAX_URL_TIMEOUT = 60 * 1000;

/**
 * Maximum number of redirects to follow (5)
 */
const MAX_REDIRECTS = 5;

/**
 * Validate and normalize a file path to prevent path traversal attacks
 * @param filePath - The file path to validate
 * @returns The normalized absolute path
 * @throws Error if path traversal is detected
 */
function validateFilePath(filePath: string): string {
  if (!filePath || typeof filePath !== 'string') {
    throw new Error('File path is required');
  }

  // Check for null bytes (potential injection)
  if (filePath.includes('\0')) {
    throw new Error('Null byte detected in file path');
  }

  // Normalize the path to resolve any . or .. components
  const normalized = normalize(filePath);

  // Check if normalized path still contains .. (shouldn't happen, but be safe)
  if (normalized.includes('..')) {
    throw new Error('Path traversal detected in file path');
  }

  // Resolve to absolute path
  const resolved = resolve(normalized);

  // Final check: ensure no .. components remain after resolution
  const pathParts = resolved.split(/[/\\]/);
  for (const part of pathParts) {
    if (part === '..') {
      throw new Error('Path traversal detected in file path');
    }
  }

  return resolved;
}

/**
 * Load override file from a local file path
 * @param filePath - The file path to load the override file from
 * @returns The loaded override file
 * @throws Error if the file cannot be read or the override file is invalid
 */
export async function loadOverrideFromFile(filePath: string): Promise<OverrideFile> {
  // Validate and normalize the path
  const validatedPath = validateFilePath(filePath);

  let content: string;
  try {
    content = await readFile(validatedPath, 'utf-8');
  } catch (error) {
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
      throw new Error(`Override file not found: ${filePath}`);
    }
    throw error;
  }

  // Basic size limit check
  if (content.length > MAX_OVERRIDE_FILE_SIZE) {
    throw new Error(
      `Override file too large: ${content.length} bytes (max: ${MAX_OVERRIDE_FILE_SIZE} bytes)`
    );
  }

  return parseOverrideFile(content);
}

/**
 * Result of loading an override file from URL, including ETag for conditional requests
 */
export interface OverrideLoadResult {
  /** The loaded override file */
  overrideFile: OverrideFile;
  /** ETag from response header (for conditional requests) */
  etag?: string;
}

/**
 * Load override file from a URL with ETag support for conditional requests
 * @param url - The URL to load the override file from
 * @param etag - Optional ETag from previous request (for conditional GET)
 * @param timeout - Request timeout in milliseconds (default: 10000, max: 1 minute)
 * @param logger - Optional logger for warnings and errors
 * @returns The loaded override file and ETag
 * @throws Error if the request fails, times out, or the override file is invalid
 */
export async function loadOverrideFromURL(
  url: string,
  etag?: string,
  timeout = DEFAULT_URL_TIMEOUT,
  logger?: {
    warn: (message: string, error?: Error) => void;
    error: (message: string, error?: Error) => void;
  }
): Promise<OverrideLoadResult> {
  // Basic URL validation
  try {
    new URL(url);
  } catch {
    throw new Error(`Invalid URL: ${url}`);
  }

  // Only allow http and https protocols
  if (!url.startsWith('http://') && !url.startsWith('https://')) {
    throw new Error(`Unsupported URL protocol. Only http:// and https:// are allowed: ${url}`);
  }

  // Cap timeout at maximum allowed
  const effectiveTimeout = Math.min(timeout, MAX_URL_TIMEOUT);

  // Follow redirects manually with limit
  let currentUrl = url;
  let redirectCount = 0;
  let response: Response | null = null;

  while (redirectCount <= MAX_REDIRECTS) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), effectiveTimeout);

    try {
      // Build headers with ETag if provided
      const headers: Record<string, string> = {};
      if (etag) {
        headers['If-None-Match'] = etag;
      }

      response = await fetch(currentUrl, {
        signal: controller.signal,
        redirect: 'manual', // Handle redirects manually
        headers,
      });
      clearTimeout(timeoutId);

      // Handle redirects
      if (response.status >= 300 && response.status < 400) {
        // Check if we've exceeded the redirect limit before processing
        if (redirectCount >= MAX_REDIRECTS) {
          throw new Error(`Too many redirects (max: ${MAX_REDIRECTS})`);
        }

        const location = response.headers.get('location');
        if (!location) {
          throw new Error(`Redirect without location header: ${response.status}`);
        }

        // Resolve relative redirects
        try {
          currentUrl = new URL(location, currentUrl).toString();
          redirectCount++;
          continue;
        } catch {
          throw new Error(`Invalid redirect URL: ${location}`);
        }
      }

      // Handle 304 Not Modified (file hasn't changed)
      if (response.status === 304) {
        // Return null to indicate no change (caller should keep existing override file)
        throw new OverrideFileNotModifiedError();
      }

      // Break out of loop if not a redirect
      break;
    } catch (error) {
      clearTimeout(timeoutId);
      if (error instanceof Error && error.name === 'AbortError') {
        throw new Error(
          `Timeout loading override file from URL ${url} after ${effectiveTimeout}ms`
        );
      }
      if (error instanceof OverrideFileNotModifiedError) {
        throw error; // Re-throw to be handled by caller
      }
      throw error;
    }
  }

  if (!response) {
    throw new Error(`Failed to load override file from URL ${url}`);
  }

  try {
    if (!response.ok) {
      throw new Error(
        `Failed to load override file from URL ${url}: ${response.status} ${response.statusText}`
      );
    }

    // Validate content type if available
    const contentType = response.headers.get('content-type');
    if (
      contentType &&
      !contentType.includes('application/json') &&
      !contentType.includes('text/json')
    ) {
      // Warn but don't fail - some servers may not set correct content type
      if (logger) {
        logger.warn(
          `Unexpected Content-Type for override file: ${contentType}. Expected application/json.`
        );
      }
    }

    const content = await response.text();

    // Basic size limit check
    if (content.length > MAX_OVERRIDE_FILE_SIZE) {
      throw new Error(
        `Override file too large: ${content.length} bytes (max: ${MAX_OVERRIDE_FILE_SIZE} bytes)`
      );
    }

    const overrideFile = parseOverrideFile(content);

    // Extract ETag from response headers
    const etag = response.headers.get('etag') || undefined;

    return {
      overrideFile,
      etag: etag ? etag : undefined,
    };
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      throw new Error(`Timeout loading override file from URL ${url} after ${effectiveTimeout}ms`);
    }
    if (error instanceof Error) {
      throw error;
    }
    throw new Error(`Unknown error loading override file from URL ${url}: ${String(error)}`);
  }
}

/**
 * Type guard to check if a value is an OverrideFile
 * @param value - Value to check
 * @returns True if value is a valid OverrideFile
 */
function isOverrideFile(value: unknown): value is OverrideFile {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return false;
  }

  const obj = value as Record<string, unknown>;

  // Validate version
  if (typeof obj.version !== 'string') {
    return false;
  }

  // Validate overrides object
  if (!obj.overrides || typeof obj.overrides !== 'object' || Array.isArray(obj.overrides)) {
    return false;
  }

  const overrides = obj.overrides as Record<string, unknown>;

  // Validate each override value (can be string or object with value field)
  for (const [flagName, overrideValue] of Object.entries(overrides)) {
    if (typeof flagName !== 'string' || flagName.length === 0) {
      return false;
    }

    // Override value can be:
    // 1. Simple string: "ON", "OFF", "V1", etc.
    // 2. Full object: { value: "ON", timestamp?: "...", reason?: "...", operator?: "..." }
    if (typeof overrideValue === 'string') {
      // Simple format - valid
      continue;
    } else if (
      overrideValue &&
      typeof overrideValue === 'object' &&
      !Array.isArray(overrideValue)
    ) {
      // Full format - must have value field
      const overrideObj = overrideValue as Record<string, unknown>;
      if (typeof overrideObj.value !== 'string') {
        return false;
      }
      // Optional fields: timestamp, reason, operator (all validated as strings if present)
      if (overrideObj.timestamp !== undefined && typeof overrideObj.timestamp !== 'string') {
        return false;
      }
      if (overrideObj.reason !== undefined && typeof overrideObj.reason !== 'string') {
        return false;
      }
      if (overrideObj.operator !== undefined && typeof overrideObj.operator !== 'string') {
        return false;
      }
    } else {
      return false;
    }
  }

  return true;
}

/**
 * Parse and validate override file JSON content
 * @param content - JSON string content
 * @returns The parsed and validated override file
 * @throws Error if JSON is invalid or structure doesn't match OverrideFile format
 */
function parseOverrideFile(content: string): OverrideFile {
  let parsed: unknown;
  try {
    parsed = JSON.parse(content);
  } catch (error) {
    throw new Error(
      `Failed to parse override file JSON: ${error instanceof Error ? error.message : String(error)}`
    );
  }

  // Validate structure using type guard
  if (!isOverrideFile(parsed)) {
    throw new Error('Invalid override file format: structure validation failed');
  }

  return parsed;
}
