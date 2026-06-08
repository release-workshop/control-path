/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { writeFileSync, mkdirSync, rmSync, utimesSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { refreshFromFile, type FileFingerprint } from './file-refresh';
import type { Logger } from './types';

describe('refreshFromFile', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = join(
      tmpdir(),
      'cp-file-refresh-test',
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

  it('skips read when mtime and size are unchanged', async () => {
    const filePath = join(testDir, 'data.json');
    writeFileSync(filePath, '{"version":"2.0","flags":{}}');
    const load = vi.fn().mockResolvedValue({ ok: true });

    const first = await refreshFromFile(filePath, undefined, load);
    expect(first.status).toBe('updated');
    expect(load).toHaveBeenCalledTimes(1);

    load.mockClear();
    const second = await refreshFromFile(filePath, first.fingerprint, load);
    expect(second.status).toBe('not-modified');
    expect(load).not.toHaveBeenCalled();
  });

  it('reloads when file content changes', async () => {
    const filePath = join(testDir, 'data.json');
    writeFileSync(filePath, '{"version":"2.0","flags":{"a":true}}');
    const load = vi.fn().mockResolvedValueOnce({ a: true }).mockResolvedValueOnce({ a: false });

    const first = await refreshFromFile(filePath, undefined, load);
    expect(first.status).toBe('updated');
    expect(first.value).toEqual({ a: true });

    writeFileSync(filePath, '{"version":"2.0","flags":{"a":false}}');
    const second = await refreshFromFile(filePath, first.fingerprint, load);
    expect(second.status).toBe('updated');
    if (second.status === 'updated') {
      expect(second.value).toEqual({ a: false });
    }
    expect(load).toHaveBeenCalledTimes(2);
  });

  it('keeps prior value when file is missing', async () => {
    const filePath = join(testDir, 'missing.json');
    const prior: FileFingerprint = { mtimeMs: 1, size: 10 };
    const priorValue = { kept: true };
    const load = vi.fn();
    const warn = vi.fn();
    const logger: Logger = { warn, error: vi.fn() };

    const result = await refreshFromFile(filePath, prior, load, logger, priorValue);
    expect(result.status).toBe('failed');
    if (result.status === 'failed') {
      expect(result.value).toEqual(priorValue);
    }
    expect(load).not.toHaveBeenCalled();
    expect(warn).toHaveBeenCalled();
  });

  it('keeps prior value when loader rejects invalid bytes', async () => {
    const filePath = join(testDir, 'bad.json');
    writeFileSync(filePath, '{ not json');
    const priorValue = { good: true };
    const load = vi.fn().mockRejectedValue(new Error('parse failed'));
    const warn = vi.fn();
    const logger: Logger = { warn, error: vi.fn() };

    const result = await refreshFromFile(filePath, undefined, load, logger, priorValue);
    expect(result.status).toBe('failed');
    if (result.status === 'failed') {
      expect(result.value).toEqual(priorValue);
    }
    expect(warn).toHaveBeenCalled();
  });

  it('detects change when only mtime advances with same size', async () => {
    const filePath = join(testDir, 'touch.json');
    writeFileSync(filePath, '{"same":true}');
    const load = vi.fn().mockResolvedValue({ same: true });

    const first = await refreshFromFile(filePath, undefined, load);
    expect(first.status).toBe('updated');

    const later = new Date(Date.now() + 2000);
    utimesSync(filePath, later, later);

    load.mockClear();
    const second = await refreshFromFile(filePath, first.fingerprint, load);
    expect(second.status).toBe('updated');
    expect(load).toHaveBeenCalledTimes(1);
  });
});
