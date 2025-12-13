/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

// Control Path AST Compiler
// Main entry point for the compiler package

export { Validator, ValidationResult, ValidationError, convertAjvErrors } from './validator';
export { validateDefinitions } from './validator/definitions';
export { validateDeployment } from './validator/deployment';

// Export parsers
export {
  parseDefinitions,
  parseDefinitionsFromString,
  parseDeployment,
  parseDeploymentFromString,
  type FlagDefinitions,
  type FlagDefinition,
  type FlagType,
  type FlagValue,
  type FlagVariation,
  type ContextSchema,
  type Deployment,
  type DeploymentRule,
  type FlagRules,
  type SegmentDefinition,
  type ParseError,
} from './parser';

// Export embedded schemas for CLI bundling (Node.js/CommonJS)
// Note: For Deno, import JSON files directly with import assertions
export { definitionsSchema, deploymentSchema } from './schemas/index';
