/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Control Path Runtime SDK
 *
 * Low-level runtime SDK for loading AST artifacts and evaluating flags.
 * Provides OpenFeature-compliant Provider interface.
 */

export { Provider, type ProviderOptions } from './provider';
export { loadFromFile, loadFromURL, loadFromBuffer, type LoadOptions } from './ast-loader';
export {
  loadOverrideFromFile,
  loadOverrideFromURL,
  OverrideFileNotModifiedError,
  type OverrideLoadResult,
} from './override-loader';
export { evaluate, evaluateRule } from './evaluator';
export { buildFlagNameMap, buildFlagNameMapFromArtifact } from './utils';
export type {
  Artifact,
  Rule,
  Expression,
  Variation,
  User,
  Context,
  ResolutionDetails,
  Logger,
  OverrideFile,
  OverrideValue,
  OverrideState,
} from './types';
export {
  RuleType,
  ExpressionType,
  BinaryOp,
  LogicalOp,
  FuncCode,
  isArtifact,
  isRule,
  isVariation,
  isExpression,
} from './types';
