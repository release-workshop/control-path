/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Control Path Runtime SDK
 *
 * Low-level runtime SDK for loading AST artifacts and evaluating boolean flags.
 */
export {
  loadFromFile,
  loadFromURL,
  loadFromBuffer,
  ArtifactNotModifiedError,
  type LoadOptions,
  type ArtifactLoadResult,
} from './ast-loader';
export {
  loadKillSwitchFromFile,
  loadKillSwitchFromURL,
  KillSwitchFileNotModifiedError,
  type KillSwitchLoadResult,
} from './kill-switch-loader';
export { evaluate, evaluateBoolean, evaluateRule, coerceServePayloadToBoolean } from './evaluator';
export { resolveBooleanFlag, type ResolveBooleanFlagOptions } from './resolve-flag';
export { refreshFromFile, type FileFingerprint, type FileRefreshResult } from './file-refresh';
export {
  refreshKillSwitchFromUrl,
  refreshKillSwitchFromPath,
  KillSwitchRefreshCoordinator,
  startKillSwitchPoll,
  killSwitchInitDelayMs,
  startJitteredPoll,
  pollInitDelayMs,
  type KillSwitchRefreshState,
  type KillSwitchRefreshResult,
  type KillSwitchPollOptions,
  type JitteredPollOptions,
} from './kill-switch-polling';
export {
  refreshArtifactFromUrl,
  ArtifactRefreshCoordinator,
  validateArtifactPoll,
  assertArtifactAccepted,
  resolveExpectedArtifactEnv,
  shouldValidateArtifactAtInit,
  artifactFlagNames,
  type ArtifactRefreshState,
  type ArtifactRefreshResult,
} from './artifact-polling';
export {
  GeneratedEvaluatorRuntime,
  DEFAULT_GENERATED_KILL_SWITCH_POLL_MS,
  DEFAULT_GENERATED_KILL_SWITCH_INIT_JITTER_MS,
  DEFAULT_GENERATED_KILL_SWITCH_POLL_JITTER_MS,
  DEFAULT_GENERATED_ARTIFACT_POLL_MS,
  DEFAULT_GENERATED_ARTIFACT_INIT_JITTER_MS,
  DEFAULT_GENERATED_ARTIFACT_POLL_JITTER_MS,
  type GeneratedEvaluatorRuntimeConfig,
  type GeneratedEvaluatorInitOptions,
  type EvaluateBooleanFlagOptions,
} from './generated-evaluator-runtime';
export { buildFlagNameMap, buildFlagNameMapFromArtifact } from './utils';
export type {
  Artifact,
  Rule,
  Expression,
  BaseAttributes,
  Attributes,
  AttributesInput,
  Logger,
  KillSwitchFile,
} from './types';
export {
  RuleType,
  ExpressionType,
  BinaryOp,
  LogicalOp,
  FuncCode,
  isArtifact,
  isRule,
  isExpression,
} from './types';
