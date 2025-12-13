/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * AST (Abstract Syntax Tree) data structures for compiled Control Path artifacts.
 * These structures match the format specified in specs/ast-format.md.
 *
 * The AST uses compact array-based formats optimized for:
 * - Small size (MessagePack encoded)
 * - Fast evaluation (direct array access)
 * - Minimal memory footprint
 */

/**
 * Top-level AST artifact structure.
 * This is the root structure that gets serialized to MessagePack.
 */
export interface Artifact {
  /** Format version (e.g., "1.0") */
  v: string;
  /** Environment name */
  env: string;
  /** String table - all strings referenced by index (uint16) */
  strs: string[];
  /** Array of flag rule arrays, indexed by flag definition order */
  flags: Rule[][];
  /** Optional segment definitions as [name_index, expression] tuples */
  segments?: [number, Expression][];
  /** Optional Ed25519 signature */
  sig?: Uint8Array;
}

/**
 * Rule type codes (first element of rule array)
 */
export const RuleType = {
  /** Serve rule - payload is string | number (string table index) */
  SERVE: 0,
  /** Variations rule - payload is Variation[] */
  VARIATIONS: 1,
  /** Rollout rule - payload is [value_index, pct] tuple */
  ROLLOUT: 2,
} as const;

export type RuleType = (typeof RuleType)[keyof typeof RuleType];

/**
 * Rule structure: [type, when?, payload]
 *
 * Types:
 * - serve: [0, undefined, string | number] or [0, Expression, string | number]
 * - variations: [1, undefined, Variation[]] or [1, Expression, Variation[]]
 * - rollout: [2, undefined, [string | number, number]] or [2, Expression, [string | number, number]]
 */
export type Rule =
  | [0, undefined, string | number] // serve without when
  | [0, Expression, string | number] // serve with when
  | [1, undefined, Variation[]] // variations without when
  | [1, Expression, Variation[]] // variations with when
  | [2, undefined, [string | number, number]] // rollout without when
  | [2, Expression, [string | number, number]]; // rollout with when

/**
 * Variation structure: [var_index, pct]
 * - var_index: string table index (uint16)
 * - pct: percentage (uint8, 0-100)
 */
export type Variation = [number, number];

/**
 * Expression node type codes (first element of expression array)
 */
export const ExpressionType = {
  /** Binary operator: [0, op_code, left, right] */
  BINARY_OP: 0,
  /** Logical operator: [1, op_code, left, right?] (NOT has no right) */
  LOGICAL_OP: 1,
  /** Property access: [2, prop_index] */
  PROPERTY: 2,
  /** Literal value: [3, value] */
  LITERAL: 3,
  /** Function call: [4, func_code, args[]] */
  FUNC: 4,
} as const;

export type ExpressionType = (typeof ExpressionType)[keyof typeof ExpressionType];

/**
 * Binary operator codes (second element of binary_op expression)
 */
export const BinaryOp = {
  EQ: 0, // ==
  NE: 1, // !=
  GT: 2, // >
  LT: 3, // <
  GTE: 4, // >=
  LTE: 5, // <=
} as const;

export type BinaryOp = (typeof BinaryOp)[keyof typeof BinaryOp];

/**
 * Logical operator codes (second element of logical_op expression)
 */
export const LogicalOp = {
  AND: 6,
  OR: 7,
  NOT: 8,
} as const;

export type LogicalOp = (typeof LogicalOp)[keyof typeof LogicalOp];

/**
 * Function codes (second element of func expression)
 */
export const FuncCode = {
  STARTS_WITH: 0,
  ENDS_WITH: 1,
  CONTAINS: 2,
  IN: 3,
  MATCHES: 4,
  UPPER: 5,
  LOWER: 6,
  LENGTH: 7,
  INTERSECTS: 8,
  SEMVER_EQ: 9,
  SEMVER_GT: 10,
  SEMVER_GTE: 11,
  SEMVER_LT: 12,
  SEMVER_LTE: 13,
  HASH: 14, // Maps from HASHED_PARTITION
  COALESCE: 15,
  IS_BETWEEN: 16,
  IS_AFTER: 17,
  IS_BEFORE: 18,
  DAY_OF_WEEK: 19, // Maps from CURRENT_DAY_OF_WEEK_UTC
  HOUR_OF_DAY: 20, // Maps from CURRENT_HOUR_UTC
  DAY_OF_MONTH: 21, // Maps from CURRENT_DAY_OF_MONTH_UTC
  MONTH: 22, // Maps from CURRENT_MONTH_UTC
  CURRENT_TIMESTAMP: 23,
  IN_SEGMENT: 24,
} as const;

