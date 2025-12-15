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
export type FlagType = 'boolean' | 'multivariate';

export type FlagValue = string | boolean | number;

export interface FlagVariation {
  name: string;
  value: FlagValue;
  description?: string;
}

export interface FlagDefinition {
  name: string;
  type: FlagType;
  defaultValue: FlagValue;
  description?: string;
  variations?: FlagVariation[];
  kind?: string;
  metadata?: Record<string, unknown>;
  lifecycle?: Record<string, unknown>;
}

export interface ContextSchema {
  [key: string]: string | ContextSchema;
}

export interface FlagDefinitions {
  flags: FlagDefinition[];
  context?: ContextSchema;
}

/**
 * Deployment types
 */
export interface DeploymentRule {
  name?: string;
  when?: string;
  serve?: FlagValue;
  variations?: Array<{
    variation: string;
    weight: number;
  }>;
  rollout?: {
    variation: string;
    percentage: number;
  };
}

export interface FlagRules {
  default: FlagValue;
  rules?: DeploymentRule[];
}

export interface SegmentDefinition {
  when: string;
}

export interface Deployment {
  environment: string;
  rules: Record<string, FlagRules>;
  segments?: Record<string, SegmentDefinition>;
}

/**
 * Parser error types
 */
export class ParseError extends Error {
  constructor(
    message: string,
    public readonly filePath: string,
    public readonly cause?: Error
  ) {
    super(message);
    this.name = 'ParseError';
  }
}
