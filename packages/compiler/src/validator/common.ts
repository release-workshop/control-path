/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import Ajv, { ValidateFunction } from 'ajv';
import { ValidationResult, ValidationError, convertAjvErrors } from '../validator';
import { compileSchema } from './schema-utils';

/**
 * Common validation pattern for both definitions and deployment files.
 * Compiles schema, validates data, and combines schema errors with additional validation errors.
 */
export function validateWithSchema(
  ajv: Ajv,
  schema: unknown,
  filePath: string,
  data: unknown,
  additionalValidation: (filePath: string, data: unknown) => ValidationError[]
): ValidationResult {
  let validate: ValidateFunction;
  try {
    validate = compileSchema(ajv, schema, filePath);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    return {
      valid: false,
      errors: [
        {
          file: filePath,
          message: `Failed to compile schema: ${errorMessage}`,
        },
      ],
    };
  }

  const isValid = validate(data);
  const schemaErrors = isValid ? [] : convertAjvErrors(filePath, validate.errors);
  const additionalErrors = additionalValidation(filePath, data);
  const allErrors = [...schemaErrors, ...additionalErrors];

  return {
    valid: allErrors.length === 0,
    errors: allErrors,
  };
}
