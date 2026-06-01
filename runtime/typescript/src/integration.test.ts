/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { writeFile, mkdir, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { spawnSync } from 'child_process';
import { loadFromBuffer, loadFromFile } from './ast-loader';
import { evaluateBoolean } from './evaluator';
import { resolveBooleanFlag } from './resolve-flag';
import type { Attributes, KillSwitchFile } from './types';

function getRustCliPath(): string {
  const releasePath = join(__dirname, '../../../target/release/controlpath');
  try {
    require('fs').readFileSync(releasePath);
    return releasePath;
  } catch {
    const debugPath = join(__dirname, '../../../target/debug/controlpath');
    try {
      require('fs').readFileSync(debugPath);
      return debugPath;
    } catch {
      throw new Error(
        'Rust CLI binary not found. Please build it first: cargo build --release --bin controlpath'
      );
    }
  }
}

async function compileV2Catalog(catalogDir: string, astFile: string): Promise<Buffer> {
  const rustCli = getRustCliPath();
  const catalog = `catalog:
  id: test-service
mode: local
flags:
  new_dashboard:
    default: false
    kind: release
  enable_analytics:
    default: false
    kind: release
environments:
  production:
    rules:
      new_dashboard:
        - when: "user.role == 'admin'"
          serve: true
        - serve: false
      enable_analytics:
        - serve: true
`;

  await writeFile(join(catalogDir, 'control-path.yaml'), catalog);

  const result = spawnSync(
    rustCli,
    ['compile', '--env', 'production', '--output', astFile],
    {
      encoding: 'utf-8',
      stdio: 'pipe',
      cwd: catalogDir,
    }
  );

  if (result.status !== 0) {
    const errorMsg = result.stderr?.toString() || result.stdout?.toString() || 'Unknown error';
    throw new Error(`Rust CLI failed: ${errorMsg}`);
  }

  return readFile(astFile);
}

describe('Integration Tests with v2 AST Artifacts', () => {
  const testDir = join(
    tmpdir(),
    'controlpath-test',
    `integration-${Date.now()}-${Math.random().toString(36).substring(7)}`
  );
  const astFile = join(testDir, 'production.ast');

  beforeAll(async () => {
    await mkdir(testDir, { recursive: true });
  });

  afterAll(async () => {
    try {
      await rm(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors.
    }
  });

  it('should compile and load AST artifact from Rust CLI', async () => {
    await compileV2Catalog(testDir, astFile);
    const loaded = await loadFromFile(astFile);

    expect(loaded.v).toBe('1.0');
    expect(loaded.env).toBe('production');
    expect(loaded.strs.length).toBeGreaterThan(0);
    expect(loaded.flags.length).toBe(2);
  });

  it('should evaluate boolean flags from compiled AST artifact', async () => {
    const buffer = await compileV2Catalog(testDir, astFile);
    const loaded = await loadFromBuffer(buffer);

    const flagNameMap: Record<string, number> = {};
    loaded.flagNames.forEach((nameIndex, flagIndex) => {
      const flagName = loaded.strs[nameIndex];
      if (flagName) {
        flagNameMap[flagName] = flagIndex;
      }
    });

    const adminAttributes: Attributes = { id: 'admin1', role: 'admin' };
    expect(evaluateBoolean(flagNameMap['new_dashboard'], loaded, adminAttributes)).toBe(true);

    const regularAttributes: Attributes = { id: 'user1', role: 'user' };
    expect(evaluateBoolean(flagNameMap['new_dashboard'], loaded, regularAttributes)).toBe(false);
    expect(evaluateBoolean(flagNameMap['enable_analytics'], loaded, regularAttributes)).toBe(true);
  });

  it('should apply kill switch file before AST rules', async () => {
    const buffer = await compileV2Catalog(testDir, astFile);
    const loaded = await loadFromBuffer(buffer);
    const flagIndex = loaded.flagNames.findIndex((idx) => loaded.strs[idx] === 'new_dashboard');

    const killSwitchFile: KillSwitchFile = {
      version: '2.0',
      flags: { new_dashboard: false },
    };

    const adminAttributes: Attributes = { id: 'admin1', role: 'admin' };
    const value = resolveBooleanFlag({
      qualifiedName: 'new_dashboard',
      flagIndex,
      artifact: loaded,
      catalogDefault: false,
      killSwitchFile,
      attributes: adminAttributes,
    });

    expect(value).toBe(false);
  });
});
