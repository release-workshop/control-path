/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Flag evaluator module for evaluating flags using AST rules.
 * This module handles rule matching and expression evaluation.
 */

import type { Artifact, Rule, Expression, Attributes, Variation } from './types';
import {
  RuleType,
  ExpressionType,
  BinaryOp,
  LogicalOp,
  FuncCode,
  isExpression,
  PROTOTYPE_POLLUTING_KEYS,
} from './types';
import * as semver from 'semver';

/**
 * Evaluate a flag by index using the provided artifact and attributes.
 * Returns the evaluated value or undefined if no rules match.
 * @param flagIndex - The index of the flag in the flags array
 * @param artifact - The AST artifact containing flag definitions
 * @param attributes - Optional attributes object with user identity, attributes, and context
 * @returns The evaluated value or undefined if no rules match
 */
export function evaluate(flagIndex: number, artifact: Artifact, attributes?: Attributes): unknown {
  // Type guard: ensure flags exists and is an array
  if (!Array.isArray(artifact.flags) || flagIndex < 0 || flagIndex >= artifact.flags.length) {
    return undefined;
  }

  const flagRules: Rule[] = artifact.flags[flagIndex];
  if (!Array.isArray(flagRules) || flagRules.length === 0) {
    return undefined;
  }

  // Iterate through rules in order - first matching rule wins
  for (const rule of flagRules) {
    const result = evaluateRule(rule, artifact, attributes);
    if (result !== undefined) {
      return result;
    }
  }

  return undefined;
}

/**
 * Evaluate a single rule against attributes.
 * Returns the rule's payload value if the rule matches, undefined otherwise.
 * @param rule - The rule to evaluate
 * @param artifact - The AST artifact containing flag definitions and string table
 * @param attributes - Optional attributes object with user identity, attributes, and context
 * @returns The rule's payload value if the rule matches, undefined otherwise
 */
