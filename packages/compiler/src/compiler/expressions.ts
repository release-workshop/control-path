/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { ExpressionType, BinaryOp, LogicalOp } from '../ast';

/**
 * Intermediate expression type used during parsing.
 * Properties and string literals use strings initially, which are then
 * converted to string table indices by the string table processor.
 */
type IntermediateExpression =
  | [0, number, IntermediateExpression, IntermediateExpression] // binary_op
  | [1, number, IntermediateExpression, IntermediateExpression?] // logical_op
  | [2, string] // property (string path, will be converted to index)
  | [3, unknown] // literal (may be string, will be converted to index if string)
  | [4, number, IntermediateExpression[]]; // func

/**
 * Basic expression parser for Phase 1.
 * Parses simple expressions like:
 * - "true", "false"
 * - "user.role == 'admin'"
 * - "user.role != 'admin' AND user.subscription_tier == 'premium'"
 * - "NOT user.role == 'guest'"
 *
 * This is a basic implementation for Phase 1. Full expression parsing
 * will be implemented in Phase 4.
 */

interface Token {
  type: 'IDENTIFIER' | 'STRING' | 'NUMBER' | 'BOOLEAN' | 'OPERATOR' | 'LPAREN' | 'RPAREN' | 'EOF';
  value: string | number | boolean;
  position: number;
}

class ExpressionParser {
  private tokens: Token[] = [];
  private current = 0;

  /**
   * Parse an expression string into an intermediate Expression.
   * The result will be processed by StringTable to convert strings to indices.
   */
  parse(expr: string): IntermediateExpression {
    this.tokens = this.tokenize(expr);
    this.current = 0;
    const result = this.parseLogicalOr();
    if (this.peek().type !== 'EOF') {
      throw new Error(`Unexpected token at position ${this.peek().position}: ${this.peek().value}`);
    }
    return result;
  }

  private tokenize(expr: string): Token[] {
    const tokens: Token[] = [];
    let i = 0;

    while (i < expr.length) {
      const char = expr[i];

      // Skip whitespace
      if (/\s/.test(char)) {
        i++;
        continue;
      }

      // String literals
      if (char === "'" || char === '"') {
        const quote = char;
        i++; // Skip opening quote
        let value = '';
        while (i < expr.length && expr[i] !== quote) {
          if (expr[i] === '\\' && i + 1 < expr.length) {
            i++;
            value += expr[i];
          } else {
            value += expr[i];
          }
          i++;
        }
        if (i >= expr.length) {
          throw new Error(`Unterminated string literal at position ${i}`);
        }
        tokens.push({ type: 'STRING', value, position: i - value.length - 1 });
        i++; // Skip closing quote
        continue;
      }

      // Numbers
      if (/\d/.test(char)) {
        let value = '';
        while (i < expr.length && (/\d/.test(expr[i]) || expr[i] === '.')) {
          value += expr[i];
          i++;
        }
        tokens.push({
          type: 'NUMBER',
          value: value.includes('.') ? parseFloat(value) : parseInt(value, 10),
          position: i - value.length,
        });
        continue;
      }

      // Operators
      if (char === '=' && i + 1 < expr.length && expr[i + 1] === '=') {
        tokens.push({ type: 'OPERATOR', value: '==', position: i });
        i += 2;
        continue;
      }
      if (char === '!' && i + 1 < expr.length && expr[i + 1] === '=') {
        tokens.push({ type: 'OPERATOR', value: '!=', position: i });
        i += 2;
        continue;
      }
      if (char === '>' && i + 1 < expr.length && expr[i + 1] === '=') {
        tokens.push({ type: 'OPERATOR', value: '>=', position: i });
        i += 2;
        continue;
      }
      if (char === '<' && i + 1 < expr.length && expr[i + 1] === '=') {
        tokens.push({ type: 'OPERATOR', value: '<=', position: i });
        i += 2;
        continue;
      }
      if (char === '>') {
        tokens.push({ type: 'OPERATOR', value: '>', position: i });
        i++;
        continue;
      }
      if (char === '<') {
        tokens.push({ type: 'OPERATOR', value: '<', position: i });
        i++;
        continue;
      }

      // Parentheses
      if (char === '(') {
        tokens.push({ type: 'LPAREN', value: '(', position: i });
        i++;
        continue;
      }
      if (char === ')') {
        tokens.push({ type: 'RPAREN', value: ')', position: i });
        i++;
        continue;
      }

      // Identifiers and keywords
      if (/[a-zA-Z_]/.test(char)) {
        let value = '';
        while (i < expr.length && /[a-zA-Z0-9_.]/.test(expr[i])) {
          value += expr[i];
          i++;
        }

        // Check for boolean literals
        if (value === 'true') {
          tokens.push({ type: 'BOOLEAN', value: true, position: i - value.length });
        } else if (value === 'false') {
          tokens.push({ type: 'BOOLEAN', value: false, position: i - value.length });
        } else {
          tokens.push({ type: 'IDENTIFIER', value, position: i - value.length });
        }
        continue;
      }

      throw new Error(`Unexpected character at position ${i}: ${char}`);
    }

    tokens.push({ type: 'EOF', value: '', position: i });
    return tokens;
  }

