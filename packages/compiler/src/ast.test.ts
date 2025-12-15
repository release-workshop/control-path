/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import {
  Artifact,
  Rule,
  RuleType,
  Variation,
  Expression,
  ExpressionType,
  BinaryOp,
  LogicalOp,
  FuncCode,
  isArtifact,
  isRule,
  isVariation,
  isExpression,
} from './ast';

describe('AST Data Structures', () => {
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

      it('should return false for invalid artifact', () => {
        expect(isArtifact(null)).toBe(false);
        expect(isArtifact(undefined)).toBe(false);
        expect(isArtifact('string')).toBe(false);
        expect(isArtifact([])).toBe(false);
        expect(isArtifact({ v: '1.0' })).toBe(false); // missing required fields
      });
    });

    describe('isRule', () => {
      it('should return true for valid serve rule', () => {
        const rule: Rule = [RuleType.SERVE, undefined, 0];
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

      it('should return true for valid rollout rule', () => {
        const rule: Rule = [RuleType.ROLLOUT, undefined, [0, 10]];
        expect(isRule(rule)).toBe(true);
      });

      it('should return false for invalid rule', () => {
        expect(isRule(null)).toBe(false);
        expect(isRule([])).toBe(false);
        expect(isRule([0])).toBe(false); // too short
        expect(isRule([99, undefined, 0])).toBe(false); // invalid type
      });
    });

    describe('isVariation', () => {
      it('should return true for valid variation', () => {
        const variation: Variation = [0, 50];
        expect(isVariation(variation)).toBe(true);
      });

      it('should return false for invalid variation', () => {
        expect(isVariation(null)).toBe(false);
        expect(isVariation([])).toBe(false);
        expect(isVariation([0])).toBe(false); // too short
        expect(isVariation(['string', 50])).toBe(false); // wrong type
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

      it('should return false for invalid expression', () => {
        expect(isExpression(null)).toBe(false);
        expect(isExpression([])).toBe(false);
        expect(isExpression([99, 0])).toBe(false); // invalid type
        expect(isExpression([ExpressionType.PROPERTY])).toBe(false); // missing prop_index
      });
    });
  });
});
