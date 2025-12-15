/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import Ajv, { ValidateFunction } from 'ajv';
import { ValidationError } from '../validator';

/**
 * Compile a JSON schema for validation.
 * @param ajv - AJV instance
 * @param schema - JSON schema object
 * @param filePath - File path for error messages
 * @returns Compiled validate function
 * @throws Error if schema compilation fails
 */
export function compileSchema(ajv: Ajv, schema: unknown, filePath: string): ValidateFunction {
  try {
    return ajv.compile(schema as Record<string, unknown>);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to compile schema for ${filePath}: ${errorMessage}`);
  }
}

/**
 * Validate data against a compiled schema and return validation errors.
 * @param validate - Compiled validate function
 * @param data - Data to validate
 * @param filePath - File path for error messages
 * @returns Array of validation errors (empty if valid)
 */
export function validateAgainstSchema(
  validate: ValidateFunction,
  data: unknown,
  _filePath: string
): ValidationError[] {
  const isValid = validate(data);
  if (isValid) {
    return [];
  }
  // Errors will be converted by convertAjvErrors in the calling function
  return [];
}
