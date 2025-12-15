/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Shared type guards for validation logic.
 */

/**
 * Type guard to check if value is a record/object.
 */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

/**
 * Type guard to check if value has a name property.
 */
export function hasName(value: unknown): value is { name: string } {
  return isRecord(value) && typeof value.name === 'string';
}

/**
 * Type guard to check if value is a flag definition object.
 */
export function isFlagDefinition(value: unknown): value is {
  name?: string;
  type?: string;
  variations?: unknown[];
} {
  return isRecord(value);
}

/**
 * Type guard to check if value is a variation object.
 */
export function isVariation(value: unknown): value is { weight?: number } {
  return isRecord(value);
}

/**
 * Type guard to check if value is a rollout object.
 */
export function isRollout(value: unknown): value is { percentage?: number } {
  return isRecord(value);
}

/**
 * Type guard to check if value is a flag definitions object with flags array.
 */
export function isFlagDefinitions(value: unknown): value is {
  flags: unknown[];
} {
  return isRecord(value) && Array.isArray(value.flags);
}

/**
 * Type guard to check if value is a deployment object with rules.
 */
export function isDeployment(value: unknown): value is {
  rules: Record<string, unknown>;
} {
  return isRecord(value) && isRecord(value.rules);
}
