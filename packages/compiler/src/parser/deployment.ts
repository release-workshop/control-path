/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { Deployment } from './types';
import { ParseError } from './parse-error';
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
  const parsedData = parseYamlOrJson(content, filePath);

  if (!parsedData || typeof parsedData !== 'object' || Array.isArray(parsedData)) {
    throw new ParseError('Invalid deployment: expected an object', filePath);
  }

  const deploymentObject = parsedData as Record<string, unknown>;

  if (!('environment' in deploymentObject)) {
    throw new ParseError('Invalid deployment: missing required field "environment"', filePath);
  }

  if (typeof deploymentObject.environment !== 'string') {
    throw new ParseError('Invalid deployment: "environment" must be a string', filePath);
  }

  if (!('rules' in deploymentObject)) {
    throw new ParseError('Invalid deployment: missing required field "rules"', filePath);
  }

  if (
    !deploymentObject.rules ||
    typeof deploymentObject.rules !== 'object' ||
    Array.isArray(deploymentObject.rules)
  ) {
    throw new ParseError('Invalid deployment: "rules" must be an object', filePath);
  }

  // Type assertion - full validation should be done by schema validator
  return deploymentObject as unknown as Deployment;
}
