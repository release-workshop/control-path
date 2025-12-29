/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * OpenFeature Provider implementation for Control Path.
 * This provider directly implements the OpenFeature Provider interface.
 */

import type { Artifact, ResolutionDetails, Logger, User, Context } from './types';
import { loadFromFile, loadFromURL } from './ast-loader';
import { evaluate } from './evaluator';

/**
 * Options for Provider constructor
 */
export interface ProviderOptions {
  /** Optional logger for error and debug logging */
  logger?: Logger;
  /** Optional flag name to index mapping (for name-based flag lookup) */
  flagNameMap?: Record<string, number>;
}

/**
 * Control Path Provider for OpenFeature.
 * Implements the OpenFeature Provider interface directly.
 */
export class Provider {
  private artifact: Artifact | null = null;
  private logger?: Logger;
  private flagNameMap: Record<string, number>;

  /**
   * Metadata for OpenFeature compliance
   */
  readonly metadata = {
    name: 'controlpath',
  };

  /**
   * Hooks array for OpenFeature (optional)
   */
  readonly hooks: unknown[] = [];

  /**
   * Create a new Provider instance
   * @param options - Optional provider configuration
   */
  constructor(options?: ProviderOptions) {
    this.logger = options?.logger;
    this.flagNameMap = options?.flagNameMap ?? {};
  }

