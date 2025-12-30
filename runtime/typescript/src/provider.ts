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
import { PROTOTYPE_POLLUTING_KEYS } from './types';
import { loadFromFile, loadFromURL, type LoadOptions } from './ast-loader';
import { evaluate } from './evaluator';

/**
 * Options for Provider constructor
 */
export interface ProviderOptions {
  /** Optional logger for error and debug logging */
  logger?: Logger;
  /**
   * Flag name to index mapping (for name-based flag lookup).
   * This maps flag names (strings) to their indices in the AST flags array.
   *
   * @example
   * ```typescript
   * const flagNameMap = {
   *   'new_dashboard': 0,
   *   'enable_analytics': 1,
   *   'theme_color': 2
   * };
   * ```
   *
   * You can build this map from flag definitions using the `buildFlagNameMap` helper function.
   */
  flagNameMap?: Record<string, number>;
  /** Optional public key for Ed25519 signature verification (base64 or hex encoded) */
  publicKey?: string | Uint8Array;
  /** Whether to require a signature (default: false - signature is optional) */
  requireSignature?: boolean;
  /** Whether to enable result caching (default: true) */
  enableCache?: boolean;
  /** Cache TTL in milliseconds (default: 5 minutes) */
  cacheTTL?: number;
}

/**
 * Control Path Provider for OpenFeature.
 * Implements the OpenFeature Provider interface directly.
 */
/**
 * Cache entry for evaluation results
 */
interface CacheEntry {
  details: ResolutionDetails<unknown>;
  timestamp: number;
}

/**
 * Default cache TTL: 5 minutes
 */
const DEFAULT_CACHE_TTL = 5 * 60 * 1000;

export class Provider {
  private artifact: Artifact | null = null;
  private logger?: Logger;
  private flagNameMap: Record<string, number>;
  private loadOptions?: LoadOptions;
  private cache: Map<string, CacheEntry> = new Map();
  private cacheEnabled: boolean = true;
  private cacheTTL: number = DEFAULT_CACHE_TTL;

  /**
   * Metadata for OpenFeature compliance
   */
  readonly metadata = {
    name: 'controlpath',
  };

  /**
   * Hooks array for OpenFeature (optional)
   */
  readonly hooks: Array<unknown> = [];

