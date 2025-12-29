/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import type { Artifact, Rule, Expression, Variation } from './types';
import {
  RuleType,
  ExpressionType,
  BinaryOp,
  LogicalOp,
  FuncCode,
  isArtifact,
  isRule,
  isVariation,
  isExpression,
} from './types';

describe('Type Guards', () => {
  describe('isArtifact', () => {
    it('should return true for valid artifact', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'production',
        strs: [],
        flags: [],
      };

      expect(isArtifact(artifact)).toBe(true);
    });

    it('should return true for artifact with optional fields', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'production',
        strs: ['test'],
        flags: [],
        segments: [[0, [ExpressionType.PROPERTY, 1]]],
        sig: new Uint8Array([1, 2, 3]),
      };

      expect(isArtifact(artifact)).toBe(true);
    });

    it('should return false for invalid artifact', () => {
      expect(isArtifact(null)).toBe(false);
      expect(isArtifact(undefined)).toBe(false);
      expect(isArtifact('string')).toBe(false);
      expect(isArtifact([])).toBe(false);
      expect(isArtifact({ v: '1.0' })).toBe(false); // missing required fields
      expect(isArtifact({ v: '1.0', env: 'prod' })).toBe(false); // missing strs and flags
      expect(isArtifact({ v: '1.0', env: 'prod', strs: 'not-array' })).toBe(false); // invalid strs
    });
  });

  describe('isRule', () => {
    it('should return true for valid serve rule without when', () => {
      const rule: Rule = [RuleType.SERVE, undefined, 0];
      expect(isRule(rule)).toBe(true);
    });

    it('should return true for valid serve rule with string value', () => {
      const rule: Rule = [RuleType.SERVE, undefined, 'on'];
      expect(isRule(rule)).toBe(true);
    });

    it('should return true for valid serve rule with when clause', () => {
      const when: Expression = [ExpressionType.PROPERTY, 0];
      const rule: Rule = [RuleType.SERVE, when, 0];
      expect(isRule(rule)).toBe(true);
    });

    it('should return true for valid variations rule', () => {
      const rule: Rule = [
        RuleType.VARIATIONS,
        undefined,
        [
          [0, 50],
          [1, 50],
        ],
      ];
      expect(isRule(rule)).toBe(true);
    });

    it('should return true for valid variations rule with when clause', () => {
      const when: Expression = [ExpressionType.PROPERTY, 0];
      const rule: Rule = [
        RuleType.VARIATIONS,
        when,
        [
          [0, 50],
          [1, 50],
        ],
      ];
      expect(isRule(rule)).toBe(true);
    });

    it('should return true for valid rollout rule', () => {
      const rule: Rule = [RuleType.ROLLOUT, undefined, [0, 10]];
      expect(isRule(rule)).toBe(true);
    });

    it('should return true for valid rollout rule with string value', () => {
      const rule: Rule = [RuleType.ROLLOUT, undefined, ['on', 10]];
      expect(isRule(rule)).toBe(true);
    });

    it('should return false for invalid rule', () => {
      expect(isRule(null)).toBe(false);
      expect(isRule(undefined)).toBe(false);
      expect(isRule([])).toBe(false);
      expect(isRule([0])).toBe(false); // too short
      expect(isRule([99, undefined, 0])).toBe(false); // invalid type
      expect(isRule([RuleType.SERVE, undefined, {}])).toBe(false); // invalid payload type
      expect(isRule([RuleType.VARIATIONS, undefined, 'not-array'])).toBe(false); // invalid payload
      expect(isRule([RuleType.ROLLOUT, undefined, [0]])).toBe(false); // invalid rollout payload
    });

    it('should return false for rule with invalid when clause', () => {
      const rule = [RuleType.SERVE, 'not-expression', 0];
      expect(isRule(rule)).toBe(false);
    });
  });

  describe('isVariation', () => {
    it('should return true for valid variation', () => {
      const variation: Variation = [0, 50];
      expect(isVariation(variation)).toBe(true);
    });

    it('should return false for invalid variation', () => {
      expect(isVariation(null)).toBe(false);
      expect(isVariation(undefined)).toBe(false);
      expect(isVariation([])).toBe(false);
      expect(isVariation([0])).toBe(false); // too short
      expect(isVariation(['string', 50])).toBe(false); // wrong type
      expect(isVariation([0, 'string'])).toBe(false); // wrong type
      expect(isVariation([0, 50, 100])).toBe(false); // too long
    });
  });

  describe('isExpression', () => {
    it('should return true for property expression', () => {
      const expr: Expression = [ExpressionType.PROPERTY, 2];
      expect(isExpression(expr)).toBe(true);
    });

    it('should return true for literal expression', () => {
      const expr: Expression = [ExpressionType.LITERAL, true];
      expect(isExpression(expr)).toBe(true);
    });

    it('should return true for literal expression with string', () => {
      const expr: Expression = [ExpressionType.LITERAL, 'test'];
      expect(isExpression(expr)).toBe(true);
    });

    it('should return true for binary op expression', () => {
      const left: Expression = [ExpressionType.PROPERTY, 2];
      const right: Expression = [ExpressionType.LITERAL, 'admin'];
      const expr: Expression = [ExpressionType.BINARY_OP, BinaryOp.EQ, left, right];
      expect(isExpression(expr)).toBe(true);
    });

    it('should return true for logical op expression', () => {
      const left: Expression = [ExpressionType.PROPERTY, 2];
      const right: Expression = [ExpressionType.PROPERTY, 3];
      const expr: Expression = [ExpressionType.LOGICAL_OP, LogicalOp.AND, left, right];
      expect(isExpression(expr)).toBe(true);
    });

    it('should return true for NOT expression', () => {
      const left: Expression = [ExpressionType.PROPERTY, 2];
      const expr: Expression = [ExpressionType.LOGICAL_OP, LogicalOp.NOT, left];
      expect(isExpression(expr)).toBe(true);
    });

    it('should return true for function expression', () => {
      const arg: Expression = [ExpressionType.PROPERTY, 2];
      const expr: Expression = [ExpressionType.FUNC, FuncCode.STARTS_WITH, [arg]];
      expect(isExpression(expr)).toBe(true);
    });

    it('should return true for nested expressions', () => {
      const inner: Expression = [ExpressionType.PROPERTY, 1];
      const left: Expression = [
        ExpressionType.BINARY_OP,
        BinaryOp.EQ,
        inner,
        [ExpressionType.LITERAL, 'admin'],
      ];
      const right: Expression = [ExpressionType.PROPERTY, 2];
      const expr: Expression = [ExpressionType.LOGICAL_OP, LogicalOp.AND, left, right];
      expect(isExpression(expr)).toBe(true);
    });

    it('should return false for invalid expression', () => {
      expect(isExpression(null)).toBe(false);
      expect(isExpression(undefined)).toBe(false);
      expect(isExpression([])).toBe(false);
      expect(isExpression([99, 0])).toBe(false); // invalid type
      expect(isExpression([ExpressionType.PROPERTY])).toBe(false); // missing prop_index
      expect(isExpression([ExpressionType.BINARY_OP])).toBe(false); // missing operands
      expect(isExpression([ExpressionType.BINARY_OP, BinaryOp.EQ])).toBe(false); // missing operands
      expect(isExpression([ExpressionType.FUNC, FuncCode.STARTS_WITH, 'not-array'])).toBe(false); // invalid args
    });
  });
});
