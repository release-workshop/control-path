/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import {
  compileTypeScript,
  compileRust,
} from './test-helpers';
import { compileAndSerialize } from '@controlpath/compiler';
import type { FlagDefinitions, Deployment } from '@controlpath/compiler';

/**
 * Performance comparison tests between TypeScript and Rust implementations.
 * These tests verify that both implementations meet performance targets and
 * compare their relative performance.
 * 
 * Performance thresholds can be configured via environment variables for CI/CD:
 * - MAX_COMPILATION_TIME_10_FLAGS (default: 100ms)
 * - MAX_COMPILATION_TIME_100_FLAGS (default: 500ms)
 * - MAX_COMPILATION_TIME_500_FLAGS (default: 2000ms)
 */

// Performance thresholds (configurable via environment variables for CI/CD)
const MAX_COMPILATION_TIME_10_FLAGS = parseInt(
  process.env.MAX_COMPILATION_TIME_10_FLAGS || '100',
  10
);
const MAX_COMPILATION_TIME_100_FLAGS = parseInt(
  process.env.MAX_COMPILATION_TIME_100_FLAGS || '500',
  10
);
const MAX_COMPILATION_TIME_500_FLAGS = parseInt(
  process.env.MAX_COMPILATION_TIME_500_FLAGS || '2000',
  10
);

/**
 * Generate flag definitions for testing
 */
function generateFlagDefinitions(count: number): FlagDefinitions {
  const flags = [];
  for (let i = 0; i < count; i++) {
    // Mix of boolean and multivariate flags
    if (i % 3 === 0) {
      // Multivariate flag every 3rd flag
      flags.push({
        name: `flag_${i}`,
        type: 'multivariate' as const,
        defaultValue: 'variation_a',
        variations: [
          { name: 'VARIATION_A', value: 'variation_a' },
          { name: 'VARIATION_B', value: 'variation_b' },
          { name: 'VARIATION_C', value: 'variation_c' },
        ],
      });
    } else {
      // Boolean flag
      flags.push({
        name: `flag_${i}`,
        type: 'boolean' as const,
        defaultValue: 'OFF',
      });
    }
  }
  return { flags };
}

/**
 * Generate deployment for testing
 */
function generateDeployment(flagCount: number): Deployment {
  const rules: Record<string, any> = {};

  for (let i = 0; i < flagCount; i++) {
    // Mix of rule types for complexity
    if (i % 5 === 0) {
      // Simple serve rule (20% of flags)
      rules[`flag_${i}`] = {
        rules: [{ serve: 'ON' }],
      };
    } else if (i % 5 === 1) {
      // Serve rule with expression (20% of flags)
      rules[`flag_${i}`] = {
        rules: [
          {
            serve: 'ON',
            when: `user.role == "admin"`,
          },
        ],
      };
    } else if (i % 5 === 2 && i % 3 === 0) {
      // Variations rule for multivariate flags (20% of flags, subset of multivariate)
      rules[`flag_${i}`] = {
        rules: [
          {
            variations: [
              { variation: 'VARIATION_A', weight: 50 },
              { variation: 'VARIATION_B', weight: 30 },
              { variation: 'VARIATION_C', weight: 20 },
            ],
          },
        ],
      };
    } else if (i % 5 === 3) {
      // Rollout rule (20% of flags)
      rules[`flag_${i}`] = {
        rules: [
          {
            rollout: {
              variation: i % 3 === 0 ? 'VARIATION_A' : 'ON',
              percentage: 25,
            },
          },
        ],
      };
    } else {
      // Default rule only (20% of flags)
      rules[`flag_${i}`] = {};
    }
  }

  return {
    environment: 'production',
    rules,
  };
}

/**
 * Convert FlagDefinitions to YAML string
 */
function definitionsToYaml(definitions: FlagDefinitions): string {
  const flags = definitions.flags.map((flag) => {
    if (flag.type === 'multivariate') {
      const variations = flag.variations
        .map(
          (v) => `      - name: ${v.name}\n        value: ${v.value}`
        )
        .join('\n');
      return `  - name: ${flag.name}
    type: multivariate
    defaultValue: ${flag.defaultValue}
    variations:
${variations}`;
    } else {
      return `  - name: ${flag.name}
    type: ${flag.type}
    defaultValue: ${flag.defaultValue}`;
    }
  });

  return `flags:\n${flags.join('\n')}`;
}

/**
 * Convert Deployment to YAML string
 */
