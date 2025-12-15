/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import { compile, serialize, compileAndSerialize } from './index';
import { FlagDefinitions, Deployment } from '../parser/types';
import { Artifact, RuleType, ExpressionType, BinaryOp } from '../ast';
import { unpack } from 'msgpackr';

describe('compile', () => {
  describe('Basic compilation', () => {
    it('should compile simple deployment with no rules', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'new_dashboard',
            type: 'boolean',
            defaultValue: 'OFF',
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          new_dashboard: {
            default: 'OFF',
          },
        },
      };

      const artifact = compile(deployment, definitions);

      expect(artifact.v).toBe('1.0');
      expect(artifact.env).toBe('production');
      expect(artifact.flags).toHaveLength(1);
      expect(artifact.flags[0]).toEqual([]); // No rules
    });

    it('should include format version and environment', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'test_flag',
            type: 'boolean',
            defaultValue: false,
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'staging',
        rules: {
          test_flag: {
            default: false,
          },
        },
      };

      const artifact = compile(deployment, definitions);

      expect(artifact.v).toBe('1.0');
      expect(artifact.env).toBe('staging');
    });

    it('should create flags array matching definitions order', () => {
      const definitions: FlagDefinitions = {
        flags: [
          { name: 'flag1', type: 'boolean', defaultValue: false },
          { name: 'flag2', type: 'boolean', defaultValue: false },
          { name: 'flag3', type: 'boolean', defaultValue: false },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          flag3: { default: false },
          flag1: { default: false },
          flag2: { default: false },
        },
      };

      const artifact = compile(deployment, definitions);

      expect(artifact.flags).toHaveLength(3);
      // Flags should be in definition order, not deployment order
      expect(artifact.flags[0]).toEqual([]); // flag1
      expect(artifact.flags[1]).toEqual([]); // flag2
      expect(artifact.flags[2]).toEqual([]); // flag3
    });
  });

  describe('Serve rules', () => {
    it('should compile serve rule without when clause', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'new_dashboard',
            type: 'boolean',
            defaultValue: 'OFF',
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          new_dashboard: {
            default: 'OFF',
            rules: [
              {
                name: 'Enable for all',
                serve: 'ON',
              },
            ],
          },
        },
      };

      const artifact = compile(deployment, definitions);

      expect(artifact.flags[0]).toHaveLength(1);
      const rule = artifact.flags[0][0];
      expect(rule[0]).toBe(RuleType.SERVE);
      expect(rule[1]).toBeUndefined(); // No when clause
      expect(typeof rule[2]).toBe('number'); // Value index
      expect(artifact.strs[rule[2] as number]).toBe('ON');
    });

    it('should compile serve rule with when clause', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'new_dashboard',
            type: 'boolean',
            defaultValue: 'OFF',
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          new_dashboard: {
            default: 'OFF',
            rules: [
              {
                name: 'Enable for admins',
                when: "user.role == 'admin'",
                serve: 'ON',
              },
            ],
          },
        },
      };

      const artifact = compile(deployment, definitions);

      expect(artifact.flags[0]).toHaveLength(1);
      const rule = artifact.flags[0][0];
      expect(rule[0]).toBe(RuleType.SERVE);
      expect(rule[1]).toBeDefined(); // Has when clause
      expect(rule[1]![0]).toBe(ExpressionType.BINARY_OP);
      expect(artifact.strs[rule[2] as number]).toBe('ON');
    });

    it('should normalize boolean values for boolean flags', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'enable_feature',
            type: 'boolean',
            defaultValue: false,
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          enable_feature: {
            default: false,
            rules: [
              {
                serve: true,
              },
              {
                serve: false,
              },
            ],
          },
        },
      };

      const artifact = compile(deployment, definitions);

      expect(artifact.flags[0]).toHaveLength(2);
      expect(artifact.strs[artifact.flags[0][0][2] as number]).toBe('ON');
      expect(artifact.strs[artifact.flags[0][1][2] as number]).toBe('OFF');
    });
  });

  describe('Variations rules', () => {
    it('should compile variations rule', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'button_color',
            type: 'multivariate',
            defaultValue: 'blue',
            variations: [
              { name: 'blue', value: 'blue' },
              { name: 'red', value: 'red' },
              { name: 'green', value: 'green' },
            ],
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          button_color: {
            default: 'blue',
            rules: [
              {
                variations: [
                  { variation: 'blue', weight: 50 },
                  { variation: 'red', weight: 30 },
                  { variation: 'green', weight: 20 },
                ],
              },
            ],
          },
        },
      };

      const artifact = compile(deployment, definitions);

      expect(artifact.flags[0]).toHaveLength(1);
      const rule = artifact.flags[0][0];
      expect(rule[0]).toBe(RuleType.VARIATIONS);
      expect(Array.isArray(rule[2])).toBe(true);
      const variations = rule[2] as Array<[number, number]>;
      expect(variations).toHaveLength(3);
      expect(variations[0][1]).toBe(50);
      expect(variations[1][1]).toBe(30);
      expect(variations[2][1]).toBe(20);
    });

    it('should throw error if variation not found', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'button_color',
            type: 'multivariate',
            defaultValue: 'blue',
            variations: [{ name: 'blue', value: 'blue' }],
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          button_color: {
            default: 'blue',
            rules: [
              {
                variations: [{ variation: 'invalid', weight: 100 }],
              },
            ],
          },
        },
      };

      expect(() => compile(deployment, definitions)).toThrow(
        'Variation "invalid" not found'
      );
    });
  });

  describe('Rollout rules', () => {
    it('should compile rollout rule', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'new_dashboard',
            type: 'boolean',
            defaultValue: 'OFF',
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          new_dashboard: {
            default: 'OFF',
            rules: [
              {
                rollout: {
                  variation: 'ON',
                  percentage: 10,
                },
              },
            ],
          },
        },
      };

      const artifact = compile(deployment, definitions);

      expect(artifact.flags[0]).toHaveLength(1);
      const rule = artifact.flags[0][0];
      expect(rule[0]).toBe(RuleType.ROLLOUT);
      const payload = rule[2] as [number, number];
      expect(payload[1]).toBe(10);
    });

    it('should clamp percentage to 0-100', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'new_dashboard',
            type: 'boolean',
            defaultValue: 'OFF',
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          new_dashboard: {
            default: 'OFF',
            rules: [
              {
                rollout: {
                  variation: 'ON',
                  percentage: 150, // Over 100
                },
              },
            ],
          },
        },
      };

      const artifact = compile(deployment, definitions);
      const rule = artifact.flags[0][0];
      const payload = rule[2] as [number, number];
      expect(payload[1]).toBe(100);
    });
  });

  describe('Segments', () => {
    it('should compile segments if present', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'new_dashboard',
            type: 'boolean',
            defaultValue: 'OFF',
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          new_dashboard: {
            default: 'OFF',
          },
        },
        segments: {
          beta_users: {
            when: "user.role == 'beta'",
          },
        },
      };

      const artifact = compile(deployment, definitions);

      expect(artifact.segments).toBeDefined();
      expect(artifact.segments!).toHaveLength(1);
      const [nameIndex, expr] = artifact.segments![0];
      expect(artifact.strs[nameIndex]).toBe('beta_users');
      expect(expr[0]).toBe(ExpressionType.BINARY_OP);
    });

    it('should not include segments field if no segments', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'test_flag',
            type: 'boolean',
            defaultValue: false,
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          test_flag: {
            default: false,
          },
        },
      };

      const artifact = compile(deployment, definitions);

      expect(artifact.segments).toBeUndefined();
    });
  });

  describe('String table', () => {
    it('should build string table with all strings', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'new_dashboard',
            type: 'boolean',
            defaultValue: 'OFF',
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          new_dashboard: {
            default: 'OFF',
            rules: [
              {
                when: "user.role == 'admin'",
                serve: 'ON',
              },
            ],
          },
        },
      };

      const artifact = compile(deployment, definitions);

      expect(artifact.strs.length).toBeGreaterThan(0);
      expect(artifact.strs).toContain('ON');
      expect(artifact.strs).toContain('user.role');
      expect(artifact.strs).toContain('admin');
    });

    it('should deduplicate strings in string table', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'flag1',
            type: 'boolean',
            defaultValue: 'OFF',
          },
          {
            name: 'flag2',
            type: 'boolean',
            defaultValue: 'OFF',
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          flag1: {
            default: 'OFF',
            rules: [{ serve: 'ON' }],
          },
          flag2: {
            default: 'OFF',
            rules: [{ serve: 'ON' }],
          },
        },
      };

      const artifact = compile(deployment, definitions);

      // 'ON' should only appear once in string table
      const onIndices = artifact.strs
        .map((s, i) => (s === 'ON' ? i : -1))
        .filter((i) => i !== -1);
      expect(onIndices.length).toBe(1);
    });
  });

  describe('Error cases', () => {
    it('should throw if flag not found in definitions', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'existing_flag',
            type: 'boolean',
            defaultValue: false,
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          missing_flag: {
            default: false,
          },
        },
      };

      expect(() => compile(deployment, definitions)).toThrow(
        'Flag "missing_flag" not found in flag definitions'
      );
    });

    it('should throw if variations rule used on non-multivariate flag', () => {
      const definitions: FlagDefinitions = {
        flags: [
          {
            name: 'boolean_flag',
            type: 'boolean',
            defaultValue: false,
          },
        ],
      };

      const deployment: Deployment = {
        environment: 'production',
        rules: {
          boolean_flag: {
            default: false,
            rules: [
              {
                variations: [{ variation: 'ON', weight: 100 }],
              },
            ],
          },
        },
      };

      expect(() => compile(deployment, definitions)).toThrow(
        'does not have variations defined'
      );
    });
  });
});