  /**
   * Load AST artifact from file path or URL
   * @param artifact - File path (string) or URL (string | URL) to load the AST artifact from
   * @throws Error if the artifact path is invalid or loading fails
   */
  async loadArtifact(artifact: string | URL): Promise<void> {
    if (!artifact) {
      throw new Error('Artifact path or URL is required');
    }

    const artifactPath = artifact instanceof URL ? artifact.toString() : artifact;

    if (typeof artifactPath !== 'string' || artifactPath.trim().length === 0) {
      throw new Error('Artifact path or URL must be a non-empty string');
    }

    try {
      if (artifactPath.startsWith('http://') || artifactPath.startsWith('https://')) {
        this.artifact = await loadFromURL(artifactPath, 30000, this.logger);
      } else if (artifactPath.startsWith('file://')) {
        // Handle file:// URLs by removing the protocol
        const filePath = artifactPath.replace(/^file:\/\//, '');
        this.artifact = await loadFromFile(filePath);
      } else {
        this.artifact = await loadFromFile(artifactPath);
      }
    } catch (error) {
      if (this.logger) {
        this.logger.error(
          'Failed to load AST artifact',
          error instanceof Error ? error : new Error(String(error))
        );
      }
      throw error;
    }
  }

  /**
   * Reload AST artifact (replaces cached AST)
   */
  async reloadArtifact(artifact: string | URL): Promise<void> {
    await this.loadArtifact(artifact);
  }

  /**
   * Resolve boolean flag evaluation (OpenFeature interface)
   * @param flagKey - The name of the flag to evaluate
   * @param defaultValue - Default boolean value to return if evaluation fails
   * @param evalContext - OpenFeature EvaluationContext (optional)
   * @returns ResolutionDetails with the evaluated value or default
   */
  resolveBooleanEvaluation(
    flagKey: string,
    defaultValue: boolean,
    evalContext?: unknown
  ): ResolutionDetails<boolean> {
    try {
      if (!this.artifact) {
        if (this.logger) {
          this.logger.debug('No artifact loaded, returning default value');
        }
        return {
          value: defaultValue,
          reason: 'DEFAULT',
        };
      }

      const { user, context } = this.mapEvaluationContext(evalContext);
      const flagIndex = this.getFlagIndex(flagKey);

      if (flagIndex === undefined) {
        if (this.logger) {
          this.logger.warn(`Flag "${flagKey}" not found in flag name map`);
        }
        return {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: 'FLAG_NOT_FOUND',
        };
      }

      const result = evaluate(flagIndex, this.artifact, user, context);

      if (result === undefined) {
        return {
          value: defaultValue,
          reason: 'DEFAULT',
        };
      }

      // Convert result to boolean
      const boolValue = result === true || result === 'true' || result === 'ON' || result === 1;
      return {
        value: boolValue,
        reason: 'TARGETING_MATCH',
      };
    } catch (error) {
      if (this.logger) {
        this.logger.error(
          `Error evaluating boolean flag "${flagKey}"`,
          error instanceof Error ? error : new Error(String(error))
        );
      }
      return {
        value: defaultValue,
        reason: 'ERROR',
        errorCode: 'GENERAL',
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Resolve string flag evaluation (OpenFeature interface)
   * @param flagKey - The name of the flag to evaluate
   * @param defaultValue - Default string value to return if evaluation fails
   * @param evalContext - OpenFeature EvaluationContext (optional)
   * @returns ResolutionDetails with the evaluated value or default
   */
  resolveStringEvaluation(
    flagKey: string,
    defaultValue: string,
    evalContext?: unknown
  ): ResolutionDetails<string> {
    try {
      if (!this.artifact) {
        if (this.logger) {
          this.logger.debug('No artifact loaded, returning default value');
        }
        return {
          value: defaultValue,
          reason: 'DEFAULT',
        };
      }

      const { user, context } = this.mapEvaluationContext(evalContext);
      const flagIndex = this.getFlagIndex(flagKey);

      if (flagIndex === undefined) {
        if (this.logger) {
          this.logger.warn(`Flag "${flagKey}" not found in flag name map`);
        }
        return {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: 'FLAG_NOT_FOUND',
        };
      }

      const result = evaluate(flagIndex, this.artifact, user, context);

      if (result === undefined) {
        return {
          value: defaultValue,
          reason: 'DEFAULT',
        };
      }

      // Convert result to string
      const stringValue = String(result);
      return {
        value: stringValue,
        reason: 'TARGETING_MATCH',
      };
    } catch (error) {
      if (this.logger) {
        this.logger.error(
          `Error evaluating string flag "${flagKey}"`,
          error instanceof Error ? error : new Error(String(error))
        );
      }
      return {
        value: defaultValue,
        reason: 'ERROR',
        errorCode: 'GENERAL',
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Resolve number flag evaluation (OpenFeature interface)
   * @param flagKey - The name of the flag to evaluate
   * @param defaultValue - Default number value to return if evaluation fails
   * @param evalContext - OpenFeature EvaluationContext (optional)
   * @returns ResolutionDetails with the evaluated value or default
   */
  resolveNumberEvaluation(
    flagKey: string,
    defaultValue: number,
    evalContext?: unknown
  ): ResolutionDetails<number> {
    try {
      if (!this.artifact) {
        if (this.logger) {
          this.logger.debug('No artifact loaded, returning default value');
        }
        return {
          value: defaultValue,
          reason: 'DEFAULT',
        };
      }

      const { user, context } = this.mapEvaluationContext(evalContext);
      const flagIndex = this.getFlagIndex(flagKey);

      if (flagIndex === undefined) {
        if (this.logger) {
          this.logger.warn(`Flag "${flagKey}" not found in flag name map`);
        }
        return {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: 'FLAG_NOT_FOUND',
        };
      }

      const result = evaluate(flagIndex, this.artifact, user, context);

      if (result === undefined) {
        return {
          value: defaultValue,
          reason: 'DEFAULT',
        };
      }

      // Convert result to number
      const numValue = typeof result === 'number' ? result : parseFloat(String(result));
      if (isNaN(numValue)) {
        return {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: 'TYPE_MISMATCH',
        };
      }

      return {
        value: numValue,
        reason: 'TARGETING_MATCH',
      };
    } catch (error) {
      if (this.logger) {
        this.logger.error(
          `Error evaluating number flag "${flagKey}"`,
          error instanceof Error ? error : new Error(String(error))
        );
      }
      return {
        value: defaultValue,
        reason: 'ERROR',
        errorCode: 'GENERAL',
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Resolve object flag evaluation (OpenFeature interface)
   * @param flagKey - The name of the flag to evaluate
   * @param defaultValue - Default object value to return if evaluation fails
   * @param evalContext - OpenFeature EvaluationContext (optional)
   * @returns ResolutionDetails with the evaluated value or default
   */
  resolveObjectEvaluation<T extends Record<string, unknown>>(
    flagKey: string,
    defaultValue: T,
    evalContext?: unknown
  ): ResolutionDetails<T> {
    try {
      if (!this.artifact) {
        if (this.logger) {
          this.logger.debug('No artifact loaded, returning default value');
        }
        return {
          value: defaultValue,
          reason: 'DEFAULT',
        };
      }

      const { user, context } = this.mapEvaluationContext(evalContext);
      const flagIndex = this.getFlagIndex(flagKey);

      if (flagIndex === undefined) {
        if (this.logger) {
          this.logger.warn(`Flag "${flagKey}" not found in flag name map`);
        }
        return {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: 'FLAG_NOT_FOUND',
        };
      }

      const result = evaluate(flagIndex, this.artifact, user, context);

      if (result === undefined) {
        return {
          value: defaultValue,
          reason: 'DEFAULT',
        };
      }

      // Convert result to object
      let objValue: T;
      if (typeof result === 'object' && result !== null && !Array.isArray(result)) {
        objValue = result as T;
      } else if (typeof result === 'string') {
        try {
          objValue = JSON.parse(result) as T;
        } catch {
          return {
            value: defaultValue,
            reason: 'DEFAULT',
            errorCode: 'TYPE_MISMATCH',
          };
        }
      } else {
        return {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: 'TYPE_MISMATCH',
        };
      }

      return {
        value: objValue,
        reason: 'TARGETING_MATCH',
      };
    } catch (error) {
      if (this.logger) {
        this.logger.error(
          `Error evaluating object flag "${flagKey}"`,
          error instanceof Error ? error : new Error(String(error))
        );
      }
      return {
        value: defaultValue,
        reason: 'ERROR',
        errorCode: 'GENERAL',
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Get flag index from flag name.
   * @param flagKey - Flag name
   * @returns Flag index or undefined if not found
   */
  private getFlagIndex(flagKey: string): number | undefined {
    return this.flagNameMap[flagKey];
  }

  /**
   * Map OpenFeature EvaluationContext to Control Path User and Context.
   *
   * Mapping strategy:
   * - Top-level properties (id, email, role) → User object
   * - Properties prefixed with "user." → User object (e.g., "user.role" → user.role)
   * - Context properties (environment, device, app_version) → Context object
   * - All other properties → User object (extensible)
   *
   * @param evalContext - OpenFeature EvaluationContext (object with attributes)
   * @returns User and Context objects
   */
  private mapEvaluationContext(evalContext?: unknown): { user: User; context?: Context } {
    // Validate input
    if (!evalContext) {
      return { user: {} };
    }

    if (typeof evalContext !== 'object' || Array.isArray(evalContext) || evalContext === null) {
      if (this.logger) {
        this.logger.warn('Invalid EvaluationContext type, expected object');
      }
      return { user: {} };
    }

    const context = evalContext as Record<string, unknown>;

    // Standard user properties (top-level)
    const user: User = {};

    // Extract standard user properties with type validation
    if (typeof context.id === 'string') {
      user.id = context.id;
    }
    if (typeof context.email === 'string') {
      user.email = context.email;
    }
    if (typeof context.role === 'string') {
      user.role = context.role;
    }

    // Build context object (environment, device, etc.)
    const controlPathContext: Context = {};

    if (typeof context.environment === 'string') {
      controlPathContext.environment = context.environment;
    }
    if (typeof context.device === 'string') {
      controlPathContext.device = context.device;
    }
    if (typeof context.app_version === 'string') {
      controlPathContext.app_version = context.app_version;
    }

    // Process all properties
    for (const [key, value] of Object.entries(context)) {
      // Skip already processed standard properties
      if (['id', 'email', 'role', 'environment', 'device', 'app_version'].includes(key)) {
        continue;
      }

      // Handle nested user properties (e.g., "user.role")
      if (key.startsWith('user.')) {
        const userKey = key.substring(5);
        if (userKey.length > 0) {
          user[userKey] = value;
        }
        continue;
      }

      // Handle nested context properties (e.g., "context.environment")
      if (key.startsWith('context.')) {
        const contextKey = key.substring(8);
        if (contextKey.length > 0) {
          controlPathContext[contextKey] = value;
        }
        continue;
      }

      // All other properties go to user (extensible)
      user[key] = value;
    }

    return { user, context: controlPathContext };
  }
}
