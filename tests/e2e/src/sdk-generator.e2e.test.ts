/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * End-to-end tests for SDK generator (v2 boolean catalog workflow).
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { mkdir, rm, readFile } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  writeCatalog,
  generateSdk,
  compileAst,
  setupGeneratedSdk,
  loadGeneratedSdk,
  loadGeneratedSdkModule,
} from './e2e-harness';

describe.sequential('SDK Generator E2E Tests', () => {
  const testDir = join(
    tmpdir(),
    'controlpath-e2e',
    `test-${Date.now()}-${Math.random().toString(36).substring(7)}`
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

  describe('Simple Rules', () => {
    const simpleRules = `      new_dashboard:
        - serve: true
      enable_analytics:
        - serve: false
      beta_ui:
        - serve: true`;

    it('should generate SDK and evaluate flags with simple rules', async () => {
      await writeCatalog(simpleRules, catalogPath);
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

  describe('Conditional Rules', () => {
    const conditionalRules = `      new_dashboard:
        - when: "user.role == 'admin'"
          serve: true
        - serve: false
      enable_analytics:
        - when: "user.plan == 'premium'"
          serve: true
        - serve: false
      beta_ui:
        - when: "user.role == 'admin'"
          serve: true
        - serve: false`;

    it('should generate SDK and evaluate flags with conditional rules', async () => {
      await writeCatalog(conditionalRules, catalogPath);
      await generateSdk(testDir, sdkDir);
      await compileAst(testDir, astFile);
      await setupGeneratedSdk(sdkDir);

      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      const adminUser = { id: 'admin1', role: 'admin' };
      expect(await evaluator.newDashboard(adminUser)).toBe(true);
      expect(await evaluator.betaUi(adminUser)).toBe(true);

      const regularUser = { id: 'user1', role: 'user' };
      expect(await evaluator.newDashboard(regularUser)).toBe(false);
      expect(await evaluator.betaUi(regularUser)).toBe(false);

      const premiumUser = { id: 'premium1', plan: 'premium' };
      expect(await evaluator.enableAnalytics(premiumUser)).toBe(true);

      const freeUser = { id: 'free1', plan: 'free' };
      expect(await evaluator.enableAnalytics(freeUser)).toBe(false);
    });
  });

  describe('Default Values', () => {
    const defaultRules = `      new_dashboard:
        - serve: false
      enable_analytics:
        - serve: false
      beta_ui:
        - serve: false`;

    it('should return default values when no context provided', async () => {
      await writeCatalog(defaultRules, catalogPath);
      await generateSdk(testDir, sdkDir);
      await compileAst(testDir, astFile);
      await setupGeneratedSdk(sdkDir);

      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      expect(await evaluator.newDashboard()).toBe(false);

      const user = { id: 'user1' };
      expect(await evaluator.newDashboard(user)).toBe(false);
      expect(await evaluator.betaUi(user)).toBe(false);
    });
  });

  describe('Batch Evaluation', () => {
    const batchRules = `      new_dashboard:
        - serve: true
      enable_analytics:
        - serve: false
      beta_ui:
        - serve: true`;

    it('should generate and use type-safe batch evaluation methods', async () => {
      await writeCatalog(batchRules, catalogPath);
      await generateSdk(testDir, sdkDir);
      await compileAst(testDir, astFile);
      await setupGeneratedSdk(sdkDir);

      const evaluator = await loadGeneratedSdk(sdkDir, astFile);
      const user = { id: 'user1' };

      const batchResult = await evaluator.evaluateBatch(
        ['newDashboard', 'enableAnalytics'] as const,
        user
      );
      expect(batchResult.newDashboard).toBe(true);
      expect(batchResult.enableAnalytics).toBe(false);

      const allResult = await evaluator.evaluateAll(user);
      expect(allResult.newDashboard).toBe(true);
      expect(allResult.enableAnalytics).toBe(false);
      expect(allResult.betaUi).toBe(true);
    });
  });

  describe('Context Management', () => {
    const contextRules = `      new_dashboard:
        - serve: true
      enable_analytics:
        - serve: false
      beta_ui:
        - serve: false`;

    it('should use attribute management methods correctly', async () => {
      await writeCatalog(contextRules, catalogPath);
      await generateSdk(testDir, sdkDir);
      await compileAst(testDir, astFile);
      await setupGeneratedSdk(sdkDir);

      const evaluator = await loadGeneratedSdk(sdkDir, astFile);
      const user = { id: 'user1', role: 'user' };

      evaluator.setAttributes(user);
      expect(await evaluator.newDashboard()).toBe(true);

      evaluator.clearAttributes();
      expect(await evaluator.newDashboard()).toBe(false);

      const explicitUser = { id: 'user2' };
      expect(await evaluator.newDashboard(explicitUser)).toBe(true);
    });
  });

  describe('Method Overloads', () => {
    const overloadRules = `      new_dashboard:
        - serve: true
      enable_analytics:
        - serve: false
      beta_ui:
        - serve: false`;

    it('should work with all method overload variants', async () => {
      await writeCatalog(overloadRules, catalogPath);
      await generateSdk(testDir, sdkDir);
      await compileAst(testDir, astFile);
      await setupGeneratedSdk(sdkDir);

      const evaluator = await loadGeneratedSdk(sdkDir, astFile);
      const user = { id: 'user1' };
      const context = { id: 'user1', environment: 'production' };

      evaluator.setAttributes(user);
      const result1 = await evaluator.newDashboard();
      const result2 = await evaluator.newDashboard(user);
      const result3 = await evaluator.newDashboard(context);

      expect(result1).toBe(true);
      expect(result2).toBe(true);
      expect(result3).toBe(true);
    });
  });

  describe('Error Handling', () => {
    const errorRules = `      new_dashboard:
        - serve: true
      enable_analytics:
        - serve: false
      beta_ui:
        - serve: false`;

    it('should never throw errors, always return defaults', async () => {
      await writeCatalog(errorRules, catalogPath);
      await generateSdk(testDir, sdkDir);
      await compileAst(testDir, astFile);
      await setupGeneratedSdk(sdkDir);

      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      expect(await evaluator.newDashboard()).toBe(false);

      const invalidUser = { id: '' };
      expect(await evaluator.newDashboard(invalidUser)).toBe(false);

      expect(typeof (await evaluator.enableAnalytics())).toBe('boolean');
      expect(typeof (await evaluator.betaUi())).toBe('boolean');
    });
  });

  describe('Runtime SDK Integration', () => {
    const integrationRules = `      new_dashboard:
        - serve: true
      enable_analytics:
        - serve: false
      beta_ui:
        - serve: true`;

    it('should integrate with runtime SDK correctly', async () => {
      await writeCatalog(integrationRules, catalogPath);
      await generateSdk(testDir, sdkDir);
      await compileAst(testDir, astFile);
      await setupGeneratedSdk(sdkDir);

      const evaluator = await loadGeneratedSdk(sdkDir, astFile);
      const user = { id: 'user1' };

      expect(await evaluator.newDashboard(user)).toBe(true);
      expect(await evaluator.betaUi(user)).toBe(true);
      expect(await evaluator.newDashboard(user)).not.toBe(false);
    });
  });

  describe('Generated SDK Execution', () => {
    const executionRules = `      new_dashboard:
        - serve: true
      enable_analytics:
        - serve: false
      beta_ui:
        - serve: true`;

    it('should generate SDK that can be imported and used', async () => {
      await writeCatalog(executionRules, catalogPath);
      await generateSdk(testDir, sdkDir);
      await compileAst(testDir, astFile);

      expect(await readFile(join(sdkDir, 'index.ts'), 'utf-8')).toBeTruthy();
      expect(await readFile(join(sdkDir, 'types.ts'), 'utf-8')).toBeTruthy();

      await setupGeneratedSdk(sdkDir);
      const evaluator = await loadGeneratedSdk(sdkDir, astFile);
      const user = { id: 'user1' };

      expect(await evaluator.newDashboard(user)).toBe(true);
      expect(await evaluator.betaUi(user)).toBe(true);
    });

    it('keeps runtime state isolated across evaluator instances', async () => {
      await writeCatalog(executionRules, catalogPath);
      await generateSdk(testDir, sdkDir);
      await compileAst(testDir, astFile);
      await setupGeneratedSdk(sdkDir);

      const sdkModule = await loadGeneratedSdkModule(sdkDir);
      const { Evaluator, evaluator } = sdkModule;
      const a = new Evaluator();
      const b = new Evaluator();
      await a.init({ artifact: astFile });
      await b.init({ artifact: astFile });
      await evaluator.init({ artifact: astFile });

      a.setAttributes({ id: 'a-user' });
      b.clearAttributes();
      evaluator.setAttributes({ id: 'singleton-user' });

      expect(await a.newDashboard()).toBe(true);
      expect(await b.newDashboard()).toBe(false);
      expect(await evaluator.newDashboard()).toBe(true);
    });
  });
});
