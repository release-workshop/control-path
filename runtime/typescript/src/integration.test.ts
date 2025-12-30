/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { writeFile, mkdir, rm } from 'fs/promises';
import { join } from 'path';
import { compile, serialize } from '@controlpath/compiler';
import { parseDefinitions, parseDeployment } from '@controlpath/compiler';
import { loadFromBuffer, loadFromFile } from './ast-loader';
import { evaluate } from './evaluator';
import { Provider } from './provider';
import type { User, Context } from './types';

describe('Integration Tests with Real AST Artifacts', () => {
  const testDir = join(__dirname, '../test-fixtures/integration');
  const definitionsFile = join(testDir, 'flags.definitions.yaml');
  const deploymentFile = join(testDir, 'production.deployment.yaml');
  const astFile = join(testDir, 'production.ast');

  const flagsDefinitions = `flags:
  - name: new_dashboard
    type: boolean
    defaultValue: OFF
    description: "New dashboard UI feature"
  
  - name: enable_analytics
    type: boolean
    defaultValue: false
    description: "Enable analytics tracking"
  
  - name: theme_color
    type: multivariate
    defaultValue: blue
    description: "Application theme color"
    variations:
      - name: BLUE
        value: "blue"
      - name: GREEN
        value: "green"
      - name: DARK
        value: "dark"
`;

  const deployment = `environment: production
rules:
  new_dashboard:
    rules:
      - when: "user.role == 'admin'"
        serve: ON
      - serve: OFF
  
  enable_analytics:
    rules:
      - serve: true
  
  theme_color:
    rules:
      - when: "user.role == 'admin'"
        serve: DARK
      - serve: BLUE
`;

  beforeAll(async () => {
    await mkdir(testDir, { recursive: true });
    await writeFile(definitionsFile, flagsDefinitions);
    await writeFile(deploymentFile, deployment);
  });

  afterAll(async () => {
    try {
      await rm(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  it('should compile and load AST artifact from Phase 1 compiler', async () => {
    // Parse definitions and deployment
    const definitions = parseDefinitions(definitionsFile);
    const deploymentData = parseDeployment(deploymentFile);

    // Compile to AST
    const artifact = compile(deploymentData, definitions);

    // Serialize to MessagePack (returns Uint8Array)
    const bytes = serialize(artifact);
    const buffer = Buffer.from(bytes);

    // Write to file
    await writeFile(astFile, buffer);

    // Load from file
    const loaded = await loadFromFile(astFile);

    expect(loaded.v).toBe('1.0');
    expect(loaded.env).toBe('production');
    expect(loaded.strs.length).toBeGreaterThan(0);
    expect(loaded.flags.length).toBe(3); // 3 flags
  });

  it('should evaluate flags from compiled AST artifact', async () => {
    // Parse and compile
    const definitions = parseDefinitions(definitionsFile);
    const deploymentData = parseDeployment(deploymentFile);
    const artifact = compile(deploymentData, definitions);
    const bytes = serialize(artifact);
    const buffer = Buffer.from(bytes);
    const loaded = await loadFromBuffer(buffer);

    // Create flag name to index map
    const flagNameMap: Record<string, number> = {};
    definitions.flags.forEach((flag, index) => {
      flagNameMap[flag.name] = index;
    });

    // Test flag 0: new_dashboard
    const adminUser: User = { id: 'admin1', role: 'admin' };
    const result1 = evaluate(flagNameMap['new_dashboard'], loaded, adminUser);
    expect(result1).toBe('ON'); // Admin should get ON

    const regularUser: User = { id: 'user1', role: 'user' };
    const result2 = evaluate(flagNameMap['new_dashboard'], loaded, regularUser);
    expect(result2).toBe('OFF'); // Regular user should get OFF

    // Test flag 1: enable_analytics (always true)
    // Note: The compiler converts boolean true to "ON" string in the string table
    const result3 = evaluate(flagNameMap['enable_analytics'], loaded, regularUser);
    expect(result3).toBe('ON'); // Compiler normalizes true to "ON"

    // Test flag 2: theme_color
    const result4 = evaluate(flagNameMap['theme_color'], loaded, adminUser);
    expect(result4).toBe('DARK'); // Admin should get DARK

    const result5 = evaluate(flagNameMap['theme_color'], loaded, regularUser);
    expect(result5).toBe('BLUE'); // Regular user should get BLUE
  });

  it('should work with Provider class using compiled AST', async () => {
    // Parse and compile
    const definitions = parseDefinitions(definitionsFile);
    const deploymentData = parseDeployment(deploymentFile);
    const artifact = compile(deploymentData, definitions);
    const bytes = serialize(artifact);
    const buffer = Buffer.from(bytes);
    await writeFile(astFile, buffer);

    // Create flag name to index map
    const flagNameMap: Record<string, number> = {};
    definitions.flags.forEach((flag, index) => {
      flagNameMap[flag.name] = index;
    });

    // Create provider and load artifact
    const provider = new Provider({ flagNameMap });
    await provider.loadArtifact(astFile);

    // Test evaluation
    const adminUser = { id: 'admin1', role: 'admin' };
    const result1 = provider.resolveBooleanEvaluation('new_dashboard', false, adminUser);
    expect(result1.value).toBe(true); // ON converts to true
    expect(result1.reason).toBe('TARGETING_MATCH');

    const regularUser = { id: 'user1', role: 'user' };
    const result2 = provider.resolveBooleanEvaluation('new_dashboard', false, regularUser);
    expect(result2.value).toBe(false); // OFF converts to false
    expect(result2.reason).toBe('TARGETING_MATCH');

    // Test string evaluation
    const result3 = provider.resolveStringEvaluation('theme_color', 'blue', adminUser);
    expect(result3.value).toBe('DARK');
    expect(result3.reason).toBe('TARGETING_MATCH');
  });

  it('should handle context in evaluation', async () => {
    const definitions = parseDefinitions(definitionsFile);
    const deploymentData = parseDeployment(deploymentFile);
    const artifact = compile(deploymentData, definitions);
    const bytes = serialize(artifact);
    const buffer = Buffer.from(bytes);
    const loaded = await loadFromBuffer(buffer);

    const flagNameMap: Record<string, number> = {};
    definitions.flags.forEach((flag, index) => {
      flagNameMap[flag.name] = index;
    });

    const user: User = { id: 'user1', role: 'admin' };
    const context: Context = { environment: 'production', device: 'desktop' };

    // Evaluation should work with context
    const result = evaluate(flagNameMap['new_dashboard'], loaded, user, context);
    expect(result).toBe('ON');
  });
});