  private peek(): Token {
    return this.tokens[this.current] ?? this.tokens[this.tokens.length - 1];
  }

  private advance(): Token {
    if (this.current < this.tokens.length) {
      return this.tokens[this.current++];
    }
    return this.tokens[this.tokens.length - 1];
  }

  private parseLogicalOr(): IntermediateExpression {
    let left = this.parseLogicalAnd();

    while (this.peek().type === 'IDENTIFIER' && this.peek().value === 'OR') {
      this.advance(); // consume OR
      const right = this.parseLogicalAnd();
      left = [ExpressionType.LOGICAL_OP, LogicalOp.OR, left, right];
    }

    return left;
  }

  private parseLogicalAnd(): IntermediateExpression {
    let left = this.parseLogicalNot();

    while (this.peek().type === 'IDENTIFIER' && this.peek().value === 'AND') {
      this.advance(); // consume AND
      const right = this.parseLogicalNot();
      left = [ExpressionType.LOGICAL_OP, LogicalOp.AND, left, right];
    }

    return left;
  }

  private parseLogicalNot(): IntermediateExpression {
    if (this.peek().type === 'IDENTIFIER' && this.peek().value === 'NOT') {
      this.advance(); // consume NOT
      const operand = this.parseLogicalNot();
      return [ExpressionType.LOGICAL_OP, LogicalOp.NOT, operand];
    }

    return this.parseComparison();
  }

  private parseComparison(): IntermediateExpression {
    const left = this.parsePrimary();

    if (this.peek().type === 'OPERATOR') {
      const op = this.peek().value as string;
      let opCode: number;

      switch (op) {
        case '==':
          opCode = BinaryOp.EQ;
          break;
        case '!=':
          opCode = BinaryOp.NE;
          break;
        case '>':
          opCode = BinaryOp.GT;
          break;
        case '<':
          opCode = BinaryOp.LT;
          break;
        case '>=':
          opCode = BinaryOp.GTE;
          break;
        case '<=':
          opCode = BinaryOp.LTE;
          break;
        default:
          return left; // Not a comparison operator, return left as-is
      }

      this.advance(); // consume operator
      const right = this.parsePrimary();
      return [ExpressionType.BINARY_OP, opCode, left, right];
    }

    return left;
  }

  private parsePrimary(): IntermediateExpression {
    const token = this.advance();

    switch (token.type) {
      case 'BOOLEAN':
        return [ExpressionType.LITERAL, token.value as boolean];

      case 'STRING':
        return [ExpressionType.LITERAL, token.value as string];

      case 'NUMBER':
        return [ExpressionType.LITERAL, token.value as number];

      case 'IDENTIFIER':
        // Property access (e.g., user.role, device.type)
        // Store the full path as a string - will be converted to index by string table
        // In Phase 4, we'll properly parse nested properties
        return [ExpressionType.PROPERTY, token.value as string];

      case 'LPAREN': {
        const expr = this.parseLogicalOr();
        if (this.peek().type !== 'RPAREN') {
          throw new Error(`Expected ')' at position ${this.peek().position}`);
        }
        this.advance(); // consume ')'
        return expr;
      }

      default:
        throw new Error(`Unexpected token at position ${token.position}: ${token.value}`);
    }
  }
}

/**
 * Parse an expression string into an intermediate Expression.
 * This is a basic implementation for Phase 1.
 * The result should be processed by StringTable.processExpression() to convert
 * strings to string table indices.
 *
 * @param expr - Expression string (e.g., "user.role == 'admin'")
 * @returns Intermediate Expression (with strings, not indices)
 * @throws Error if expression is invalid
 */
export function parseExpression(expr: string): IntermediateExpression {
  const parser = new ExpressionParser();
  return parser.parse(expr.trim());
}

export type { IntermediateExpression };