  /**
   * Create a new Provider instance
   * @param options - Optional provider configuration
   */
  constructor(options?: ProviderOptions) {
    this.logger = options?.logger;
    this.flagNameMap = options?.flagNameMap ?? {};
    this.cacheEnabled = options?.enableCache ?? true;
    this.cacheTTL = options?.cacheTTL ?? DEFAULT_CACHE_TTL;
    if (options?.publicKey || options?.requireSignature) {
      this.loadOptions = {
        publicKey: options.publicKey,
        requireSignature: options.requireSignature,
      };
    }
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
        // Use default timeout from ast-loader
        this.artifact = await loadFromURL(artifactPath, undefined, this.logger, this.loadOptions);
      } else if (artifactPath.startsWith('file://')) {
        // Handle file:// URLs by removing the protocol
        const filePath = artifactPath.replace(/^file:\/\//, '');
        this.artifact = await loadFromFile(filePath, this.loadOptions);
      } else {
        this.artifact = await loadFromFile(artifactPath, this.loadOptions);
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
   * This also clears the evaluation result cache.
   */
  async reloadArtifact(artifact: string | URL): Promise<void> {
    await this.loadArtifact(artifact);
    this.clearCache();
  }

  /**
   * Clear the evaluation result cache
   */
  clearCache(): void {
    this.cache.clear();
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
      // Check cache first
      if (this.cacheEnabled) {
        const cacheKey = this.getCacheKey(flagKey, evalContext);
        const cached = this.getCachedResult(cacheKey);
        if (cached) {
          return cached as ResolutionDetails<boolean>;
        }
      }

      if (!this.artifact) {
        if (this.logger) {
          this.logger.debug('No artifact loaded, returning default value');
        }
        const details: ResolutionDetails<boolean> = {
          value: defaultValue,
          reason: 'DEFAULT',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      const { user, context } = this.mapEvaluationContext(evalContext);
      const flagIndex = this.getFlagIndex(flagKey);

      if (flagIndex === undefined) {
        if (this.logger) {
          this.logger.warn(`Flag "${flagKey}" not found in flag name map`);
        }
        const details: ResolutionDetails<boolean> = {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: 'FLAG_NOT_FOUND',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      const result = evaluate(flagIndex, this.artifact, user, context);

      if (result === undefined) {
        const details: ResolutionDetails<boolean> = {
          value: defaultValue,
          reason: 'DEFAULT',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      // Convert result to boolean
      const boolValue = result === true || result === 'true' || result === 'ON' || result === 1;
      const details: ResolutionDetails<boolean> = {
        value: boolValue,
        reason: 'TARGETING_MATCH',
      };
      this.setCachedResult(flagKey, evalContext, details);
      return details;
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
      // Check cache first
      if (this.cacheEnabled) {
        const cacheKey = this.getCacheKey(flagKey, evalContext);
        const cached = this.getCachedResult(cacheKey);
        if (cached) {
          return cached as ResolutionDetails<string>;
        }
      }

      if (!this.artifact) {
        if (this.logger) {
          this.logger.debug('No artifact loaded, returning default value');
        }
        const details: ResolutionDetails<string> = {
          value: defaultValue,
          reason: 'DEFAULT',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      const { user, context } = this.mapEvaluationContext(evalContext);
      const flagIndex = this.getFlagIndex(flagKey);

      if (flagIndex === undefined) {
        if (this.logger) {
          this.logger.warn(`Flag "${flagKey}" not found in flag name map`);
        }
        const details: ResolutionDetails<string> = {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: 'FLAG_NOT_FOUND',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      const result = evaluate(flagIndex, this.artifact, user, context);

      if (result === undefined) {
        const details: ResolutionDetails<string> = {
          value: defaultValue,
          reason: 'DEFAULT',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      // Convert result to string
      const stringValue = String(result);
      // Determine variant for multivariate flags
      const variant = this.getVariant(flagIndex, result);
      const details: ResolutionDetails<string> = {
        value: stringValue,
        reason: 'TARGETING_MATCH',
        variant,
      };
      this.setCachedResult(flagKey, evalContext, details);
      return details;
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
      // Check cache first
      if (this.cacheEnabled) {
        const cacheKey = this.getCacheKey(flagKey, evalContext);
        const cached = this.getCachedResult(cacheKey);
        if (cached) {
          return cached as ResolutionDetails<number>;
        }
      }

      if (!this.artifact) {
        if (this.logger) {
          this.logger.debug('No artifact loaded, returning default value');
        }
        const details: ResolutionDetails<number> = {
          value: defaultValue,
          reason: 'DEFAULT',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      const { user, context } = this.mapEvaluationContext(evalContext);
      const flagIndex = this.getFlagIndex(flagKey);

      if (flagIndex === undefined) {
        if (this.logger) {
          this.logger.warn(`Flag "${flagKey}" not found in flag name map`);
        }
        const details: ResolutionDetails<number> = {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: 'FLAG_NOT_FOUND',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      const result = evaluate(flagIndex, this.artifact, user, context);

      if (result === undefined) {
        const details: ResolutionDetails<number> = {
          value: defaultValue,
          reason: 'DEFAULT',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      // Convert result to number
      const numValue = typeof result === 'number' ? result : parseFloat(String(result));
      if (isNaN(numValue)) {
        const details: ResolutionDetails<number> = {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: 'TYPE_MISMATCH',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      const details: ResolutionDetails<number> = {
        value: numValue,
        reason: 'TARGETING_MATCH',
      };
      this.setCachedResult(flagKey, evalContext, details);
      return details;
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
      // Check cache first
      if (this.cacheEnabled) {
        const cacheKey = this.getCacheKey(flagKey, evalContext);
        const cached = this.getCachedResult(cacheKey);
        if (cached) {
          return cached as ResolutionDetails<T>;
        }
      }

      if (!this.artifact) {
        if (this.logger) {
          this.logger.debug('No artifact loaded, returning default value');
        }
        const details: ResolutionDetails<T> = {
          value: defaultValue,
          reason: 'DEFAULT',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      const { user, context } = this.mapEvaluationContext(evalContext);
      const flagIndex = this.getFlagIndex(flagKey);

      if (flagIndex === undefined) {
        if (this.logger) {
          this.logger.warn(`Flag "${flagKey}" not found in flag name map`);
        }
        const details: ResolutionDetails<T> = {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: 'FLAG_NOT_FOUND',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      const result = evaluate(flagIndex, this.artifact, user, context);

      if (result === undefined) {
        const details: ResolutionDetails<T> = {
          value: defaultValue,
          reason: 'DEFAULT',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      // Convert result to object
      let objValue: T;
      if (typeof result === 'object' && result !== null && !Array.isArray(result)) {
        objValue = result as T;
      } else if (typeof result === 'string') {
        try {
          objValue = JSON.parse(result) as T;
        } catch {
          const details: ResolutionDetails<T> = {
            value: defaultValue,
            reason: 'DEFAULT',
            errorCode: 'TYPE_MISMATCH',
          };
          this.setCachedResult(flagKey, evalContext, details);
          return details;
        }
      } else {
        const details: ResolutionDetails<T> = {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: 'TYPE_MISMATCH',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      // Determine variant for multivariate flags
      const variant = this.getVariant(flagIndex, result);
      const details: ResolutionDetails<T> = {
        value: objValue,
        reason: 'TARGETING_MATCH',
        variant,
      };
      this.setCachedResult(flagKey, evalContext, details);
      return details;
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
   * Type guard to check if a value is a record-like object
   */
  private isRecordLike(value: unknown): value is Record<string, unknown> {
    return (
      value !== null && value !== undefined && typeof value === 'object' && !Array.isArray(value)
    );
  }

  /**
   * Get cache key from flag key and evaluation context.
   * Sanitizes context to prevent prototype pollution and normalizes keys for better cache hit rates.
   */
  private getCacheKey(flagKey: string, evalContext?: unknown): string {
    if (!this.isRecordLike(evalContext)) {
      return flagKey;
    }

    // Create a safe copy without prototype chain to prevent prototype pollution
    const safeContext: Record<string, unknown> = {};
    const entries: Array<[string, unknown]> = Object.entries(evalContext);
    for (const [key, value] of entries) {
      // Skip prototype-polluting keys
      if (PROTOTYPE_POLLUTING_KEYS.includes(key as (typeof PROTOTYPE_POLLUTING_KEYS)[number])) {
        continue;
      }
      safeContext[key] = value;
    }

    // Normalize keys by sorting for consistent cache keys
    const sortedKeys = Object.keys(safeContext).sort();
    const normalizedContext: Record<string, unknown> = {};
    for (const key of sortedKeys) {
      const value: unknown = safeContext[key];
      normalizedContext[key] = value;
    }

    const contextStr = JSON.stringify(normalizedContext);
    return `${flagKey}:${contextStr}`;
  }

  /**
   * Get cached result if available and not expired
   */
  private getCachedResult(cacheKey: string): ResolutionDetails<unknown> | undefined {
    const entry = this.cache.get(cacheKey);
    if (!entry) {
      return undefined;
    }

    // Check if expired
    const now = Date.now();
    if (now - entry.timestamp > this.cacheTTL) {
      this.cache.delete(cacheKey);
      return undefined;
    }

    return entry.details;
  }

  /**
   * Set cached result
   */
  private setCachedResult(
    flagKey: string,
    evalContext: unknown,
    details: ResolutionDetails<unknown>
  ): void {
    if (!this.cacheEnabled) {
      return;
    }

    const cacheKey = this.getCacheKey(flagKey, evalContext);
    this.cache.set(cacheKey, {
      details,
      timestamp: Date.now(),
    });
  }

  /**
   * Get variant name for multivariate flags
   * This looks up the variation name from the string table if the result matches a variation value
   */
  private getVariant(flagIndex: number, result: unknown): string | undefined {
    if (!this.artifact || flagIndex < 0 || flagIndex >= this.artifact.flags.length) {
      return undefined;
    }

    const flagRules = this.artifact.flags[flagIndex];
    if (!flagRules) {
      return undefined;
    }

    // Look for variations rule that matches the result
    for (const rule of flagRules) {
      if (Array.isArray(rule) && rule.length >= 3 && rule[0] === 1) {
        // RuleType.VARIATIONS
        const variations = rule[2];
        if (Array.isArray(variations)) {
          // Check if result matches any variation
          for (const [varIndex, _pct] of variations) {
            if (typeof varIndex === 'number' && this.artifact.strs[varIndex] === result) {
              // Find the variation name - we need to check if there's a way to get the name
              // For now, return the string value as variant
              return String(result);
            }
          }
        }
      }
    }

    return undefined;
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

    const context: Record<string, unknown> = evalContext as Record<string, unknown>;

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
