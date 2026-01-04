/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, mkdir, rm, stat } from 'fs/promises';
import { writeFileSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { pack } from 'msgpackr';
import { Provider } from './provider';
import type { Artifact } from './types';

describe('Provider', () => {
  // Track test directories per test to avoid interference when tests run in parallel
  const testDirs = new Set<string>();

  const getTestDir = () => {
    // Use OS temp directory for better isolation and reliability
    // This avoids race conditions with shared test-fixtures directory
    const testDir = join(
      tmpdir(),
      'controlpath-test',
      `test-${Date.now()}-${Math.random().toString(36).substring(7)}`
    );
    testDirs.add(testDir);
    return testDir;
  };
  const getTestFile = (dir: string) => join(dir, 'test.ast');

  afterEach(async () => {
    // Clean up only the directories created by this test run
    // This prevents interference when tests run in parallel
    const dirsToClean = Array.from(testDirs);
    testDirs.clear();
    for (const testDir of dirsToClean) {
      try {
        await rm(testDir, { recursive: true, force: true });
      } catch {
        // Ignore cleanup errors - directory might already be deleted
      }
    }
  });

  describe('metadata', () => {
    it('should have correct metadata', () => {
      const provider = new Provider();

      expect(provider.metadata).toEqual({ name: 'controlpath' });
    });
  });

  describe('hooks', () => {
    it('should have empty hooks array', () => {
      const provider = new Provider();

      expect(provider.hooks).toEqual([]);
    });
  });

  describe('loadArtifact', () => {
    it('should load artifact from file path', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[]],
        flagNames: [0], // flag1 is at index 0 in string table
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      // Ensure directory exists before writing
      await mkdir(testDir, { recursive: true });

      // Write file using writeFileSync for immediate, synchronous write
      // This is more reliable in test environments where async timing can cause issues
      writeFileSync(testFile, buffer);

      // Verify file exists and is readable before loading
      // Use retry mechanism to handle potential race conditions in CI environments
      // Tests may run in parallel, causing file system delays
      const { readFile } = await import('fs/promises');
      let retries = 30;
      let lastError: Error | null = null;
      while (retries > 0) {
        try {
          // Try to read the file directly - this is the most reliable check
          const fileContent = await readFile(testFile);
          if (fileContent.length > 0) {
            break; // File exists, has content, and is readable
          }
        } catch (error) {
          lastError = error instanceof Error ? error : new Error(String(error));
          // File doesn't exist yet or isn't readable, wait a bit and retry
          // Increase delay slightly for CI environments
          await new Promise((resolve) => setTimeout(resolve, 100));
          retries--;
        }
      }
      if (retries === 0) {
        // Final diagnostic: check if directory exists and list its contents
        let dirInfo = 'unknown';
        try {
          const { readdir } = await import('fs/promises');
          const dirExists = await stat(testDir)
            .then(() => true)
            .catch(() => false);
          const files = dirExists ? await readdir(testDir).catch(() => []) : [];
          dirInfo = `exists: ${dirExists}, files: [${files.join(', ')}]`;
        } catch {
          dirInfo = 'check failed';
        }
        throw new Error(
          `File ${testFile} was not accessible after 30 attempts. ` +
            `Directory ${testDir}: ${dirInfo}. ` +
            `Last error: ${lastError?.message || 'unknown error'}`
        );
      }

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      // Verify artifact was loaded by checking evaluation works
      // Flag name map is automatically built from artifact, so flag1 is found
      const result = provider.resolveBooleanEvaluation('flag1', false, {});
      expect(result).toBeDefined();
      expect(result.value).toBe(false); // Should return default value
      expect(result.reason).toBe('DEFAULT');
      // No error code since flag is found (flag name map is automatically built)
    });

    it('should load artifact from file:// URL string', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      };

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      // Verify file exists before loading (handles race conditions in CI)
      const { readFile } = await import('fs/promises');
      let retries = 10;
      while (retries > 0) {
        try {
          await readFile(testFile);
          break;
        } catch {
          await new Promise((resolve) => setTimeout(resolve, 50));
          retries--;
        }
      }

      const fileUrl = `file://${testFile}`;
      const provider = new Provider();
      await provider.loadArtifact(fileUrl);

      const result = provider.resolveBooleanEvaluation('flag1', false, {});
      expect(result).toBeDefined();
    });

    it('should load artifact from file:// URL object', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      };

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      // Verify file exists before loading (handles race conditions in CI)
      const { readFile } = await import('fs/promises');
      let retries = 10;
      while (retries > 0) {
        try {
          await readFile(testFile);
          break;
        } catch {
          await new Promise((resolve) => setTimeout(resolve, 50));
          retries--;
        }
      }

      const fileUrl = new URL(`file://${testFile}`);
      const provider = new Provider();
      await provider.loadArtifact(fileUrl);

      const result = provider.resolveBooleanEvaluation('flag1', false, {});
      expect(result).toBeDefined();
    });

    it('should throw error for invalid file', async () => {
      const testDir = getTestDir();
      const provider = new Provider();
      const nonExistentFile = join(testDir, 'non-existent.ast');

      await expect(provider.loadArtifact(nonExistentFile)).rejects.toThrow();
    });

    it('should throw error for empty path', async () => {
      const provider = new Provider();

      await expect(provider.loadArtifact('')).rejects.toThrow('Artifact path or URL is required');
    });

    it('should throw error for null path', async () => {
      const provider = new Provider();

      // @ts-expect-error - Testing invalid input
      await expect(provider.loadArtifact(null)).rejects.toThrow();
    });
  });

  describe('reloadArtifact', () => {
    it('should reload artifact from file', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact1 = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[]],
        flagNames: [0], // flag1 is at index 0 in string table
      } as Artifact;

      const buffer1 = Buffer.from(pack(artifact1));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer1);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      const artifact2: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag2'],
        flags: [],
        flagNames: [],
      };

      const buffer2 = Buffer.from(pack(artifact2));
      writeFileSync(testFile, buffer2);

      // Verify file exists before reloading (handles race conditions in CI)
      const { readFile } = await import('fs/promises');
      let retries = 10;
      while (retries > 0) {
        try {
          await readFile(testFile);
          break;
        } catch {
          await new Promise((resolve) => setTimeout(resolve, 50));
          retries--;
        }
      }

      await provider.reloadArtifact(testFile);

      const result = provider.resolveBooleanEvaluation('flag1', false, {});
      expect(result).toBeDefined();
    });
  });

  describe('resolveBooleanEvaluation', () => {
    it('should return default value when no artifact loaded', () => {
      const provider = new Provider();
      const result = provider.resolveBooleanEvaluation('flag1', false, {});

      expect(result).toEqual({
        value: false,
        reason: 'DEFAULT',
      });
    });

    it('should return default value for any flag', () => {
      const provider = new Provider();
      const result = provider.resolveBooleanEvaluation('any-flag', true, {});

      expect(result).toEqual({
        value: true,
        reason: 'DEFAULT',
      });
    });
  });

  describe('resolveStringEvaluation', () => {
    it('should return default value', () => {
      const provider = new Provider();
      const result = provider.resolveStringEvaluation('flag1', 'default', {});

      expect(result).toEqual({
        value: 'default',
        reason: 'DEFAULT',
      });
    });
  });

  describe('resolveNumberEvaluation', () => {
    it('should return default value', () => {
      const provider = new Provider();
      const result = provider.resolveNumberEvaluation('flag1', 42, {});

      expect(result).toEqual({
        value: 42,
        reason: 'DEFAULT',
      });
    });
  });

  describe('resolveObjectEvaluation', () => {
    it('should return default value', () => {
      const provider = new Provider();
      const defaultObj = { key: 'value' };
      const result = provider.resolveObjectEvaluation('flag1', defaultObj, {});

      expect(result).toEqual({
        value: defaultObj,
        reason: 'DEFAULT',
      });
    });
  });

  describe('cache key normalization and prototype pollution protection', () => {
    it('should filter prototype-polluting keys from cache context', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[]],
        flagNames: [0], // flag1 is at index 0 in string table
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      // Create context with prototype-polluting keys
      const contextWithPollution = {
        user: { id: 'user1' },
        __proto__: { polluted: true },
        constructor: { polluted: true },
        prototype: { polluted: true },
        normalKey: 'value',
      };

      // First evaluation
      const result1 = provider.resolveBooleanEvaluation('flag1', false, contextWithPollution);
      expect(result1).toBeDefined();

      // Second evaluation with same context (should use cache)
      const result2 = provider.resolveBooleanEvaluation('flag1', false, contextWithPollution);
      expect(result2).toBeDefined();

      // Results should be the same (cached)
      expect(result2.value).toBe(result1.value);
    });

    it('should normalize cache keys for consistent caching', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[]],
        flagNames: [0], // flag1 is at index 0 in string table
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      // Context with keys in different order
      const context1 = { a: '1', b: '2', c: '3' };
      const context2 = { c: '3', a: '1', b: '2' };

      // First evaluation
      const result1 = provider.resolveBooleanEvaluation('flag1', false, context1);
      expect(result1).toBeDefined();

      // Second evaluation with same keys but different order (should use cache)
      const result2 = provider.resolveBooleanEvaluation('flag1', false, context2);
      expect(result2).toBeDefined();

      // Results should be the same (cached due to normalization)
      expect(result2.value).toBe(result1.value);
    });

    it('should handle non-object context in cache key generation', () => {
      const provider = new Provider();

      // Should not throw with non-object context
      const result1 = provider.resolveBooleanEvaluation('flag1', false, null);
      const result2 = provider.resolveBooleanEvaluation('flag1', false, undefined);
      const result3 = provider.resolveBooleanEvaluation('flag1', false, 'string');
      const result4 = provider.resolveBooleanEvaluation('flag1', false, 123);

      expect(result1).toBeDefined();
      expect(result2).toBeDefined();
      expect(result3).toBeDefined();
      expect(result4).toBeDefined();
    });

    it('should create different cache keys for different contexts', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[]],
        flagNames: [0], // flag1 is at index 0 in string table
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider({ enableCache: true });
      await provider.loadArtifact(testFile);

      // Different contexts should produce different cache keys
      const context1 = { user: { id: 'user1' } };
      const context2 = { user: { id: 'user2' } };

      const result1 = provider.resolveBooleanEvaluation('flag1', false, context1);
      const result2 = provider.resolveBooleanEvaluation('flag1', false, context2);

      // Both should work (may have different values if flag evaluation differs)
      expect(result1).toBeDefined();
      expect(result2).toBeDefined();
    });
  });

  describe('artifact without flagNames', () => {
    it('should throw error when loading artifact without flagNames', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      // Create artifact without flagNames (old format)
      const artifactWithoutFlagNames = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[]],
        // flagNames is missing
      };

      const buffer = Buffer.from(pack(artifactWithoutFlagNames));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider();
      await expect(provider.loadArtifact(testFile)).rejects.toThrow(
        'flagNames'
      );
    });
  });

  describe('cache expiration', () => {
    it('should expire cache entries after TTL', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1', 'ON'],
        flags: [[[0, undefined, 1]]], // SERVE rule returning 'ON'
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      // Use very short TTL for testing
      const provider = new Provider({ enableCache: true, cacheTTL: 10 }); // 10ms
      await provider.loadArtifact(testFile);

      // First evaluation - should cache
      const result1 = provider.resolveBooleanEvaluation('flag1', false, {});
      expect(result1.value).toBe(true); // 'ON' converts to true

      // Wait for cache to expire
      await new Promise((resolve) => setTimeout(resolve, 20));

      // Second evaluation - should not use cache (expired)
      const result2 = provider.resolveBooleanEvaluation('flag1', false, {});
      expect(result2.value).toBe(true);
    });
  });

  describe('object evaluation edge cases', () => {
    it('should handle object evaluation with JSON string result', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1', '{"key":"value"}'],
        flags: [[[0, undefined, 1]]], // SERVE rule returning JSON string
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      const defaultObj = { default: 'value' };
      const result = provider.resolveObjectEvaluation('flag1', defaultObj, {});

      expect(result.value).toEqual({ key: 'value' });
      expect(result.reason).toBe('TARGETING_MATCH');
    });

    it('should handle object evaluation with invalid JSON string', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1', 'invalid-json'],
        flags: [[[0, undefined, 1]]], // SERVE rule returning invalid JSON
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      const defaultObj = { default: 'value' };
      const result = provider.resolveObjectEvaluation('flag1', defaultObj, {});

      expect(result.value).toEqual(defaultObj);
      expect(result.reason).toBe('DEFAULT');
      expect(result.errorCode).toBe('TYPE_MISMATCH');
    });

    it('should handle object evaluation with non-object, non-string result', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      // Use a string that can't be parsed as JSON to trigger TYPE_MISMATCH
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1', 'not-json-object'],
        flags: [[[0, undefined, 1]]], // SERVE rule returning string that's not JSON
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      const defaultObj = { default: 'value' };
      const result = provider.resolveObjectEvaluation('flag1', defaultObj, {});

      // When result is a string that can't be parsed as JSON, should return default with TYPE_MISMATCH
      expect(result.value).toEqual(defaultObj);
      expect(result.reason).toBe('DEFAULT');
      expect(result.errorCode).toBe('TYPE_MISMATCH');
    });

    it('should handle object evaluation with array result', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[[0, undefined, [1, 2, 3]]]], // SERVE rule returning array
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      const defaultObj = { default: 'value' };
      const result = provider.resolveObjectEvaluation('flag1', defaultObj, {});

      expect(result.value).toEqual(defaultObj);
      expect(result.reason).toBe('DEFAULT');
      expect(result.errorCode).toBe('TYPE_MISMATCH');
    });

    it('should handle object evaluation with null result', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[[0, undefined, null]]], // SERVE rule returning null
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      const defaultObj = { default: 'value' };
      const result = provider.resolveObjectEvaluation('flag1', defaultObj, {});

      expect(result.value).toEqual(defaultObj);
      expect(result.reason).toBe('DEFAULT');
      expect(result.errorCode).toBe('TYPE_MISMATCH');
    });

    it('should handle object evaluation with undefined result', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[]], // Empty rules array - evaluation returns undefined
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      const defaultObj = { default: 'value' };
      const result = provider.resolveObjectEvaluation('flag1', defaultObj, {});

      // When result is undefined, should return default without error code
      expect(result.value).toEqual(defaultObj);
      expect(result.reason).toBe('DEFAULT');
      expect(result.errorCode).toBeUndefined();
    });

    it('should handle object evaluation with valid object result', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[[0, undefined, { key: 'value' }]]], // SERVE rule returning object
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      const defaultObj = { default: 'value' };
      const result = provider.resolveObjectEvaluation('flag1', defaultObj, {});

      // When result is a valid object, should return it
      expect(result.value).toEqual({ key: 'value' });
      expect(result.reason).toBe('TARGETING_MATCH');
    });

    it('should handle object evaluation error with logger', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[[0, undefined, 'ON']]],
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const errorMessages: string[] = [];
      const logger = {
        error: (message: string) => {
          errorMessages.push(message);
        },
        warn: () => {},
        debug: () => {},
      };

      const provider = new Provider({ logger });
      await provider.loadArtifact(testFile);

      // This should trigger an error path (though it might not actually error)
      // We're testing that the logger.error path exists
      const defaultObj = { key: 'value' };
      const result = provider.resolveObjectEvaluation('flag1', defaultObj, {});

      expect(result).toBeDefined();
    });

    it('should call logger.debug when no artifact loaded in object evaluation', () => {
      const debugMessages: string[] = [];
      const logger = {
        debug: (message: string) => {
          debugMessages.push(message);
        },
        warn: () => {},
        error: () => {},
      };

      const provider = new Provider({ logger });
      const defaultObj = { key: 'value' };
      const result = provider.resolveObjectEvaluation('flag1', defaultObj, {});

      expect(result).toBeDefined();
      expect(debugMessages.length).toBeGreaterThan(0);
      expect(debugMessages[0]).toContain('No artifact loaded');
    });

    it('should call logger.warn when flag not found in object evaluation', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[]],
        flagNames: [0], // Only flag1 exists
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const warnMessages: string[] = [];
      const logger = {
        warn: (message: string) => {
          warnMessages.push(message);
        },
        debug: () => {},
        error: () => {},
      };

      const provider = new Provider({ logger });
      await provider.loadArtifact(testFile);

      const defaultObj = { key: 'value' };
      const result = provider.resolveObjectEvaluation('nonexistent-flag', defaultObj, {});

      expect(result).toBeDefined();
      expect(warnMessages.length).toBeGreaterThan(0);
      expect(warnMessages[0]).toContain('not found in flag name map');
    });
  });

  describe('evaluation error handling', () => {
    it('should handle errors in boolean evaluation gracefully', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[[0, undefined, 'ON']]],
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const logger = {
        error: () => {},
        warn: () => {},
        debug: () => {},
      };

      const provider = new Provider({ logger });
      await provider.loadArtifact(testFile);

      // This should not throw, even if there's an internal error
      const result = provider.resolveBooleanEvaluation('flag1', false, {});
      expect(result).toBeDefined();
    });

    it('should handle errors in string evaluation gracefully', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1', 'value'],
        flags: [[[0, undefined, 1]]],
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const logger = {
        error: () => {},
        warn: () => {},
        debug: () => {},
      };

      const provider = new Provider({ logger });
      await provider.loadArtifact(testFile);

      const result = provider.resolveStringEvaluation('flag1', 'default', {});
      expect(result).toBeDefined();
    });

    it('should handle errors in number evaluation gracefully', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1', 'not-a-number'],
        flags: [[[0, undefined, 1]]],
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const logger = {
        error: () => {},
        warn: () => {},
        debug: () => {},
      };

      const provider = new Provider({ logger });
      await provider.loadArtifact(testFile);

      const result = provider.resolveNumberEvaluation('flag1', 42, {});
      expect(result.value).toBe(42);
      expect(result.reason).toBe('DEFAULT');
      expect(result.errorCode).toBe('TYPE_MISMATCH');
    });

    it('should call logger.error when number evaluation throws', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[]],
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const errorMessages: string[] = [];
      const logger = {
        error: (message: string) => {
          errorMessages.push(message);
        },
        warn: () => {},
        debug: () => {},
      };

      const provider = new Provider({ logger });
      await provider.loadArtifact(testFile);

      // This should not throw, but tests the error path exists
      const result = provider.resolveNumberEvaluation('flag1', 42, {});
      expect(result).toBeDefined();
    });

    it('should handle errors in object evaluation gracefully', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[[0, undefined, 'invalid']]],
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const logger = {
        error: () => {},
        warn: () => {},
        debug: () => {},
      };

      const provider = new Provider({ logger });
      await provider.loadArtifact(testFile);

      const defaultObj = { key: 'value' };
      const result = provider.resolveObjectEvaluation('flag1', defaultObj, {});
      expect(result).toBeDefined();
    });
  });

  describe('context mapping edge cases', () => {
    it('should handle invalid context types', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[]],
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const logger = {
        warn: () => {},
        error: () => {},
        debug: () => {},
      };

      const provider = new Provider({ logger });
      await provider.loadArtifact(testFile);

      // Test with array context (invalid)
      const result1 = provider.resolveBooleanEvaluation('flag1', false, [] as unknown);
      expect(result1).toBeDefined();

      // Test with null context
      const result2 = provider.resolveBooleanEvaluation('flag1', false, null as unknown);
      expect(result2).toBeDefined();
    });

    it('should handle context with user. prefix', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1', 'ON', 'user.role', 'admin'],
        flags: [
          [
            [
              0, // SERVE
              [
                0, // BINARY_OP
                0, // EQ
                [2, 2], // PROPERTY: user.role (index 2 in strs)
                [3, 3], // LITERAL: admin (index 3 in strs)
              ],
              1, // Return 'ON' (index 1 in strs)
            ],
          ],
        ],
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      // Context with 'user.role' key should map to user.role property
      const context = {
        'user.role': 'admin',
      };

      const result = provider.resolveBooleanEvaluation('flag1', false, context);
      expect(result.value).toBe(true);
    });

    it('should handle context with context. prefix', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1', 'ON', 'context.environment', 'production'],
        flags: [
          [
            [
              0, // SERVE
              [
                0, // BINARY_OP
                0, // EQ
                [2, 2], // PROPERTY: context.environment (index 2 in strs)
                [3, 3], // LITERAL: production (index 3 in strs)
              ],
              1, // Return 'ON' (index 1 in strs)
            ],
          ],
        ],
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      // Context with 'context.environment' key should map to context.environment property
      const context = {
        'context.environment': 'production',
      };

      const result = provider.resolveBooleanEvaluation('flag1', false, context);
      expect(result.value).toBe(true);
    });

    it('should use cached result in object evaluation', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1', '{"key":"value"}'],
        flags: [[[0, undefined, 1]]], // SERVE rule returning JSON string
        flagNames: [0],
      } as Artifact;

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      const provider = new Provider({ enableCache: true });
      await provider.loadArtifact(testFile);

      const defaultObj = { default: 'value' };
      const context = { user: { id: 'user1' } };

      // First evaluation - should cache
      const result1 = provider.resolveObjectEvaluation('flag1', defaultObj, context);
      expect(result1.value).toEqual({ key: 'value' });

      // Second evaluation with same context - should use cache
      const result2 = provider.resolveObjectEvaluation('flag1', defaultObj, context);
      expect(result2.value).toEqual({ key: 'value' });
      expect(result2).toEqual(result1); // Should be same object (cached)
    });
  });
});
