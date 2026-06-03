/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Type definitions for Control Path runtime SDK.
 *
 * AST types are defined here to match the AST format specification.
 * Runtime constants and type guards are defined here.
 */

/**
 * Artifact structure matching AST format.
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
  /** Flag names as string table indices (one per flag, same order as flags array) */
  flagNames: number[];
  /** Optional segment definitions as [name_index, expression] tuples */
  segments?: [number, Expression][];
  /** Optional Ed25519 signature */
  sig?: Uint8Array;
}

/**
 * Boolean rule structure: [type, when?, payload]
 *
 * - serve: `[0, undefined, string | number]` or `[0, Expression, string | number]`
 * - rollout: `[2, undefined, [string | number, number]]` or `[2, Expression, [string | number, number]]`
 *
 * Legacy multivariate rule type `1` is not part of this union; `isRule` rejects it.
 */
export type Rule =
  | [0, undefined, string | number]
  | [0, Expression, string | number]
  | [2, undefined, [string | number, number]]
  | [2, Expression, [string | number, number]];

/**
 * Expression node structure: [type, ...]
 *
 * Types:
 * - binary_op: [0, op_code, left, right]
 * - logical_op: [1, op_code, left, right?] (NOT has no right)
 * - property: [2, prop_index]
 * - literal: [3, value]
 * - func: [4, func_code, args[]]
 */
export type Expression =
  | [0, number, Expression, Expression] // binary_op
  | [1, number, Expression, Expression?] // logical_op
  | [2, number] // property
  | [3, unknown] // literal
  | [4, number, Expression[]]; // func

/**
 * Keys that are rejected to prevent prototype pollution attacks.
 * These keys should not be allowed in property paths or object keys.
 */
export const PROTOTYPE_POLLUTING_KEYS = ['__proto__', 'constructor', 'prototype'] as const;

/**
 * Rule type codes (first element of rule array)
 * These constants are needed at runtime for evaluation.
 */
export const RuleType = {
  /** Serve rule - payload is string | number (string table index) */
  SERVE: 0,
  /** Rollout rule - payload is [value_index, pct] tuple */
  ROLLOUT: 2,
} as const;

export type RuleType = (typeof RuleType)[keyof typeof RuleType];

/**
 * Expression node type codes (first element of expression array)
 * These constants are needed at runtime for evaluation.
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
 * These constants are needed at runtime for evaluation.
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
 * These constants are needed at runtime for evaluation.
 */
export const LogicalOp = {
  AND: 6,
  OR: 7,
  NOT: 8,
} as const;

export type LogicalOp = (typeof LogicalOp)[keyof typeof LogicalOp];

/**
 * Function codes (second element of func expression)
 * These constants are needed at runtime for evaluation.
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
 * Type guard to check if a value is an Artifact
 */
export function isArtifact(value: unknown): value is Artifact {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return false;
  }

  const artifact: Record<string, unknown> = value as Record<string, unknown>;

  return (
    typeof artifact.v === 'string' &&
    typeof artifact.env === 'string' &&
    Array.isArray(artifact.strs) &&
    artifact.strs.every((s) => typeof s === 'string') &&
    Array.isArray(artifact.flags) &&
    artifact.flags.every((flag) => Array.isArray(flag)) &&
    Array.isArray(artifact.flagNames) &&
    artifact.flagNames.every((idx) => typeof idx === 'number' && idx >= 0) &&
    artifact.flagNames.length === artifact.flags.length
  );
}

/**
 * Type guard to check if a value is a Rule
 */
export function isRule(value: unknown): value is Rule {
  if (!Array.isArray(value) || value.length < 2) {
    return false;
  }

  // Access array elements with proper type narrowing
  const type: unknown = value[0];
  const when: unknown = value.length > 1 ? value[1] : undefined;
  const payload: unknown = value.length > 2 ? value[2] : undefined;

  // v2 boolean AST: serve (0) and rollout (2) only
  if (typeof type !== 'number' || (type !== RuleType.SERVE && type !== RuleType.ROLLOUT)) {
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
 * Type guard to check if a value is an Expression
 */
export function isExpression(value: unknown): value is Expression {
  if (!Array.isArray(value) || value.length < 2) {
    return false;
  }

  // Access array element with proper type narrowing
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

/**
 * Platform-owned evaluation attribute fields (not redeclared in catalog attribute schema).
 */
export interface BaseAttributes {
  /** User ID */
  id?: string;
  /** User email */
  email?: string;
  /** User role */
  role?: string;
  /** Environment name */
  environment?: string;
  /** Device information */
  device?: string;
  /** Application version */
  app_version?: string;
}

/**
 * Attributes object - represents all attributes used for flag evaluation.
 * Consolidates user identity, user attributes, and environmental context into a single object.
 */
export interface Attributes extends BaseAttributes {
  /** Additional attributes */
  [key: string]: unknown;
}

/**
 * Evaluation attributes accepted at API boundaries.
 * Closed generated catalog types (without an index signature) assign here.
 */
export type AttributesInput = object;

/**
 * Kill switch file format (v2 boolean map).
 */
export interface KillSwitchFile {
  /** Format version (e.g., "2.0") */
  version: string;
  /** Map of qualified flag names to boolean values */
  flags: Record<string, boolean>;
}

/**
 * Logger interface for error and debug logging
 */
export interface Logger {
  /** Log debug message */
  debug(message: string, ...args: unknown[]): void;
  /** Log info message */
  info(message: string, ...args: unknown[]): void;
  /** Log warning message */
  warn(message: string, ...args: unknown[]): void;
  /** Log error message */
  error(message: string, error?: Error, ...args: unknown[]): void;
}
