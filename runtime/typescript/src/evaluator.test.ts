/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import { evaluate, evaluateRule } from './evaluator';
import type { Artifact, User, Context, Rule } from './types';
import { RuleType, ExpressionType, BinaryOp, LogicalOp } from './types';

describe('Evaluator', () => {
  const mockArtifact: Artifact = {
    v: '1.0',
    env: 'test',
    strs: ['ON', 'OFF', 'user.role', 'admin'],
    flags: [
      // Flag 0: simple serve rule
      [[RuleType.SERVE, undefined, 0]], // Returns 'ON'
      // Flag 1: serve rule with when clause
      [
        [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [ExpressionType.PROPERTY, 2],
            [ExpressionType.LITERAL, 3],
          ],
          0,
        ],
      ], // user.role == 'admin' -> 'ON'
    ],
  };

  const mockUser: User = {
    id: 'user1',
    email: 'test@example.com',
    role: 'admin',
  };

  const mockContext: Context = {
    environment: 'production',
    device: 'desktop',
  };

  describe('evaluate', () => {
    it('should evaluate flag with simple serve rule', () => {
      const result = evaluate(0, mockArtifact, mockUser, mockContext);

      expect(result).toBe('ON');
    });

    it('should evaluate flag with when clause', () => {
      const result = evaluate(1, mockArtifact, mockUser, mockContext);

      expect(result).toBe('ON');
    });

    it('should return undefined for invalid flag index', () => {
      const result = evaluate(999, mockArtifact, mockUser, mockContext);

      expect(result).toBeUndefined();
    });

    it('should handle missing context', () => {
      const result = evaluate(0, mockArtifact, mockUser);

      expect(result).toBe('ON');
    });
  });

  describe('evaluateRule', () => {
    it('should evaluate serve rule without when clause', () => {
      const rule: Rule = [RuleType.SERVE, undefined, 0];
      const result = evaluateRule(rule, mockArtifact, mockUser, mockContext);

      expect(result).toBe('ON');
    });

    it('should evaluate serve rule with matching when clause', () => {
      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 2],
          [ExpressionType.LITERAL, 3],
        ],
        0,
      ];
      const result = evaluateRule(rule, mockArtifact, mockUser, mockContext);

      expect(result).toBe('ON');
    });

    it('should return undefined when when clause does not match', () => {
      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 2],
          [ExpressionType.LITERAL, 1],
        ], // user.role == 'OFF' (should not match)
        0,
      ];
      const result = evaluateRule(rule, mockArtifact, mockUser, mockContext);

      expect(result).toBeUndefined();
    });

    it('should handle missing context', () => {
      const rule: Rule = [RuleType.SERVE, undefined, 0];
      const result = evaluateRule(rule, mockArtifact, mockUser);

      expect(result).toBe('ON');
    });

    it('should evaluate variations rule', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['variant_a', 'variant_b', 'variant_c'],
        flags: [],
      };
      const rule: Rule = [
        RuleType.VARIATIONS,
        undefined,
        [
          [0, 50], // variant_a: 50%
          [1, 30], // variant_b: 30%
          [2, 20], // variant_c: 20%
        ],
      ];

      // Test with consistent user ID for deterministic results
      const user1: User = { id: 'user1' };
      const result1 = evaluateRule(rule, artifact, user1);
      expect(['variant_a', 'variant_b', 'variant_c']).toContain(result1);

      // Same user should get same variation
      const result2 = evaluateRule(rule, artifact, user1);
      expect(result2).toBe(result1);
    });

    it('should evaluate rollout rule', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF'],
        flags: [],
      };

      // 50% rollout
      const rule: Rule = [RuleType.ROLLOUT, undefined, [0, 50]];

      const user1: User = { id: 'user1' };
      const result1 = evaluateRule(rule, artifact, user1);

      // Should be either 'ON' or undefined (depending on hash)
      expect(result1 === 'ON' || result1 === undefined).toBe(true);

      // Same user should get consistent result
      const result2 = evaluateRule(rule, artifact, user1);
      expect(result2).toBe(result1);
    });

    it('should return undefined for variations with invalid string indices', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['variant_a'],
        flags: [],
      };
      const rule: Rule = [
        RuleType.VARIATIONS,
        undefined,
        [
          [999, 100], // Invalid string index
        ],
      ];

      const result = evaluateRule(rule, artifact, mockUser);
      expect(result).toBeUndefined();
    });

    it('should handle variations with zero total percentage', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['variant_a'],
        flags: [],
      };
      const rule: Rule = [
        RuleType.VARIATIONS,
        undefined,
        [
          [0, 0], // 0% - should return first variation
        ],
      ];

      const result = evaluateRule(rule, artifact, mockUser);
      expect(result).toBe('variant_a');
    });
  });

  describe('complex expressions', () => {
    it('should evaluate AND logical operator', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF', 'user.role', 'admin', 'user.email', 'test@example.com'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.LOGICAL_OP,
          LogicalOp.AND,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [ExpressionType.PROPERTY, 2],
            [ExpressionType.LITERAL, 3],
          ], // user.role == 'admin'
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [ExpressionType.PROPERTY, 4],
            [ExpressionType.LITERAL, 5],
          ], // user.email == 'test@example.com'
        ],
        0,
      ];

      const user: User = { id: 'user1', role: 'admin', email: 'test@example.com' };
      const result = evaluateRule(rule, artifact, user);

      expect(result).toBe('ON');
    });

    it('should evaluate OR logical operator', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF', 'user.role', 'admin', 'user'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.LOGICAL_OP,
          LogicalOp.OR,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [ExpressionType.PROPERTY, 2],
            [ExpressionType.LITERAL, 3],
          ], // user.role == 'admin'
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [ExpressionType.PROPERTY, 2],
            [ExpressionType.LITERAL, 4],
          ], // user.role == 'user'
        ],
        0,
      ];

      const user: User = { id: 'user1', role: 'user' };
      const result = evaluateRule(rule, artifact, user);

      expect(result).toBe('ON');
    });

    it('should evaluate NOT logical operator', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF', 'user.role', 'admin'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.LOGICAL_OP,
          LogicalOp.NOT,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [ExpressionType.PROPERTY, 2],
            [ExpressionType.LITERAL, 3],
          ], // NOT (user.role == 'admin')
        ],
        0,
      ];

      const user: User = { id: 'user1', role: 'user' };
      const result = evaluateRule(rule, artifact, user);

      expect(result).toBe('ON');
    });

    it('should evaluate nested logical operators', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'user.role', 'admin', 'user.email', 'test@example.com'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.LOGICAL_OP,
          LogicalOp.AND,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [ExpressionType.PROPERTY, 1],
            [ExpressionType.LITERAL, 2],
          ], // user.role == 'admin'
          [
            ExpressionType.LOGICAL_OP,
            LogicalOp.OR,
            [
              ExpressionType.BINARY_OP,
              BinaryOp.EQ,
              [ExpressionType.PROPERTY, 3],
              [ExpressionType.LITERAL, 4],
            ], // user.email == 'test@example.com'
            [ExpressionType.LITERAL, true], // OR true
          ],
        ],
        0,
      ];

      const user: User = { id: 'user1', role: 'admin', email: 'test@example.com' };
      const result = evaluateRule(rule, artifact, user);

      expect(result).toBe('ON');
    });

    it('should evaluate comparison operators', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'user.id'],
        flags: [],
      };

      // Test GT (greater than)
      const ruleGT: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.GT,
          [ExpressionType.LITERAL, 10],
          [ExpressionType.LITERAL, 5],
        ],
        0,
      ];
      expect(evaluateRule(ruleGT, artifact, mockUser)).toBe('ON');

      // Test LT (less than)
      const ruleLT: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.LT,
          [ExpressionType.LITERAL, 5],
          [ExpressionType.LITERAL, 10],
        ],
        0,
      ];
      expect(evaluateRule(ruleLT, artifact, mockUser)).toBe('ON');

      // Test GTE (greater than or equal)
      const ruleGTE: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.GTE,
          [ExpressionType.LITERAL, 10],
          [ExpressionType.LITERAL, 10],
        ],
        0,
      ];
      expect(evaluateRule(ruleGTE, artifact, mockUser)).toBe('ON');

      // Test LTE (less than or equal)
      const ruleLTE: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.LTE,
          [ExpressionType.LITERAL, 5],
          [ExpressionType.LITERAL, 10],
        ],
        0,
      ];
      expect(evaluateRule(ruleLTE, artifact, mockUser)).toBe('ON');
    });
  });

  describe('edge cases', () => {
    it('should handle missing user properties', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF', 'user.role', 'admin'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 2],
          [ExpressionType.LITERAL, 3],
        ],
        0,
      ];

      const userWithoutRole: User = { id: 'user1' };
      const result = evaluateRule(rule, artifact, userWithoutRole);

      // Should not match since user.role is undefined
      expect(result).toBeUndefined();
    });

    it('should handle nested property access', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'user.profile.role', 'admin'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      const user: User = {
        id: 'user1',
        profile: {
          role: 'admin',
        },
      };

      const result = evaluateRule(rule, artifact, user);
      expect(result).toBe('ON');
    });

    it('should handle multiple rules with precedence', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF', 'user.role', 'admin'],
        flags: [
          [
            [
              RuleType.SERVE,
              [
                ExpressionType.BINARY_OP,
                BinaryOp.EQ,
                [ExpressionType.PROPERTY, 2],
                [ExpressionType.LITERAL, 3],
              ],
              0,
            ], // user.role == 'admin' -> 'ON'
            [RuleType.SERVE, undefined, 1], // default -> 'OFF'
          ],
        ],
      };

      const adminUser: User = { id: 'user1', role: 'admin' };
      const result1 = evaluate(0, artifact, adminUser);
      expect(result1).toBe('ON'); // First rule matches

      const regularUser: User = { id: 'user2', role: 'user' };
      const result2 = evaluate(0, artifact, regularUser);
      expect(result2).toBe('OFF'); // First rule doesn't match, second rule matches
    });

    it('should handle empty variations array', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
      };

      const rule: Rule = [RuleType.VARIATIONS, undefined, []];
      const result = evaluateRule(rule, artifact, mockUser);

      expect(result).toBeUndefined();
    });

    it('should handle rollout with 0%', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON'],
        flags: [],
      };

      const rule: Rule = [RuleType.ROLLOUT, undefined, [0, 0]];
      const result = evaluateRule(rule, artifact, mockUser);

      expect(result).toBeUndefined();
    });

    it('should handle rollout with 100%', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON'],
        flags: [],
      };

      const rule: Rule = [RuleType.ROLLOUT, undefined, [0, 100]];
      const result = evaluateRule(rule, artifact, mockUser);

      expect(result).toBe('ON');
    });
  });

  describe('prototype pollution protection', () => {
    it('should reject property paths containing __proto__', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', '__proto__.polluted', 'value'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      const user: User = { id: 'user1' };
      const result = evaluateRule(rule, artifact, user);

      // Should return undefined because __proto__ path is rejected
      expect(result).toBeUndefined();
    });

    it('should reject property paths containing constructor', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'constructor.polluted', 'value'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      const user: User = { id: 'user1' };
      const result = evaluateRule(rule, artifact, user);

      // Should return undefined because constructor path is rejected
      expect(result).toBeUndefined();
    });

    it('should reject property paths containing prototype', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'prototype.polluted', 'value'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      const user: User = { id: 'user1' };
      const result = evaluateRule(rule, artifact, user);

      // Should return undefined because prototype path is rejected
      expect(result).toBeUndefined();
    });

    it('should reject nested prototype-polluting paths', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'user.__proto__.polluted', 'value'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      const user: User = { id: 'user1' };
      const result = evaluateRule(rule, artifact, user);

      // Should return undefined because __proto__ in path is rejected
      expect(result).toBeUndefined();
    });

    it('should allow valid property paths', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'user.role', 'admin'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      const user: User = { id: 'user1', role: 'admin' };
      const result = evaluateRule(rule, artifact, user);

      // Should work normally for valid paths
      expect(result).toBe('ON');
    });
  });

  describe('invalid rule formats', () => {
    it('should return undefined for invalid rule (not array)', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON'],
        flags: [],
      };

      // @ts-expect-error - Testing invalid input
      const result = evaluateRule(null, artifact, mockUser);
      expect(result).toBeUndefined();
    });

    it('should return undefined for rule with insufficient length', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON'],
        flags: [],
      };

      // @ts-expect-error - Testing invalid input
      const result = evaluateRule([RuleType.SERVE], artifact, mockUser);
      expect(result).toBeUndefined();
    });
  });

  describe('property access edge cases', () => {
    it('should handle property access when object becomes null during navigation', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'user.profile.role', 'admin'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      // User with profile that becomes null
      const user: User = {
        id: 'user1',
        profile: null as unknown as Record<string, unknown>,
      };

      const result = evaluateRule(rule, artifact, user);
      expect(result).toBeUndefined();
    });

    it('should handle property access when intermediate property is undefined', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'user.profile.role', 'admin'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      // User without profile property
      const user: User = {
        id: 'user1',
      };

      const result = evaluateRule(rule, artifact, user);
      expect(result).toBeUndefined();
    });

    it('should handle property access when property path is empty', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', '', 'admin'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      const result = evaluateRule(rule, artifact, mockUser);
      expect(result).toBeUndefined();
    });

    it('should handle property access with context root', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'context.environment', 'production'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      const context: Context = {
        environment: 'production',
      };

      const result = evaluateRule(rule, artifact, mockUser, context);
      expect(result).toBe('ON');
    });

    it('should handle property access with non-user, non-context root', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'custom.field', 'value'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      const user: User = {
        id: 'user1',
        custom: {
          field: 'value',
        },
      };

      const result = evaluateRule(rule, artifact, user);
      expect(result).toBe('ON');
    });
  });

  describe('function evaluation', () => {
    it('should return false for unimplemented IN function', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.FUNC,
          0, // FuncCode.IN
          [
            [ExpressionType.LITERAL, 'value'],
            [ExpressionType.LITERAL, 'array'],
          ],
        ],
        0,
      ];

      const result = evaluateRule(rule, artifact, mockUser);
      // IN function is not implemented, so rule should not match
      expect(result).toBeUndefined();
    });

    it('should return false for function with insufficient args', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.FUNC,
          0, // FuncCode.IN
          [[ExpressionType.LITERAL, 'value']], // Only one arg, needs at least 2
        ],
        0,
      ];

      const result = evaluateRule(rule, artifact, mockUser);
      expect(result).toBeUndefined();
    });

    it('should return false for unknown function code', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.FUNC,
          999, // Unknown function code
          [[ExpressionType.LITERAL, 'value']],
        ],
        0,
      ];

      const result = evaluateRule(rule, artifact, mockUser);
      expect(result).toBeUndefined();
    });

    it('should return false for function with non-array args', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.FUNC,
          0, // FuncCode.IN
          'not-an-array' as unknown as Expression[], // Invalid args type
        ],
        0,
      ];

      const result = evaluateRule(rule, artifact, mockUser);
      expect(result).toBeUndefined();
    });

    it('should handle function with sufficient args (but still return false for IN)', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.FUNC,
          0, // FuncCode.IN
          [
            [ExpressionType.LITERAL, 'value'],
            [ExpressionType.LITERAL, 'array'],
          ], // Two args (sufficient)
        ],
        0,
      ];

      const result = evaluateRule(rule, artifact, mockUser);
      // IN function is not implemented, so should return undefined (rule doesn't match)
      expect(result).toBeUndefined();
    });

    it('should return false for function with non-array args in expression', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.FUNC,
          0, // FuncCode.IN
          'not-an-array' as unknown as Expression[], // Invalid args type
        ],
        0,
      ];

      const result = evaluateRule(rule, artifact, mockUser);
      // Should return undefined because args is not an array
      expect(result).toBeUndefined();
    });
  });

  describe('property access with empty path', () => {
    it('should handle empty property path', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', '', 'value'], // Empty string at index 1
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1], // Empty path
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      const result = evaluateRule(rule, artifact, mockUser);
      // Should return undefined because empty path splits to empty array
      expect(result).toBeUndefined();
    });
  });

  describe('evaluateExpressionValue edge cases', () => {
    it('should handle BINARY_OP in evaluateExpressionValue (default case)', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF'],
        flags: [],
      };

      // BINARY_OP in evaluateExpressionValue should return undefined (not supported)
      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.BINARY_OP, BinaryOp.EQ, [ExpressionType.LITERAL, 1], [ExpressionType.LITERAL, 2]], // Nested BINARY_OP
          [ExpressionType.LITERAL, 3],
        ],
        0,
      ];

      // This tests that BINARY_OP in evaluateExpressionValue returns undefined
      const result = evaluateRule(rule, artifact, mockUser);
      // Should not match because nested BINARY_OP can't be evaluated as a value
      expect(result).toBeUndefined();
    });
  });

  describe('property access with null/undefined during navigation', () => {
    it('should handle property access when intermediate property becomes null', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'user.profile.role', 'admin'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      // User with profile that becomes null during navigation
      const user: User = {
        id: 'user1',
        profile: {
          role: null as unknown as string, // role is null
        },
      };

      const result = evaluateRule(rule, artifact, user);
      // Should return undefined because role is null
      expect(result).toBeUndefined();
    });

    it('should handle property access when intermediate property becomes non-object', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'user.profile.role', 'admin'],
        flags: [],
      };

      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 1],
          [ExpressionType.LITERAL, 2],
        ],
        0,
      ];

      // User with profile that is a string (not an object)
      const user: User = {
        id: 'user1',
        profile: 'not-an-object' as unknown as Record<string, unknown>,
      };

      const result = evaluateRule(rule, artifact, user);
      // Should return undefined because profile is not an object
      expect(result).toBeUndefined();
    });
  });
});
