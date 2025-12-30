/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile, writeFile, mkdir, rm } from 'fs/promises';
import { join } from 'path';
import { pack } from 'msgpackr';
import { getPublicKey, sign } from '@noble/ed25519';
import { loadFromFile, loadFromURL, loadFromBuffer } from './ast-loader';
import type { Artifact } from './types';

describe('AST Loader', () => {
  const testDir = join(__dirname, '../test-fixtures');
  const testFile = join(testDir, 'test.ast');

  beforeEach(async () => {
    try {
      await mkdir(testDir, { recursive: true });
    } catch {
      // Directory might already exist
    }
  });

  afterEach(async () => {
    try {
      await rm(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('loadFromBuffer', () => {
    it('should load valid AST from buffer', async () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1', 'flag2'],
        flags: [],
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
          flags: [],
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
          flags: [],
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
          flags: [],
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
          flags: [],
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
          flags: [],
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
          flags: [],
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
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [],
      };

      const buffer = Buffer.from(pack(artifact));
      await writeFile(testFile, buffer);

      const loaded = await loadFromFile(testFile);

      expect(loaded.v).toBe('1.0');
      expect(loaded.env).toBe('test');
      expect(loaded.strs).toEqual(['flag1']);
    });

    it('should throw error for non-existent file', async () => {
      const nonExistentFile = join(testDir, 'non-existent.ast');

      await expect(loadFromFile(nonExistentFile)).rejects.toThrow();
    });

    it('should throw error for invalid file content', async () => {
      // Write completely invalid binary data
      await writeFile(testFile, Buffer.from([0x00, 0x01, 0x02, 0x03, 0x04]));

      // Should fail validation even if msgpackr parses it
      await expect(loadFromFile(testFile)).rejects.toThrow();
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
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
      };

      const buffer = Buffer.from(pack(artifact));
      await writeFile(testFile, buffer);

      // Test that normalized paths work
      const normalizedPath = testFile.replace(/\\/g, '/'); // Normalize separators
      const loaded = await loadFromFile(normalizedPath);

      expect(loaded.v).toBe('1.0');
      expect(loaded.env).toBe('test');
    });
  });

  describe('loadFromURL', () => {
    it.skip('should load AST from HTTP URL', () => {
      // TODO: Implement HTTP URL test with a test server (e.g., nock or msw)
      // This test is skipped because it requires a running HTTP server
      // In a real scenario, you'd use a test HTTP server or mocking library
    });

    it('should throw error for invalid URL', async () => {
      await expect(loadFromURL('not-a-valid-url')).rejects.toThrow('Invalid URL');
    });

    it('should throw error for unsupported protocol', async () => {
      await expect(loadFromURL('ftp://example.com/test.ast')).rejects.toThrow(
        'Unsupported URL protocol'
      );
    });

    it('should throw error for 404 response', async () => {
      // Use a URL that will return 404
      await expect(loadFromURL('https://httpbin.org/status/404')).rejects.toThrow(
        'Failed to load AST from URL'
      );
    });

    it('should handle timeout', async () => {
      // Use a URL that will timeout (very short timeout)
      await expect(loadFromURL('https://httpbin.org/delay/10', 100)).rejects.toThrow('Timeout');
    });
  });
});
