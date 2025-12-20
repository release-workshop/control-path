/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { type ValidationResult, Validator } from '../../../compiler/src/validator.ts';
import { parseDefinitions, parseDeployment } from '../../../compiler/src/parser/index.ts';
import definitionsSchema from '../../../compiler/src/schemas/flag-definitions.schema.v1.json' with {
  type: 'json',
};
import deploymentSchema from '../../../compiler/src/schemas/flag-deployment.schema.v1.json' with {
  type: 'json',
};

interface ValidateOptions {
  definitions?: string;
  deployment?: string;
  env?: string;
  all?: boolean;
}

type FileToValidate = { type: 'definitions' | 'deployment'; path: string };
type ValidationFileResult = { file: string; result: ValidationResult };

/**
 * Create a validator instance with embedded schemas.
 */
function createValidator(): Validator {
  return new Validator({
    definitions: definitionsSchema,
    deployment: deploymentSchema,
  });
}

/**
 * Collect files to validate from command-line options.
 */
function collectFilesFromOptions(options: ValidateOptions): FileToValidate[] {
  const files: FileToValidate[] = [];

  if (options.definitions) {
    files.push({ type: 'definitions', path: options.definitions });
  }

  if (options.deployment) {
    files.push({ type: 'deployment', path: options.deployment });
  }

  if (options.env) {
    const deploymentPath = `.controlpath/${options.env}.deployment.yaml`;
    files.push({ type: 'deployment', path: deploymentPath });
  }

  return files;
}

/**
 * Check if flags.definitions.yaml exists and add it to the list.
 */
async function findDefinitionsFile(files: FileToValidate[]): Promise<void> {
  try {
    await Deno.stat('flags.definitions.yaml');
    files.push({ type: 'definitions', path: 'flags.definitions.yaml' });
  } catch {
    // File doesn't exist, skip
  }
}

/**
 * Find all deployment files in .controlpath directory.
 */
async function findDeploymentFiles(files: FileToValidate[]): Promise<void> {
  try {
    const controlpathDir = await Deno.readDir('.controlpath');
    for await (const entry of controlpathDir) {
      if (entry.isFile && entry.name.endsWith('.deployment.yaml')) {
        files.push({
          type: 'deployment',
          path: `.controlpath/${entry.name}`,
        });
      }
    }
  } catch {
    // .controlpath directory doesn't exist, skip
  }
}

/**
 * Auto-detect files to validate.
 */
async function autoDetectFiles(): Promise<FileToValidate[]> {
  const files: FileToValidate[] = [];
  await findDefinitionsFile(files);
  await findDeploymentFiles(files);
  return files;
}

/**
 * Validate a single file and return the result.
 */
function validateFile(validator: Validator, file: FileToValidate): ValidationFileResult | null {
  try {
    let result: ValidationResult;

    if (file.type === 'definitions') {
      const data = parseDefinitions(file.path);
      result = validator.validateDefinitions(file.path, data);
    } else {
      const data = parseDeployment(file.path);
      result = validator.validateDeployment(file.path, data);
    }

    return { file: file.path, result };
  } catch (error) {
    console.error(`✗ Failed to validate ${file.path}`);
    console.error(`  Error: ${error instanceof Error ? error.message : String(error)}`);
    return null;
  }
}

/**
 * Display error message when no files are found to validate.
 */
function displayNoFilesError(): void {
  console.error('✗ No files to validate');
  console.error('  Use --definitions <file> or --deployment <file> to specify files');
  console.error(
    '  Or run in a directory with flags.definitions.yaml or .controlpath/*.deployment.yaml',
  );
}

/**
 * Display validation success message.
 */
function displaySuccessMessage(validCount: number): void {
  console.log(`✓ Validation passed (${validCount} file${validCount > 1 ? 's' : ''})`);
}

/**
 * Display validation errors for all invalid files.
 */
function displayValidationErrors(validator: Validator, results: ValidationFileResult[]): void {
  for (const { result } of results) {
    if (!result.valid && result.errors.length > 0) {
      console.error(validator.formatErrors(result.errors));
    }
  }
}

/**
 * Validate flag definitions and/or deployment files.
 */
export async function validateCommand(options: ValidateOptions): Promise<number> {
  const validator = createValidator();

  // Collect files to validate
  let filesToValidate = collectFilesFromOptions(options);

  // Auto-detect if no flags provided or --all flag is used
  if (filesToValidate.length === 0 || options.all) {
    const autoDetectedFiles = await autoDetectFiles();
    filesToValidate = [...filesToValidate, ...autoDetectedFiles];
  }

  if (filesToValidate.length === 0) {
    displayNoFilesError();
    return 1;
  }

  // Validate each file
  const results: ValidationFileResult[] = [];
  let hasErrors = false;

  for (const file of filesToValidate) {
    const result = validateFile(validator, file);
    if (result) {
      results.push(result);
      if (!result.result.valid) {
        hasErrors = true;
      }
    } else {
      hasErrors = true;
    }
  }

  // Print results
  const validCount = results.filter((r) => r.result.valid).length;
  const invalidCount = results.filter((r) => !r.result.valid).length;

  if (validCount > 0 && invalidCount === 0) {
    displaySuccessMessage(validCount);
    return 0;
  }

  displayValidationErrors(validator, results);

  return hasErrors ? 1 : 0;
}
