/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import Ajv, { ValidateFunction } from 'ajv';
import { ValidationResult, ValidationError, convertAjvErrors } from '../validator';

/**
 * Validate flag definitions against the definitions schema.
 */
export function validateDefinitions(
  ajv: Ajv,
  schema: unknown,
  filePath: string,
  data: unknown
): ValidationResult {
  // Compile schema if not already compiled
  let validate: ValidateFunction;
  try {
    // Schema is expected to be a valid JSON schema object
    validate = ajv.compile(schema as Record<string, unknown>);
  } catch (error) {
    return {
      valid: false,
      errors: [
        {
          file: filePath,
          message: `Failed to compile schema: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
  }

  // Validate data
  const valid = validate(data);

  // Convert AJV errors to our format
  const schemaErrors = valid ? [] : convertAjvErrors(filePath, validate.errors);

  // Always run additional validation for flag-specific rules (even if schema passes)
  const additionalErrors = validateFlagSpecificRules(filePath, data);

  // Combine all errors
  const allErrors = [...schemaErrors, ...additionalErrors];

  return {
    valid: allErrors.length === 0,
    errors: allErrors,
  };
}

/**
 * Type guard to check if value is a record/object.
 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

/**
 * Type guard to check if value has a name property.
 */
function hasName(value: unknown): value is { name: string } {
  return isRecord(value) && typeof value.name === 'string';
}

/**
 * Type guard to check if value is a flag definition.
 */
function isFlagDefinition(value: unknown): value is {
  name?: string;
  type?: string;
  variations?: unknown[];
} {
  return isRecord(value);
}

/**
 * Validate flag-specific business rules that aren't covered by JSON schema.
 */
function validateFlagSpecificRules(filePath: string, data: unknown): ValidationError[] {
  const errors: ValidationError[] = [];

  if (!isRecord(data)) {
    return errors;
  }

  if (!Array.isArray(data.flags)) {
    return errors;
  }

  // Check for duplicate flag names
  const flagNames = new Set<string>();
  data.flags.forEach((flag: unknown, index: number) => {
    if (hasName(flag)) {
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
    }
  });

  // Validate multivariate flags have variations
  data.flags.forEach((flag: unknown, index: number) => {
    if (isFlagDefinition(flag)) {
      if (flag.type === 'multivariate') {
        if (!Array.isArray(flag.variations) || flag.variations.length === 0) {
          const flagName = hasName(flag) ? flag.name : 'unnamed';
          errors.push({
            file: filePath,
            message: `Multivariate flag '${flagName}' must have at least one variation. Missing variations array.`,
            path: `/flags/${index}/variations`,
            suggestion: `Add a 'variations' array with at least one variation.`,
          });
        } else {
          // Check for duplicate variation names
          const variationNames = new Set<string>();
          flag.variations.forEach((variation: unknown, varIndex: number) => {
            if (hasName(variation)) {
              if (variationNames.has(variation.name)) {
                const flagName = hasName(flag) ? flag.name : 'unnamed';
                errors.push({
                  file: filePath,
                  message: `Duplicate variation name '${variation.name}' in flag '${flagName}'`,
                  path: `/flags/${index}/variations/${varIndex}/name`,
                  suggestion: `Variation names must be unique within a flag.`,
                });
              } else {
                variationNames.add(variation.name);
              }
            }
          });
        }
      }
    }
  });

  return errors;
}
