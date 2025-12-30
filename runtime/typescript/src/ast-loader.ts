/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * AST loader module for loading AST artifacts from file, URL, or Buffer.
 * This module handles MessagePack decoding and AST structure validation.
 */

import { unpack, pack } from 'msgpackr';
import { readFile } from 'fs/promises';
import { resolve, normalize } from 'path';
import { verify } from '@noble/ed25519';
import type { Artifact } from './types';

/**
 * Maximum size for AST artifacts (10MB)
 */
const MAX_ARTIFACT_SIZE = 10 * 1024 * 1024;

/**
 * Default timeout for URL loading (30 seconds)
 */
const DEFAULT_URL_TIMEOUT = 30000;

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

  // Check for null bytes (potential injection) - check early
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
  // This is the key security check - after resolution, there should be no .. components
  // Split and check each part
  const pathParts = resolved.split(/[/\\]/);
  for (const part of pathParts) {
    if (part === '..') {
      throw new Error('Path traversal detected in file path');
    }
  }

  return resolved;
}

/**
 * Load AST artifact from a local file path
 * @param filePath - The file path to load the AST artifact from
 * @param options - Optional loading options including signature verification
 * @returns The loaded AST artifact
 * @throws Error if the file cannot be read or the artifact is invalid
 */
export async function loadFromFile(filePath: string, options?: LoadOptions): Promise<Artifact> {
  // Validate and normalize the path
  const validatedPath = validateFilePath(filePath);

  let buffer: Buffer;
  try {
    buffer = await readFile(validatedPath);
  } catch (error) {
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
      throw new Error(`AST file not found: ${filePath}`);
    }
    throw error;
  }

  // Basic size limit check
  if (buffer.length > MAX_ARTIFACT_SIZE) {
    throw new Error(
      `AST artifact too large: ${buffer.length} bytes (max: ${MAX_ARTIFACT_SIZE} bytes)`
    );
  }

  return await loadFromBuffer(buffer, options);
}

/**
 * Load AST artifact from a URL
 * @param url - The URL to load the AST artifact from
 * @param timeout - Request timeout in milliseconds (default: 30000)
 * @param logger - Optional logger for warnings
 * @param options - Optional loading options including signature verification
 * @returns The loaded AST artifact
 * @throws Error if the request fails, times out, or the artifact is invalid
 */
export async function loadFromURL(
  url: string,
  timeout = DEFAULT_URL_TIMEOUT,
  logger?: { warn: (message: string) => void },
  options?: LoadOptions
): Promise<Artifact> {
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

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  try {
    const response = await fetch(url, { signal: controller.signal });
    clearTimeout(timeoutId);

    if (!response.ok) {
      throw new Error(
        `Failed to load AST from URL ${url}: ${response.status} ${response.statusText}`
      );
    }

    // Validate content type if available
    const contentType = response.headers.get('content-type');
    if (
      contentType &&
      !contentType.includes('application/octet-stream') &&
      !contentType.includes('application/x-msgpack')
    ) {
      // Warn but don't fail - some servers may not set correct content type
      if (logger) {
        logger.warn(
          `Unexpected Content-Type for AST artifact: ${contentType}. Expected application/octet-stream or application/x-msgpack.`
        );
      }
    }

    const arrayBuffer = await response.arrayBuffer();

    // Basic size limit check
    if (arrayBuffer.byteLength > MAX_ARTIFACT_SIZE) {
      throw new Error(
        `AST artifact too large: ${arrayBuffer.byteLength} bytes (max: ${MAX_ARTIFACT_SIZE} bytes)`
      );
    }

    const buffer = Buffer.from(arrayBuffer);
    return loadFromBuffer(buffer, options);
  } catch (error) {
    clearTimeout(timeoutId);
    if (error instanceof Error && error.name === 'AbortError') {
      throw new Error(`Timeout loading AST from URL ${url} after ${timeout}ms`);
    }
    if (error instanceof Error) {
      throw error;
    }
    throw new Error(`Unknown error loading AST from URL ${url}: ${String(error)}`);
  }
}

/**
 * Options for loading AST artifacts with signature verification
 */
export interface LoadOptions {
  /** Optional public key for Ed25519 signature verification (base64 or hex encoded) */
  publicKey?: string | Uint8Array;
  /** Whether to require a signature (default: false - signature is optional) */
  requireSignature?: boolean;
}

/**
 * Load AST artifact from a Buffer
 * @param buffer - The buffer containing the MessagePack-encoded AST artifact
 * @param options - Optional loading options including signature verification
 * @returns The loaded AST artifact
 * @throws Error if the artifact is invalid or signature verification fails
 */
