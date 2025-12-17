/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

interface InitOptions {
  force?: boolean;
  exampleFlags?: boolean;
  noExamples?: boolean;
}

/**
 * Check if a file or directory exists.
 */
async function fileExists(path: string): Promise<boolean> {
  try {
    await Deno.stat(path);
    return true;
  } catch {
    return false;
  }
}

/**
 * Check if project files already exist.
 */
async function checkExistingProject(): Promise<boolean> {
  const hasDefinitionsFile = await fileExists('flags.definitions.yaml');
  const hasControlpathDir = await fileExists('.controlpath');
  return hasDefinitionsFile || hasControlpathDir;
}

/**
 * Create the .controlpath directory if it doesn't exist.
 */
async function ensureControlpathDirectory(): Promise<void> {
  try {
    await Deno.mkdir('.controlpath', { recursive: true });
  } catch {
    // Directory might already exist, ignore
  }
}

/**
 * Create the flags.definitions.yaml file with example content.
 */
async function createDefinitionsFile(): Promise<void> {
  const definitionsContent = `flags:
  - name: example_flag
    type: boolean
    defaultValue: false
    description: An example feature flag
`;
  await Deno.writeTextFile('flags.definitions.yaml', definitionsContent);
}

/**
 * Create the production deployment file.
 */
async function createDeploymentFile(): Promise<void> {
  const deploymentContent = `environment: production
rules:
  example_flag:
    rules:
      - serve: false
`;
  await Deno.writeTextFile('.controlpath/production.deployment.yaml', deploymentContent);
}

/**
 * Display success message and next steps.
 */
function displaySuccessMessage(createdDefinitions: boolean): void {
  console.log('✓ Project initialized');
  if (createdDefinitions) {
    console.log('  Created flags.definitions.yaml');
  }
  console.log('  Created .controlpath/production.deployment.yaml');
  console.log('');
  console.log('Next steps:');
  console.log('  1. Validate your files: controlpath validate');
  console.log('  2. Compile AST: controlpath compile --env production');
  console.log('  3. Add more flags: Edit flags.definitions.yaml');
}

/**
 * Display error message for initialization failure.
 */
function displayErrorMessage(error: unknown): void {
  console.error('✗ Initialization failed');
  console.error(`  Error: ${error instanceof Error ? error.message : String(error)}`);
}

/**
 * Initialize a new Control Path project.
 */
export async function initCommand(options: InitOptions): Promise<number> {
  const hasExistingFiles = await checkExistingProject();

  if (hasExistingFiles && !options.force) {
    console.error('✗ Initialization failed');
    console.error('  Error: Project already initialized');
    console.error('  Use --force to overwrite existing files');
    return 1;
  }

  try {
    await ensureControlpathDirectory();

    const createDefinitions = !options.noExamples || options.exampleFlags;
    if (createDefinitions) {
      await createDefinitionsFile();
    }

    await createDeploymentFile();

    displaySuccessMessage(createDefinitions === true);
    return 0;
  } catch (error) {
    displayErrorMessage(error);
    return 1;
  }
}
