/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import { parseExpression } from './expressions';
import { ExpressionType, BinaryOp, LogicalOp } from '../ast';

describe('parseExpression', () => {
  describe('Boolean literals', () => {
    it('should parse true', () => {
      const expr = parseExpression('true');
      expect(expr).toEqual([ExpressionType.LITERAL, true]);
    });

    it('should parse false', () => {
      const expr = parseExpression('false');
      expect(expr).toEqual([ExpressionType.LITERAL, false]);
    });
  });

  describe('String literals', () => {
    it('should parse single-quoted strings', () => {
      const expr = parseExpression("'admin'");
      expect(expr).toEqual([ExpressionType.LITERAL, 'admin']);
    });

    it('should parse double-quoted strings', () => {
      const expr = parseExpression('"premium"');
      expect(expr).toEqual([ExpressionType.LITERAL, 'premium']);
    });

    it('should handle escaped quotes', () => {
      const expr = parseExpression("'it\\'s a test'");
      expect(expr).toEqual([ExpressionType.LITERAL, "it's a test"]);
    });
  });

  describe('Number literals', () => {
    it('should parse integers', () => {
      const expr = parseExpression('42');
      expect(expr).toEqual([ExpressionType.LITERAL, 42]);
    });

    it('should parse floats', () => {
      const expr = parseExpression('3.14');
      expect(expr).toEqual([ExpressionType.LITERAL, 3.14]);
    });
  });

  describe('Property access', () => {
    it('should parse simple property', () => {
      const expr = parseExpression('user.role');
      expect(expr).toEqual([ExpressionType.PROPERTY, 'user.role']);
    });

    it('should parse nested property', () => {
      const expr = parseExpression('user.subscription.tier');
      expect(expr).toEqual([ExpressionType.PROPERTY, 'user.subscription.tier']);
    });

    it('should parse device property', () => {
      const expr = parseExpression('device.type');
      expect(expr).toEqual([ExpressionType.PROPERTY, 'device.type']);
    });
  });

  describe('Binary operators', () => {
    it('should parse equality', () => {
      const expr = parseExpression("user.role == 'admin'");
      expect(expr).toEqual([
        ExpressionType.BINARY_OP,
        BinaryOp.EQ,
        [ExpressionType.PROPERTY, 'user.role'],
        [ExpressionType.LITERAL, 'admin'],
      ]);
    });

    it('should parse inequality', () => {
      const expr = parseExpression("user.role != 'guest'");
      expect(expr).toEqual([
        ExpressionType.BINARY_OP,
        BinaryOp.NE,
        [ExpressionType.PROPERTY, 'user.role'],
        [ExpressionType.LITERAL, 'guest'],
      ]);
    });

    it('should parse greater than', () => {
      const expr = parseExpression('user.age > 18');
      expect(expr).toEqual([
        ExpressionType.BINARY_OP,
        BinaryOp.GT,
        [ExpressionType.PROPERTY, 'user.age'],
        [ExpressionType.LITERAL, 18],
      ]);
    });

    it('should parse less than', () => {
      const expr = parseExpression('user.age < 65');
      expect(expr).toEqual([
        ExpressionType.BINARY_OP,
        BinaryOp.LT,
        [ExpressionType.PROPERTY, 'user.age'],
        [ExpressionType.LITERAL, 65],
      ]);
    });

    it('should parse greater than or equal', () => {
      const expr = parseExpression('user.age >= 21');
      expect(expr).toEqual([
        ExpressionType.BINARY_OP,
        BinaryOp.GTE,
        [ExpressionType.PROPERTY, 'user.age'],
        [ExpressionType.LITERAL, 21],
      ]);
    });

    it('should parse less than or equal', () => {
      const expr = parseExpression('user.age <= 100');
      expect(expr).toEqual([
        ExpressionType.BINARY_OP,
        BinaryOp.LTE,
        [ExpressionType.PROPERTY, 'user.age'],
        [ExpressionType.LITERAL, 100],
      ]);
    });
  });

  describe('Logical operators', () => {
    it('should parse AND', () => {
      const expr = parseExpression("user.role == 'admin' AND user.subscription_tier == 'premium'");
      expect(expr).toEqual([
        ExpressionType.LOGICAL_OP,
        LogicalOp.AND,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 'user.role'],
          [ExpressionType.LITERAL, 'admin'],
        ],
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 'user.subscription_tier'],
          [ExpressionType.LITERAL, 'premium'],
        ],
      ]);
    });

    it('should parse OR', () => {
      const expr = parseExpression("user.role == 'admin' OR user.role == 'moderator'");
      expect(expr).toEqual([
        ExpressionType.LOGICAL_OP,
        LogicalOp.OR,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 'user.role'],
          [ExpressionType.LITERAL, 'admin'],
        ],
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 'user.role'],
          [ExpressionType.LITERAL, 'moderator'],
        ],
      ]);
    });

    it('should parse NOT', () => {
      const expr = parseExpression("NOT user.role == 'guest'");
      expect(expr).toEqual([
        ExpressionType.LOGICAL_OP,
        LogicalOp.NOT,
        [
          ExpressionType.BINARY_OP,
          BinaryOp.EQ,
          [ExpressionType.PROPERTY, 'user.role'],
          [ExpressionType.LITERAL, 'guest'],
        ],
      ]);
    });

    it('should handle operator precedence (AND before OR)', () => {
      const expr = parseExpression("user.role == 'admin' OR user.role == 'moderator' AND user.active == true");
      // Should be: (admin) OR ((moderator) AND (active))
      expect(expr[0]).toBe(ExpressionType.LOGICAL_OP);
      expect(expr[1]).toBe(LogicalOp.OR);
    });

    it('should handle complex logical expressions', () => {
      const expr = parseExpression("user.role == 'admin' AND user.subscription_tier == 'premium' OR device.type == 'mobile'");
      expect(expr[0]).toBe(ExpressionType.LOGICAL_OP);
    });
  });

  describe('Parentheses', () => {
    it('should handle parentheses for grouping', () => {
      const expr = parseExpression("(user.role == 'admin' OR user.role == 'moderator') AND user.active == true");
      expect(expr[0]).toBe(ExpressionType.LOGICAL_OP);
      expect(expr[1]).toBe(LogicalOp.AND);
    });
  });

  describe('Whitespace handling', () => {
    it('should handle extra whitespace', () => {
      const expr = parseExpression("  user.role   ==   'admin'  ");
      expect(expr).toEqual([
        ExpressionType.BINARY_OP,
        BinaryOp.EQ,
        [ExpressionType.PROPERTY, 'user.role'],
        [ExpressionType.LITERAL, 'admin'],
      ]);
    });
  });

  describe('Error cases', () => {
    it('should throw on invalid expression', () => {
      expect(() => parseExpression('user.role ==')).toThrow();
    });

    it('should throw on unterminated string', () => {
      expect(() => parseExpression("user.role == 'admin")).toThrow();
    });

    it('should throw on unexpected character', () => {
      expect(() => parseExpression('user.role @ admin')).toThrow();
    });

    it('should throw on unmatched parentheses', () => {
      expect(() => parseExpression("(user.role == 'admin'")).toThrow();
    });
  });
});
