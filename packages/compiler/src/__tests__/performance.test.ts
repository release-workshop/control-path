/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import { compileAndSerialize } from '../compiler';
import { FlagDefinitions, Deployment } from '../parser/types';
import { unpack } from 'msgpackr';
import { Artifact, isArtifact } from '../ast';

/**
 * Performance tests to verify AST size targets.
 * Target: < 12KB for 500 flags (mixed complexity)
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

describe('Performance: AST Size Targets', () => {
  it('should meet size target for 100 flags (< 2.4KB)', () => {
    // Given: 100 flags with mixed complexity
    const definitions = generateFlagDefinitions(100);
    const deployment = generateDeployment(100);

    // When: We compile and serialize
    const serialized = compileAndSerialize(deployment, definitions);
    const sizeKB = serialized.length / 1024;
    const margin = ((2.4 - sizeKB) / 2.4) * 100;

    // Log actual performance for analysis
    console.log(
      `\n[Performance] 100 flags: ${sizeKB.toFixed(2)} KB (target: < 2.4 KB, margin: ${margin.toFixed(1)}%)`
    );

    // Then: Size should be under target (2.4KB for 100 flags, scaled from 12KB for 500)
    expect(sizeKB).toBeLessThan(2.4);
    expect(serialized.length).toBeGreaterThan(0);

    // Verify it can be deserialized
    const deserialized = unpack(serialized) as Artifact;
    expect(isArtifact(deserialized)).toBe(true);
    expect(deserialized.flags).toHaveLength(100);
  });

  it('should meet size target for 250 flags (< 6KB)', () => {
    // Given: 250 flags with mixed complexity
    const definitions = generateFlagDefinitions(250);
    const deployment = generateDeployment(250);

    // When: We compile and serialize
    const serialized = compileAndSerialize(deployment, definitions);
    const sizeKB = serialized.length / 1024;
    const margin = ((6 - sizeKB) / 6) * 100;

    // Log actual performance for analysis
    console.log(
      `\n[Performance] 250 flags: ${sizeKB.toFixed(2)} KB (target: < 6 KB, margin: ${margin.toFixed(1)}%)`
    );

    // Then: Size should be under target (6KB for 250 flags, scaled from 12KB for 500)
    expect(sizeKB).toBeLessThan(6);
    expect(serialized.length).toBeGreaterThan(0);

    // Verify it can be deserialized
    const deserialized = unpack(serialized) as Artifact;
    expect(isArtifact(deserialized)).toBe(true);
    expect(deserialized.flags).toHaveLength(250);
  });

  it('should meet size target for 500 flags (< 12KB)', () => {
    // Given: 500 flags with mixed complexity
    const definitions = generateFlagDefinitions(500);
    const deployment = generateDeployment(500);

    // When: We compile and serialize
    const serialized = compileAndSerialize(deployment, definitions);
    const sizeKB = serialized.length / 1024;
    const bytesPerFlag = serialized.length / 500;
    const margin = ((12 - sizeKB) / 12) * 100;

    // Log actual performance for analysis
    console.log(
      `\n[Performance] 500 flags: ${sizeKB.toFixed(2)} KB (target: < 12 KB, margin: ${margin.toFixed(1)}%)`
    );
    console.log(`[Performance] Bytes per flag: ${bytesPerFlag.toFixed(2)}`);

    // Then: Size should be under target (12KB for 500 flags)
    expect(sizeKB).toBeLessThan(12);
    expect(serialized.length).toBeGreaterThan(0);

    // Verify it can be deserialized
    const deserialized = unpack(serialized) as Artifact;
    expect(isArtifact(deserialized)).toBe(true);
    expect(deserialized.flags).toHaveLength(500);
    expect(deserialized.v).toBe('1.0');
    expect(deserialized.env).toBe('production');
  });

  it('should meet size target for 1000 flags (< 24KB)', () => {
    // Given: 1000 flags with mixed complexity
    const definitions = generateFlagDefinitions(1000);
    const deployment = generateDeployment(1000);

    // When: We compile and serialize
    const serialized = compileAndSerialize(deployment, definitions);
    const sizeKB = serialized.length / 1024;
    const margin = ((24 - sizeKB) / 24) * 100;

    // Log actual performance for analysis
    console.log(
      `\n[Performance] 1000 flags: ${sizeKB.toFixed(2)} KB (target: < 24 KB, margin: ${margin.toFixed(1)}%)`
    );

    // Then: Size should be under target (24KB for 1000 flags)
    expect(sizeKB).toBeLessThan(24);
    expect(serialized.length).toBeGreaterThan(0);

    // Verify it can be deserialized
    const deserialized = unpack(serialized) as Artifact;
    expect(isArtifact(deserialized)).toBe(true);
    expect(deserialized.flags).toHaveLength(1000);
  });

  it('should have reasonable size for simple flags (no rules)', () => {
    // Given: 100 simple flags with no rules (just defaults)
    const definitions: FlagDefinitions = {
      flags: Array.from({ length: 100 }, (_, i) => ({
        name: `flag_${i}`,
        type: 'boolean' as const,
        defaultValue: 'OFF',
      })),
    };

    const deployment: Deployment = {
      environment: 'production',
      rules: Object.fromEntries(Array.from({ length: 100 }, (_, i) => [`flag_${i}`, {}])),
    };

    // When: We compile and serialize
    const serialized = compileAndSerialize(deployment, definitions);
    const sizeKB = serialized.length / 1024;
    const bytesPerFlag = serialized.length / 100;

    // Log actual performance for analysis
    console.log(
      `\n[Performance] 100 simple flags: ${sizeKB.toFixed(2)} KB, ${bytesPerFlag.toFixed(2)} bytes/flag (target: ~5-8 bytes/flag)`
    );

    // Then: Should be very compact (target: ~5-8 bytes per flag)
    expect(bytesPerFlag).toBeLessThan(10);
    expect(sizeKB).toBeLessThan(1); // Should be well under 1KB for 100 simple flags

    // Verify it can be deserialized
    const deserialized = unpack(serialized) as Artifact;
    expect(isArtifact(deserialized)).toBe(true);
    expect(deserialized.flags).toHaveLength(100);
  });

  it('should have reasonable size for flags with expressions', () => {
    // Given: 50 flags with expression rules
    const definitions: FlagDefinitions = {
      flags: Array.from({ length: 50 }, (_, i) => ({
        name: `flag_${i}`,
        type: 'boolean' as const,
        defaultValue: 'OFF',
      })),
    };

    const deployment: Deployment = {
      environment: 'production',
      rules: Object.fromEntries(
        Array.from({ length: 50 }, (_, i) => [
          `flag_${i}`,
          {
            rules: [
              {
                serve: 'ON',
                when: `user.role == "admin" AND user.department == "engineering"`,
              },
            ],
          },
        ])
      ),
    };

    // When: We compile and serialize
    const serialized = compileAndSerialize(deployment, definitions);
    const sizeKB = serialized.length / 1024;
    const bytesPerFlag = serialized.length / 50;

    // Log actual performance for analysis
    console.log(
      `\n[Performance] 50 flags with expressions: ${sizeKB.toFixed(2)} KB, ${bytesPerFlag.toFixed(2)} bytes/flag (target: ~25-50 bytes/flag)`
    );

    // Then: Should be reasonable (target: ~25-50 bytes per flag with expression)
    expect(bytesPerFlag).toBeLessThan(60);
    expect(sizeKB).toBeLessThan(3); // Should be under 3KB for 50 flags with expressions

    // Verify it can be deserialized
    const deserialized = unpack(serialized) as Artifact;
    expect(isArtifact(deserialized)).toBe(true);
    expect(deserialized.flags).toHaveLength(50);
  });

  it('should verify string table deduplication helps with size', () => {
    // Given: Flags with many repeated strings
    const definitions: FlagDefinitions = {
      flags: Array.from({ length: 100 }, (_, i) => ({
        name: `flag_${i}`,
        type: 'boolean' as const,
        defaultValue: 'OFF',
      })),
    };

    const deployment: Deployment = {
      environment: 'production',
      rules: Object.fromEntries(
        Array.from({ length: 100 }, (_, i) => [
          `flag_${i}`,
          {
            rules: [
              {
                serve: 'ON',
                when: `user.role == "admin"`,
              },
            ],
          },
        ])
      ),
    };

    // When: We compile and serialize
    const serialized = compileAndSerialize(deployment, definitions);
    const deserialized = unpack(serialized) as Artifact;

    // Then: String table should deduplicate repeated strings
    // "ON", "user.role", "admin" should each appear only once
    const onCount = deserialized.strs.filter((s) => s === 'ON').length;
    const roleCount = deserialized.strs.filter((s) => s === 'user.role').length;
    const adminCount = deserialized.strs.filter((s) => s === 'admin').length;

    expect(onCount).toBe(1);
    expect(roleCount).toBe(1);
    expect(adminCount).toBe(1);
    expect(isArtifact(deserialized)).toBe(true);
  });
});
