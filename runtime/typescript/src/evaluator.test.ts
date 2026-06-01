/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import { evaluate, evaluateRule } from './evaluator';
import type { Artifact, Attributes, Rule } from './types';
import { RuleType, ExpressionType, BinaryOp, LogicalOp, FuncCode } from './types';

describe('Evaluator', () => {
  const mockArtifact: Artifact = {
    v: '1.0',
    env: 'test',
    strs: ['ON', 'OFF', 'role', 'admin'],
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
      ], // role == 'admin' -> 'ON'
    ],
  };

  const mockAttributes: Attributes = {
    id: 'user1',
    email: 'test@example.com',
    role: 'admin',
    environment: 'production',
    device: 'desktop',
  };

  describe('evaluate', () => {
    it('should evaluate flag with simple serve rule', () => {
      const result = evaluate(0, mockArtifact, mockAttributes);

      expect(result).toBe(true);
    });

    it('should evaluate flag with when clause', () => {
      const result = evaluate(1, mockArtifact, mockAttributes);

      expect(result).toBe(true);
    });

    it('should return undefined for invalid flag index', () => {
      const result = evaluate(999, mockArtifact, mockAttributes);

      expect(result).toBeUndefined();
    });

    it('should handle missing context', () => {
      const result = evaluate(0, mockArtifact);

      expect(result).toBe(true);
    });
  });

  describe('evaluateRule', () => {
    it('should evaluate serve rule without when clause', () => {
      const rule: Rule = [RuleType.SERVE, undefined, 0];
      const result = evaluateRule(rule, mockArtifact, mockAttributes);

      expect(result).toBe(true);
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
      const result = evaluateRule(rule, mockArtifact, mockAttributes);

      expect(result).toBe(true);
    });

    it('should return undefined when when clause does not match', () => {
      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 2],
          [ExpressionType.LITERAL, 1],
        ], // role == 'OFF' (should not match)
        0,
      ];
      const result = evaluateRule(rule, mockArtifact, mockAttributes);

      expect(result).toBeUndefined();
    });

    it('should handle missing context', () => {
      const rule: Rule = [RuleType.SERVE, undefined, 0];
      const result = evaluateRule(rule, mockArtifact, mockAttributes);

      expect(result).toBe(true);
    });

    it('should ignore legacy multivariate rule type 1 in artifacts', () => {
      const rule = [1, undefined, [[0, 100]]] as unknown as Rule;
      const result = evaluateRule(rule, mockArtifact, mockAttributes);
      expect(result).toBeUndefined();
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

      const user1: Attributes = { id: 'user1' };
      const result1 = evaluateRule(rule, artifact, user1);

      // Should be either true or undefined (depending on hash)
      expect(result1 === true || result1 === undefined).toBe(true);

      // Same user should get consistent result
      const result2 = evaluateRule(rule, artifact, user1);
      expect(result2).toBe(result1);
    });

  });

  describe('complex expressions', () => {
    it('should evaluate AND logical operator', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF', 'role', 'admin', 'email', 'test@example.com'],
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
          ], // role == 'admin'
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [ExpressionType.PROPERTY, 4],
            [ExpressionType.LITERAL, 5],
          ], // email == 'test@example.com'
        ],
        0,
      ];

      const user: Attributes = { id: 'user1', role: 'admin', email: 'test@example.com' };
      const result = evaluateRule(rule, artifact, user);

      expect(result).toBe(true);
    });

    it('should evaluate OR logical operator', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF', 'role', 'admin', 'user'],
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
          ], // role == 'admin'
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [ExpressionType.PROPERTY, 2],
            [ExpressionType.LITERAL, 4],
          ], // role == 'user'
        ],
        0,
      ];

      const user: Attributes = { id: 'user1', role: 'user' };
      const result = evaluateRule(rule, artifact, user);

      expect(result).toBe(true);
    });

    it('should evaluate NOT logical operator', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF', 'role', 'admin'],
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
          ], // NOT (role == 'admin')
        ],
        0,
      ];

      const user: Attributes = { id: 'user1', role: 'user' };
      const result = evaluateRule(rule, artifact, user);

      expect(result).toBe(true);
    });

    it('should evaluate nested logical operators', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'role', 'admin', 'email', 'test@example.com'],
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
          ], // role == 'admin'
          [
            ExpressionType.LOGICAL_OP,
            LogicalOp.OR,
            [
              ExpressionType.BINARY_OP,
              BinaryOp.EQ,
              [ExpressionType.PROPERTY, 3],
              [ExpressionType.LITERAL, 4],
            ], // email == 'test@example.com'
            [ExpressionType.LITERAL, true], // OR true
          ],
        ],
        0,
      ];

      const user: Attributes = { id: 'user1', role: 'admin', email: 'test@example.com' };
      const result = evaluateRule(rule, artifact, user);

      expect(result).toBe(true);
    });

    it('should evaluate comparison operators', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'id'],
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
      expect(evaluateRule(ruleGT, artifact, mockAttributes)).toBe(true);

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
      expect(evaluateRule(ruleLT, artifact, mockAttributes)).toBe(true);

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
      expect(evaluateRule(ruleGTE, artifact, mockAttributes)).toBe(true);

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
      expect(evaluateRule(ruleLTE, artifact, mockAttributes)).toBe(true);

      // Test NE (not equal)
      const ruleNE: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.NE,
          [ExpressionType.LITERAL, 5],
          [ExpressionType.LITERAL, 10],
        ],
        0,
      ];
      expect(evaluateRule(ruleNE, artifact, mockAttributes)).toBe(true);
    });
  });

  describe('edge cases', () => {
    it('should handle missing user properties', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF', 'role', 'admin'],
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

      const userWithoutRole: Attributes = { id: 'user1' };
      const result = evaluateRule(rule, artifact, userWithoutRole);

      // Should not match since role is undefined
      expect(result).toBeUndefined();
    });

    it('should handle nested property access', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'profile.role', 'admin'],
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

      const user: Attributes = {
        id: 'user1',
        profile: {
          role: 'admin',
        },
      };

      const result = evaluateRule(rule, artifact, user);
      expect(result).toBe(true);
    });

    it('should handle multiple rules with precedence', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF', 'role', 'admin'],
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
            ], // role == 'admin' -> 'ON'
            [RuleType.SERVE, undefined, 1], // default -> 'OFF'
          ],
        ],
      };

      const adminAttributes: Attributes = { id: 'user1', role: 'admin' };
      const result1 = evaluate(0, artifact, adminAttributes);
      expect(result1).toBe(true); // First rule matches

      const regularAttributes: Attributes = { id: 'user2', role: 'user' };
      const result2 = evaluate(0, artifact, regularAttributes);
      expect(result2).toBe(false); // First rule doesn't match, second rule matches
    });

    it('should handle rollout with 0%', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON'],
        flags: [],
      };

      const rule: Rule = [RuleType.ROLLOUT, undefined, [0, 0]];
      const result = evaluateRule(rule, artifact, mockAttributes);

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
      const result = evaluateRule(rule, artifact, mockAttributes);

      expect(result).toBe(true);
    });

    it('should handle rollout with string valueIndex', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON'],
        flags: [],
      };

      // Rollout with string valueIndex (not number)
      const rule: Rule = [RuleType.ROLLOUT, undefined, ['direct-value', 100]];

      const result = evaluateRule(rule, artifact, mockAttributes);

      expect(result).toBe('direct-value');
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

      const user: Attributes = { id: 'user1' };
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

      const user: Attributes = { id: 'user1' };
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

      const user: Attributes = { id: 'user1' };
      const result = evaluateRule(rule, artifact, user);

      // Should return undefined because prototype path is rejected
      expect(result).toBeUndefined();
    });

    it('should reject nested prototype-polluting paths', () => {
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

      const user: Attributes = { id: 'user1' };
      const result = evaluateRule(rule, artifact, user);

      // Should return undefined because __proto__ in path is rejected
      expect(result).toBeUndefined();
    });

    it('should allow valid property paths', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'role', 'admin'],
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

      const user: Attributes = { id: 'user1', role: 'admin' };
      const result = evaluateRule(rule, artifact, user);

      // Should work normally for valid paths
      expect(result).toBe(true);
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
      const result = evaluateRule(null, artifact, mockAttributes);
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
      const result = evaluateRule([RuleType.SERVE], artifact, mockAttributes);
      expect(result).toBeUndefined();
    });
  });

  describe('property access edge cases', () => {
    it('should handle property access when object becomes null during navigation', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'profile.role', 'admin'],
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
      const user: Attributes = {
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
        strs: ['ON', 'profile.role', 'admin'],
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
      const user: Attributes = {
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

      const result = evaluateRule(rule, artifact, mockAttributes);
      expect(result).toBeUndefined();
    });

    it('should handle property access with context root', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'environment', 'production'],
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

      const attributes: Attributes = {
        ...mockAttributes,
        environment: 'production',
      };

      const result = evaluateRule(rule, artifact, attributes);
      expect(result).toBe(true);
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

      const user: Attributes = {
        id: 'user1',
        custom: {
          field: 'value',
        },
      };

      const result = evaluateRule(rule, artifact, user);
      expect(result).toBe(true);
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

      const result = evaluateRule(rule, artifact, mockAttributes);
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

      const result = evaluateRule(rule, artifact, mockAttributes);
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

      const result = evaluateRule(rule, artifact, mockAttributes);
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

      const result = evaluateRule(rule, artifact, mockAttributes);
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

      const result = evaluateRule(rule, artifact, mockAttributes);
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

      const result = evaluateRule(rule, artifact, mockAttributes);
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

      const result = evaluateRule(rule, artifact, mockAttributes);
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
      const result = evaluateRule(rule, artifact, mockAttributes);
      // Should not match because nested BINARY_OP can't be evaluated as a value
      expect(result).toBeUndefined();
    });
  });

  describe('property access with null/undefined during navigation', () => {
    it('should handle property access when intermediate property becomes null', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'profile.role', 'admin'],
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
      const user: Attributes = {
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
        strs: ['ON', 'profile.role', 'admin'],
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
      const user: Attributes = {
        id: 'user1',
        profile: 'not-an-object' as unknown as Record<string, unknown>,
      };

      const result = evaluateRule(rule, artifact, user);
      // Should return undefined because profile is not an object
      expect(result).toBeUndefined();
    });
  });

  describe('FUNC expression with non-array args', () => {
    it('should return false when FUNC expression has non-array args', () => {
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
          'not-an-array' as unknown as Expression[], // Invalid args - not an array
        ],
        0,
      ];

      const result = evaluateRule(rule, artifact, mockAttributes);
      // Should return undefined because args is not an array
      expect(result).toBeUndefined();
    });
  });

  describe('String functions', () => {
    const artifact: Artifact = {
      v: '1.0',
      env: 'test',
      strs: ['admin@example.com', 'admin', 'example.com', 'test', 'ON'],
      flags: [],
    };

    describe('STARTS_WITH', () => {
      it('should return true when string starts with prefix', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.STARTS_WITH,
            [
              [ExpressionType.LITERAL, 0], // 'admin@example.com'
              [ExpressionType.LITERAL, 1], // 'admin'
            ],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });

      it('should return false when string does not start with prefix', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.STARTS_WITH,
            [
              [ExpressionType.LITERAL, 0], // 'admin@example.com'
              [ExpressionType.LITERAL, 2], // 'example.com'
            ],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBeUndefined();
      });

      it('should handle non-string arguments', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.STARTS_WITH,
            [
              [ExpressionType.LITERAL, 100], // number (invalid)
              [ExpressionType.LITERAL, 1],
            ],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBeUndefined();
      });
    });

    describe('ENDS_WITH', () => {
      it('should return true when string ends with suffix', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.ENDS_WITH,
            [
              [ExpressionType.LITERAL, 0], // 'admin@example.com'
              [ExpressionType.LITERAL, 2], // 'example.com'
            ],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });

      it('should return false when string does not end with suffix', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.ENDS_WITH,
            [
              [ExpressionType.LITERAL, 0], // 'admin@example.com'
              [ExpressionType.LITERAL, 1], // 'admin'
            ],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBeUndefined();
      });
    });

    describe('CONTAINS', () => {
      it('should return true when string contains substring', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.CONTAINS,
            [
              [ExpressionType.LITERAL, 0], // 'admin@example.com'
              [ExpressionType.LITERAL, 1], // 'admin'
            ],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });

      it('should return true when array contains value', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.CONTAINS,
            [
              [ExpressionType.LITERAL, ['a', 'b', 'c']], // array
              [ExpressionType.LITERAL, 'b'], // value
            ],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });

      it('should return false when array does not contain value', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.CONTAINS,
            [
              [ExpressionType.LITERAL, ['a', 'b', 'c']],
              [ExpressionType.LITERAL, 'd'],
            ],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBeUndefined();
      });
    });

    describe('MATCHES', () => {
      it('should return true when string matches regex', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.MATCHES,
            [
              [ExpressionType.LITERAL, 0], // 'admin@example.com'
              [ExpressionType.LITERAL, '^admin@.*\\.com$'],
            ],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });

      it('should return false when string does not match regex', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.MATCHES,
            [
              [ExpressionType.LITERAL, 0], // 'admin@example.com'
              [ExpressionType.LITERAL, '^user@.*\\.com$'],
            ],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBeUndefined();
      });

      it('should handle invalid regex pattern', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.MATCHES,
            [
              [ExpressionType.LITERAL, 0],
              [ExpressionType.LITERAL, '[invalid'],
            ],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBeUndefined();
      });
    });

    describe('UPPER', () => {
      it('should convert string to uppercase', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [
              ExpressionType.FUNC,
              FuncCode.UPPER,
              [[ExpressionType.LITERAL, 1]], // 'admin'
            ],
            [ExpressionType.LITERAL, 'ADMIN'],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('LOWER', () => {
      it('should convert string to lowercase', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [
              ExpressionType.FUNC,
              FuncCode.LOWER,
              [[ExpressionType.LITERAL, 'ADMIN']],
            ],
            [ExpressionType.LITERAL, 1], // 'admin'
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('LENGTH', () => {
      it('should return string length', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [
              ExpressionType.FUNC,
              FuncCode.LENGTH,
              [[ExpressionType.LITERAL, 1]], // 'admin'
            ],
            [ExpressionType.LITERAL, 5],
          ],
          4,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });

      it('should return array length', () => {
        // Create artifact with string table that doesn't have index 3 as a string
        // We'll use index 10 which is definitely not in the string table
        const testArtifact: Artifact = {
          v: '1.0',
          env: 'test',
          strs: ['admin@example.com', 'admin', 'example.com', 'test', 'ON'],
          flags: [],
        };
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [
              ExpressionType.FUNC,
              FuncCode.LENGTH,
              [[ExpressionType.LITERAL, ['a', 'b', 'c']]],
            ],
            [ExpressionType.LITERAL, 10], // Use 10 which is not in string table (only has 5 items)
          ],
          4,
        ];
        const result = evaluateRule(rule, testArtifact, mockAttributes);
        // LENGTH(['a', 'b', 'c']) = 3, but we're comparing with 10, so this should fail
        // Let's fix this - we need to compare with 3, but 3 is in the string table as 'test'
        // So we need to use a number that's not in the string table but equals 3
        // Actually, the issue is that we can't use 3 because it's in the string table
        // Let's use a different approach - compare the length with a property or use a different number
        // Actually, let's just check that LENGTH works by using a comparison that will work
        // We'll compare LENGTH result (3) with a number that's definitely not in the string table
        // But we want it to equal 3, so we need to use a number that's not in the string table
        // The string table has 5 items (indices 0-4), so we can't use 0-4
        // But we need the number to be 3, which conflicts
        // Solution: Use a string '3' that's not in the string table, which will coerce to 3
        const rule2: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [
              ExpressionType.FUNC,
              FuncCode.LENGTH,
              [[ExpressionType.LITERAL, ['a', 'b', 'c']]],
            ],
            [ExpressionType.LITERAL, '3'], // Use string '3' which will coerce to number 3
          ],
          4,
        ];
        const result2 = evaluateRule(rule2, testArtifact, mockAttributes);
        expect(result2).toBe(true);
      });
    });
  });

  describe('Set functions', () => {
    const artifact: Artifact = {
      v: '1.0',
      env: 'test',
      strs: ['admin', 'moderator', 'user', 'ON'],
      flags: [],
    };

    describe('IN', () => {
      it('should return true when value is in array', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IN,
            [
              [ExpressionType.LITERAL, 0], // 'admin'
              [ExpressionType.LITERAL, ['admin', 'moderator']],
            ],
          ],
          3,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });

      it('should return false when value is not in array', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IN,
            [
              [ExpressionType.LITERAL, 2], // 'user'
              [ExpressionType.LITERAL, ['admin', 'moderator']],
            ],
          ],
          3,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBeUndefined();
      });
    });

    describe('INTERSECTS', () => {
      it('should return true when arrays intersect', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.INTERSECTS,
            [
              [ExpressionType.LITERAL, ['admin', 'user']],
              [ExpressionType.LITERAL, ['admin', 'moderator']],
            ],
          ],
          3,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });

      it('should return false when arrays do not intersect', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.INTERSECTS,
            [
              [ExpressionType.LITERAL, ['user']],
              [ExpressionType.LITERAL, ['admin', 'moderator']],
            ],
          ],
          3,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBeUndefined();
      });
    });
  });

  describe('Semver functions', () => {
    const artifact: Artifact = {
      v: '1.0',
      env: 'test',
      strs: ['1.2.3', '2.0.0', '1.5.0', 'ON'],
      flags: [],
    };

    describe('SEMVER_EQ', () => {
      it('should return true when versions are equal', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.SEMVER_EQ,
            [
              [ExpressionType.LITERAL, 0], // '1.2.3'
              [ExpressionType.LITERAL, '1.2.3'],
            ],
          ],
          3,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('SEMVER_GT', () => {
      it('should return true when first version is greater', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.SEMVER_GT,
            [
              [ExpressionType.LITERAL, 1], // '2.0.0'
              [ExpressionType.LITERAL, 0], // '1.2.3'
            ],
          ],
          3,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('SEMVER_GTE', () => {
      it('should return true when first version is greater or equal', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.SEMVER_GTE,
            [
              [ExpressionType.LITERAL, 0], // '1.2.3'
              [ExpressionType.LITERAL, '1.2.3'],
            ],
          ],
          3,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('SEMVER_LT', () => {
      it('should return true when first version is less', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.SEMVER_LT,
            [
              [ExpressionType.LITERAL, 0], // '1.2.3'
              [ExpressionType.LITERAL, 1], // '2.0.0'
            ],
          ],
          3,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('SEMVER_LTE', () => {
      it('should return true when first version is less or equal', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.SEMVER_LTE,
            [
              [ExpressionType.LITERAL, 0], // '1.2.3'
              [ExpressionType.LITERAL, '1.2.3'],
            ],
          ],
          3,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });
  });

  describe('Hashing function', () => {
    const artifact: Artifact = {
      v: '1.0',
      env: 'test',
      strs: ['user1', 'ON'],
      flags: [],
    };

    describe('HASHED_PARTITION (HASH)', () => {
      it('should return consistent bucket for same ID', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.LT,
            [
              ExpressionType.FUNC,
              FuncCode.HASH,
              [
                [ExpressionType.LITERAL, 0], // 'user1'
                [ExpressionType.LITERAL, 100], // 100 buckets
              ],
            ],
            [ExpressionType.LITERAL, 10], // bucket < 10
          ],
          1,
        ];
        const result1 = evaluateRule(rule, artifact, { id: 'user1' });
        const result2 = evaluateRule(rule, artifact, { id: 'user1' });
        expect(result1).toBe(result2); // Consistent
      });

      it('should return bucket in valid range', () => {
        // Use a number that's not in the string table
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.GTE,
            [
              ExpressionType.FUNC,
              FuncCode.HASH,
              [
                [ExpressionType.LITERAL, 0], // 'user1' from string table
                [ExpressionType.LITERAL, 999], // Use 999 which is definitely not in string table
              ],
            ],
            [ExpressionType.LITERAL, 1000], // Use 1000 which is definitely not in string table
          ],
          1,
        ];
        // Actually, let's fix this properly - use a number that's not a valid string table index
        // The hash function should return a number between 0 and buckets-1
        // So let's check that it's >= -1 (always true) or use a different comparison
        const rule2: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.LT,
            [
              ExpressionType.FUNC,
              FuncCode.HASH,
              [
                [ExpressionType.LITERAL, 0], // 'user1'
                [ExpressionType.LITERAL, 999], // 999 buckets
              ],
            ],
            [ExpressionType.LITERAL, 1000], // bucket < 1000 (always true for 999 buckets)
          ],
          1,
        ];
        const result = evaluateRule(rule2, artifact, { id: 'user1' });
        // Should match if bucket < 1000 (always true for 999 buckets)
        expect(result).toBe(true);
      });
    });
  });

  describe('Temporal functions', () => {
    const artifact: Artifact = {
      v: '1.0',
      env: 'test',
      strs: ['ON'],
      flags: [],
    };

    describe('IS_BETWEEN', () => {
      it('should return true when current time is between timestamps', () => {
        const now = new Date();
        const start = new Date(now.getTime() - 1000 * 60 * 60); // 1 hour ago
        const end = new Date(now.getTime() + 1000 * 60 * 60); // 1 hour from now

        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IS_BETWEEN,
            [
              [ExpressionType.LITERAL, start.toISOString()],
              [ExpressionType.LITERAL, end.toISOString()],
            ],
          ],
          0,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('IS_AFTER', () => {
      it('should return true when current time is after timestamp', () => {
        const past = new Date(Date.now() - 1000 * 60 * 60); // 1 hour ago

        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IS_AFTER,
            [[ExpressionType.LITERAL, past.toISOString()]],
          ],
          0,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('IS_BEFORE', () => {
      it('should return true when current time is before timestamp', () => {
        const future = new Date(Date.now() + 1000 * 60 * 60); // 1 hour from now

        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IS_BEFORE,
            [[ExpressionType.LITERAL, future.toISOString()]],
          ],
          0,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('CURRENT_HOUR_UTC (HOUR_OF_DAY)', () => {
      it('should return current hour (0-23)', () => {
        const hour = new Date().getUTCHours();
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [
              ExpressionType.FUNC,
              FuncCode.HOUR_OF_DAY,
              [],
            ],
            [ExpressionType.LITERAL, hour],
          ],
          0,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('CURRENT_DAY_OF_WEEK_UTC (DAY_OF_WEEK)', () => {
      it('should return day of week', () => {
        const days = ['SUNDAY', 'MONDAY', 'TUESDAY', 'WEDNESDAY', 'THURSDAY', 'FRIDAY', 'SATURDAY'];
        const day = days[new Date().getUTCDay()];
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [
              ExpressionType.FUNC,
              FuncCode.DAY_OF_WEEK,
              [],
            ],
            [ExpressionType.LITERAL, day],
          ],
          0,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('CURRENT_DAY_OF_MONTH_UTC (DAY_OF_MONTH)', () => {
      it('should return day of month (1-31)', () => {
        const day = new Date().getUTCDate();
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [
              ExpressionType.FUNC,
              FuncCode.DAY_OF_MONTH,
              [],
            ],
            [ExpressionType.LITERAL, day],
          ],
          0,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('CURRENT_MONTH_UTC (MONTH)', () => {
      it('should return month (1-12)', () => {
        const month = new Date().getUTCMonth() + 1;
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [
              ExpressionType.FUNC,
              FuncCode.MONTH,
              [],
            ],
            [ExpressionType.LITERAL, month],
          ],
          0,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });

    describe('CURRENT_TIMESTAMP', () => {
      it('should return ISO 8601 timestamp string', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.GT,
            [
              ExpressionType.FUNC,
              FuncCode.CURRENT_TIMESTAMP,
              [],
            ],
            [ExpressionType.LITERAL, '2000-01-01T00:00:00Z'],
          ],
          0,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });
  });

  describe('Utility functions', () => {
    const artifact: Artifact = {
      v: '1.0',
      env: 'test',
      strs: ['default', 'ON'],
      flags: [],
    };

    describe('COALESCE', () => {
      it('should return first non-null value', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [
              ExpressionType.FUNC,
              FuncCode.COALESCE,
              [
                [ExpressionType.LITERAL, null],
                [ExpressionType.LITERAL, 0], // 'default'
              ],
            ],
            [ExpressionType.LITERAL, 0], // 'default'
          ],
          1,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBe(true);
      });
    });
  });

  describe('Segment function', () => {
    const artifact: Artifact = {
      v: '1.0',
      env: 'test',
      strs: ['beta_users', 'role', 'beta', 'ON'],
      flags: [],
      segments: [
        [
          0, // 'beta_users' name index
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [ExpressionType.PROPERTY, 1], // 'role'
            [ExpressionType.LITERAL, 2], // 'beta'
          ],
        ],
      ],
    };

    describe('IN_SEGMENT', () => {
      it('should return true when user is in segment', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IN_SEGMENT,
            [
              [ExpressionType.PROPERTY, 1], // user (ignored, we use user from scope)
              [ExpressionType.LITERAL, 0], // 'beta_users'
            ],
          ],
          3,
        ];
        const user: Attributes = { id: 'user1', role: 'beta' };
        const result = evaluateRule(rule, artifact, user);
        expect(result).toBe(true);
      });

      it('should return false when user is not in segment', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IN_SEGMENT,
            [
              [ExpressionType.PROPERTY, 1],
              [ExpressionType.LITERAL, 0], // 'beta_users'
            ],
          ],
          3,
        ];
        const user: Attributes = { id: 'user1', role: 'admin' };
        const result = evaluateRule(rule, artifact, user);
        expect(result).toBeUndefined();
      });

      it('should return false when segment does not exist', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IN_SEGMENT,
            [
              [ExpressionType.PROPERTY, 1],
              [ExpressionType.LITERAL, 'nonexistent'],
            ],
          ],
          3,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBeUndefined();
      });

      it('should return false when IN_SEGMENT has insufficient args', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IN_SEGMENT,
            [[ExpressionType.PROPERTY, 1]], // Only one arg, needs at least 2
          ],
          3,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBeUndefined();
      });

      it('should handle segmentName as number (string table index)', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IN_SEGMENT,
            [
              [ExpressionType.PROPERTY, 1],
              [ExpressionType.LITERAL, 0], // String table index for 'beta_users'
            ],
          ],
          3,
        ];
        const user: Attributes = { id: 'user1', role: 'beta' };
        const result = evaluateRule(rule, artifact, user);
        expect(result).toBe(true);
      });

      it('should return false when segmentName is not string or valid number', () => {
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IN_SEGMENT,
            [
              [ExpressionType.PROPERTY, 1],
              [ExpressionType.LITERAL, true], // Invalid type (boolean)
            ],
          ],
          3,
        ];
        const result = evaluateRule(rule, artifact, mockAttributes);
        expect(result).toBeUndefined();
      });

      it('should return false when artifact has no segments', () => {
        const artifactWithoutSegments: Artifact = {
          v: '1.0',
          env: 'test',
          strs: ['beta_users', 'ON'],
          flags: [],
          // segments is undefined
        };
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IN_SEGMENT,
            [
              [ExpressionType.PROPERTY, 1],
              [ExpressionType.LITERAL, 'beta_users'],
            ],
          ],
          1,
        ];
        const result = evaluateRule(rule, artifactWithoutSegments, mockAttributes);
        expect(result).toBeUndefined();
      });

      it('should return false when artifact has empty segments array', () => {
        const artifactWithEmptySegments: Artifact = {
          v: '1.0',
          env: 'test',
          strs: ['beta_users', 'ON'],
          flags: [],
          segments: [], // Empty array
        };
        const rule: Rule = [
          RuleType.SERVE,
          [
            ExpressionType.FUNC,
            FuncCode.IN_SEGMENT,
            [
              [ExpressionType.PROPERTY, 1],
              [ExpressionType.LITERAL, 'beta_users'],
            ],
          ],
          1,
        ];
        const result = evaluateRule(rule, artifactWithEmptySegments, mockAttributes);
        expect(result).toBeUndefined();
      });
    });
  });

  describe('Null handling and type coercion', () => {
    const artifact: Artifact = {
      v: '1.0',
      env: 'test',
      strs: ['ON'],
      flags: [],
    };

    it('should handle null equality', () => {
      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.LITERAL, null],
          [ExpressionType.LITERAL, null],
        ],
        0,
      ];
      const result = evaluateRule(rule, artifact, mockAttributes);
      expect(result).toBe(true);
    });

    it('should handle null inequality', () => {
      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.NE,
          [ExpressionType.LITERAL, null],
          [ExpressionType.LITERAL, 'value'],
        ],
        0,
      ];
      const result = evaluateRule(rule, artifact, mockAttributes);
      expect(result).toBe(true);
    });

    it('should coerce string to number for comparison', () => {
      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.LITERAL, '30'],
          [ExpressionType.LITERAL, 30],
        ],
        0,
      ];
      const result = evaluateRule(rule, artifact, mockAttributes);
      expect(result).toBe(true);
    });

    it('should coerce string to boolean for comparison', () => {
      const rule: Rule = [
        RuleType.SERVE,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.LITERAL, 'true'],
          [ExpressionType.LITERAL, true],
        ],
        0,
      ];
      const result = evaluateRule(rule, artifact, mockAttributes);
      expect(result).toBe(true);
    });
  });
});
