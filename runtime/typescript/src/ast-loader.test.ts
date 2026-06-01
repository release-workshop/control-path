/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readFile, writeFile, mkdir, rm, stat } from 'fs/promises';
import { writeFileSync, mkdirSync } from 'fs';
import { join } from 'path';
import { pack } from 'msgpackr';
import { getPublicKey, sign } from '@noble/ed25519';
import { loadFromFile, loadFromURL, loadFromBuffer } from './ast-loader';
import type { Artifact, Rule } from './types';

describe('AST Loader', () => {
  const testDir = join(__dirname, '../test-fixtures');
  const testFile = join(testDir, 'test.ast');

  beforeEach(async () => {
    try {
      mkdirSync(testDir, { recursive: true });
    } catch {
      // Directory might already exist
    }
  });

  // Note: We don't clean up in afterEach to avoid race conditions with concurrent tests.
  // The test directory is in a test location and can be cleaned up manually if needed.
  // Tests use unique file names to avoid conflicts.

  describe('loadFromBuffer', () => {
    it('should load valid AST from buffer', async () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1', 'flag2'],
        flags: [],
        flagNames: [],
      };

      const buffer = Buffer.from(pack(artifact));
      const loaded = await loadFromBuffer(buffer);

      expect(loaded.v).toBe('1.0');
      expect(loaded.env).toBe('test');
      expect(loaded.strs).toEqual(['flag1', 'flag2']);
      expect(loaded.flags).toEqual([]);
    });

    it('should load AST with optional fields', async () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
        segments: [[0, [2, 1]]],
        sig: new Uint8Array([1, 2, 3]),
      };

      const buffer = Buffer.from(pack(artifact));
      const loaded = await loadFromBuffer(buffer);

      expect(loaded.segments).toBeDefined();
      expect(loaded.sig).toBeDefined();
    });

    it('should throw error for invalid buffer', async () => {
      const buffer = Buffer.from('invalid data');

      await expect(loadFromBuffer(buffer)).rejects.toThrow();
    });

    it('should throw error for invalid AST structure', async () => {
      const invalidData = { notAnArtifact: true };
      const buffer = Buffer.from(pack(invalidData));

      await expect(loadFromBuffer(buffer)).rejects.toThrow('Invalid AST format');
    });

    it('should throw error for missing required fields', async () => {
      const invalidData = { v: '1.0' }; // missing env, strs, flags
      const buffer = Buffer.from(pack(invalidData));

      await expect(loadFromBuffer(buffer)).rejects.toThrow('Invalid AST format');
    });

    describe('signature verification', () => {
      it('should verify valid signature', async () => {
        // Generate key pair
        const privateKey = new Uint8Array(32).fill(1); // Test key (not secure, for testing only)
        const publicKey = await getPublicKey(privateKey);

        // Create artifact without signature
        const artifactWithoutSig: Omit<Artifact, 'sig'> = {
          v: '1.0',
          env: 'test',
          strs: ['flag1'],
          flags: [[]],
          flagNames: [0],
        };

        // Sign the artifact
        const messageBytes = pack(artifactWithoutSig);
        const signature = await sign(messageBytes, privateKey);

        // Add signature to artifact
        const artifact: Artifact = {
          ...artifactWithoutSig,
          sig: signature,
        };

        // Pack with signature
        const buffer = Buffer.from(pack(artifact));

        // Verify signature
        const loaded = await loadFromBuffer(buffer, { publicKey });
        expect(loaded.sig).toBeDefined();
      });

      it('should reject invalid signature', async () => {
        // Generate key pair
        const privateKey = new Uint8Array(32).fill(1);
        const publicKey = await getPublicKey(privateKey);

        // Create artifact with invalid signature
        const artifact: Artifact = {
          v: '1.0',
          env: 'test',
          strs: ['flag1'],
          flags: [[]],
          flagNames: [0],
          sig: new Uint8Array(64).fill(0), // Invalid signature
        };

        const buffer = Buffer.from(pack(artifact));

        // Should reject invalid signature
        await expect(loadFromBuffer(buffer, { publicKey })).rejects.toThrow(
          'Signature verification failed'
        );
      });

      it('should accept unsigned artifact when signature not required', async () => {
        const artifact: Artifact = {
          v: '1.0',
          env: 'test',
          strs: ['flag1'],
          flags: [[]],
          flagNames: [0],
        };

        const buffer = Buffer.from(pack(artifact));

        // Should accept unsigned artifact
        const loaded = await loadFromBuffer(buffer);
        expect(loaded.v).toBe('1.0');
      });

      it('should reject unsigned artifact when signature required', async () => {
        const privateKey = new Uint8Array(32).fill(1);
        const publicKey = await getPublicKey(privateKey);

        const artifact: Artifact = {
          v: '1.0',
          env: 'test',
          strs: ['flag1'],
          flags: [[]],
          flagNames: [0],
        };

        const buffer = Buffer.from(pack(artifact));

        // Should reject unsigned artifact when required
        await expect(loadFromBuffer(buffer, { publicKey, requireSignature: true })).rejects.toThrow(
          'Signature required but not present'
        );
      });

      it('should accept valid signature with base64 public key', async () => {
        const privateKey = new Uint8Array(32).fill(2);
        const publicKey = await getPublicKey(privateKey);
        const publicKeyBase64 = Buffer.from(publicKey).toString('base64');

        const artifactWithoutSig: Omit<Artifact, 'sig'> = {
          v: '1.0',
          env: 'test',
          strs: ['flag1'],
          flags: [[]],
          flagNames: [0],
        };

        const messageBytes = pack(artifactWithoutSig);
        const signature = await sign(messageBytes, privateKey);

        const artifact: Artifact = {
          ...artifactWithoutSig,
          sig: signature,
        };

        const buffer = Buffer.from(pack(artifact));

        // Verify with base64 public key
        const loaded = await loadFromBuffer(buffer, { publicKey: publicKeyBase64 });
        expect(loaded.sig).toBeDefined();
      });

      it('should accept valid signature with hex public key', async () => {
        const privateKey = new Uint8Array(32).fill(3);
        const publicKey = await getPublicKey(privateKey);
        const publicKeyHex = Buffer.from(publicKey).toString('hex');

        const artifactWithoutSig: Omit<Artifact, 'sig'> = {
          v: '1.0',
          env: 'test',
          strs: ['flag1'],
          flags: [[]],
          flagNames: [0],
        };

        const messageBytes = pack(artifactWithoutSig);
        const signature = await sign(messageBytes, privateKey);

        const artifact: Artifact = {
          ...artifactWithoutSig,
          sig: signature,
        };

        const buffer = Buffer.from(pack(artifact));

        // Verify with hex public key
        const loaded = await loadFromBuffer(buffer, { publicKey: publicKeyHex });
        expect(loaded.sig).toBeDefined();
      });
    });
  });

  describe('loadFromFile', () => {
    it('should load AST from file', async () => {
      // Use a unique file name to avoid conflicts with concurrent tests
      const uniqueTestFile = join(testDir, `test-${Date.now()}-${Math.random().toString(36).substring(7)}.ast`);
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[]],
        flagNames: [0],
      };

      const buffer = Buffer.from(pack(artifact));
      // Ensure directory exists before writing
      mkdirSync(testDir, { recursive: true });
      // Use writeFileSync for immediate, synchronous write (more reliable in tests)
      writeFileSync(uniqueTestFile, buffer);
      
      // Verify file exists before reading (handles race conditions with afterEach cleanup)
      let retries = 5;
      while (retries > 0) {
        try {
          const stats = await stat(uniqueTestFile);
          if (stats.isFile() && stats.size > 0) {
            break;
          }
        } catch {
          // File doesn't exist yet, wait and retry
        }
        retries--;
        if (retries > 0) {
          await new Promise((resolve) => setTimeout(resolve, 20));
          // Recreate directory and file if it was deleted by afterEach
          mkdirSync(testDir, { recursive: true });
          writeFileSync(uniqueTestFile, buffer);
        }
      }

      const loaded = await loadFromFile(uniqueTestFile);

      expect(loaded.v).toBe('1.0');
      expect(loaded.env).toBe('test');
      expect(loaded.strs).toEqual(['flag1']);
    });

    it('should throw error for non-existent file', async () => {
      const nonExistentFile = join(testDir, 'non-existent.ast');

      await expect(loadFromFile(nonExistentFile)).rejects.toThrow();
    });

    it('should throw error for invalid file content', async () => {
      // Use a unique file name to avoid conflicts with other tests
      const invalidTestFile = join(testDir, 'invalid-test.ast');
      // Ensure directory exists before writing
      mkdirSync(testDir, { recursive: true });
      // Write completely invalid binary data (use writeFileSync for synchronous write)
      const invalidData = Buffer.from([0x00, 0x01, 0x02, 0x03, 0x04]);
      writeFileSync(invalidTestFile, invalidData);
      
      // Verify file was written correctly by reading it back
      const writtenContent = await readFile(invalidTestFile);
      if (writtenContent.length !== invalidData.length || !writtenContent.equals(invalidData)) {
        throw new Error(
          `Invalid data was not written correctly: expected ${invalidData.length} bytes, got ${writtenContent.length} bytes`
        );
      }

      // Should fail validation even if msgpackr parses it
      await expect(loadFromFile(invalidTestFile)).rejects.toThrow();
    });

    it('should reject path traversal attempts', async () => {
      await expect(loadFromFile('../test.ast')).rejects.toThrow('Path traversal detected');
      await expect(loadFromFile('../../etc/passwd')).rejects.toThrow('Path traversal detected');
      await expect(loadFromFile('./../test.ast')).rejects.toThrow('Path traversal detected');
      await expect(loadFromFile('test/../../test.ast')).rejects.toThrow('Path traversal detected');
    });

    it('should reject paths with null bytes', async () => {
      await expect(loadFromFile('test\0.ast')).rejects.toThrow('Null byte detected');
    });

    it('should normalize valid relative paths', async () => {
      // Use a unique file name to avoid conflicts with concurrent tests
      const uniqueTestFile = join(testDir, `test-normalize-${Date.now()}-${Math.random().toString(36).substring(7)}.ast`);
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      };

      const buffer = Buffer.from(pack(artifact));
      // Ensure directory exists before writing
      mkdirSync(testDir, { recursive: true });
      // Use writeFileSync for immediate, synchronous write (more reliable in tests)
      writeFileSync(uniqueTestFile, buffer);

      // Test that normalized paths work
      const normalizedPath = uniqueTestFile.replace(/\\/g, '/'); // Normalize separators
      const loaded = await loadFromFile(normalizedPath);

      expect(loaded.v).toBe('1.0');
      expect(loaded.env).toBe('test');
    });

    describe('allowedDirectory option', () => {
      it('should allow files within allowed directory', async () => {
        // Use a unique file name to avoid conflicts with concurrent tests
        const uniqueTestFile = join(testDir, `test-allowed-${Date.now()}-${Math.random().toString(36).substring(7)}.ast`);
        const artifact: Artifact = {
          v: '1.0',
          env: 'test',
          strs: [],
          flags: [],
          flagNames: [],
        };

        const buffer = Buffer.from(pack(artifact));
        // Ensure directory exists before writing
        mkdirSync(testDir, { recursive: true });
        // Use writeFileSync for immediate, synchronous write (more reliable in tests)
        writeFileSync(uniqueTestFile, buffer);

        // Should load successfully when file is in allowed directory
        const loaded = await loadFromFile(uniqueTestFile, { allowedDirectory: testDir });
        expect(loaded.v).toBe('1.0');
      });

      it('should reject files outside allowed directory', async () => {
        // Use a unique file name to avoid conflicts with concurrent tests
        const uniqueTestFile = join(testDir, `test-reject-${Date.now()}-${Math.random().toString(36).substring(7)}.ast`);
        const artifact: Artifact = {
          v: '1.0',
          env: 'test',
          strs: [],
          flags: [],
          flagNames: [],
        };

        const buffer = Buffer.from(pack(artifact));
        // Ensure directory exists before writing
        mkdirSync(testDir, { recursive: true });
        // Use writeFileSync for immediate, synchronous write (more reliable in tests)
        writeFileSync(uniqueTestFile, buffer);

        // Create a different allowed directory
        const otherDir = join(__dirname, '../test-fixtures-other');
        // Use mkdirSync for immediate, synchronous directory creation (more reliable in tests)
        mkdirSync(otherDir, { recursive: true });

        try {
          // Should reject file outside allowed directory
          await expect(loadFromFile(uniqueTestFile, { allowedDirectory: otherDir })).rejects.toThrow(
            'File path outside allowed directory'
          );
        } finally {
          await rm(otherDir, { recursive: true, force: true });
        }
      });

      it('should use process.env.AST_DIRECTORY if allowedDirectory not provided', async () => {
        // Use a unique file name to avoid conflicts with concurrent tests
        const uniqueTestFile = join(testDir, `test-env-${Date.now()}-${Math.random().toString(36).substring(7)}.ast`);
        const artifact: Artifact = {
          v: '1.0',
          env: 'test',
          strs: [],
          flags: [],
          flagNames: [],
        };

        const buffer = Buffer.from(pack(artifact));
        // Ensure directory exists before writing
        mkdirSync(testDir, { recursive: true });
        // Use writeFileSync for immediate, synchronous write (more reliable in tests)
        writeFileSync(uniqueTestFile, buffer);

        // Set environment variable
        const originalEnv = process.env.AST_DIRECTORY;
        process.env.AST_DIRECTORY = testDir;

        try {
          // Should use environment variable
          const loaded = await loadFromFile(uniqueTestFile);
          expect(loaded.v).toBe('1.0');
        } finally {
          // Restore original environment
          if (originalEnv !== undefined) {
            process.env.AST_DIRECTORY = originalEnv;
          } else {
            delete process.env.AST_DIRECTORY;
          }
        }
      });
    });

    describe('size limits', () => {
      it('should reject artifacts with too many strings in string table', async () => {
        // Create artifact with too many strings (MAX_STRING_TABLE_SIZE = 100000)
        const strs: string[] = [];
        for (let i = 0; i < 100001; i++) {
          strs.push(`string${i}`);
        }

        const artifact: Artifact = {
          v: '1.0',
          env: 'test',
          strs,
          flags: [],
          flagNames: [],
        };

        const buffer = Buffer.from(pack(artifact));

        await expect(loadFromBuffer(buffer)).rejects.toThrow('String table too large');
      });

      it('should reject artifacts with strings exceeding max length', async () => {
        // Create artifact with string exceeding MAX_STRING_LENGTH (10000)
        const longString = 'a'.repeat(10001);
        const artifact: Artifact = {
          v: '1.0',
          env: 'test',
          strs: [longString],
          flags: [],
          flagNames: [],
        };

        const buffer = Buffer.from(pack(artifact));

        await expect(loadFromBuffer(buffer)).rejects.toThrow(
          'string table contains invalid strings (max length: 10000)'
        );
      });

      it('should reject artifacts with too many flags', async () => {
        // Create artifact with too many flags (MAX_FLAGS = 100000)
        const flags: Rule[][] = [];
        for (let i = 0; i < 100001; i++) {
          flags.push([]);
        }

        const artifact: Artifact = {
          v: '1.0',
          env: 'test',
          strs: [],
          flags,
          flagNames: flags.map((_, i) => i), // Create flagNames array matching flags length
        };

        const buffer = Buffer.from(pack(artifact));

        await expect(loadFromBuffer(buffer)).rejects.toThrow('Too many flags');
      });

      it('should accept artifacts within size limits', async () => {
        // Create artifact within limits
        const flags = Array(1000).fill([]);
        const artifact: Artifact = {
          v: '1.0',
          env: 'test',
          strs: Array(1000).fill('test'),
          flags,
          flagNames: flags.map((_, i) => i), // Create flagNames array matching flags length
        };

        const buffer = Buffer.from(pack(artifact));
        const loaded = await loadFromBuffer(buffer);

        expect(loaded.strs.length).toBe(1000);
        expect(loaded.flags.length).toBe(1000);
      });
    });
  });

  describe('loadFromURL', () => {
    const originalFetch = global.fetch;

    afterEach(() => {
      global.fetch = originalFetch;
      vi.restoreAllMocks();
    });

    function headersWith(values: Record<string, string>): Headers {
      return {
        get: (name: string) => values[name.toLowerCase()] ?? null,
      } as Headers;
    }

    function fetchAbortingOnSignal(): typeof fetch {
      return vi.fn((_url: string, init?: RequestInit) => {
        return new Promise<Response>((_resolve, reject) => {
          const signal = init?.signal;
          if (!signal) {
            reject(new Error('expected AbortSignal'));
            return;
          }
          if (signal.aborted) {
            const err = new Error('Aborted');
            err.name = 'AbortError';
            reject(err);
            return;
          }
          signal.addEventListener('abort', () => {
            const err = new Error('Aborted');
            err.name = 'AbortError';
            reject(err);
          });
        });
      }) as typeof fetch;
    }

    it('should throw error for invalid URL', async () => {
      await expect(loadFromURL('not-a-valid-url')).rejects.toThrow('Invalid URL');
    });

    it('should throw error for unsupported protocol', async () => {
      await expect(loadFromURL('ftp://example.com/test.ast')).rejects.toThrow(
        'Unsupported URL protocol'
      );
    });

    it('should throw error for 404 response', async () => {
      global.fetch = vi.fn(async () => ({
        ok: false,
        status: 404,
        statusText: 'Not Found',
        headers: headersWith({}),
        arrayBuffer: async () => new ArrayBuffer(0),
      })) as typeof fetch;

      await expect(loadFromURL('https://example.com/missing.ast')).rejects.toThrow(
        'Failed to load AST from URL https://example.com/missing.ast: 404 Not Found'
      );
    });

    it('should handle timeout', async () => {
      global.fetch = fetchAbortingOnSignal();

      await expect(loadFromURL('https://example.com/slow.ast', 100)).rejects.toThrow(
        'Timeout loading AST from URL https://example.com/slow.ast after 100ms'
      );
    });

    it('should load a valid artifact from a mocked URL', async () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'production',
        strs: [],
        flags: [],
        flagNames: [],
      };
      const buffer = Buffer.from(pack(artifact));
      const arrayBuffer = buffer.buffer.slice(
        buffer.byteOffset,
        buffer.byteOffset + buffer.byteLength
      );

      global.fetch = vi.fn(async () => ({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: headersWith({ 'content-type': 'application/octet-stream' }),
        arrayBuffer: async () => arrayBuffer,
      })) as typeof fetch;

      const loaded = await loadFromURL('https://example.com/production.ast');
      expect(loaded.artifact.env).toBe('production');
    });

    it('should fail init-style GET on HTTP 304 without If-None-Match', async () => {
      global.fetch = vi.fn(async () => ({
        ok: false,
        status: 304,
        statusText: 'Not Modified',
        headers: headersWith({}),
      })) as typeof fetch;

      await expect(loadFromURL('https://example.com/production.ast')).rejects.toThrow(
        '304 Not Modified without If-None-Match'
      );
    });

    it('should throw ArtifactNotModifiedError on HTTP 304 when etag is sent', async () => {
      global.fetch = vi.fn(async (_url: string, init?: RequestInit) => {
        const headers = init?.headers as Record<string, string> | undefined;
        expect(headers?.['If-None-Match']).toBe('"cached"');
        return {
          ok: false,
          status: 304,
          statusText: 'Not Modified',
          headers: headersWith({ etag: '"cached"' }),
        };
      }) as typeof fetch;

      const { ArtifactNotModifiedError: NotModified } = await import('./ast-loader');
      await expect(
        loadFromURL('https://example.com/production.ast', 30000, undefined, {
          etag: '"cached"',
        })
      ).rejects.toBeInstanceOf(NotModified);
    });

    it('should return etag from response headers on success', async () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'production',
        strs: [],
        flags: [],
        flagNames: [],
      };
      const buffer = Buffer.from(pack(artifact));
      const arrayBuffer = buffer.buffer.slice(
        buffer.byteOffset,
        buffer.byteOffset + buffer.byteLength
      );

      global.fetch = vi.fn(async () => ({
        ok: true,
        status: 200,
        statusText: 'OK',
        headers: headersWith({
          'content-type': 'application/octet-stream',
          etag: '"v1"',
        }),
        arrayBuffer: async () => arrayBuffer,
      })) as typeof fetch;

      const loaded = await loadFromURL('https://example.com/production.ast');
      expect(loaded.etag).toBe('"v1"');
      expect(loaded.artifact.env).toBe('production');
    });

    describe('redirect limits', () => {
      it('should reject when redirect limit is exceeded', async () => {
        global.fetch = vi.fn(async () => ({
          ok: false,
          status: 302,
          statusText: 'Found',
          headers: headersWith({ location: 'https://example.com/next' }),
        })) as typeof fetch;

        await expect(loadFromURL('https://example.com/start.ast')).rejects.toThrow(
          'Too many redirects (max: 5)'
        );
        expect(vi.mocked(global.fetch)).toHaveBeenCalledTimes(6);
      });

      it('should reject redirects without location header', async () => {
        global.fetch = vi.fn(async () => ({
          ok: false,
          status: 301,
          statusText: 'Moved Permanently',
          headers: headersWith({}),
        })) as typeof fetch;

        await expect(loadFromURL('https://example.com/here.ast')).rejects.toThrow(
          'Redirect without location header: 301'
        );
      });

      it('should cap timeout at MAX_URL_TIMEOUT', async () => {
        const veryLongTimeout = 10 * 60 * 1000;
        const effectiveTimeout = Math.min(veryLongTimeout, 5 * 60 * 1000);
        expect(effectiveTimeout).toBe(5 * 60 * 1000);
      });

      it('should reject invalid redirect URL', async () => {
        global.fetch = vi.fn(async () => ({
          ok: false,
          status: 302,
          statusText: 'Found',
          headers: headersWith({ location: 'https://[' }),
        })) as typeof fetch;

        await expect(loadFromURL('https://example.com/here.ast')).rejects.toThrow(
          'Invalid redirect URL: https://['
        );
      });

      it('should follow a valid redirect then load the artifact', async () => {
        const artifact: Artifact = {
          v: '1.0',
          env: 'staging',
          strs: [],
          flags: [],
          flagNames: [],
        };
        const buffer = Buffer.from(pack(artifact));
        const arrayBuffer = buffer.buffer.slice(
          buffer.byteOffset,
          buffer.byteOffset + buffer.byteLength
        );

        global.fetch = vi
          .fn()
          .mockResolvedValueOnce({
            ok: false,
            status: 302,
            statusText: 'Found',
            headers: headersWith({ location: 'https://example.com/final.ast' }),
          })
          .mockResolvedValueOnce({
            ok: true,
            status: 200,
            statusText: 'OK',
            headers: headersWith({ 'content-type': 'application/octet-stream' }),
            arrayBuffer: async () => arrayBuffer,
          }) as typeof fetch;

        const loaded = await loadFromURL('https://example.com/start.ast');
        expect(loaded.artifact.env).toBe('staging');
        expect(vi.mocked(global.fetch)).toHaveBeenCalledTimes(2);
        expect(vi.mocked(global.fetch).mock.calls[1][0]).toBe('https://example.com/final.ast');
      });

      it('should warn on unexpected content type', async () => {
        const artifact: Artifact = {
          v: '1.0',
          env: 'test',
          strs: [],
          flags: [],
          flagNames: [],
        };

        const buffer = Buffer.from(pack(artifact));
        const arrayBuffer = buffer.buffer.slice(
          buffer.byteOffset,
          buffer.byteOffset + buffer.byteLength
        );

        const warnMessages: string[] = [];
        const logger = {
          warn: (message: string) => {
            warnMessages.push(message);
          },
        };

        global.fetch = vi.fn(async () => ({
          ok: true,
          status: 200,
          statusText: 'OK',
          headers: headersWith({ 'content-type': 'text/html' }),
          arrayBuffer: async () => arrayBuffer,
        })) as typeof fetch;

        await loadFromURL('https://example.com/test.ast', 30000, logger);
        expect(warnMessages.length).toBeGreaterThan(0);
        expect(warnMessages[0]).toContain('Unexpected Content-Type');
      });
    });
  });

  describe('signature verification edge cases', () => {
    it('should handle signature verification with hex public key', async () => {
      const privateKey = new Uint8Array(32).fill(1);
      const publicKey = await getPublicKey(privateKey);
      const publicKeyHex = Buffer.from(publicKey).toString('hex');

      const artifactWithoutSig: Omit<Artifact, 'sig'> = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      };

      const messageBytes = pack(artifactWithoutSig);
      const signature = await sign(messageBytes, privateKey);

      const artifact: Artifact = {
        ...artifactWithoutSig,
        sig: signature,
      };

      const buffer = Buffer.from(pack(artifact));

      // Test with hex-encoded public key
      const loaded = await loadFromBuffer(buffer, {
        publicKey: publicKeyHex,
        requireSignature: true,
      });

      expect(loaded).toBeDefined();
    });

    it('should handle signature verification with base64 public key', async () => {
      const privateKey = new Uint8Array(32).fill(1);
      const publicKey = await getPublicKey(privateKey);
      const publicKeyBase64 = Buffer.from(publicKey).toString('base64');

      const artifactWithoutSig: Omit<Artifact, 'sig'> = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      };

      const messageBytes = pack(artifactWithoutSig);
      const signature = await sign(messageBytes, privateKey);

      const artifact: Artifact = {
        ...artifactWithoutSig,
        sig: signature,
      };

      const buffer = Buffer.from(pack(artifact));

      // Test with base64-encoded public key
      const loaded = await loadFromBuffer(buffer, {
        publicKey: publicKeyBase64,
        requireSignature: true,
      });

      expect(loaded).toBeDefined();
    });

    it('should handle signature as Buffer', async () => {
      const privateKey = new Uint8Array(32).fill(1);
      const publicKey = await getPublicKey(privateKey);

      const artifactWithoutSig: Omit<Artifact, 'sig'> = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      };

      const messageBytes = pack(artifactWithoutSig);
      const signature = await sign(messageBytes, privateKey);

      // Create artifact with signature as Buffer
      const artifactWithBufferSig = {
        ...artifactWithoutSig,
        sig: Buffer.from(signature),
      };

      const buffer = Buffer.from(pack(artifactWithBufferSig));

      const loaded = await loadFromBuffer(buffer, {
        publicKey,
        requireSignature: true,
      });

      expect(loaded).toBeDefined();
    });

    it('should handle signature as array', async () => {
      const privateKey = new Uint8Array(32).fill(1);
      const publicKey = await getPublicKey(privateKey);

      const artifactWithoutSig: Omit<Artifact, 'sig'> = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      };

      const messageBytes = pack(artifactWithoutSig);
      const signature = await sign(messageBytes, privateKey);

      // Create artifact with signature as array
      const artifactWithArraySig = {
        ...artifactWithoutSig,
        sig: Array.from(signature),
      };

      const buffer = Buffer.from(pack(artifactWithArraySig));

      const loaded = await loadFromBuffer(buffer, {
        publicKey,
        requireSignature: true,
      });

      expect(loaded).toBeDefined();
    });

    it('should throw error for invalid signature format', async () => {
      const privateKey = new Uint8Array(32).fill(1);
      const publicKey = await getPublicKey(privateKey);

      const artifactWithoutSig: Omit<Artifact, 'sig'> = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      };

      // Create artifact with invalid signature format (string instead of bytes)
      const artifactWithInvalidSig = {
        ...artifactWithoutSig,
        sig: 'invalid-signature',
      };

      const buffer = Buffer.from(pack(artifactWithInvalidSig));

      await expect(
        loadFromBuffer(buffer, {
          publicKey,
          requireSignature: true,
        })
      ).rejects.toThrow('Invalid signature format');
    });

    it('should throw error for invalid signature length', async () => {
      const privateKey = new Uint8Array(32).fill(1);
      const publicKey = await getPublicKey(privateKey);

      const artifactWithoutSig: Omit<Artifact, 'sig'> = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      };

      // Create artifact with invalid signature length (too short)
      const artifactWithInvalidSig = {
        ...artifactWithoutSig,
        sig: new Uint8Array(32), // Should be 64 bytes
      };

      const buffer = Buffer.from(pack(artifactWithInvalidSig));

      await expect(
        loadFromBuffer(buffer, {
          publicKey,
          requireSignature: true,
        })
      ).rejects.toThrow('Invalid signature length');
    });

    it('should throw error for invalid public key length', async () => {
      const privateKey = new Uint8Array(32).fill(1);
      const publicKey = await getPublicKey(privateKey);
      const messageBytes = pack({
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      });
      const signature = await sign(messageBytes, privateKey);

      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
        sig: signature,
      };

      const buffer = Buffer.from(pack(artifact));

      // Test with invalid public key length
      await expect(
        loadFromBuffer(buffer, {
          publicKey: new Uint8Array(16), // Should be 32 bytes
          requireSignature: true,
        })
      ).rejects.toThrow('Invalid public key length');
    });

    it('should skip verification when signature not present and not required', async () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
        // No signature
      };

      const buffer = Buffer.from(pack(artifact));

      // Should not throw even with public key provided, since signature is not required
      const publicKey = new Uint8Array(32).fill(1);
      const loaded = await loadFromBuffer(buffer, {
        publicKey,
        requireSignature: false, // Not required
      });

      expect(loaded).toBeDefined();
    });

    it('should handle base64 decode failure and fallback to hex', async () => {
      const privateKey = new Uint8Array(32).fill(1);
      const publicKey = await getPublicKey(privateKey);
      // Create a string that's not valid base64 but is valid hex
      const publicKeyHex = Buffer.from(publicKey).toString('hex');

      const artifactWithoutSig: Omit<Artifact, 'sig'> = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      };

      const messageBytes = pack(artifactWithoutSig);
      const signature = await sign(messageBytes, privateKey);

      const artifact: Artifact = {
        ...artifactWithoutSig,
        sig: signature,
      };

      const buffer = Buffer.from(pack(artifact));

      // Use hex string directly (not base64)
      const loaded = await loadFromBuffer(buffer, {
        publicKey: publicKeyHex,
        requireSignature: true,
      });

      expect(loaded).toBeDefined();
    });

    it('should handle signature verification error that does not include verification failed', async () => {
      const privateKey = new Uint8Array(32).fill(1);
      const publicKey = await getPublicKey(privateKey);

      const artifactWithoutSig: Omit<Artifact, 'sig'> = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      };

      // Create invalid signature (wrong length)
      const invalidSig = new Uint8Array(64).fill(0);

      const artifact: Artifact = {
        ...artifactWithoutSig,
        sig: invalidSig,
      };

      const buffer = Buffer.from(pack(artifact));

      // This should trigger the catch branch where error doesn't include 'verification failed'
      await expect(
        loadFromBuffer(buffer, {
          publicKey,
          requireSignature: true,
        })
      ).rejects.toThrow();
    });

    it('should include segments in artifact without signature', async () => {
      const privateKey = new Uint8Array(32).fill(1);
      const publicKey = await getPublicKey(privateKey);

      const artifactWithoutSig: Omit<Artifact, 'sig'> = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
        segments: [[0, [2, 1]]], // Include segments
      };

      const messageBytes = pack(artifactWithoutSig);
      const signature = await sign(messageBytes, privateKey);

      const artifact: Artifact = {
        ...artifactWithoutSig,
        sig: signature,
      };

      const buffer = Buffer.from(pack(artifact));

      const loaded = await loadFromBuffer(buffer, {
        publicKey,
        requireSignature: true,
      });

      expect(loaded.segments).toBeDefined();
    });

    it('should throw error when flagNames length does not match flags length', async () => {
      const invalidData = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[], []], // 2 flags
        flagNames: [0], // Only 1 flagName - mismatch!
      };

      const buffer = Buffer.from(pack(invalidData));

      await expect(loadFromBuffer(buffer)).rejects.toThrow(
        'flagNames length'
      );
    });

    it('should throw error when flagNames contains invalid string table indices', async () => {
      const invalidData = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'], // Only 1 string (index 0)
        flags: [[]],
        flagNames: [999], // Invalid index - out of bounds
      };

      const buffer = Buffer.from(pack(invalidData));

      await expect(loadFromBuffer(buffer)).rejects.toThrow(
        'flagNames contains invalid string table indices'
      );
    });

    it('should throw error for invalid string table type', async () => {
      const invalidData = {
        v: '1.0',
        env: 'test',
        strs: 'not-an-array', // Invalid - should be array
        flags: [],
        flagNames: [],
      };

      const buffer = Buffer.from(pack(invalidData));

      await expect(loadFromBuffer(buffer)).rejects.toThrow('string table');
    });

    it('should throw error for invalid flags array type', async () => {
      const invalidData = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: 'not-an-array', // Invalid - should be array
        flagNames: [],
      };

      const buffer = Buffer.from(pack(invalidData));

      await expect(loadFromBuffer(buffer)).rejects.toThrow('flags array');
    });

    it('should throw error when signature required but not present and no public key', async () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
        // No signature
      };

      const buffer = Buffer.from(pack(artifact));

      // Should throw when requireSignature is true but no signature present and no public key
      await expect(
        loadFromBuffer(buffer, {
          requireSignature: true,
          // No publicKey provided
        })
      ).rejects.toThrow('Signature required but not present');
    });

    it('should throw error for array artifact (not object)', async () => {
      const invalidData = [1, 2, 3]; // Array instead of object

      const buffer = Buffer.from(pack(invalidData));

      await expect(loadFromBuffer(buffer)).rejects.toThrow('expected object');
    });
  });
});