export function evaluateRule(rule: Rule, artifact: Artifact, attributes?: Attributes): unknown {
  if (!Array.isArray(rule) || rule.length < 2) {
    return undefined;
  }

  // Rule is a tuple type, so we can safely access elements
  // The type is: [type, when?, payload] where type is 0, 1, or 2
  // Note: Rust serializes None as null, so we need to check for both undefined and null
  const ruleType = rule[0];
  const when: Expression | undefined | null = rule.length > 1 ? rule[1] : undefined;
  const payload: unknown = rule.length > 2 ? rule[2] : undefined;

  // Evaluate when clause if present (check for both undefined and null since Rust serializes None as null)
  if (when !== undefined && when !== null) {
    const whenResult = evaluateExpression(when, artifact, attributes);
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
      // Type guard: ensure payload is Variation[]
      const variations = payload as Variation[];
      return selectVariation(variations, artifact, attributes);
    }

    case RuleType.ROLLOUT: {
      // Rollout rule: payload is [value_index, pct] tuple
      if (!Array.isArray(payload) || payload.length !== 2) {
        return undefined;
      }
      const rolloutPayload = payload as [string | number, number];
      const [valueIndex, pct] = rolloutPayload;
      if (selectRollout(pct, attributes)) {
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
 * Evaluate an expression against attributes.
 * Returns the boolean result of the expression.
 * @param expr - The expression to evaluate
 * @param artifact - The AST artifact containing string table
 * @param attributes - Optional attributes object with user identity, attributes, and context
 * @returns The boolean result of the expression
 */
export function evaluateExpression(
  expr: Expression,
  artifact: Artifact,
  attributes?: Attributes
): boolean {
  if (!Array.isArray(expr) || expr.length < 2) {
    return false;
  }

  // Expression is a tuple type, so we can safely access elements
  const type = expr[0];
  const operands: unknown[] = expr.slice(1);

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
      const left = evaluateExpressionValue(leftExpr, artifact, attributes);
      const right = evaluateExpressionValue(rightExpr, artifact, attributes);
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
      const left = evaluateExpression(leftExpr, artifact, attributes);

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
      const right = evaluateExpression(rightExpr, artifact, attributes);

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
      const value = getProperty(propPath, attributes);
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
      const result = evaluateFunction(
        funcCode as number,
        args as Expression[],
        artifact,
        attributes
      );
      // Coerce function result to boolean
      return Boolean(result);
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
  attributes?: Attributes
): unknown {
  if (!Array.isArray(expr) || expr.length < 2) {
    return undefined;
  }

  // Expression is a tuple type, so we can safely access elements
  const type = expr[0];
  const operands: unknown[] = expr.slice(1);

  switch (type) {
    case ExpressionType.PROPERTY: {
      const propIndex = operands[0] as number;
      const propPath = artifact.strs[propIndex];
      if (!propPath) {
        return undefined;
      }
      return getProperty(propPath, attributes);
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
      return evaluateFunction(funcCode as number, args as Expression[], artifact, attributes);
    }

    default:
      return undefined;
  }
}

/**
 * Evaluate a binary operator.
 * Handles null comparisons and type coercion.
 */
function evaluateBinaryOp(opCode: number, left: unknown, right: unknown): boolean {
  switch (opCode) {
    case BinaryOp.EQ:
      // Handle null comparisons
      if (left === null || right === null) {
        return left === right;
      }
      // Type coercion for equality
      return coerceAndCompare(left, right) === 0;
    case BinaryOp.NE:
      // Handle null comparisons
      if (left === null || right === null) {
        return left !== right;
      }
      // Type coercion for inequality
      return coerceAndCompare(left, right) !== 0;
    case BinaryOp.GT:
      // Null comparisons always return false for ordering
      if (left === null || right === null) {
        return false;
      }
      return compareValues(left, right) > 0;
    case BinaryOp.LT:
      // Null comparisons always return false for ordering
      if (left === null || right === null) {
        return false;
      }
      return compareValues(left, right) < 0;
    case BinaryOp.GTE:
      // Null comparisons always return false for ordering
      if (left === null || right === null) {
        return false;
      }
      return compareValues(left, right) >= 0;
    case BinaryOp.LTE:
      // Null comparisons always return false for ordering
      if (left === null || right === null) {
        return false;
      }
      return compareValues(left, right) <= 0;
    default:
      return false;
  }
}

/**
 * Compare two values for ordering with type coercion.
 * Attempts to coerce to numbers first, then falls back to string comparison.
 */
function compareValues(left: unknown, right: unknown): number {
  // Try number coercion
  const leftNum = coerceToNumber(left);
  const rightNum = coerceToNumber(right);

  if (leftNum !== null && rightNum !== null) {
    return leftNum - rightNum;
  }

  // String comparison
  const leftStr = String(left);
  const rightStr = String(right);
  return leftStr.localeCompare(rightStr);
}

/**
 * Coerce and compare two values (for equality operations).
 * Returns 0 if equal, non-zero if not equal.
 */
function coerceAndCompare(left: unknown, right: unknown): number {
  // Exact match (including null/undefined)
  if (left === right) {
    return 0;
  }

  // Try number coercion
  const leftNum = coerceToNumber(left);
  const rightNum = coerceToNumber(right);
  if (leftNum !== null && rightNum !== null) {
    return leftNum === rightNum ? 0 : 1;
  }

  // Try boolean coercion
  const leftBool = coerceToBoolean(left);
  const rightBool = coerceToBoolean(right);
  if (leftBool !== null && rightBool !== null) {
    return leftBool === rightBool ? 0 : 1;
  }

  // String comparison
  return String(left).localeCompare(String(right));
}

/**
 * Coerce a value to a number if possible.
 * Returns null if coercion is not possible.
 */
function coerceToNumber(value: unknown): number | null {
  if (typeof value === 'number') {
    return value;
  }
  if (typeof value === 'string') {
    const num = parseFloat(value);
    if (!isNaN(num) && isFinite(num)) {
      return num;
    }
  }
  if (typeof value === 'boolean') {
    return value ? 1 : 0;
  }
  return null;
}

/**
 * Coerce a value to a boolean if possible.
 * Returns null if coercion is not possible.
 */
function coerceToBoolean(value: unknown): boolean | null {
  if (typeof value === 'boolean') {
    return value;
  }
  if (typeof value === 'string') {
    const lower = value.toLowerCase();
    if (lower === 'true' || lower === '1') {
      return true;
    }
    if (lower === 'false' || lower === '0') {
      return false;
    }
  }
  if (typeof value === 'number') {
    return value !== 0;
  }
  return null;
}

/**
 * Get a property value from attributes using dot notation.
 * Rejects prototype-polluting property paths for security.
 */
function getProperty(propPath: string, attributes?: Attributes): unknown {
  if (!attributes) {
    return undefined;
  }

  const parts = propPath.split('.');
  if (parts.length === 0) {
    return undefined;
  }

  // Reject prototype-polluting paths
  if (
    parts.some((part) =>
      PROTOTYPE_POLLUTING_KEYS.includes(part as (typeof PROTOTYPE_POLLUTING_KEYS)[number])
    )
  ) {
    return undefined;
  }

  // Navigate properties directly from attributes object
  let obj: unknown = attributes;

  for (let i = 0; i < parts.length; i++) {
    if (typeof obj !== 'object' || obj === null) {
      return undefined;
    }
    const objRecord: Record<string, unknown> = obj as Record<string, unknown>;
    obj = objRecord[parts[i]];
    if (obj === undefined || obj === null) {
      return undefined;
    }
  }

  return obj;
}

/**
 * Select a variation based on user ID hash.
 */
function selectVariation(
  variations: Variation[],
  artifact: Artifact,
  attributes?: Attributes
): unknown {
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
  const userId = attributes?.id || '';
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
function selectRollout(pct: number, attributes?: Attributes): boolean {
  if (pct <= 0) {
    return false;
  }
  if (pct >= 100) {
    return true;
  }

  // Use user ID for consistent hashing
  const userId = attributes?.id || '';
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
 * Returns the function result (which may be boolean, string, number, etc.).
 * When used in a boolean context, the result will be coerced to boolean.
 *
 * @param funcCode - Function code from FuncCode enum
 * @param args - Function arguments (expressions)
 * @param artifact - AST artifact containing string table and segments
 * @param attributes - Optional attributes object with user identity, attributes, and context
 * @returns Function result (type depends on function)
 */
function evaluateFunction(
  funcCode: number,
  args: Expression[],
  artifact: Artifact,
  attributes?: Attributes
): unknown {
  switch (funcCode) {
    // String functions
    case FuncCode.STARTS_WITH: {
      if (args.length < 2) {
        return false;
      }
      const str = evaluateExpressionValue(args[0], artifact, attributes);
      const prefix = evaluateExpressionValue(args[1], artifact, attributes);
      if (typeof str !== 'string' || typeof prefix !== 'string') {
        return false;
      }
      return str.startsWith(prefix);
    }

    case FuncCode.ENDS_WITH: {
      if (args.length < 2) {
        return false;
      }
      const str = evaluateExpressionValue(args[0], artifact, attributes);
      const suffix = evaluateExpressionValue(args[1], artifact, attributes);
      if (typeof str !== 'string' || typeof suffix !== 'string') {
        return false;
      }
      return str.endsWith(suffix);
    }

    case FuncCode.CONTAINS: {
      if (args.length < 2) {
        return false;
      }
      const container = evaluateExpressionValue(args[0], artifact, attributes);
      const value = evaluateExpressionValue(args[1], artifact, attributes);

      // Support both string and array containers
      if (typeof container === 'string' && typeof value === 'string') {
        return container.includes(value);
      }
      if (Array.isArray(container)) {
        return container.includes(value);
      }
      return false;
    }

    case FuncCode.MATCHES: {
      if (args.length < 2) {
        return false;
      }
      const str = evaluateExpressionValue(args[0], artifact, attributes);
      const pattern = evaluateExpressionValue(args[1], artifact, attributes);
      if (typeof str !== 'string' || typeof pattern !== 'string') {
        return false;
      }
      try {
        const regex = new RegExp(pattern);
        return regex.test(str);
      } catch {
        // Invalid regex pattern
        return false;
      }
    }

    case FuncCode.UPPER: {
      if (args.length < 1) {
        return '';
      }
      const str = evaluateExpressionValue(args[0], artifact, attributes);
      if (typeof str !== 'string') {
        return String(str).toUpperCase();
      }
      return str.toUpperCase();
    }

    case FuncCode.LOWER: {
      if (args.length < 1) {
        return '';
      }
      const str = evaluateExpressionValue(args[0], artifact, attributes);
      if (typeof str !== 'string') {
        return String(str).toLowerCase();
      }
      return str.toLowerCase();
    }

    case FuncCode.LENGTH: {
      if (args.length < 1) {
        return 0;
      }
      const value = evaluateExpressionValue(args[0], artifact, attributes);
      if (typeof value === 'string' || Array.isArray(value)) {
        return value.length;
      }
      return 0;
    }

    // Set functions
    case FuncCode.IN: {
      if (args.length < 2) {
        return false;
      }
      const value = evaluateExpressionValue(args[0], artifact, attributes);
      const list = evaluateExpressionValue(args[1], artifact, attributes);
      if (!Array.isArray(list)) {
        return false;
      }
      return list.includes(value);
    }

    case FuncCode.INTERSECTS: {
      if (args.length < 2) {
        return false;
      }
      const arr1 = evaluateExpressionValue(args[0], artifact, attributes);
      const arr2 = evaluateExpressionValue(args[1], artifact, attributes);
      if (!Array.isArray(arr1) || !Array.isArray(arr2)) {
        return false;
      }
      return arr1.some((item) => arr2.includes(item));
    }

    // Semver functions
    case FuncCode.SEMVER_EQ: {
      if (args.length < 2) {
        return false;
      }
      const v1 = evaluateExpressionValue(args[0], artifact, attributes);
      const v2 = evaluateExpressionValue(args[1], artifact, attributes);
      if (typeof v1 !== 'string' || typeof v2 !== 'string') {
        return false;
      }
      try {
        return semver.eq(v1, v2);
      } catch {
        return false;
      }
    }

    case FuncCode.SEMVER_GT: {
      if (args.length < 2) {
        return false;
      }
      const v1 = evaluateExpressionValue(args[0], artifact, attributes);
      const v2 = evaluateExpressionValue(args[1], artifact, attributes);
      if (typeof v1 !== 'string' || typeof v2 !== 'string') {
        return false;
      }
      try {
        return semver.gt(v1, v2);
      } catch {
        return false;
      }
    }

    case FuncCode.SEMVER_GTE: {
      if (args.length < 2) {
        return false;
      }
      const v1 = evaluateExpressionValue(args[0], artifact, attributes);
      const v2 = evaluateExpressionValue(args[1], artifact, attributes);
      if (typeof v1 !== 'string' || typeof v2 !== 'string') {
        return false;
      }
      try {
        return semver.gte(v1, v2);
      } catch {
        return false;
      }
    }

    case FuncCode.SEMVER_LT: {
      if (args.length < 2) {
        return false;
      }
      const v1 = evaluateExpressionValue(args[0], artifact, attributes);
      const v2 = evaluateExpressionValue(args[1], artifact, attributes);
      if (typeof v1 !== 'string' || typeof v2 !== 'string') {
        return false;
      }
      try {
        return semver.lt(v1, v2);
      } catch {
        return false;
      }
    }

    case FuncCode.SEMVER_LTE: {
      if (args.length < 2) {
        return false;
      }
      const v1 = evaluateExpressionValue(args[0], artifact, attributes);
      const v2 = evaluateExpressionValue(args[1], artifact, attributes);
      if (typeof v1 !== 'string' || typeof v2 !== 'string') {
        return false;
      }
      try {
        return semver.lte(v1, v2);
      } catch {
        return false;
      }
    }

    // Hashing function
    case FuncCode.HASH: {
      // HASHED_PARTITION(id, buckets) - returns bucket number (0 to buckets-1)
      if (args.length < 2) {
        return 0;
      }
      const id = evaluateExpressionValue(args[0], artifact, attributes);
      const buckets = evaluateExpressionValue(args[1], artifact, attributes);

      const idStr = String(id ?? '');
      const bucketsNum = typeof buckets === 'number' ? buckets : Number(buckets);

      if (!Number.isInteger(bucketsNum) || bucketsNum <= 0) {
        return 0;
      }

      // Use consistent hashing (SHA-256 would be better, but djb2 is simpler and sufficient)
      const hash = hashString(idStr);
      return hash % bucketsNum;
    }

    // Utility functions
    case FuncCode.COALESCE: {
      if (args.length < 2) {
        return null;
      }
      // Return first non-null value
      for (const arg of args) {
        const value = evaluateExpressionValue(arg, artifact, attributes);
        if (value !== null && value !== undefined) {
          return value;
        }
      }
      return null;
    }

    // Temporal functions
    case FuncCode.IS_BETWEEN: {
      if (args.length < 2) {
        return false;
      }
      const start = evaluateExpressionValue(args[0], artifact, attributes);
      const end = evaluateExpressionValue(args[1], artifact, attributes);
      if (typeof start !== 'string' || typeof end !== 'string') {
        return false;
      }
      try {
        const startTime = new Date(start).getTime();
        const endTime = new Date(end).getTime();
        const now = Date.now();
        return now >= startTime && now <= endTime;
      } catch {
        return false;
      }
    }

    case FuncCode.IS_AFTER: {
      if (args.length < 1) {
        return false;
      }
      const timestamp = evaluateExpressionValue(args[0], artifact, attributes);
      if (typeof timestamp !== 'string') {
        return false;
      }
      try {
        const timestampTime = new Date(timestamp).getTime();
        return Date.now() > timestampTime;
      } catch {
        return false;
      }
    }

    case FuncCode.IS_BEFORE: {
      if (args.length < 1) {
        return false;
      }
      const timestamp = evaluateExpressionValue(args[0], artifact, attributes);
      if (typeof timestamp !== 'string') {
        return false;
      }
      try {
        const timestampTime = new Date(timestamp).getTime();
        return Date.now() < timestampTime;
      } catch {
        return false;
      }
    }

    case FuncCode.HOUR_OF_DAY: {
      // CURRENT_HOUR_UTC - returns 0-23
      return new Date().getUTCHours();
    }

    case FuncCode.DAY_OF_WEEK: {
      // CURRENT_DAY_OF_WEEK_UTC - returns day name (MONDAY, TUESDAY, etc.)
      const days = ['SUNDAY', 'MONDAY', 'TUESDAY', 'WEDNESDAY', 'THURSDAY', 'FRIDAY', 'SATURDAY'];
      return days[new Date().getUTCDay()];
    }

    case FuncCode.DAY_OF_MONTH: {
      // CURRENT_DAY_OF_MONTH_UTC - returns 1-31
      return new Date().getUTCDate();
    }

    case FuncCode.MONTH: {
      // CURRENT_MONTH_UTC - returns 1-12
      return new Date().getUTCMonth() + 1;
    }

    case FuncCode.CURRENT_TIMESTAMP: {
      // Returns ISO 8601 timestamp string in UTC
      return new Date().toISOString();
    }

    // Segment function
    case FuncCode.IN_SEGMENT: {
      if (args.length < 2) {
        return false;
      }
      // First arg is user (we ignore it since we have user in scope)
      const _userArg = evaluateExpressionValue(args[0], artifact, attributes);
      const segmentName = evaluateExpressionValue(args[1], artifact, attributes);

      // First arg should be user (we can ignore it since we have user in scope)
      // Second arg is segment name (string table index or string)
      let segmentNameStr: string;
      if (typeof segmentName === 'number' && artifact.strs[segmentName] !== undefined) {
        segmentNameStr = artifact.strs[segmentName];
      } else if (typeof segmentName === 'string') {
        segmentNameStr = segmentName;
      } else {
        return false;
      }

      // Look up segment in artifact
      if (!artifact.segments || artifact.segments.length === 0) {
        return false;
      }

      // Find segment by name (segment name is stored as string table index)
      const segment = artifact.segments.find(([nameIndex]) => {
        const name = artifact.strs[nameIndex];
        return name === segmentNameStr;
      });

      if (!segment) {
        return false;
      }

      // Evaluate segment expression (same as when clause)
      const [, segmentExpr] = segment;
      return evaluateExpression(segmentExpr, artifact, attributes);
    }

    default:
      // Unknown function code
      return false;
  }
}