export type FuncCode = (typeof FuncCode)[keyof typeof FuncCode];

/**
 * Expression structure: [type, ...operands]
 *
 * Types:
 * - binary_op: [0, op_code, left_expr, right_expr]
 * - logical_op: [1, op_code, left_expr, right_expr?] (NOT has no right)
 * - property: [2, prop_index] (prop_index is string table index)
 * - literal: [3, value] (value can be string table index for strings)
 * - func: [4, func_code, [arg_expr, ...]]
 */
export type Expression =
  | [0, number, Expression, Expression] // binary_op: [0, op_code, left, right]
  | [1, number, Expression, Expression?] // logical_op: [1, op_code, left, right?] (NOT has no right)
  | [2, number] // property: [2, prop_index]
  | [3, unknown] // literal: [3, value]
  | [4, number, Expression[]]; // func: [4, func_code, args]

/**
 * Type guard to check if a value is an Artifact
 */
export function isArtifact(value: unknown): value is Artifact {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return false;
  }

  const artifact = value as Record<string, unknown>;

  return (
    typeof artifact.v === 'string' &&
    typeof artifact.env === 'string' &&
    Array.isArray(artifact.strs) &&
    artifact.strs.every((s) => typeof s === 'string') &&
    Array.isArray(artifact.flags) &&
    artifact.flags.every((flag) => Array.isArray(flag))
  );
}

/**
 * Type guard to check if a value is a Rule
 */
export function isRule(value: unknown): value is Rule {
  if (!Array.isArray(value) || value.length < 2) {
    return false;
  }

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const type: unknown = value[0];
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const when: unknown = value[1];
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const payload: unknown = value[2];

  // Type must be a valid RuleType
  if (typeof type !== 'number' || type < 0 || type > 2) {
    return false;
  }

  // when is optional (can be undefined or Expression)
  if (when !== undefined && !isExpression(when)) {
    return false;
  }

  // Payload validation depends on rule type
  if (type === RuleType.SERVE) {
    return typeof payload === 'string' || typeof payload === 'number';
  }

  if (type === RuleType.VARIATIONS) {
    return Array.isArray(payload) && payload.every(isVariation);
  }

  if (type === RuleType.ROLLOUT) {
    return (
      Array.isArray(payload) &&
      payload.length === 2 &&
      (typeof payload[0] === 'string' || typeof payload[0] === 'number') &&
      typeof payload[1] === 'number'
    );
  }

  return false;
}

/**
 * Type guard to check if a value is a Variation
 */
export function isVariation(value: unknown): value is Variation {
  return (
    Array.isArray(value) &&
    value.length === 2 &&
    typeof value[0] === 'number' &&
    typeof value[1] === 'number'
  );
}

/**
 * Type guard to check if a value is an Expression
 */
export function isExpression(value: unknown): value is Expression {
  if (!Array.isArray(value) || value.length < 2) {
    return false;
  }

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const type: unknown = value[0];

  if (typeof type !== 'number' || type < 0 || type > 4) {
    return false;
  }

  switch (type) {
    case ExpressionType.BINARY_OP:
      return (
        value.length === 4 &&
        typeof value[1] === 'number' &&
        isExpression(value[2]) &&
        isExpression(value[3])
      );

    case ExpressionType.LOGICAL_OP:
      return (
        value.length >= 3 &&
        value.length <= 4 &&
        typeof value[1] === 'number' &&
        isExpression(value[2]) &&
        (value[3] === undefined || isExpression(value[3]))
      );

    case ExpressionType.PROPERTY:
      return value.length === 2 && typeof value[1] === 'number';

    case ExpressionType.LITERAL:
      return value.length === 2;

    case ExpressionType.FUNC:
      return (
        value.length === 3 &&
        typeof value[1] === 'number' &&
        Array.isArray(value[2]) &&
        value[2].every(isExpression)
      );

    default:
      return false;
  }
}
