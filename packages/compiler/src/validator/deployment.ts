/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import Ajv, { ValidateFunction } from 'ajv';
import { ValidationResult, ValidationError, convertAjvErrors } from '../validator';

/**
 * Validate deployment file against the deployment schema.
 */
export function validateDeployment(
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

  // Always run additional validation for deployment-specific rules (even if schema passes)
  const additionalErrors = validateDeploymentSpecificRules(filePath, data);

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
 * Type guard to check if value is a variation object.
 */
function isVariation(value: unknown): value is { weight?: number } {
  return isRecord(value);
}

/**
 * Type guard to check if value is a rollout object.
 */
function isRollout(value: unknown): value is { percentage?: number } {
  return isRecord(value);
}

/**
 * Validate deployment-specific business rules that aren't covered by JSON schema.
 */
function validateDeploymentSpecificRules(filePath: string, data: unknown): ValidationError[] {
  const errors: ValidationError[] = [];

  if (!isRecord(data)) {
    return errors;
  }

  if (!isRecord(data.rules)) {
    return errors;
  }

  // Validate rule structure
  for (const [flagName, flagRules] of Object.entries(data.rules)) {
    if (!isRecord(flagRules)) {
      continue;
    }

    const rules = flagRules.rules;
    if (Array.isArray(rules)) {
      rules.forEach((rule: unknown, ruleIndex: number) => {
        if (!isRecord(rule)) {
          return;
        }

        // Validate that rule has at least one of: serve, variations, rollout
        const hasServe = 'serve' in rule;
        const hasVariations = 'variations' in rule && Array.isArray(rule.variations);
        const hasRollout = 'rollout' in rule && isRollout(rule.rollout);

        if (!hasServe && !hasVariations && !hasRollout) {
          errors.push({
            file: filePath,
            message: `Rule in flag '${flagName}' must have 'serve', 'variations', or 'rollout'`,
            path: `/rules/${flagName}/rules/${ruleIndex}`,
            suggestion: `Add 'serve', 'variations', or 'rollout' to this rule.`,
          });
        }

        // Validate variations array if present
        if (hasVariations && Array.isArray(rule.variations)) {
          const totalWeight = rule.variations.reduce(
            (sum: number, v: unknown) =>
              sum + (isVariation(v) && typeof v.weight === 'number' ? v.weight : 0),
            0
          );

          if (totalWeight > 100) {
            errors.push({
              file: filePath,
              message: `Variation weights for flag '${flagName}' exceed 100% (total: ${totalWeight}%)`,
              path: `/rules/${flagName}/rules/${ruleIndex}/variations`,
              suggestion: `Adjust weights so they sum to 100% or less.`,
            });
          }
        }

        // Validate rollout if present
        if (hasRollout && rule.rollout && isRollout(rule.rollout)) {
          const percentage = rule.rollout.percentage;
          if (typeof percentage === 'number' && (percentage < 0 || percentage > 100)) {
            errors.push({
              file: filePath,
              message: `Rollout percentage for flag '${flagName}' must be between 0 and 100`,
              path: `/rules/${flagName}/rules/${ruleIndex}/rollout/percentage`,
              suggestion: `Set percentage between 0 and 100.`,
            });
          }
        }
      });
    }
  }

  return errors;
}
