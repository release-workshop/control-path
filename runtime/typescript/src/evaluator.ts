/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Flag evaluator module for evaluating flags using AST rules.
 * This module handles rule matching and expression evaluation.
 */

import type { Artifact, Rule, Expression, User, Context, Variation } from './types';
import { RuleType, ExpressionType, BinaryOp, LogicalOp, FuncCode, isExpression } from './types';

/**
 * Evaluate a flag by index using the provided artifact, user, and context.
 * Returns the evaluated value or undefined if no rules match.
 * @param flagIndex - The index of the flag in the flags array
 * @param artifact - The AST artifact containing flag definitions
 * @param user - User object with identity and attributes
 * @param context - Optional context object with environmental data
 * @returns The evaluated value or undefined if no rules match
 */
export function evaluate(
  flagIndex: number,
  artifact: Artifact,
  user: User,
  context?: Context
): unknown {
  if (!artifact.flags || flagIndex < 0 || flagIndex >= artifact.flags.length) {
    return undefined;
  }

  const flagRules = artifact.flags[flagIndex];
  if (!flagRules || flagRules.length === 0) {
    return undefined;
  }

  // Iterate through rules in order - first matching rule wins
  for (const rule of flagRules) {
    const result = evaluateRule(rule, artifact, user, context);
    if (result !== undefined) {
      return result;
    }
  }

  return undefined;
}

/**
 * Evaluate a single rule against user and context.
 * Returns the rule's payload value if the rule matches, undefined otherwise.
 * @param rule - The rule to evaluate
 * @param artifact - The AST artifact containing flag definitions and string table
 * @param user - User object with identity and attributes
 * @param context - Optional context object with environmental data
 * @returns The rule's payload value if the rule matches, undefined otherwise
 */
export function evaluateRule(
  rule: Rule,
  artifact: Artifact,
  user: User,
  context?: Context
): unknown {
  if (!Array.isArray(rule) || rule.length < 2) {
    return undefined;
  }

  const [ruleType, when, payload] = rule;

  // Evaluate when clause if present
  if (when !== undefined) {
    const whenResult = evaluateExpression(when, artifact, user, context);
    if (!whenResult) {
      // When clause doesn't match, skip this rule
      return undefined;
    }
  }

  // Rule matches - return payload based on rule type
  switch (ruleType) {
    case RuleType.SERVE: {
      // Serve rule: payload is string table index or direct value
      if (typeof payload === 'number') {
        return artifact.strs[payload];
      }
      return payload;
    }

    case RuleType.VARIATIONS: {
      // Variations rule: payload is Variation[]
      if (!Array.isArray(payload)) {
        return undefined;
      }
      return selectVariation(payload, artifact, user);
    }

    case RuleType.ROLLOUT: {
      // Rollout rule: payload is [value_index, pct] tuple
      if (!Array.isArray(payload) || payload.length !== 2) {
        return undefined;
      }
      const [valueIndex, pct] = payload;
      if (selectRollout(user, pct)) {
        if (typeof valueIndex === 'number') {
          return artifact.strs[valueIndex];
        }
        return valueIndex;
      }
      return undefined;
    }

    default:
      return undefined;
  }
}

/**
 * Evaluate an expression against user and context.
 * Returns the boolean result of the expression.
 * @param expr - The expression to evaluate
 * @param artifact - The AST artifact containing string table
 * @param user - User object with identity and attributes
 * @param context - Optional context object with environmental data
 * @returns The boolean result of the expression
 */
