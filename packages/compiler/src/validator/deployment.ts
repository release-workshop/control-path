/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import Ajv from 'ajv';
import { ValidationResult, ValidationError } from '../validator';
import { validateWithSchema } from './common';
import { isRecord, isVariation, isRollout, isDeployment } from './type-guards';
import { MAX_PERCENTAGE, MIN_PERCENTAGE } from './constants';

/**
 * Validate deployment file against the deployment schema.
 */
export function validateDeployment(
  ajv: Ajv,
  schema: unknown,
  filePath: string,
  data: unknown
): ValidationResult {
  return validateWithSchema(ajv, schema, filePath, data, validateDeploymentSpecificRules);
}

/**
 * Validate deployment-specific business rules that aren't covered by JSON schema.
 */
function validateDeploymentSpecificRules(filePath: string, data: unknown): ValidationError[] {
  if (!isDeployment(data)) {
    return [];
  }

  const errors: ValidationError[] = [];

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

        errors.push(...validateRuleStructure(filePath, flagName, rule, ruleIndex));
        errors.push(...validateVariationWeights(filePath, flagName, rule, ruleIndex));
        errors.push(...validateRolloutPercentage(filePath, flagName, rule, ruleIndex));
      });
    }
  }

  return errors;
}

/**
 * Validate that a rule has at least one of: serve, variations, or rollout.
 */
function validateRuleStructure(
  filePath: string,
  flagName: string,
  rule: Record<string, unknown>,
  ruleIndex: number
): ValidationError[] {
  const hasServe = 'serve' in rule;
  const hasVariations = 'variations' in rule && Array.isArray(rule.variations);
  const hasRollout = 'rollout' in rule && isRollout(rule.rollout);

  if (!hasServe && !hasVariations && !hasRollout) {
    return [
      {
        file: filePath,
        message: `Rule in flag '${flagName}' must have 'serve', 'variations', or 'rollout'`,
        path: `/rules/${flagName}/rules/${ruleIndex}`,
        suggestion: `Add 'serve', 'variations', or 'rollout' to this rule.`,
      },
    ];
  }

  return [];
}

/**
 * Validate that variation weights don't exceed 100%.
 */
function validateVariationWeights(
  filePath: string,
  flagName: string,
  rule: Record<string, unknown>,
  ruleIndex: number
): ValidationError[] {
  if (!('variations' in rule) || !Array.isArray(rule.variations)) {
    return [];
  }

  const totalWeight = rule.variations.reduce(
    (sum: number, variation: unknown) =>
      sum + (isVariation(variation) && typeof variation.weight === 'number' ? variation.weight : 0),
    0
  );

  if (totalWeight > MAX_PERCENTAGE) {
    return [
      {
        file: filePath,
        message: `Variation weights for flag '${flagName}' exceed ${MAX_PERCENTAGE}% (total: ${totalWeight}%)`,
        path: `/rules/${flagName}/rules/${ruleIndex}/variations`,
        suggestion: `Adjust weights so they sum to ${MAX_PERCENTAGE}% or less.`,
      },
    ];
  }

  return [];
}

/**
 * Validate that rollout percentage is between 0 and 100.
 */
function validateRolloutPercentage(
  filePath: string,
  flagName: string,
  rule: Record<string, unknown>,
  ruleIndex: number
): ValidationError[] {
  if (!('rollout' in rule) || !isRollout(rule.rollout)) {
    return [];
  }

  const percentage = rule.rollout.percentage;
  if (
    typeof percentage === 'number' &&
    (percentage < MIN_PERCENTAGE || percentage > MAX_PERCENTAGE)
  ) {
    return [
      {
        file: filePath,
        message: `Rollout percentage for flag '${flagName}' must be between ${MIN_PERCENTAGE} and ${MAX_PERCENTAGE}`,
        path: `/rules/${flagName}/rules/${ruleIndex}/rollout/percentage`,
        suggestion: `Set percentage between ${MIN_PERCENTAGE} and ${MAX_PERCENTAGE}.`,
      },
    ];
  }

  return [];
}