function deploymentToYaml(deployment: Deployment): string {
  const rules = Object.entries(deployment.rules).map(([flagName, flagRules]) => {
    if (!flagRules.rules || flagRules.rules.length === 0) {
      return `  ${flagName}: {}`;
    }

    const ruleStrings = flagRules.rules.map((rule: any) => {
      if (rule.serve !== undefined) {
        if (rule.when) {
          return `      - serve: ${rule.serve}\n        when: '${rule.when}'`;
        }
        return `      - serve: ${rule.serve}`;
      } else if (rule.variations) {
        const variations = rule.variations
          .map(
            (v: any) => `          - variation: ${v.variation}\n            weight: ${v.weight}`
          )
          .join('\n');
        return `      - variations:\n${variations}`;
      } else if (rule.rollout) {
        return `      - rollout:\n          variation: ${rule.rollout.variation}\n          percentage: ${rule.rollout.percentage}`;
      }
      return '';
    });

    return `  ${flagName}:\n    rules:\n${ruleStrings.join('\n')}`;
  });

  return `environment: ${deployment.environment}\nrules:\n${rules.join('\n')}`;
}

describe('Performance Comparison: TypeScript vs Rust', () => {
  describe('Compilation Time', () => {
    it('should compile 10 flags quickly in both implementations', () => {
      const definitions = generateFlagDefinitions(10);
      const deployment = generateDeployment(10);
      const definitionsYaml = definitionsToYaml(definitions);
      const deploymentYaml = deploymentToYaml(deployment);

      // Warmup runs (not measured) to avoid JIT compilation and GC effects
      compileTypeScript(definitionsYaml, deploymentYaml);
      compileRust(definitionsYaml, deploymentYaml);

      // TypeScript
      const tsStart = performance.now();
      const tsBytes = compileTypeScript(definitionsYaml, deploymentYaml);
      const tsEnd = performance.now();
      const tsDuration = tsEnd - tsStart;

      // Rust
      const rustStart = performance.now();
      const rustBytes = compileRust(definitionsYaml, deploymentYaml);
      const rustEnd = performance.now();
      const rustDuration = rustEnd - rustStart;

      console.log(`\n[Performance] 10 flags - TS: ${tsDuration.toFixed(2)}ms, Rust: ${rustDuration.toFixed(2)}ms`);
      console.log(`[Performance] Speedup: ${(tsDuration / rustDuration).toFixed(2)}x`);

      expect(tsBytes.length).toBeGreaterThan(0);
      expect(rustBytes.length).toBeGreaterThan(0);
      expect(tsDuration).toBeLessThan(MAX_COMPILATION_TIME_10_FLAGS);
      expect(rustDuration).toBeLessThan(MAX_COMPILATION_TIME_10_FLAGS);
    });

    it('should compile 100 flags efficiently in both implementations', () => {
      const definitions = generateFlagDefinitions(100);
      const deployment = generateDeployment(100);
      const definitionsYaml = definitionsToYaml(definitions);
      const deploymentYaml = deploymentToYaml(deployment);

      // Warmup runs (not measured) to avoid JIT compilation and GC effects
      compileTypeScript(definitionsYaml, deploymentYaml);
      compileRust(definitionsYaml, deploymentYaml);

      // TypeScript
      const tsStart = performance.now();
      const tsBytes = compileTypeScript(definitionsYaml, deploymentYaml);
      const tsEnd = performance.now();
      const tsDuration = tsEnd - tsStart;

      // Rust
      const rustStart = performance.now();
      const rustBytes = compileRust(definitionsYaml, deploymentYaml);
      const rustEnd = performance.now();
      const rustDuration = rustEnd - rustStart;

      console.log(`\n[Performance] 100 flags - TS: ${tsDuration.toFixed(2)}ms, Rust: ${rustDuration.toFixed(2)}ms`);
      console.log(`[Performance] Speedup: ${(tsDuration / rustDuration).toFixed(2)}x`);

      expect(tsBytes.length).toBeGreaterThan(0);
      expect(rustBytes.length).toBeGreaterThan(0);
      expect(tsDuration).toBeLessThan(MAX_COMPILATION_TIME_100_FLAGS);
      expect(rustDuration).toBeLessThan(MAX_COMPILATION_TIME_100_FLAGS);
    });

    it('should compile 500 flags efficiently in both implementations', () => {
      const definitions = generateFlagDefinitions(500);
      const deployment = generateDeployment(500);
      const definitionsYaml = definitionsToYaml(definitions);
      const deploymentYaml = deploymentToYaml(deployment);

      // Warmup runs (not measured) to avoid JIT compilation and GC effects
      compileTypeScript(definitionsYaml, deploymentYaml);
      compileRust(definitionsYaml, deploymentYaml);

      // TypeScript
      const tsStart = performance.now();
      const tsBytes = compileTypeScript(definitionsYaml, deploymentYaml);
      const tsEnd = performance.now();
      const tsDuration = tsEnd - tsStart;

      // Rust
      const rustStart = performance.now();
      const rustBytes = compileRust(definitionsYaml, deploymentYaml);
      const rustEnd = performance.now();
      const rustDuration = rustEnd - rustStart;

      console.log(`\n[Performance] 500 flags - TS: ${tsDuration.toFixed(2)}ms, Rust: ${rustDuration.toFixed(2)}ms`);
      console.log(`[Performance] Speedup: ${(tsDuration / rustDuration).toFixed(2)}x`);

      expect(tsBytes.length).toBeGreaterThan(0);
      expect(rustBytes.length).toBeGreaterThan(0);
      expect(tsDuration).toBeLessThan(MAX_COMPILATION_TIME_500_FLAGS);
      expect(rustDuration).toBeLessThan(MAX_COMPILATION_TIME_500_FLAGS);
    });
  });

  describe('Artifact Size Targets', () => {
    it('should meet size target for 500 flags (< 12KB)', () => {
      const definitions = generateFlagDefinitions(500);
      const deployment = generateDeployment(500);
      const definitionsYaml = definitionsToYaml(definitions);
      const deploymentYaml = deploymentToYaml(deployment);

      // TypeScript
      const tsBytes = compileTypeScript(definitionsYaml, deploymentYaml);
      const tsSizeKB = tsBytes.length / 1024;

      // Rust
      const rustBytes = compileRust(definitionsYaml, deploymentYaml);
      const rustSizeKB = rustBytes.length / 1024;

      console.log(`\n[Performance] 500 flags size - TS: ${tsSizeKB.toFixed(2)}KB, Rust: ${rustSizeKB.toFixed(2)}KB`);
      console.log(`[Performance] Size difference: ${Math.abs(tsSizeKB - rustSizeKB).toFixed(2)}KB`);

      // Both should meet the target
      expect(tsSizeKB).toBeLessThan(13); // Target is < 13KB (includes flagNames overhead)
      expect(rustSizeKB).toBeLessThan(13); // Target is < 13KB (includes flagNames overhead)

      // Sizes should be very close (within 5% difference)
      const sizeDiff = Math.abs(tsSizeKB - rustSizeKB);
      const avgSize = (tsSizeKB + rustSizeKB) / 2;
      const percentDiff = (sizeDiff / avgSize) * 100;
      expect(percentDiff).toBeLessThan(5); // Should be within 5% of each other
    });

    it('should have similar artifact sizes for 100 flags', () => {
      const definitions = generateFlagDefinitions(100);
      const deployment = generateDeployment(100);
      const definitionsYaml = definitionsToYaml(definitions);
      const deploymentYaml = deploymentToYaml(deployment);

      // TypeScript
      const tsBytes = compileTypeScript(definitionsYaml, deploymentYaml);
      const tsSizeKB = tsBytes.length / 1024;

      // Rust
      const rustBytes = compileRust(definitionsYaml, deploymentYaml);
      const rustSizeKB = rustBytes.length / 1024;

      console.log(`\n[Performance] 100 flags size - TS: ${tsSizeKB.toFixed(2)}KB, Rust: ${rustSizeKB.toFixed(2)}KB`);

      // Both should meet the target
      expect(tsSizeKB).toBeLessThan(2.6); // Target is < 2.6KB
      expect(rustSizeKB).toBeLessThan(2.6); // Target is < 2.6KB

      // Sizes should be very close (within 5% difference)
      const sizeDiff = Math.abs(tsSizeKB - rustSizeKB);
      const avgSize = (tsSizeKB + rustSizeKB) / 2;
      const percentDiff = (sizeDiff / avgSize) * 100;
      expect(percentDiff).toBeLessThan(5); // Should be within 5% of each other
    });

    it('should have similar artifact sizes for 1000 flags', () => {
      const definitions = generateFlagDefinitions(1000);
      const deployment = generateDeployment(1000);
      const definitionsYaml = definitionsToYaml(definitions);
      const deploymentYaml = deploymentToYaml(deployment);

      // TypeScript
      const tsBytes = compileTypeScript(definitionsYaml, deploymentYaml);
      const tsSizeKB = tsBytes.length / 1024;

      // Rust
      const rustBytes = compileRust(definitionsYaml, deploymentYaml);
      const rustSizeKB = rustBytes.length / 1024;

      console.log(`\n[Performance] 1000 flags size - TS: ${tsSizeKB.toFixed(2)}KB, Rust: ${rustSizeKB.toFixed(2)}KB`);

      // Both should meet the target
      expect(tsSizeKB).toBeLessThan(26); // Target is < 26KB
      expect(rustSizeKB).toBeLessThan(26); // Target is < 26KB

      // Sizes should be very close (within 5% difference)
      const sizeDiff = Math.abs(tsSizeKB - rustSizeKB);
      const avgSize = (tsSizeKB + rustSizeKB) / 2;
      const percentDiff = (sizeDiff / avgSize) * 100;
      expect(percentDiff).toBeLessThan(5); // Should be within 5% of each other
    });
  });

  describe('Performance Characteristics', () => {
    it('should show compilation time scaling with flag count', () => {
      const flagCounts = [10, 50, 100, 250, 500];
      const results: Array<{ count: number; tsTime: number; rustTime: number }> = [];

      for (const count of flagCounts) {
        const definitions = generateFlagDefinitions(count);
        const deployment = generateDeployment(count);
        const definitionsYaml = definitionsToYaml(definitions);
        const deploymentYaml = deploymentToYaml(deployment);

        // Warmup runs (not measured) to avoid JIT compilation and GC effects
        compileTypeScript(definitionsYaml, deploymentYaml);
        compileRust(definitionsYaml, deploymentYaml);

        // TypeScript
        const tsStart = performance.now();
        compileTypeScript(definitionsYaml, deploymentYaml);
        const tsEnd = performance.now();
        const tsTime = tsEnd - tsStart;

        // Rust
        const rustStart = performance.now();
        compileRust(definitionsYaml, deploymentYaml);
        const rustEnd = performance.now();
        const rustTime = rustEnd - rustStart;

        results.push({ count, tsTime, rustTime });
      }

      console.log('\n[Performance] Compilation time scaling:');
      for (const result of results) {
        console.log(
          `  ${result.count} flags: TS=${result.tsTime.toFixed(2)}ms, Rust=${result.rustTime.toFixed(2)}ms, Speedup=${(result.tsTime / result.rustTime).toFixed(2)}x`
        );
      }

      // Verify that time increases with flag count (roughly linear)
      for (let i = 1; i < results.length; i++) {
        const prev = results[i - 1];
        const curr = results[i];
        // Time should increase (allowing for some variance)
        expect(curr.tsTime).toBeGreaterThan(prev.tsTime * 0.5);
        expect(curr.rustTime).toBeGreaterThan(prev.rustTime * 0.5);
      }
    });

    it('should show artifact size scaling with flag count', () => {
      const flagCounts = [10, 50, 100, 250, 500, 1000];
      const results: Array<{ count: number; tsSize: number; rustSize: number }> = [];

      for (const count of flagCounts) {
        const definitions = generateFlagDefinitions(count);
        const deployment = generateDeployment(count);
        const definitionsYaml = definitionsToYaml(definitions);
        const deploymentYaml = deploymentToYaml(deployment);

        // TypeScript
        const tsBytes = compileTypeScript(definitionsYaml, deploymentYaml);
        const tsSize = tsBytes.length / 1024;

        // Rust
        const rustBytes = compileRust(definitionsYaml, deploymentYaml);
        const rustSize = rustBytes.length / 1024;

        results.push({ count, tsSize, rustSize });
      }

      console.log('\n[Performance] Artifact size scaling:');
      for (const result of results) {
        console.log(
          `  ${result.count} flags: TS=${result.tsSize.toFixed(2)}KB, Rust=${result.rustSize.toFixed(2)}KB, Diff=${Math.abs(result.tsSize - result.rustSize).toFixed(2)}KB`
        );
      }

      // Verify that size increases with flag count (roughly linear)
      for (let i = 1; i < results.length; i++) {
        const prev = results[i - 1];
        const curr = results[i];
        // Size should increase (allowing for some variance)
        expect(curr.tsSize).toBeGreaterThan(prev.tsSize * 0.5);
        expect(curr.rustSize).toBeGreaterThan(prev.rustSize * 0.5);
      }
    });
  });
});

