/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { writeFileSync, mkdirSync, rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  loadKillSwitchFromFile,
  loadKillSwitchFromURL,
  KillSwitchFileNotModifiedError,
} from './kill-switch-loader';
import type { KillSwitchFile } from './types';

describe('Kill Switch Loader', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = join(
      tmpdir(),
      'controlpath-kill-switch-test',
      `${Date.now()}-${Math.random().toString(36).substring(7)}`
    );
    mkdirSync(testDir, { recursive: true });
  });

  afterEach(() => {
    try {
      rmSync(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors.
    }
  });

  describe('loadKillSwitchFromFile', () => {
    it('should load v2 boolean kill switch file', async () => {
      const testFile = join(testDir, 'production.kill-switches.json');
      const killSwitchFile: KillSwitchFile = {
        version: '2.0',
        flags: {
          emergency_kill_switch: false,
          new_dashboard: true,
        },
      };
      writeFileSync(testFile, JSON.stringify(killSwitchFile));

      const loaded = await loadKillSwitchFromFile(testFile);

      expect(loaded.version).toBe('2.0');
      expect(loaded.flags.emergency_kill_switch).toBe(false);
      expect(loaded.flags.new_dashboard).toBe(true);
    });

    it('should throw error for invalid JSON', async () => {
      const testFile = join(testDir, 'invalid.json');
      writeFileSync(testFile, '{ invalid json }');

      await expect(loadKillSwitchFromFile(testFile)).rejects.toThrow(
        'Failed to parse kill switch file JSON'
      );
    });

    it('should throw error for missing flags field', async () => {
      const testFile = join(testDir, 'missing-flags.json');
      writeFileSync(testFile, JSON.stringify({ version: '2.0' }));

      await expect(loadKillSwitchFromFile(testFile)).rejects.toThrow(
        'Invalid kill switch file format'
      );
    });

    it('should throw error for non-boolean flag values', async () => {
      const testFile = join(testDir, 'bad-value.json');
      writeFileSync(
        testFile,
        JSON.stringify({
          version: '2.0',
          flags: { my_flag: 'ON' },
        })
      );

      await expect(loadKillSwitchFromFile(testFile)).rejects.toThrow(
        'Invalid kill switch file format'
      );
    });
  });

  describe('loadKillSwitchFromURL', () => {
    afterEach(() => {
      vi.restoreAllMocks();
    });

    it('should load kill switch file from HTTP URL', async () => {
      const killSwitchFile: KillSwitchFile = {
        version: '2.0',
        flags: { emergency_kill_switch: true },
      };

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: {
          get: (name: string) => (name.toLowerCase() === 'etag' ? '"abc123"' : null),
        },
        text: async () => JSON.stringify(killSwitchFile),
      });

      const result = await loadKillSwitchFromURL('https://example.com/kill-switches.json');

      expect(result.killSwitchFile.flags.emergency_kill_switch).toBe(true);
      expect(result.etag).toBe('"abc123"');
    });

    it('should throw KillSwitchFileNotModifiedError on 304', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 304,
        headers: { get: () => null },
      });

      await expect(
        loadKillSwitchFromURL('https://example.com/kill-switches.json', '"etag"')
      ).rejects.toBeInstanceOf(KillSwitchFileNotModifiedError);
    });

    it('should send If-None-Match when etag provided', async () => {
      const killSwitchFile: KillSwitchFile = { version: '2.0', flags: {} };
      const fetchMock = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        headers: { get: () => null },
        text: async () => JSON.stringify(killSwitchFile),
      });
      global.fetch = fetchMock;

      await loadKillSwitchFromURL('https://example.com/kill-switches.json', '"prev-etag"');

      expect(fetchMock).toHaveBeenCalledWith(
        'https://example.com/kill-switches.json',
        expect.objectContaining({
          headers: { 'If-None-Match': '"prev-etag"' },
        })
      );
    });
  });
});