describe('serialize', () => {
  it('should serialize artifact to MessagePack', () => {
    const artifact: Artifact = {
      v: '1.0',
      env: 'production',
      strs: ['ON', 'OFF'],
      flags: [[[RuleType.SERVE, undefined, 0]]],
    };

    const bytes = serialize(artifact);

    expect(bytes).toBeInstanceOf(Uint8Array);
    expect(bytes.length).toBeGreaterThan(0);
  });

  it('should produce deserializable MessagePack', () => {
    const artifact: Artifact = {
      v: '1.0',
      env: 'production',
      strs: ['ON', 'OFF', 'user.role', 'admin'],
      flags: [
        [
          [
            RuleType.SERVE,
            [ExpressionType.BINARY_OP, BinaryOp.EQ, [ExpressionType.PROPERTY, 2], [ExpressionType.LITERAL, 3]],
            0,
          ],
        ],
      ],
    };

    const bytes = serialize(artifact);
    const deserialized = unpack(bytes);

    expect(deserialized.v).toBe('1.0');
    expect(deserialized.env).toBe('production');
    expect(deserialized.strs).toEqual(['ON', 'OFF', 'user.role', 'admin']);
    expect(deserialized.flags).toHaveLength(1);
  });
});

describe('compileAndSerialize', () => {
  it('should compile and serialize in one step', () => {
    const definitions: FlagDefinitions = {
      flags: [
        {
          name: 'new_dashboard',
          type: 'boolean',
          defaultValue: 'OFF',
        },
      ],
    };

    const deployment: Deployment = {
      environment: 'production',
      rules: {
        new_dashboard: {
          default: 'OFF',
          rules: [
            {
              serve: 'ON',
            },
          ],
        },
      },
    };

    const bytes = compileAndSerialize(deployment, definitions);

    expect(bytes).toBeInstanceOf(Uint8Array);
    expect(bytes.length).toBeGreaterThan(0);

    // Verify it can be deserialized
    const deserialized = unpack(bytes);
    expect(deserialized.v).toBe('1.0');
    expect(deserialized.env).toBe('production');
  });
});
