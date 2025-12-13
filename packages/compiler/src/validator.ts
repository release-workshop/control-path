/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import Ajv, { ValidateFunction, ErrorObject } from 'ajv';
// Use node: prefix for Deno compatibility (also works in Node.js)
// These are only used in loadSchemas() which is skipped when embedded schemas are provided
import * as fs from 'node:fs';
import * as path from 'node:path';
import { validateDefinitions } from './validator/definitions';
import { validateDeployment } from './validator/deployment';

// Try to load ajv-formats if available (optional dependency)
let addFormats: ((ajv: Ajv) => Ajv) | null = null;
try {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const ajvFormats = require('ajv-formats');
  addFormats = ajvFormats.default || ajvFormats;
} catch {
  // ajv-formats not available, skip format validation
}

export interface ValidationError {
  file: string;
  line?: number;
  column?: number;
  message: string;
  path?: string;
  suggestion?: string;
}

export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
}

/**
 * Main validator for Control Path configuration files.
 * Validates flag definitions and deployment files against JSON schemas.
 */
export class Validator {
  private ajv: Ajv;
  private definitionsSchema: any;
  private deploymentSchema: any;

  /**
   * Create a new Validator instance.
   * @param embeddedSchemas Optional embedded schemas to use instead of loading from disk.
   *                        If provided, schemas will not be loaded from disk.
   *                        Format: { definitions: {...}, deployment: {...} }
   */
  constructor(embeddedSchemas?: { definitions?: any; deployment?: any }) {
    this.ajv = new Ajv({
      allErrors: true,
      verbose: true,
      strict: false,
    });
    // Add format validation if ajv-formats is available
    if (addFormats) {
      addFormats(this.ajv);
    }

    // Load schemas (from embedded or disk)
    if (embeddedSchemas?.definitions && embeddedSchemas?.deployment) {
      this.definitionsSchema = embeddedSchemas.definitions;
      this.deploymentSchema = embeddedSchemas.deployment;
    } else {
      this.loadSchemas();
    }
  }

  private loadSchemas(): void {
    // Load schemas from the schemas directory in the monorepo root
    // Try multiple possible paths to handle both development and compiled builds
    // Note: This method is only called when embedded schemas are not provided
    // In Deno/CLI context, embedded schemas should always be provided
    
    // Get __dirname equivalent (Node.js only - Deno should use embedded schemas)
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const __dirname = typeof require !== 'undefined' && require('path') 
      ? require('path').dirname(__filename || '.')
      : '.';
    
    const cwd = typeof process !== 'undefined' && process.cwd
      ? process.cwd()
      : '.';
    
    const possiblePaths = [
      // From compiled output (dist/)
      path.resolve(__dirname, '../../../../schemas'),
      // From source (src/)
      path.resolve(__dirname, '../../../schemas'),
      // From package root
      path.resolve(__dirname, '../../schemas'),
      // Absolute path fallback
      path.resolve(cwd, 'schemas'),
    ];

    let schemasDir: string | null = null;
    for (const possiblePath of possiblePaths) {
      const testPath = path.join(possiblePath, 'flag-definitions.schema.v1.json');
      if (fs.existsSync(testPath)) {
        schemasDir = possiblePath;
        break;
      }
    }

    if (!schemasDir) {
      throw new Error(
        `Schema directory not found. Tried: ${possiblePaths.join(', ')}. ` +
          `Please ensure schemas are in the monorepo root at schemas/`
      );
    }

    const definitionsSchemaPath = path.join(schemasDir, 'flag-definitions.schema.v1.json');
    const deploymentSchemaPath = path.join(schemasDir, 'flag-deployment.schema.v1.json');

    if (!fs.existsSync(definitionsSchemaPath)) {
      throw new Error(`Schema file not found: ${definitionsSchemaPath}`);
    }
    if (!fs.existsSync(deploymentSchemaPath)) {
      throw new Error(`Schema file not found: ${deploymentSchemaPath}`);
    }

    this.definitionsSchema = JSON.parse(fs.readFileSync(definitionsSchemaPath, 'utf-8'));
    this.deploymentSchema = JSON.parse(fs.readFileSync(deploymentSchemaPath, 'utf-8'));
  }

  /**
   * Validate a flag definitions file.
   */
  validateDefinitions(filePath: string, data: any): ValidationResult {
    return validateDefinitions(this.ajv, this.definitionsSchema, filePath, data);
  }

  /**
   * Validate a deployment file.
   */
  validateDeployment(filePath: string, data: any): ValidationResult {
    return validateDeployment(this.ajv, this.deploymentSchema, filePath, data);
  }

  /**
   * Format validation errors for display.
   */
  formatErrors(errors: ValidationError[]): string {
    if (errors.length === 0) {
      return '';
    }

    const lines: string[] = [];
    lines.push('✗ Validation failed\n');

    for (const error of errors) {
      const location =
        error.line !== undefined
          ? `${error.file}:${error.line}${error.column !== undefined ? `:${error.column}` : ''}`
          : error.file;

      lines.push(location);
      lines.push(`  Error: ${error.message}`);

      if (error.path) {
        lines.push(`  Path: ${error.path}`);
      }

      if (error.suggestion) {
        lines.push(`  Suggestion: ${error.suggestion}`);
      }

      lines.push('');
    }

    return lines.join('\n');
  }
}

/**
 * Convert AJV error objects to ValidationError format.
 */
export function convertAjvErrors(
  filePath: string,
  ajvErrors: ErrorObject[] | null | undefined
): ValidationError[] {
  if (!ajvErrors || ajvErrors.length === 0) {
    return [];
  }

  return ajvErrors.map((error) => {
    const instancePath = error.instancePath || error.schemaPath || '';
    const message = error.message || 'Validation error';

    // Try to extract line/column from error params if available
    let line: number | undefined;
    let column: number | undefined;

    if (error.params) {
      // AJV sometimes includes line/column in params
      if (typeof error.params.line === 'number') {
        line = error.params.line;
      }
      if (typeof error.params.column === 'number') {
        column = error.params.column;
      }
    }

    // Generate suggestion based on error type
    let suggestion: string | undefined;
    if (error.keyword === 'required') {
      const missing = error.params?.missingProperty;
      if (missing) {
        suggestion = `Add missing required field '${missing}'`;
      }
    } else if (error.keyword === 'type') {
      const expected = error.params?.type;
      const actual = error.params?.type;
      if (expected) {
        suggestion = `Expected type '${expected}'`;
      }
    } else if (error.keyword === 'enum') {
      const allowed = error.params?.allowedValues;
      if (allowed && Array.isArray(allowed)) {
        suggestion = `Allowed values: ${allowed.join(', ')}`;
      }
    }

    return {
      file: filePath,
      line,
      column,
      message,
      path: instancePath || undefined,
      suggestion,
    };
  });
}
