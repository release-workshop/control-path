/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { Expression } from '../ast';
import { IntermediateExpression } from './expressions';

/**
 * String table builder for AST compilation.
 * Collects all strings used in the artifact and provides index-based access.
 */

export class StringTable {
  private strings: string[] = [];
  private indexMap = new Map<string, number>();

  /**
   * Add a string to the table and return its index.
   * If the string already exists, returns the existing index.
   */
  add(str: string): number {
    const existing = this.indexMap.get(str);
    if (existing !== undefined) {
      return existing;
    }

    const index = this.strings.length;
    this.strings.push(str);
    this.indexMap.set(str, index);
    return index;
  }

  /**
   * Get the string at the given index.
   */
  get(index: number): string {
    return this.strings[index];
  }

  /**
   * Get all strings as an array (for the artifact).
   */
  toArray(): string[] {
    return [...this.strings];
  }

  /**
   * Get the current size of the string table.
   */
  size(): number {
    return this.strings.length;
  }

  /**
   * Extract all strings from an expression and add them to the table.
   * Returns a new expression with string references replaced by indices.
   */
  processExpression(expr: IntermediateExpression | Expression): Expression {
    const [type, ...operands] = expr;

    switch (type) {
      case 0: {
        // BINARY_OP: [0, op_code, left, right]
        const opCode = operands[0] as number;
        return [
          type,
          opCode,
          this.processExpression(operands[1] as Expression),
          this.processExpression(operands[2] as Expression),
        ];
      }

      case 1: {
        // LOGICAL_OP: [1, op_code, left, right?]
        const opCode = operands[0] as number;
        if (operands.length === 2) {
          // NOT - only left operand
          return [type, opCode, this.processExpression(operands[1] as Expression)];
        }
        // AND/OR - both operands
        return [
          type,
          opCode,
          this.processExpression(operands[1] as Expression),
          this.processExpression(operands[2] as Expression),
        ];
      }

      case 2: {
        // PROPERTY: [2, prop_path]
        // Property path may be a string (from parser) or number (already processed)
        const propPath = operands[0];
        if (typeof propPath === 'string') {
          const propIndex = this.add(propPath);
          return [type, propIndex];
        }
        // Already an index
        return [type, propPath as number];
      }

      case 3: {
        // LITERAL: [3, value]
        const value = operands[0];
        // If value is a string, add to table and replace with index
        if (typeof value === 'string') {
          const strIndex = this.add(value);
          return [type, strIndex];
        }
        // Numbers and booleans stay as-is
        return [type, value];
      }

      case 4: {
        // FUNC: [4, func_code, args[]]
        const funcCode = operands[0] as number;
        return [
          type,
          funcCode,
          (operands[1] as (IntermediateExpression | Expression)[]).map((arg) =>
            this.processExpression(arg)
          ),
        ];
      }

      default:
        return expr;
    }
  }
}
