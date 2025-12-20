/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Type definitions for parsed flag definitions and deployments.
 * These match the JSON schema structures.
 */

/**
 * Flag definition types
 */

/**
 * Type of flag: boolean (ON/OFF) or multivariate (multiple values).
 */
export type FlagType = 'boolean' | 'multivariate';

/**
 * Valid flag value types: string, boolean, or number.
 */
export type FlagValue = string | boolean | number;

/**
 * Variation definition for multivariate flags.
 */
export interface FlagVariation {
  /** Variation name (must be unique within the flag) */
  name: string;
  /** Variation value */
  value: FlagValue;
  /** Optional description of this variation */
  description?: string;
}

/**
 * Flag definition structure.
 * Defines a feature flag with its type, default value, and optional variations.
 */
export interface FlagDefinition {
  /** Flag name (must be unique across all flags) */
  name: string;
  /** Flag type: boolean or multivariate */
  type: FlagType;
  /** Default value returned when no rules match */
  defaultValue: FlagValue;
  /** Optional description of the flag */
  description?: string;
  /** Variations for multivariate flags (required if type is 'multivariate') */
  variations?: FlagVariation[];
  /** Optional flag kind/category */
  kind?: string;
  /** Optional metadata (key-value pairs) */
  metadata?: Record<string, unknown>;
  /** Optional lifecycle information */
  lifecycle?: Record<string, unknown>;
}

/**
 * Context schema definition.
 * Defines the structure of user/context data used in flag evaluation.
 * Supports nested objects and string types.
 */
export interface ContextSchema {
  [key: string]: string | ContextSchema;
}

/**
 * Flag definitions file structure.
 * Contains all flag definitions and optional context schema.
 */
export interface FlagDefinitions {
  /** Array of flag definitions */
  flags: FlagDefinition[];
  /** Optional context schema for user/context data */
  context?: ContextSchema;
}

/**
 * Deployment types
 */

/**
 * Deployment rule for a flag.
 * Each rule can have a condition (when) and one of: serve, variations, or rollout.
 */
export interface DeploymentRule {
  /** Optional rule name for documentation */
  name?: string;
  /** Optional condition expression (e.g., "user.role == 'admin'") */
  when?: string;
  /** Serve a specific value (for boolean or multivariate flags) */
  serve?: FlagValue;
  /** Serve multiple variations with weights (for multivariate flags) */
  variations?: Array<{
    /** Variation name */
    variation: string;
    /** Weight percentage (0-100) */
    weight: number;
  }>;
  /** Gradual rollout of a variation (for multivariate flags) */
  rollout?: {
    /** Variation name to rollout */
    variation: string;
    /** Rollout percentage (0-100) */
    percentage: number;
  };
}

/**
 * Rules for a specific flag in a deployment.
 */
export interface FlagRules {
  /** Array of deployment rules (evaluated in order) */
  rules?: DeploymentRule[];
}

/**
 * Segment definition.
 * Segments are reusable groups of users defined by a condition.
 */
export interface SegmentDefinition {
  /** Condition expression that defines the segment */
  when: string;
}

/**
 * Deployment file structure.
 * Defines flag rules for a specific environment.
 */
export interface Deployment {
  /** Environment name (e.g., "production", "staging") */
  environment: string;
  /** Flag rules keyed by flag name */
  rules: Record<string, FlagRules>;
  /** Optional segment definitions */
  segments?: Record<string, SegmentDefinition>;
}

/**
 * Parser error types
 *
 * Note: ParseError class is exported from './parse-error.ts'
 */
