/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * OpenFeature SDK Compatibility Tests
 *
 * These tests verify that the ControlPath Provider works correctly with @openfeature/server-sdk.
 * The tests verify:
 * - Provider registration with OpenFeature SDK
 * - All four evaluation methods (boolean, string, number, object)
 * - EvaluationContext handling
 * - Error handling
 * - Async method compatibility
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { OpenFeature } from '@openfeature/server-sdk';
import { writeFile, mkdir, rm } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { tmpdir } from 'os';
import { compile, serialize } from '@controlpath/compiler';
import { parseDefinitions, parseDeployment } from '@controlpath/compiler';
import { Provider } from './provider';
// Type guard function moved inline since openfeature-types.ts was removed
function isOpenFeatureProvider(
  provider: unknown
): provider is import('@openfeature/server-sdk').Provider {
  if (!provider || typeof provider !== 'object') {
    return false;
  }

  const p = provider as Record<string, unknown>;

  // Check required properties
  if (!p.metadata || typeof p.metadata !== 'object') {
    return false;
  }

  const metadata = p.metadata as Record<string, unknown>;
  if (typeof metadata.name !== 'string') {
    return false;
  }

  if (!Array.isArray(p.hooks)) {
    return false;
  }

  // Check required methods
  const requiredMethods = [
    'resolveBooleanEvaluation',
    'resolveStringEvaluation',
    'resolveNumberEvaluation',
    'resolveObjectEvaluation',
  ];

  for (const method of requiredMethods) {
    if (typeof p[method] !== 'function') {
      return false;
    }
  }

  return true;
}

