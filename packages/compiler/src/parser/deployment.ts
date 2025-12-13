/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { Deployment, ParseError } from './types';
import { readFile, parseYamlOrJson } from './utils';

/**
 * Parse deployment from a file.
 * Supports both YAML and JSON formats.
 *
 * @param filePath - Path to the deployment file
 * @returns Parsed deployment
 * @throws {ParseError} If file cannot be read or parsed
 */
export function parseDeployment(filePath: string): Deployment {
  const content = readFile(filePath);
  return parseDeploymentFromString(content, filePath);
}

/**
 * Parse deployment from a string.
 * Supports both YAML and JSON formats.
 *
 * @param content - File content as string
 * @param filePath - Original file path (for error messages)
 * @returns Parsed deployment
 * @throws {ParseError} If content cannot be parsed
 */
export function parseDeploymentFromString(content: string, filePath: string): Deployment {
  const parsed = parseYamlOrJson(content, filePath);

  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new ParseError('Invalid deployment: expected an object', filePath);
  }

  const obj = parsed as Record<string, unknown>;

  // Validate required fields
  if (!('environment' in obj)) {
    throw new ParseError('Invalid deployment: missing required field "environment"', filePath);
  }

  if (typeof obj.environment !== 'string') {
    throw new ParseError('Invalid deployment: "environment" must be a string', filePath);
  }

  if (!('rules' in obj)) {
    throw new ParseError('Invalid deployment: missing required field "rules"', filePath);
  }

  if (!obj.rules || typeof obj.rules !== 'object' || Array.isArray(obj.rules)) {
    throw new ParseError('Invalid deployment: "rules" must be an object', filePath);
  }

  // Type assertion - validation should be done by schema validator
  return obj as unknown as Deployment;
}
