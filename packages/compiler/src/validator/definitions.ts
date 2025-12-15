/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import Ajv from 'ajv';
import { ValidationResult, ValidationError } from '../validator';
import { validateWithSchema } from './common';
import { hasName, isFlagDefinition, isFlagDefinitions } from './type-guards';

/**
 * Validate flag definitions against the definitions schema.
 */
export function validateDefinitions(
  ajv: Ajv,
  schema: unknown,
  filePath: string,
  data: unknown
): ValidationResult {
  return validateWithSchema(ajv, schema, filePath, data, validateFlagSpecificRules);
}

/**
 * Validate flag-specific business rules that aren't covered by JSON schema.
 */
function validateFlagSpecificRules(filePath: string, data: unknown): ValidationError[] {
  if (!isFlagDefinitions(data)) {
    return [];
  }

  const errors: ValidationError[] = [];
  errors.push(...validateDuplicateFlagNames(filePath, data.flags));
  errors.push(...validateMultivariateFlags(filePath, data.flags));

  return errors;
}

/**
 * Validate that flag names are unique.
 */
function validateDuplicateFlagNames(filePath: string, flags: unknown[]): ValidationError[] {
  const errors: ValidationError[] = [];
  const flagNames = new Set<string>();

  flags.forEach((flag: unknown, index: number) => {
    if (!hasName(flag)) {
      return;
    }

    if (flagNames.has(flag.name)) {
      errors.push({
        file: filePath,
        message: `Duplicate flag name: '${flag.name}'`,
        path: `/flags/${index}/name`,
        suggestion: `Flag names must be unique. Rename this flag or remove the duplicate.`,
      });
    } else {
      flagNames.add(flag.name);
    }
  });

  return errors;
}

/**
 * Validate that multivariate flags have variations and no duplicate variation names.
 */
function validateMultivariateFlags(filePath: string, flags: unknown[]): ValidationError[] {
  const errors: ValidationError[] = [];

  flags.forEach((flag: unknown, index: number) => {
    if (!isFlagDefinition(flag) || flag.type !== 'multivariate') {
      return;
    }

    if (!Array.isArray(flag.variations) || flag.variations.length === 0) {
      const flagName = hasName(flag) ? flag.name : 'unnamed';
      errors.push({
        file: filePath,
        message: `Multivariate flag '${flagName}' must have at least one variation. Missing variations array.`,
        path: `/flags/${index}/variations`,
        suggestion: `Add a 'variations' array with at least one variation.`,
      });
      return;
    }

    errors.push(...validateDuplicateVariationNames(filePath, flag, index));
  });

  return errors;
}

/**
 * Validate that variation names are unique within a flag.
 */
function validateDuplicateVariationNames(
  filePath: string,
  flag: { name?: string; variations?: unknown[] },
  flagIndex: number
): ValidationError[] {
  const errors: ValidationError[] = [];
  const variationNames = new Set<string>();
  const flagName = hasName(flag) ? flag.name : 'unnamed';

  if (!flag.variations) {
    return errors;
  }

  flag.variations.forEach((variation: unknown, variationIndex: number) => {
    if (!hasName(variation)) {
      return;
    }

    if (variationNames.has(variation.name)) {
      errors.push({
        file: filePath,
        message: `Duplicate variation name '${variation.name}' in flag '${flagName}'`,
        path: `/flags/${flagIndex}/variations/${variationIndex}/name`,
        suggestion: `Variation names must be unique within a flag.`,
      });
    } else {
      variationNames.add(variation.name);
    }
  });

  return errors;
}