export function evaluateExpression(
  expr: Expression,
  artifact: Artifact,
  user: User,
  context?: Context
): boolean {
  if (!Array.isArray(expr) || expr.length < 2) {
    return false;
  }

  const [type, ...operands] = expr;

  switch (type) {
    case ExpressionType.BINARY_OP: {
      // Binary operator: [0, op_code, left, right]
      if (operands.length < 3) {
        return false;
      }
      const [opCode, leftExpr, rightExpr] = operands;
      if (!isExpression(leftExpr) || !isExpression(rightExpr)) {
        return false;
      }
      const left = evaluateExpressionValue(leftExpr, artifact, user, context);
      const right = evaluateExpressionValue(rightExpr, artifact, user, context);
      return evaluateBinaryOp(opCode as number, left, right);
    }

    case ExpressionType.LOGICAL_OP: {
      // Logical operator: [1, op_code, left, right?]
      if (operands.length < 2) {
        return false;
      }
      const [opCode, leftExpr, rightExpr] = operands;
      if (!isExpression(leftExpr)) {
        return false;
      }
      const left = evaluateExpression(leftExpr, artifact, user, context);

      if (opCode === LogicalOp.NOT) {
        // NOT has no right operand
        return !left;
      }

      if (rightExpr === undefined) {
        return left;
      }

      if (!isExpression(rightExpr)) {
        return false;
      }
      const right = evaluateExpression(rightExpr, artifact, user, context);

      switch (opCode) {
        case LogicalOp.AND:
          return left && right;
        case LogicalOp.OR:
          return left || right;
        default:
          return false;
      }
    }

    case ExpressionType.PROPERTY: {
      // Property access: [2, prop_index]
      if (operands.length < 1) {
        return false;
      }
      const propIndex = operands[0] as number;
      const propPath = artifact.strs[propIndex];
      if (!propPath) {
        return false;
      }
      const value = getProperty(propPath, user, context);
      return Boolean(value);
    }

    case ExpressionType.LITERAL: {
      // Literal value: [3, value]
      if (operands.length < 1) {
        return false;
      }
      const value = operands[0];
      // Handle string table indices for string literals
      if (typeof value === 'number' && artifact.strs[value] !== undefined) {
        return Boolean(artifact.strs[value]);
      }
      return Boolean(value);
    }

    case ExpressionType.FUNC: {
      // Function call: [4, func_code, args[]]
      if (operands.length < 2) {
        return false;
      }
      const [funcCode, args] = operands;
      if (!Array.isArray(args)) {
        return false;
      }
      return evaluateFunction(funcCode as number, args as Expression[], artifact, user, context);
    }

    default:
      return false;
  }
}

/**
 * Evaluate an expression to a value (not just boolean).
 * Used for binary operations where we need the actual values.
 */
function evaluateExpressionValue(
  expr: Expression,
  artifact: Artifact,
  user: User,
  context?: Context
): unknown {
  if (!Array.isArray(expr) || expr.length < 2) {
    return undefined;
  }

  const [type, ...operands] = expr;

  switch (type) {
    case ExpressionType.PROPERTY: {
      const propIndex = operands[0] as number;
      const propPath = artifact.strs[propIndex];
      if (!propPath) {
        return undefined;
      }
      return getProperty(propPath, user, context);
    }

    case ExpressionType.LITERAL: {
      const value = operands[0];
      // Handle string table indices for string literals
      if (typeof value === 'number' && artifact.strs[value] !== undefined) {
        return artifact.strs[value];
      }
      return value;
    }

    case ExpressionType.FUNC: {
      const [funcCode, args] = operands;
      if (!Array.isArray(args)) {
        return undefined;
      }
      return evaluateFunction(funcCode as number, args as Expression[], artifact, user, context);
    }

    default:
      return undefined;
  }
}

/**
 * Evaluate a binary operator.
 */
function evaluateBinaryOp(opCode: number, left: unknown, right: unknown): boolean {
  switch (opCode) {
    case BinaryOp.EQ:
      return left === right;
    case BinaryOp.NE:
      return left !== right;
    case BinaryOp.GT:
      return compareValues(left, right) > 0;
    case BinaryOp.LT:
      return compareValues(left, right) < 0;
    case BinaryOp.GTE:
      return compareValues(left, right) >= 0;
    case BinaryOp.LTE:
      return compareValues(left, right) <= 0;
    default:
      return false;
  }
}

/**
 * Compare two values for ordering.
 */
function compareValues(left: unknown, right: unknown): number {
  // Convert to comparable types
  const leftNum =
    typeof left === 'number' ? left : typeof left === 'string' ? parseFloat(left) : NaN;
  const rightNum =
    typeof right === 'number' ? right : typeof right === 'string' ? parseFloat(right) : NaN;

  if (!isNaN(leftNum) && !isNaN(rightNum)) {
    return leftNum - rightNum;
  }

  // String comparison
  const leftStr = String(left);
  const rightStr = String(right);
  return leftStr.localeCompare(rightStr);
}

