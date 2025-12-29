/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Type definitions for Control Path runtime SDK.
 *
 * AST types are imported from @controlpath/compiler (dev dependency only, since types are compile-time).
 * Runtime constants and type guards are defined here.
 */

// Import types from compiler package (compile-time only, no runtime dependency)
import type { Artifact, Rule, Expression, Variation } from '@controlpath/compiler';

// Re-export types for consumers
export type { Artifact, Rule, Expression, Variation };

/**
 * ResolutionDetails type matching OpenFeature's ResolutionDetails interface.
 * This allows the Provider to be OpenFeature-compliant without requiring the OpenFeature package.
 */
export interface ResolutionDetails<T> {
  value: T;
  reason?: string;
  variant?: string;
  errorCode?: string;
  errorMessage?: string;
}

/**
 * Logger interface for optional error and debug logging.
 */
export interface Logger {
  /** Log an error message */
  error(message: string, error?: Error): void;
  /** Log a warning message */
  warn(message: string): void;
  /** Log an informational message */
  info(message: string): void;
  /** Log a debug message */
  debug(message: string): void;
}

/**
 * Rule type codes (first element of rule array)
 * These constants are needed at runtime for evaluation.
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

  // Access array elements with proper type narrowing
  const type: unknown = value[0];
  const when: unknown = value.length > 1 ? value[1] : undefined;
  const payload: unknown = value.length > 2 ? value[2] : undefined;

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
 * User object - represents user identity and attributes
 */
export interface User {
  /** User ID */
  id?: string;
  /** User email */
  email?: string;
  /** User role */
  role?: string;
  /** Additional user attributes */
  [key: string]: unknown;
}

/**
 * Context object - represents environmental data
 */
export interface Context {
  /** Environment name */
  environment?: string;
  /** Device information */
  device?: string;
  /** Application version */
  app_version?: string;
  /** Additional context attributes */
  [key: string]: unknown;
}