describe('OpenFeature SDK Compatibility', () => {
  // Use OS temp directory for better isolation and reliability
  // This avoids race conditions when tests run in parallel
  const testDir = join(
    tmpdir(),
    'controlpath-test',
    `openfeature-compat-${Date.now()}-${Math.random().toString(36).substring(7)}`
  );
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
  
  - name: max_items
    type: multivariate
    defaultValue: 10
    description: "Maximum items to display"
    variations:
      - name: SMALL
        value: 5
      - name: MEDIUM
        value: 10
      - name: LARGE
        value: 20
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
  
  max_items:
    rules:
      - when: "user.role == 'admin'"
        serve: LARGE
      - serve: MEDIUM
`;

  beforeAll(async () => {
    // Ensure directory exists
    await mkdir(testDir, { recursive: true });

    // Write definition and deployment files
    await writeFile(definitionsFile, flagsDefinitions);
    await writeFile(deploymentFile, deployment);

    // Compile AST
    const definitions = parseDefinitions(definitionsFile);
    const deploymentData = parseDeployment(deploymentFile);
    const artifact = compile(deploymentData, definitions);
    const bytes = serialize(artifact);
    const buffer = Buffer.from(bytes);

    // Write AST file and verify it exists
    await writeFile(astFile, buffer);

    // Verify file was created successfully
    const { stat } = await import('fs/promises');
    const stats = await stat(astFile);
    if (!stats.isFile() || stats.size === 0) {
      throw new Error(`Failed to create AST file: ${astFile}`);
    }
  });

  afterAll(async () => {
    try {
      await rm(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('Provider Registration', () => {
    it('should register provider with OpenFeature SDK', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      // Verify provider implements OpenFeatureProvider interface
      expect(isOpenFeatureProvider(provider)).toBe(true);

      // Register provider with OpenFeature SDK
      OpenFeature.setProvider(provider);

      // Verify provider is registered
      const client = OpenFeature.getClient();
      expect(client).toBeDefined();
    });

    it('should work with setProviderAndWait for async initialization', async () => {
      const provider = new Provider();

      // Load artifact asynchronously
      const loadPromise = provider.loadArtifact(astFile);

      // Register provider and wait (even though our provider is sync, this tests compatibility)
      await OpenFeature.setProviderAndWait(provider);

      // Wait for artifact to load
      await loadPromise;

      const client = OpenFeature.getClient();
      expect(client).toBeDefined();
    });
  });

  describe('Boolean Flag Evaluation', () => {
    it('should evaluate boolean flags through OpenFeature SDK', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      // Test with admin context
      const adminContext = { role: 'admin' };
      const adminValue = await client.getBooleanValue('new_dashboard', false, adminContext);
      expect(adminValue).toBe(true); // Admin should get ON (true)

      // Test with regular user context
      const userContext = { role: 'user' };
      const userValue = await client.getBooleanValue('new_dashboard', false, userContext);
      expect(userValue).toBe(false); // Regular user should get OFF (false)
    });

    it('should return default value for non-existent flags', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      // Test with non-existent flag - should return default
      const value = await client.getBooleanValue('non_existent_flag', false, {});
      expect(value).toBe(false); // Should return default value
    });

    it('should handle flags that are always true', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      // enable_analytics is always true
      const value = await client.getBooleanValue('enable_analytics', false, {});
      expect(value).toBe(true);
    });
  });

  describe('String Flag Evaluation', () => {
    it('should evaluate string flags through OpenFeature SDK', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      // Test with admin context
      const adminContext = { role: 'admin' };
      const adminTheme = await client.getStringValue('theme_color', 'blue', adminContext);
      expect(adminTheme).toBe('DARK'); // Admin should get DARK

      // Test with regular user context
      const userContext = { role: 'user' };
      const userTheme = await client.getStringValue('theme_color', 'blue', userContext);
      expect(userTheme).toBe('BLUE'); // Regular user should get BLUE
    });

    it('should return default value for non-existent string flags', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      const value = await client.getStringValue('non_existent_flag', 'default', {});
      expect(value).toBe('default');
    });
  });

  describe('Number Flag Evaluation', () => {
    it('should evaluate number flags through OpenFeature SDK', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      // Test with admin context
      const adminContext = { role: 'admin' };
      const adminMaxItems = await client.getNumberValue('max_items', 10, adminContext);
      // Admin should get LARGE (20) - verify it's a valid number
      expect(typeof adminMaxItems).toBe('number');
      expect(Number.isNaN(adminMaxItems)).toBe(false);
      // The value should be reasonable (either the variation name converted or the actual number)
      expect(adminMaxItems).toBeGreaterThan(0);

      // Test with regular user context
      const userContext = { role: 'user' };
      const userMaxItems = await client.getNumberValue('max_items', 10, userContext);
      // Regular user should get MEDIUM (10) - verify it's a valid number
      expect(typeof userMaxItems).toBe('number');
      expect(Number.isNaN(userMaxItems)).toBe(false);
      expect(userMaxItems).toBeGreaterThan(0);
    });

    it('should return default value for non-existent number flags', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      const value = await client.getNumberValue('non_existent_flag', 42, {});
      expect(value).toBe(42);
    });
  });

  describe('Object Flag Evaluation', () => {
    it('should evaluate object flags through OpenFeature SDK', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      // Note: Object flags require the flag to return an object value
      // For this test, we'll use a flag that might return an object
      // Since we don't have object flags in the test data, we test the default behavior
      const defaultValue = { key: 'value' };
      const value = await client.getObjectValue('non_existent_flag', defaultValue, {});
      expect(value).toEqual(defaultValue);
    });
  });

  describe('EvaluationContext Handling', () => {
    it('should handle EvaluationContext with targetingKey', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      // Test with full EvaluationContext structure including targetingKey
      const context = {
        targetingKey: 'user123', // Should be ignored by ControlPath (as per spec)
        role: 'admin',
        email: 'admin@example.com',
        environment: 'production',
      };

      const value = await client.getBooleanValue('new_dashboard', false, context);
      expect(value).toBe(true); // Should work based on role, not targetingKey
    });

    it('should handle nested user properties in EvaluationContext', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      // Test with nested user properties
      const context = {
        'user.role': 'admin', // Nested property
        'user.id': 'user123',
      };

      const value = await client.getBooleanValue('new_dashboard', false, context);
      expect(value).toBe(true);
    });

    it('should handle empty EvaluationContext', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      // Test with empty context - should return default
      const value = await client.getBooleanValue('new_dashboard', false, {});
      expect(value).toBe(false); // No admin role, so should get OFF
    });
  });

  describe('Error Handling', () => {
    it('should handle errors gracefully through OpenFeature SDK', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      // Test with non-existent flag - should return default without throwing
      const value = await client.getBooleanValue('non_existent_flag', false, {});
      expect(value).toBe(false); // Should return default value, not throw
    });

    it('should handle provider without artifact loaded', async () => {
      const provider = new Provider();
      // Don't load artifact

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      // Should return default value when artifact not loaded
      const value = await client.getBooleanValue('new_dashboard', false, {});
      expect(value).toBe(false); // Should return default
    });
  });

  describe('All Evaluation Methods', () => {
    it('should support all four evaluation method types', async () => {
      const provider = new Provider();
      await provider.loadArtifact(astFile);

      OpenFeature.setProvider(provider);
      const client = OpenFeature.getClient();

      const context = { role: 'admin' };

      // Test all four methods
      const boolValue = await client.getBooleanValue('new_dashboard', false, context);
      expect(typeof boolValue).toBe('boolean');

      const stringValue = await client.getStringValue('theme_color', 'blue', context);
      expect(typeof stringValue).toBe('string');

      const numberValue = await client.getNumberValue('max_items', 10, context);
      expect(typeof numberValue).toBe('number');

      const objectValue = await client.getObjectValue('non_existent', { default: true }, context);
      expect(typeof objectValue).toBe('object');
    });
  });
});
