/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import Ajv, { ErrorObject } from 'ajv';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { validateDefinitions } from './validator/definitions';
import { validateDeployment } from './validator/deployment';
import { convertAjvError } from './validator/error-utils';

/**
 * Try to load ajv-formats if available (optional dependency).
 * This enables format validation for JSON schemas.
 */
function loadAjvFormats(): ((ajv: Ajv) => Ajv) | null {
  try {
    // eslint-disable-next-line @typescript-eslint/no-require-imports, @typescript-eslint/no-var-requires, @typescript-eslint/no-unsafe-assignment
    const ajvFormats: { default?: (ajv: Ajv) => Ajv; (ajv: Ajv): Ajv } = require('ajv-formats') as {
      default?: (ajv: Ajv) => Ajv;
      (ajv: Ajv): Ajv;
    };
    return ajvFormats.default || ajvFormats;
  } catch {
    return null;
  }
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
  private definitionsSchema: unknown;
  private deploymentSchema: unknown;

  /**
   * Create a new Validator instance.
   * @param embeddedSchemas Optional embedded schemas to use instead of loading from disk.
   *                        If provided, schemas will not be loaded from disk.
   *                        Format: { definitions: {...}, deployment: {...} }
   */
  constructor(embeddedSchemas?: { definitions?: unknown; deployment?: unknown }) {
    this.ajv = new Ajv({
      allErrors: true,
      verbose: true,
      strict: false,
    });

    const ajvFormats = loadAjvFormats();
    if (ajvFormats) {
      ajvFormats(this.ajv);
    }

    if (embeddedSchemas?.definitions && embeddedSchemas?.deployment) {
      this.definitionsSchema = embeddedSchemas.definitions;
      this.deploymentSchema = embeddedSchemas.deployment;
    } else {
      this.loadSchemas();
    }
  }

  private loadSchemas(): void {
    const schemasDir = this.findSchemaDirectory();
    const definitionsSchemaPath = path.join(schemasDir, 'flag-definitions.schema.v1.json');
    const deploymentSchemaPath = path.join(schemasDir, 'flag-deployment.schema.v1.json');

    this.validateSchemaFilesExist(definitionsSchemaPath, deploymentSchemaPath);
    this.definitionsSchema = this.loadSchemaFile(definitionsSchemaPath);
    this.deploymentSchema = this.loadSchemaFile(deploymentSchemaPath);
  }

  /**
   * Find the schema directory by trying multiple possible paths.
   * Handles both development and compiled builds.
   */
  private findSchemaDirectory(): string {
    const possiblePaths = this.getPossibleSchemaPaths();

    for (const possiblePath of possiblePaths) {
      const testPath = path.join(possiblePath, 'flag-definitions.schema.v1.json');
      if (fs.existsSync(testPath)) {
        return possiblePath;
      }
    }

    throw new Error(
      `Schema directory not found. Tried: ${possiblePaths.join(', ')}. ` +
        `Please ensure schemas are in the monorepo root at schemas/`
    );
  }

  /**
   * Get possible paths to the schema directory.
   * Tries paths relative to compiled output, source, package root, and current working directory.
   */
  private getPossibleSchemaPaths(): string[] {
    const currentDir = this.getCurrentDirectory();
    const cwd = this.getCurrentWorkingDirectory();

    return [
      path.resolve(currentDir, '../../../../schemas'), // From compiled output (dist/)
      path.resolve(currentDir, '../../../schemas'), // From source (src/)
      path.resolve(currentDir, '../../schemas'), // From package root
      path.resolve(cwd, 'schemas'), // Absolute path fallback
    ];
  }

  /**
   * Get the current directory (__dirname equivalent).
   * Node.js only - Deno should use embedded schemas.
   */
  private getCurrentDirectory(): string {
    // eslint-disable-next-line @typescript-eslint/no-var-requires, @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-require-imports
    const pathModule: { dirname: (path: string) => string } | undefined =
      typeof require !== 'undefined'
        ? // eslint-disable-next-line @typescript-eslint/no-var-requires
          (require('path') as { dirname: (path: string) => string })
        : undefined;
    return pathModule ? pathModule.dirname(__filename || '.') : '.';
  }

  /**
   * Get the current working directory.
   */
  private getCurrentWorkingDirectory(): string {
    return typeof process !== 'undefined' && process.cwd ? process.cwd() : '.';
  }

  /**
   * Validate that both schema files exist.
   */
  private validateSchemaFilesExist(definitionsPath: string, deploymentPath: string): void {
    if (!fs.existsSync(definitionsPath)) {
      throw new Error(`Schema file not found: ${definitionsPath}`);
    }
    if (!fs.existsSync(deploymentPath)) {
      throw new Error(`Schema file not found: ${deploymentPath}`);
    }
  }

  /**
   * Load and parse a schema file.
   */
  private loadSchemaFile(filePath: string): unknown {
    const content = fs.readFileSync(filePath, 'utf-8');
    return JSON.parse(content);
  }

  /**
   * Validate a flag definitions file.
   */
  validateDefinitions(filePath: string, data: unknown): ValidationResult {
    return validateDefinitions(this.ajv, this.definitionsSchema, filePath, data);
  }

  /**
   * Validate a deployment file.
   */
  validateDeployment(filePath: string, data: unknown): ValidationResult {
    return validateDeployment(this.ajv, this.deploymentSchema, filePath, data);
  }

  /**
   * Format validation errors for display.
   */
  formatErrors(errors: ValidationError[]): string {
    if (errors.length === 0) {
      return '';
    }

    const errorLines: string[] = ['✗ Validation failed\n'];

    for (const error of errors) {
      errorLines.push(this.formatErrorLocation(error));
      errorLines.push(`  Error: ${error.message}`);

      if (error.path) {
        errorLines.push(`  Path: ${error.path}`);
      }

      if (error.suggestion) {
        errorLines.push(`  Suggestion: ${error.suggestion}`);
      }

      errorLines.push('');
    }

    return errorLines.join('\n');
  }

  /**
   * Format error location with file path and optional line/column.
   */
  private formatErrorLocation(error: ValidationError): string {
    if (error.line !== undefined) {
      const column = error.column !== undefined ? `:${error.column}` : '';
      return `${error.file}:${error.line}${column}`;
    }
    return error.file;
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

  return ajvErrors.map((error) => convertAjvError(filePath, error));
}
