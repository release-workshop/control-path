/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { Validator } from '../../../compiler/src/validator.ts';
import { parseDefinitions, parseDeployment } from '../../../compiler/src/parser/index.ts';
import { compileAndSerialize } from '../../../compiler/src/compiler/index.ts';
import definitionsSchema from '../../../compiler/src/schemas/flag-definitions.schema.v1.json' with {
  type: 'json',
};
import deploymentSchema from '../../../compiler/src/schemas/flag-deployment.schema.v1.json' with {
  type: 'json',
};

interface CompileOptions {
  deployment?: string;
  env?: string;
  output?: string;
  definitions?: string;
}

/**
 * Compile a deployment file to an AST artifact.
 */
export async function compileCommand(options: CompileOptions): Promise<number> {
  const validator = new Validator({
    definitions: definitionsSchema,
    deployment: deploymentSchema,
  });

  // Determine deployment file path
  let deploymentPath: string;
  if (options.deployment) {
    deploymentPath = options.deployment;
  } else if (options.env) {
    deploymentPath = `.controlpath/${options.env}.deployment.yaml`;
  } else {
    console.error('✗ Compilation failed');
    console.error('  Error: Either --deployment <file> or --env <env> must be provided');
    return 1;
  }

  // Determine output path
  let outputPath: string;
  if (options.output) {
    outputPath = options.output;
  } else if (options.env) {
    outputPath = `.controlpath/${options.env}.ast`;
  } else {
    // Infer from deployment path
    const deploymentDir = deploymentPath.substring(0, deploymentPath.lastIndexOf('/'));
    const deploymentName = deploymentPath.substring(deploymentPath.lastIndexOf('/') + 1);
    const envName = deploymentName.replace('.deployment.yaml', '');
    outputPath = `${deploymentDir}/${envName}.ast`;
  }

  // Determine definitions file path
  const definitionsPath = options.definitions || 'flags.definitions.yaml';

  try {
    // Validate and parse definitions
    const definitionsData = parseDefinitions(definitionsPath);
    const definitionsValidation = validator.validateDefinitions(definitionsPath, definitionsData);
    if (!definitionsValidation.valid) {
      console.error('✗ Compilation failed');
      console.error(`  Error: Definitions file is invalid`);
      console.error(validator.formatErrors(definitionsValidation.errors));
      return 1;
    }

    // Validate and parse deployment
    const deploymentData = parseDeployment(deploymentPath);
    const deploymentValidation = validator.validateDeployment(deploymentPath, deploymentData);
    if (!deploymentValidation.valid) {
      console.error('✗ Compilation failed');
      console.error(`  Error: Deployment file is invalid`);
      console.error(validator.formatErrors(deploymentValidation.errors));
      return 1;
    }

    // Compile to AST
    const astBytes = compileAndSerialize(deploymentData, definitionsData);

    // Create output directory if it doesn't exist
    const outputDir = outputPath.substring(0, outputPath.lastIndexOf('/'));
    if (outputDir && outputDir !== '.') {
      try {
        await Deno.mkdir(outputDir, { recursive: true });
      } catch {
        // Directory might already exist, ignore
      }
    }

    // Write AST file
    await Deno.writeFile(outputPath, astBytes);

    console.log(`✓ Compiled to ${outputPath}`);
    return 0;
  } catch (error) {
    console.error('✗ Compilation failed');
    console.error(`  Error: ${error instanceof Error ? error.message : String(error)}`);
    return 1;
  }
}
