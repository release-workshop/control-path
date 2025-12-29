/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, mkdir, rm } from 'fs/promises';
import { join } from 'path';
import { pack } from 'msgpackr';
import { Provider } from './provider';
import type { Artifact } from './types';

describe('Provider', () => {
  const getTestDir = () =>
    join(
      __dirname,
      '../test-fixtures',
      `test-${Date.now()}-${Math.random().toString(36).substring(7)}`
    );
  const getTestFile = (dir: string) => join(dir, 'test.ast');

  afterEach(async () => {
    // Clean up any leftover test directories
    try {
      const baseDir = join(__dirname, '../test-fixtures');
      const entries = await import('fs/promises').then((fs) =>
        fs.readdir(baseDir, { withFileTypes: true }).catch(() => [])
      );
      for (const entry of entries) {
        if (entry.isDirectory() && entry.name.startsWith('test-')) {
          await rm(join(baseDir, entry.name), { recursive: true, force: true }).catch(() => {
            // Ignore cleanup errors
          });
        }
      }
    } catch {
      // Ignore cleanup errors
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
      await writeFile(testFile, buffer);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      // Verify artifact was loaded by checking evaluation returns default
      const result = provider.resolveBooleanEvaluation('flag1', false, {});
      expect(result).toBeDefined();
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
      await writeFile(testFile, buffer);

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
      await writeFile(testFile, buffer);

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
      await writeFile(testFile, buffer1);

      const provider = new Provider();
      await provider.loadArtifact(testFile);

      const artifact2: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag2'],
        flags: [],
      };

      const buffer2 = Buffer.from(pack(artifact2));
      // Ensure directory still exists before writing
      await mkdir(testDir, { recursive: true });
      await writeFile(testFile, buffer2);

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