export async function loadFromBuffer(buffer: Buffer, options?: LoadOptions): Promise<Artifact> {
  const artifact: unknown = unpack(buffer);
  const validatedArtifact = validateArtifact(artifact);

  // Verify signature if public key is provided
  if (options?.publicKey) {
    await verifySignature(validatedArtifact, buffer, options.publicKey, options.requireSignature);
  } else if (options?.requireSignature && !validatedArtifact.sig) {
    throw new Error('Signature required but not present in artifact');
  }

  return validatedArtifact;
}

/**
 * Validate that the loaded data is a valid Artifact structure
 */
function validateArtifact(value: unknown): Artifact {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('Invalid AST format: expected object');
  }

  const artifact = value as Record<string, unknown>;

  if (typeof artifact.v !== 'string') {
    throw new Error('Invalid AST format: missing or invalid version');
  }

  if (typeof artifact.env !== 'string') {
    throw new Error('Invalid AST format: missing or invalid environment');
  }

  if (!Array.isArray(artifact.strs) || !artifact.strs.every((s) => typeof s === 'string')) {
    throw new Error('Invalid AST format: missing or invalid string table');
  }

  if (!Array.isArray(artifact.flags)) {
    throw new Error('Invalid AST format: missing or invalid flags array');
  }

  // At this point, we've validated all required fields
  // We've validated the structure, so we can safely assert types
  // Since we've validated, we know the types are correct
  // Use 'as unknown as' to properly narrow from Record<string, unknown> to Artifact
  const validatedArtifact = artifact as unknown as Artifact;

  const result: Artifact = {
    v: validatedArtifact.v,
    env: validatedArtifact.env,
    strs: validatedArtifact.strs,
    flags: validatedArtifact.flags,
  };

  // Add optional fields if present
  if (validatedArtifact.segments !== undefined) {
    result.segments = validatedArtifact.segments;
  }

  if (validatedArtifact.sig !== undefined) {
    result.sig = validatedArtifact.sig;
  }

  return result;
}

/**
 * Verify Ed25519 signature of an artifact.
 * The signature is computed over the MessagePack bytes of the artifact without the signature field.
 *
 * @param artifact - The validated artifact
 * @param originalBuffer - The original MessagePack buffer (may include signature)
 * @param publicKey - Public key for verification (base64, hex, or Uint8Array)
 * @param requireSignature - Whether to require a signature (default: false)
 * @throws Error if signature verification fails
 */
async function verifySignature(
  artifact: Artifact,
  originalBuffer: Buffer,
  publicKey: string | Uint8Array,
  requireSignature = false
): Promise<void> {
  // If no signature present and not required, skip verification
  if (!artifact.sig) {
    if (requireSignature) {
      throw new Error('Signature required but not present in artifact');
    }
    return;
  }

  // Normalize public key to Uint8Array
  let publicKeyBytes: Uint8Array;
  if (typeof publicKey === 'string') {
    // Try base64 first, then hex
    try {
      publicKeyBytes = Buffer.from(publicKey, 'base64');
      if (publicKeyBytes.length !== 32) {
        // Not base64, try hex
        publicKeyBytes = Buffer.from(publicKey, 'hex');
      }
    } catch {
      // Try hex
      publicKeyBytes = Buffer.from(publicKey, 'hex');
    }
  } else {
    publicKeyBytes = publicKey;
  }

  if (publicKeyBytes.length !== 32) {
    throw new Error(`Invalid public key length: expected 32 bytes, got ${publicKeyBytes.length}`);
  }

  // Normalize signature to Uint8Array
  // MessagePack may decode Uint8Array as Buffer, so handle both
  let signatureBytes: Uint8Array;
  if (artifact.sig instanceof Uint8Array) {
    signatureBytes = artifact.sig;
  } else if (Buffer.isBuffer(artifact.sig)) {
    signatureBytes = new Uint8Array(artifact.sig);
  } else if (Array.isArray(artifact.sig)) {
    signatureBytes = new Uint8Array(artifact.sig);
  } else {
    throw new Error('Invalid signature format');
  }

  if (signatureBytes.length !== 64) {
    throw new Error(`Invalid signature length: expected 64 bytes, got ${signatureBytes.length}`);
  }

  // Reconstruct artifact without signature for verification
  const artifactWithoutSig: Omit<Artifact, 'sig'> = {
    v: artifact.v,
    env: artifact.env,
    strs: artifact.strs,
    flags: artifact.flags,
  };
  if (artifact.segments) {
    artifactWithoutSig.segments = artifact.segments;
  }

  // Serialize artifact without signature
  const messageBytes = pack(artifactWithoutSig);

  // Verify signature (verify is async)
  try {
    const isValid = await verify(signatureBytes, messageBytes, publicKeyBytes);
    if (!isValid) {
      throw new Error('Signature verification failed: invalid signature');
    }
  } catch (error) {
    if (error instanceof Error && error.message.includes('verification failed')) {
      throw error;
    }
    throw new Error(
      `Signature verification failed: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}
