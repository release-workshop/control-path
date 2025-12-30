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
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [],
      };

      const buffer = Buffer.from(pack(artifact));
      // Ensure directory exists before writing
      await mkdir(testDir, { recursive: true });

      // Write file using writeFileSync for immediate, synchronous write
      // This is more reliable in test environments where async timing can cause issues
      writeFileSync(testFile, buffer);

      // Verify file exists and is readable before loading
      // Use retry mechanism to handle potential race conditions in CI/turbo environments
      // Turbo may run tests in parallel, causing file system delays
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
          // Increase delay slightly for turbo environments
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

      // Verify artifact was loaded by checking evaluation returns default
      // Note: Without flagNameMap, flag lookup will fail but should still return default
      const result = provider.resolveBooleanEvaluation('flag1', false, {});
      expect(result).toBeDefined();
      expect(result.value).toBe(false); // Should return default when flag not found
      expect(result.reason).toBe('DEFAULT');
      expect(result.errorCode).toBe('FLAG_NOT_FOUND');
    });

    it('should load artifact from file:// URL string', async () => {
      const testDir = getTestDir();
      const testFile = getTestFile(testDir);
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
      };

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      // Verify file exists before loading (handles race conditions in turbo)
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
      };

      const buffer = Buffer.from(pack(artifact));
      await mkdir(testDir, { recursive: true });
      writeFileSync(testFile, buffer);

      // Verify file exists before loading (handles race conditions in turbo)
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
      const artifact1: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [],
      };

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
      };

      const buffer2 = Buffer.from(pack(artifact2));
      writeFileSync(testFile, buffer2);

      // Verify file exists before reloading (handles race conditions in turbo)
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
});
