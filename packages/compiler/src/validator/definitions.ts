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
  schema: any,
  filePath: string,
  data: any
): ValidationResult {
  // Compile schema if not already compiled
  let validate: ValidateFunction;
  try {
    validate = ajv.compile(schema);
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
 * Validate flag-specific business rules that aren't covered by JSON schema.
 */
function validateFlagSpecificRules(filePath: string, data: any): ValidationError[] {
  const errors: ValidationError[] = [];

  if (!data || typeof data !== 'object') {
    return errors;
  }

  if (!Array.isArray(data.flags)) {
    return errors;
  }

  // Check for duplicate flag names
  const flagNames = new Set<string>();
  data.flags.forEach((flag: any, index: number) => {
    if (flag && typeof flag === 'object' && flag.name) {
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
  data.flags.forEach((flag: any, index: number) => {
    if (flag && typeof flag === 'object') {
      if (flag.type === 'multivariate') {
        if (!Array.isArray(flag.variations) || flag.variations.length === 0) {
          errors.push({
            file: filePath,
            message: `Multivariate flag '${flag.name || 'unnamed'}' must have at least one variation. Missing variations array.`,
            path: `/flags/${index}/variations`,
            suggestion: `Add a 'variations' array with at least one variation.`,
          });
        } else {
          // Check for duplicate variation names
          const variationNames = new Set<string>();
          flag.variations.forEach((variation: any, varIndex: number) => {
            if (variation && typeof variation === 'object' && variation.name) {
              if (variationNames.has(variation.name)) {
                errors.push({
                  file: filePath,
                  message: `Duplicate variation name '${variation.name}' in flag '${flag.name || 'unnamed'}'`,
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

