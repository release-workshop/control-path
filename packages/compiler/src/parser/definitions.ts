/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { FlagDefinitions, ParseError } from './types';
import { readFile, parseYamlOrJson } from './utils';

/**
 * Parse flag definitions from a file.
 * Supports both YAML and JSON formats.
 *
 * @param filePath - Path to the flag definitions file
 * @returns Parsed flag definitions
 * @throws {ParseError} If file cannot be read or parsed
 */
export function parseDefinitions(filePath: string): FlagDefinitions {
  const content = readFile(filePath);
  return parseDefinitionsFromString(content, filePath);
}

/**
 * Parse flag definitions from a string.
 * Supports both YAML and JSON formats.
 *
 * @param content - File content as string
 * @param filePath - Original file path (for error messages)
 * @returns Parsed flag definitions
 * @throws {ParseError} If content cannot be parsed
 */
export function parseDefinitionsFromString(content: string, filePath: string): FlagDefinitions {
  const parsedData = parseYamlOrJson(content, filePath);

  if (!parsedData || typeof parsedData !== 'object' || Array.isArray(parsedData)) {
    throw new ParseError('Invalid flag definitions: expected an object', filePath);
  }

  const definitionsObject = parsedData as Record<string, unknown>;

  if (!('flags' in definitionsObject)) {
    throw new ParseError('Invalid flag definitions: missing required field "flags"', filePath);
  }

  if (!Array.isArray(definitionsObject.flags)) {
    throw new ParseError('Invalid flag definitions: "flags" must be an array', filePath);
  }

  // Type assertion - full validation should be done by schema validator
  return definitionsObject as unknown as FlagDefinitions;
}
