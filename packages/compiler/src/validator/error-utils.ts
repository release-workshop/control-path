/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { ErrorObject } from 'ajv';
import { ValidationError } from '../validator';

/**
 * Extract line and column information from AJV error params.
 */
export interface ErrorLocation {
  line?: number;
  column?: number;
}

/**
 * Extract line and column from AJV error params.
 */
export function extractErrorLocation(error: ErrorObject): ErrorLocation {
  const location: ErrorLocation = {};

  if (error.params) {
    if (typeof error.params.line === 'number') {
      location.line = error.params.line;
    }
    if (typeof error.params.column === 'number') {
      location.column = error.params.column;
    }
  }

  return location;
}

/**
 * Generate a helpful suggestion based on AJV error keyword.
 */
export function generateSuggestion(error: ErrorObject): string | undefined {
  if (error.keyword === 'required') {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access
    const missing = error.params?.missingProperty;
    if (typeof missing === 'string') {
      return `Add missing required field '${missing}'`;
    }
  }

  if (error.keyword === 'type') {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access
    const expected = error.params?.type;
    if (typeof expected === 'string') {
      return `Expected type '${expected}'`;
    }
  }

  if (error.keyword === 'enum') {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access
    const allowed = error.params?.allowedValues;
    if (Array.isArray(allowed)) {
      const allowedStrings = allowed.map((v) => String(v));
      return `Allowed values: ${allowedStrings.join(', ')}`;
    }
  }

  return undefined;
}

/**
 * Convert a single AJV error to ValidationError format.
 */
export function convertAjvError(filePath: string, error: ErrorObject): ValidationError {
  const instancePath = error.instancePath || error.schemaPath || '';
  const message = error.message || 'Validation error';
  const location = extractErrorLocation(error);
  const suggestion = generateSuggestion(error);

  return {
    file: filePath,
    line: location.line,
    column: location.column,
    message,
    path: instancePath || undefined,
    suggestion,
  };
}
