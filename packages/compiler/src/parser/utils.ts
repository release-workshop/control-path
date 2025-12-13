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
    // Check if we're in Node.js environment
    if (typeof require !== 'undefined' && typeof process !== 'undefined') {
      // Node.js
      return fs.readFileSync(filePath, 'utf-8');
    } else {
      // Check for Deno (runtime check, not compile-time)
      const global = globalThis as unknown as {
        Deno?: { readTextFileSync: (path: string) => string };
      };
      if (typeof global.Deno !== 'undefined') {
        return global.Deno.readTextFileSync(filePath);
      }
      throw new Error('Unsupported environment: neither Node.js nor Deno detected');
    }
  } catch (error) {
    const err = error instanceof Error ? error : new Error(String(error));
    if ('code' in err && (err as { code?: string }).code === 'ENOENT') {
      throw new ParseError(`File not found: ${filePath}`, filePath, err);
    }
    throw new ParseError(`Failed to read file: ${err.message}`, filePath, err);
  }
}

/**
 * Parse YAML or JSON content from a string.
 * Automatically detects format based on file extension or content.
 */
export function parseYamlOrJson(content: string, filePath: string): unknown {
  try {
    // Try to detect format from file extension
    const ext = path.extname(filePath).toLowerCase();

    if (ext === '.json') {
      return JSON.parse(content);
    } else if (ext === '.yaml' || ext === '.yml') {
      return yaml.load(content, {
        filename: filePath,
        schema: yaml.DEFAULT_SCHEMA,
      });
    } else {
      // Try to parse as JSON first (more strict)
      try {
        return JSON.parse(content);
      } catch {
        // If JSON parsing fails, try YAML
        return yaml.load(content, {
          filename: filePath,
          schema: yaml.DEFAULT_SCHEMA,
        });
      }
    }
  } catch (error) {
    const err = error instanceof Error ? error : new Error(String(error));

    // Provide better error messages for common issues
    if (err.name === 'YAMLException') {
      const yamlErr = err as yaml.YAMLException;
      const message = yamlErr.reason || err.message;
      const line = yamlErr.mark?.line !== undefined ? ` at line ${yamlErr.mark.line + 1}` : '';
      const column =
        yamlErr.mark?.column !== undefined ? `, column ${yamlErr.mark.column + 1}` : '';
      throw new ParseError(`YAML parse error${line}${column}: ${message}`, filePath, err);
    }

    if (err instanceof SyntaxError) {
      throw new ParseError(`JSON parse error: ${err.message}`, filePath, err);
    }

    throw new ParseError(`Parse error: ${err.message}`, filePath, err);
  }
}
