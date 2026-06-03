/**
 * Pre-merge smoke: one CLI → compile → generate-sdk → evaluator path.
 * Run via `npm run test:smoke` (vitest.smoke.config.ts); do not rename this file path.
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { mkdir, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  SMOKE_SIMPLE_RULES,
  writeCatalog,
  generateSdk,
  compileAst,
  setupGeneratedSdk,
  loadGeneratedSdk,
} from '../e2e-harness';

describe.sequential('SDK generator smoke', () => {
  const testDir = join(
    tmpdir(),
    'controlpath-e2e-smoke',
    `smoke-${Date.now()}-${Math.random().toString(36).substring(7)}`
  );
  const catalogPath = join(testDir, 'control-path.yaml');
  const astFile = join(testDir, 'production.ast');
  const sdkDir = join(testDir, 'generated-sdk');

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

  it('exercises CLI compile, generate-sdk, and generated evaluator', async () => {
    await writeCatalog(SMOKE_SIMPLE_RULES, catalogPath);
    await generateSdk(testDir, sdkDir);
    await compileAst(testDir, astFile);
    await setupGeneratedSdk(sdkDir);

    const evaluator = await loadGeneratedSdk(sdkDir, astFile);
    const user = { id: 'user1', role: 'user' };

    expect(await evaluator.newDashboard(user)).toBe(true);
    expect(await evaluator.enableAnalytics(user)).toBe(false);
    expect(await evaluator.betaUi(user)).toBe(true);

    evaluator.setAttributes(user);
    expect(await evaluator.newDashboard()).toBe(true);

    const context = { id: 'user1', role: 'user', environment: 'production' };
    expect(await evaluator.newDashboard(context)).toBe(true);
  });
});