/**
 * Get a property value from user or context using dot notation.
 */
function getProperty(propPath: string, user: User, context?: Context): unknown {
  const parts = propPath.split('.');
  if (parts.length === 0) {
    return undefined;
  }

  // First part determines the root object
  const root = parts[0];
  let obj: unknown;

  if (root === 'user') {
    obj = user;
  } else if (root === 'context' && context) {
    obj = context;
  } else {
    // Try user first, then context
    obj = (user as Record<string, unknown>)[root] ?? (context as Record<string, unknown>)?.[root];
  }

  if (obj === undefined || obj === null) {
    return undefined;
  }

  // Navigate nested properties
  for (let i = 1; i < parts.length; i++) {
    if (typeof obj !== 'object' || obj === null) {
      return undefined;
    }
    obj = (obj as Record<string, unknown>)[parts[i]];
    if (obj === undefined || obj === null) {
      return undefined;
    }
  }

  return obj;
}

/**
 * Select a variation based on user ID hash.
 */
function selectVariation(variations: Variation[], artifact: Artifact, user: User): unknown {
  if (!variations || variations.length === 0) {
    return undefined;
  }

  // Helper to safely get string from string table
  const getString = (varIndex: number): string | undefined => {
    if (typeof varIndex !== 'number' || varIndex < 0 || varIndex >= artifact.strs.length) {
      return undefined;
    }
    return artifact.strs[varIndex];
  };

  // Use user ID for consistent hashing
  const userId = user.id || '';
  const hash = hashString(userId);

  // Calculate total percentage
  const totalPct = variations.reduce((sum, [_, pct]) => sum + pct, 0);
  if (totalPct === 0) {
    // Return first variation if no percentages
    const [varIndex] = variations[0];
    return getString(varIndex);
  }

  // Normalize hash to 0-100 range
  const bucket = hash % 100;
  let cumulative = 0;

  for (const [varIndex, pct] of variations) {
    cumulative += pct;
    if (bucket < cumulative) {
      const result = getString(varIndex);
      if (result !== undefined) {
        return result;
      }
    }
  }

  // Fallback to last variation
  const [varIndex] = variations[variations.length - 1];
  return getString(varIndex);
}

/**
 * Select rollout based on percentage.
 */
function selectRollout(user: User, pct: number): boolean {
  if (pct <= 0) {
    return false;
  }
  if (pct >= 100) {
    return true;
  }

  // Use user ID for consistent hashing
  const userId = user.id || '';
  const hash = hashString(userId);
  const bucket = hash % 100;

  return bucket < pct;
}

/**
 * Simple string hash function (djb2 algorithm).
 */
function hashString(str: string): number {
  let hash = 5381;
  for (let i = 0; i < str.length; i++) {
    hash = (hash << 5) + hash + str.charCodeAt(i);
    hash = hash | 0; // Convert to 32-bit integer
  }
  return Math.abs(hash);
}

/**
 * Evaluate a function call.
 *
 * **Phase 2 Limitation**: Most functions are not yet implemented.
 * Functions that are not implemented will return `false`, which means
 * any rule using these functions in a `when` clause will not match.
 *
 * Full function support will be implemented in Phase 4.
 *
 * @param funcCode - Function code from FuncCode enum
 * @param args - Function arguments (expressions)
 * @param _artifact - AST artifact containing string table (unused in Phase 2)
 * @param _user - User object (unused in Phase 2)
 * @param _context - Optional context object (unused in Phase 2)
 * @returns Boolean result of function evaluation, or false if not implemented
 */
function evaluateFunction(
  funcCode: number,
  args: Expression[],
  _artifact: Artifact,
  _user: User,
  _context?: Context
): boolean {
  // For Phase 2, we'll implement basic functions
  // Full function support comes in Phase 4

  switch (funcCode) {
    case FuncCode.IN: {
      // IN function: check if value is in array
      // **Not implemented in Phase 2** - returns false
      // Full implementation in Phase 4
      if (args.length < 2) {
        return false;
      }
      // Note: This function is not yet implemented
      // Returning false means rules using IN() will not match
      return false;
    }

    default:
      // For Phase 2, return false for unimplemented functions
      // This is a known limitation - full function support comes in Phase 4
      // Rules using unimplemented functions will not match
      return false;
  }
}
