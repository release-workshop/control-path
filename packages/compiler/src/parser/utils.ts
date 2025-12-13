/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import yaml from 'js-yaml';
import { ParseError } from './types';

/**
 * Read file contents from the file system.
 * Works in both Node.js and Deno environments.
 */
export function readFile(filePath: string): string {
  try {
    if (isNodeJsEnvironment()) {
      return fs.readFileSync(filePath, 'utf-8');
    }

    if (isDenoEnvironment()) {
      return getDenoReadTextFileSync()(filePath);
    }

    throw new Error('Unsupported environment: neither Node.js nor Deno detected');
  } catch (error) {
    const fileError = error instanceof Error ? error : new Error(String(error));
    if (isFileNotFoundError(fileError)) {
      throw new ParseError(`File not found: ${filePath}`, filePath, fileError);
    }
    throw new ParseError(`Failed to read file: ${fileError.message}`, filePath, fileError);
  }
}

/**
 * Check if running in Node.js environment.
 */
function isNodeJsEnvironment(): boolean {
  return typeof require !== 'undefined' && typeof process !== 'undefined';
}

/**
 * Check if running in Deno environment.
 */
function isDenoEnvironment(): boolean {
  const global = globalThis as unknown as {
    Deno?: { readTextFileSync: (path: string) => string };
  };
  return typeof global.Deno !== 'undefined';
}

/**
 * Get Deno's readTextFileSync function.
 */
function getDenoReadTextFileSync(): (path: string) => string {
  const global = globalThis as unknown as {
    Deno: { readTextFileSync: (path: string) => string };
  };
  return global.Deno.readTextFileSync;
}

/**
 * Check if error is a file not found error.
 */
function isFileNotFoundError(error: Error): boolean {
  return 'code' in error && (error as { code?: string }).code === 'ENOENT';
}

/**
 * Parse YAML or JSON content from a string.
 * Automatically detects format based on file extension or content.
 */
export function parseYamlOrJson(content: string, filePath: string): unknown {
  try {
    const fileExtension = path.extname(filePath).toLowerCase();

    if (fileExtension === '.json') {
      return JSON.parse(content);
    }

    if (fileExtension === '.yaml' || fileExtension === '.yml') {
      return parseYaml(content, filePath);
    }

    // Unknown extension: try JSON first (more strict), then YAML
    return tryParseAsJsonThenYaml(content, filePath);
  } catch (error) {
    throw formatParseError(error, filePath);
  }
}

/**
 * Parse YAML content.
 */
function parseYaml(content: string, filePath: string): unknown {
  return yaml.load(content, {
    filename: filePath,
    schema: yaml.DEFAULT_SCHEMA,
  });
}

/**
 * Try parsing as JSON first, then fall back to YAML.
 */
function tryParseAsJsonThenYaml(content: string, filePath: string): unknown {
  try {
    return JSON.parse(content);
  } catch {
    return parseYaml(content, filePath);
  }
}

/**
 * Format parse errors with helpful messages.
 */
function formatParseError(error: unknown, filePath: string): ParseError {
  const parseError = error instanceof Error ? error : new Error(String(error));

  if (isYamlException(parseError)) {
    return formatYamlError(parseError, filePath);
  }

  if (parseError instanceof SyntaxError) {
    return new ParseError(`JSON parse error: ${parseError.message}`, filePath, parseError);
  }

  return new ParseError(`Parse error: ${parseError.message}`, filePath, parseError);
}

/**
 * Check if error is a YAML exception.
 */
function isYamlException(error: Error): error is yaml.YAMLException {
  return error.name === 'YAMLException';
}

/**
 * Format YAML error with line and column information.
 */
function formatYamlError(error: yaml.YAMLException, filePath: string): ParseError {
  const message = error.reason || error.message;
  const lineInfo = formatLineInfo(error.mark);
  const columnInfo = formatColumnInfo(error.mark);
  const locationInfo = lineInfo + columnInfo;

  return new ParseError(`YAML parse error${locationInfo}: ${message}`, filePath, error);
}

/**
 * Format line information from YAML mark.
 */
function formatLineInfo(mark?: yaml.Mark): string {
  return mark?.line !== undefined ? ` at line ${mark.line + 1}` : '';
}

/**
 * Format column information from YAML mark.
 */
function formatColumnInfo(mark?: yaml.Mark): string {
  return mark?.column !== undefined ? `, column ${mark.column + 1}` : '';
}
