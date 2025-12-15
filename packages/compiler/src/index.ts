/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Control Path AST Compiler
 * Main entry point for the compiler package
 */

export { Validator, ValidationResult, ValidationError, convertAjvErrors } from './validator';
export { validateDefinitions } from './validator/definitions';
export { validateDeployment } from './validator/deployment';

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

export { definitionsSchema, deploymentSchema } from './schemas/index';

export {
  type Artifact,
  type Rule,
  type Variation,
  type Expression,
  RuleType,
  ExpressionType,
  BinaryOp,
  LogicalOp,
  FuncCode,
  isArtifact,
  isRule,
  isVariation,
  isExpression,
} from './ast';

export { compile, serialize, compileAndSerialize } from './compiler';
export { parseExpression } from './compiler/expressions';
export { StringTable } from './compiler/string-table';
