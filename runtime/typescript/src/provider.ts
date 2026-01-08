/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * OpenFeature Provider implementation for Control Path.
 * This provider directly implements the OpenFeature Provider interface.
 */

import type {
  Artifact,
  ResolutionDetails,
  Logger,
  User,
  Context,
  EvaluationContext,
  OverrideState,
  OverrideValue,
} from './types';
import { ErrorCodeValues } from './types';
import { PROTOTYPE_POLLUTING_KEYS } from './types';
import { loadFromFile, loadFromURL, type LoadOptions } from './ast-loader';
import {
  loadOverrideFromFile,
  loadOverrideFromURL,
  OverrideFileNotModifiedError,
} from './override-loader';
import { evaluate } from './evaluator';
// Import Provider type directly from OpenFeature (type-only import, no runtime dependency)
import type { Provider as OpenFeatureProvider, Hook } from '@openfeature/server-sdk';

/**
 * Options for Provider constructor
 */
export interface ProviderOptions {
  /** Optional logger for error and debug logging */
  logger?: Logger;
  /** Optional public key for Ed25519 signature verification (base64 or hex encoded) */
  publicKey?: string | Uint8Array;
  /** Whether to require a signature (default: false - signature is optional) */
  requireSignature?: boolean;
  /** Whether to enable result caching (default: true) */
  enableCache?: boolean;
  /** Cache TTL in milliseconds (default: 5 minutes) */
  cacheTTL?: number;
  /** Optional override file URL or file path (loaded and polled automatically during initialization) */
  overrideUrl?: string | URL;
  /** Polling interval in milliseconds (default: 3000ms / 3 seconds) */
  pollingInterval?: number;
  /** Enable/disable polling (default: true when overrideUrl is set) */
  enablePolling?: boolean;
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

/**
 * Default timeout for override file URL loading: 10 seconds
 */
const DEFAULT_OVERRIDE_URL_TIMEOUT = 10000;

/**
 * Control Path Provider for OpenFeature.
 * Implements the OpenFeature Provider interface directly.
 *
 * This class implements the OpenFeatureProvider interface to ensure type compatibility
 * with @openfeature/server-sdk without requiring it as a dependency.
 */
export class Provider implements OpenFeatureProvider {
  private artifact: Artifact | null = null;
  private logger?: Logger;
  private flagNameMap: Record<string, number>;
  private loadOptions?: LoadOptions;
  private cache: Map<string, CacheEntry> = new Map();
  private cacheEnabled: boolean = true;
  private cacheTTL: number = DEFAULT_CACHE_TTL;
  private overrideState: OverrideState | null = null;
  private overrideUrl?: string | URL;
  private pollingInterval: number = 3000; // Default: 3 seconds
  private enablePolling: boolean = true;
  private pollingTimer?: NodeJS.Timeout | number;

  /**
   * Metadata for OpenFeature compliance
   */
  readonly metadata = {
    name: 'controlpath',
  };

  /**
   * Hooks array for OpenFeature (optional)
   */
  readonly hooks: Array<Hook<Record<string, unknown>>> = [];

  /**
   * Create a new Provider instance
   * @param options - Optional provider configuration
   */
  constructor(options?: ProviderOptions) {
    this.logger = options?.logger;
    this.flagNameMap = {}; // Will be built automatically when artifact is loaded
    this.cacheEnabled = options?.enableCache ?? true;
    this.cacheTTL = options?.cacheTTL ?? DEFAULT_CACHE_TTL;
    if (options?.publicKey || options?.requireSignature) {
      this.loadOptions = {
        publicKey: options.publicKey,
        requireSignature: options.requireSignature,
      };
    }

    // Override configuration
    if (options?.overrideUrl) {
      this.overrideUrl = options.overrideUrl;
      this.pollingInterval = options?.pollingInterval ?? 3000;
      this.enablePolling = options?.enablePolling ?? true;

      // Load override file during initialization (async, but don't await)
      // Polling will start automatically if enablePolling is true
      this.loadOverrideFile().catch((error) => {
        if (this.logger) {
          this.logger.error(
            'Failed to load override file during initialization',
            error instanceof Error ? error : new Error(String(error))
          );
        }
        // Don't throw - graceful degradation: continue without overrides
      });
    }
  }

  /**
   * Build flag name map from artifact flagNames array.
   * This automatically infers the flag name to index mapping from the artifact.
   * @private
   */
  private buildFlagNameMapFromArtifact(): void {
    if (!this.artifact) {
      return;
    }

    // Type assertion: Artifact includes flagNames (required field in new format)
    const artifact = this.artifact as Artifact & { flagNames?: number[] };
    if (!artifact.flagNames) {
      throw new Error(
        'Artifact does not include flagNames array. This artifact format is not supported. Please use a newer artifact that includes flag names.'
      );
    }

    this.flagNameMap = {};
    artifact.flagNames.forEach((nameIndex: number, flagIndex: number) => {
      const flagName = artifact.strs[nameIndex];
      if (flagName) {
        this.flagNameMap[flagName] = flagIndex;
      }
    });
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

      // Automatically build flag name map from artifact
      this.buildFlagNameMapFromArtifact();
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
   * Normalize override values to simple strings
   * Converts both simple string format and full object format to simple strings
   * @param overrides - Override values (can be string or object with value field)
   * @returns Normalized override values as simple strings
   * @private
   */
  private normalizeOverrideValues(
    overrides: Record<string, OverrideValue>
  ): Record<string, string> {
    const normalized: Record<string, string> = {};
    for (const [flagName, overrideValue] of Object.entries(overrides)) {
      normalized[flagName] =
        typeof overrideValue === 'string' ? overrideValue : overrideValue.value;
    }
    return normalized;
  }

  /**
   * Load override file from URL or file path
   * Called automatically during initialization if overrideUrl is provided
   * @private
   */
  private async loadOverrideFile(): Promise<void> {
    if (!this.overrideUrl) {
      return;
    }

    const overrideUrlStr =
      this.overrideUrl instanceof URL ? this.overrideUrl.toString() : this.overrideUrl;

    try {
      // Determine if this is a URL or file path
      if (overrideUrlStr.startsWith('http://') || overrideUrlStr.startsWith('https://')) {
        // HTTP/HTTPS URL - use URL loader with ETag support
        const etag = this.overrideState?.etag;
        try {
          const result = await loadOverrideFromURL(
            overrideUrlStr,
            etag,
            DEFAULT_OVERRIDE_URL_TIMEOUT,
            this.logger
          );
          // Normalize override values to simple strings
          const normalizedOverrides = this.normalizeOverrideValues(result.overrideFile.overrides);
          this.overrideState = {
            overrides: normalizedOverrides,
            etag: result.etag,
            lastLoadTime: Date.now(),
          };
          // Clear cache when overrides change
          this.clearCache();
        } catch (error) {
          // Handle 304 Not Modified (file hasn't changed)
          if (error instanceof OverrideFileNotModifiedError) {
            // File hasn't changed - keep existing override state
            if (this.overrideState) {
              this.overrideState.lastLoadTime = Date.now();
            }
            return;
          }
          throw error;
        }
      } else if (overrideUrlStr.startsWith('file://')) {
        // file:// URL - remove protocol and use file loader
        const filePath = overrideUrlStr.replace(/^file:\/\//, '');
        const overrideFile = await loadOverrideFromFile(filePath);
        // Normalize override values to simple strings
        const normalizedOverrides = this.normalizeOverrideValues(overrideFile.overrides);
        this.overrideState = {
          overrides: normalizedOverrides,
          lastLoadTime: Date.now(),
        };
        // Clear cache when overrides change
        this.clearCache();
      } else {
        // Direct file path (Node.js)
        const overrideFile = await loadOverrideFromFile(overrideUrlStr);
        // Normalize override values to simple strings
        const normalizedOverrides = this.normalizeOverrideValues(overrideFile.overrides);
        this.overrideState = {
          overrides: normalizedOverrides,
          lastLoadTime: Date.now(),
        };
        // Clear cache when overrides change
        this.clearCache();
      }

      // Start polling if enabled and URL is HTTP/HTTPS
      if (
        this.enablePolling &&
        (overrideUrlStr.startsWith('http://') || overrideUrlStr.startsWith('https://'))
      ) {
        this.startPolling();
      }
    } catch (error) {
      // Log error but don't throw - graceful degradation
      if (this.logger) {
        this.logger.warn(
          `Failed to load override file from ${overrideUrlStr}`,
          error instanceof Error ? error : new Error(String(error))
        );
      }
      // Don't throw - application continues without overrides
    }
  }

  /**
   * Start polling for override file updates.
   * Only works with HTTP/HTTPS URLs. Polling starts automatically during initialization
   * when overrideUrl is provided and enablePolling is true.
   *
   * If polling is already active, it will be stopped and restarted with the current configuration.
   */
  startPolling(): void {
    if (!this.overrideUrl) {
      return;
    }

    const overrideUrlStr =
      this.overrideUrl instanceof URL ? this.overrideUrl.toString() : this.overrideUrl;

    // Only poll HTTP/HTTPS URLs (not file:// or direct paths)
    if (!overrideUrlStr.startsWith('http://') && !overrideUrlStr.startsWith('https://')) {
      return;
    }

    // Stop existing polling if any
    this.stopPolling();

    // Start polling
    this.pollingTimer = setInterval(() => {
      this.loadOverrideFile().catch((error) => {
        if (this.logger) {
          this.logger.warn(
            'Failed to poll override file',
            error instanceof Error ? error : new Error(String(error))
          );
        }
        // Don't throw - continue polling
      });
    }, this.pollingInterval);
  }

  /**
   * Stop polling for override file updates.
   * This method is safe to call even if polling is not active.
   */
  stopPolling(): void {
    if (this.pollingTimer) {
      clearInterval(this.pollingTimer);
      this.pollingTimer = undefined;
    }
  }

  // Method overloads for resolveBooleanEvaluation
  // Supports both sync (for direct usage) and async (for OpenFeature SDK) signatures
  /**
   * Resolve boolean flag evaluation (async - for @openfeature/server-sdk compatibility)
   * Matches OpenFeature Provider interface: (flagKey, defaultValue, context, logger)
   * @overload
   */
  resolveBooleanEvaluation(
    flagKey: string,
    defaultValue: boolean,
    context: EvaluationContext,
    logger: Logger
  ): Promise<ResolutionDetails<boolean>>;
  /**
   * Resolve boolean flag evaluation (synchronous - for direct usage and generated SDK)
   * @overload
   */
  resolveBooleanEvaluation(
    flagKey: string,
    defaultValue: boolean,
    evalContext?: unknown
  ): ResolutionDetails<boolean>;
  /**
   * Implementation that handles both sync and async calls
   */
  resolveBooleanEvaluation(
    flagKey: string,
    defaultValueOrContext: boolean | EvaluationContext,
    evalContextOrDefault?: boolean | EvaluationContext | Logger,
    loggerOrUndefined?: Logger
  ): ResolutionDetails<boolean> | Promise<ResolutionDetails<boolean>> {
    // Check if this is the async signature (4 parameters: flagKey, defaultValue, context, logger)
    if (
      typeof defaultValueOrContext === 'boolean' &&
      typeof evalContextOrDefault === 'object' &&
      evalContextOrDefault !== null &&
      !Array.isArray(evalContextOrDefault) &&
      typeof loggerOrUndefined === 'object' &&
      loggerOrUndefined !== null &&
      'debug' in loggerOrUndefined
    ) {
      // Async signature: (flagKey, defaultValue, context, logger)
      const defaultValue = defaultValueOrContext;
      const context = evalContextOrDefault as EvaluationContext;
      // logger is available but we use our internal logger if configured
      return Promise.resolve(this.resolveBooleanEvaluationSync(flagKey, defaultValue, context));
    } else {
      // Sync signature: (flagKey, defaultValue, evalContext?)
      const defaultValue = defaultValueOrContext as boolean;
      const evalContext = evalContextOrDefault;
      return this.resolveBooleanEvaluationSync(flagKey, defaultValue, evalContext);
    }
  }

  // Method overloads for resolveStringEvaluation
  /**
   * Resolve string flag evaluation (async - for @openfeature/server-sdk compatibility)
   * Matches OpenFeature Provider interface: (flagKey, defaultValue, context, logger)
   * @overload
   */
  resolveStringEvaluation(
    flagKey: string,
    defaultValue: string,
    context: EvaluationContext,
    logger: Logger
  ): Promise<ResolutionDetails<string>>;
  /**
   * Resolve string flag evaluation (synchronous - for direct usage and generated SDK)
   * @overload
   */
  resolveStringEvaluation(
    flagKey: string,
    defaultValue: string,
    evalContext?: unknown
  ): ResolutionDetails<string>;
  /**
   * Implementation that handles both sync and async calls
   */
  resolveStringEvaluation(
    flagKey: string,
    defaultValueOrContext: string | EvaluationContext,
    evalContextOrDefault?: string | EvaluationContext | Logger,
    loggerOrUndefined?: Logger
  ): ResolutionDetails<string> | Promise<ResolutionDetails<string>> {
    // Check if this is the async signature (4 parameters: flagKey, defaultValue, context, logger)
    if (
      typeof defaultValueOrContext === 'string' &&
      typeof evalContextOrDefault === 'object' &&
      evalContextOrDefault !== null &&
      !Array.isArray(evalContextOrDefault) &&
      typeof loggerOrUndefined === 'object' &&
      loggerOrUndefined !== null &&
      'debug' in loggerOrUndefined
    ) {
      // Async signature: (flagKey, defaultValue, context, logger)
      const defaultValue = defaultValueOrContext;
      const context = evalContextOrDefault as EvaluationContext;
      // logger is available but we use our internal logger if configured
      return Promise.resolve(this.resolveStringEvaluationSync(flagKey, defaultValue, context));
    } else {
      // Sync signature: (flagKey, defaultValue, evalContext?)
      const defaultValue = defaultValueOrContext as string;
      const evalContext = evalContextOrDefault;
      return this.resolveStringEvaluationSync(flagKey, defaultValue, evalContext);
    }
  }

  // Method overloads for resolveNumberEvaluation
  /**
   * Resolve number flag evaluation (async - for @openfeature/server-sdk compatibility)
   * Matches OpenFeature Provider interface: (flagKey, defaultValue, context, logger)
   * @overload
   */
  resolveNumberEvaluation(
    flagKey: string,
    defaultValue: number,
    context: EvaluationContext,
    logger: Logger
  ): Promise<ResolutionDetails<number>>;
  /**
   * Resolve number flag evaluation (synchronous - for direct usage and generated SDK)
   * @overload
   */
  resolveNumberEvaluation(
    flagKey: string,
    defaultValue: number,
    evalContext?: unknown
  ): ResolutionDetails<number>;
  /**
   * Implementation that handles both sync and async calls
   */
  resolveNumberEvaluation(
    flagKey: string,
    defaultValueOrContext: number | EvaluationContext,
    evalContextOrDefault?: number | EvaluationContext | Logger,
    loggerOrUndefined?: Logger
  ): ResolutionDetails<number> | Promise<ResolutionDetails<number>> {
    // Check if this is the async signature (4 parameters: flagKey, defaultValue, context, logger)
    if (
      typeof defaultValueOrContext === 'number' &&
      typeof evalContextOrDefault === 'object' &&
      evalContextOrDefault !== null &&
      !Array.isArray(evalContextOrDefault) &&
      typeof loggerOrUndefined === 'object' &&
      loggerOrUndefined !== null &&
      'debug' in loggerOrUndefined
    ) {
      // Async signature: (flagKey, defaultValue, context, logger)
      const defaultValue = defaultValueOrContext;
      const context = evalContextOrDefault as EvaluationContext;
      // logger is available but we use our internal logger if configured
      return Promise.resolve(this.resolveNumberEvaluationSync(flagKey, defaultValue, context));
    } else {
      // Sync signature: (flagKey, defaultValue, evalContext?)
      const defaultValue = defaultValueOrContext as number;
      const evalContext = evalContextOrDefault;
      return this.resolveNumberEvaluationSync(flagKey, defaultValue, evalContext);
    }
  }

  // Method overloads for resolveObjectEvaluation
  /**
   * Resolve object flag evaluation (async - for @openfeature/server-sdk compatibility)
   * Matches OpenFeature Provider interface: (flagKey, defaultValue, context, logger)
   * @overload
   */
  resolveObjectEvaluation<T extends Record<string, unknown>>(
    flagKey: string,
    defaultValue: T,
    context: EvaluationContext,
    logger: Logger
  ): Promise<ResolutionDetails<T>>;
  /**
   * Resolve object flag evaluation (synchronous - for direct usage and generated SDK)
   * @overload
   */
  resolveObjectEvaluation<T extends Record<string, unknown>>(
    flagKey: string,
    defaultValue: T,
    evalContext?: unknown
  ): ResolutionDetails<T>;
  /**
   * Implementation that handles both sync and async calls
   */
  resolveObjectEvaluation<T extends Record<string, unknown>>(
    flagKey: string,
    defaultValueOrContext: T | EvaluationContext,
    evalContextOrDefault?: T | EvaluationContext | Logger,
    loggerOrUndefined?: Logger
  ): ResolutionDetails<T> | Promise<ResolutionDetails<T>> {
    // Check if this is the async signature (4 parameters: flagKey, defaultValue, context, logger)
    if (
      typeof defaultValueOrContext === 'object' &&
      defaultValueOrContext !== null &&
      !Array.isArray(defaultValueOrContext) &&
      typeof evalContextOrDefault === 'object' &&
      evalContextOrDefault !== null &&
      !Array.isArray(evalContextOrDefault) &&
      typeof loggerOrUndefined === 'object' &&
      loggerOrUndefined !== null &&
      'debug' in loggerOrUndefined &&
      !('targetingKey' in defaultValueOrContext) // Not an EvaluationContext
    ) {
      // Async signature: (flagKey, defaultValue, context, logger)
      const defaultValue = defaultValueOrContext as T;
      const context = evalContextOrDefault as EvaluationContext;
      // logger is available but we use our internal logger if configured
      return Promise.resolve(this.resolveObjectEvaluationSync(flagKey, defaultValue, context));
    } else {
      // Sync signature: (flagKey, defaultValue, evalContext?)
      const defaultValue = defaultValueOrContext as T;
      const evalContext = evalContextOrDefault;
      return this.resolveObjectEvaluationSync(flagKey, defaultValue, evalContext);
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
   * Get override value for a flag (if any)
   * @param flagKey - Flag name
   * @returns Override value or undefined if no override exists
   * @private
   */
  private getOverrideValue(flagKey: string): string | undefined {
    return this.overrideState?.overrides[flagKey];
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
   * Get variation name for multivariate flags.
   * For serve rules, the result might be:
   * 1. The variation name directly (e.g., "BLUE") - return as-is
   * 2. The variation value (e.g., "blue") - we can't map back to name without variation definitions
   *
   * We check if the result looks like a variation name (uppercase, alphanumeric/underscore)
   * and return it if so. Otherwise, we return undefined and use the result as-is.
   */
  private getVariationName(flagIndex: number, result: unknown): string | undefined {
    if (!this.artifact || flagIndex < 0 || flagIndex >= this.artifact.flags.length) {
      return undefined;
    }

    const resultStr = String(result);

    // Check if result looks like a variation name (uppercase, alphanumeric/underscore, at least 1 char)
    // Variation names are typically uppercase (e.g., "BLUE", "DARK", "SMALL", "MEDIUM", "LARGE")
    if (
      resultStr.length > 0 &&
      resultStr === resultStr.toUpperCase() &&
      /^[A-Z_][A-Z0-9_]*$/.test(resultStr) &&
      resultStr.length <= 50 // Reasonable upper limit for variation names
    ) {
      return resultStr;
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

  // ============================================================================
  // Internal synchronous methods (called by both sync and async public methods)
  // These contain the core evaluation logic
  // ============================================================================

  // ============================================================================
  // Internal synchronous methods (renamed to avoid conflict with async overloads)
  // These are called by both the public sync methods and async wrappers
  // ============================================================================

  /**
   * Internal synchronous boolean evaluation (called by both sync and async methods)
   * @private
   */
  private resolveBooleanEvaluationSync(
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
          errorCode: ErrorCodeValues.FLAG_NOT_FOUND,
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      const result = evaluate(flagIndex, this.artifact, user, context);

      if (result === undefined || result === null) {
        const details: ResolutionDetails<boolean> = {
          value: defaultValue,
          reason: 'DEFAULT',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      // Convert result to boolean
      // Handle various boolean representations: true, 'true', 'True', 'TRUE', 'ON', 'on', 1
      // The compiler normalizes boolean true to "ON" for boolean flags
      // Also handle the case where result might be the boolean false itself
      if (
        result === false ||
        result === 0 ||
        result === '0' ||
        result === 'OFF' ||
        result === 'off' ||
        result === 'false' ||
        result === 'False' ||
        result === 'FALSE'
      ) {
        const details: ResolutionDetails<boolean> = {
          value: false,
          reason: 'TARGETING_MATCH',
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      const resultStr = String(result).toUpperCase().trim();
      const boolValue =
        result === true ||
        result === 1 ||
        resultStr === 'TRUE' ||
        resultStr === 'ON' ||
        resultStr === '1' ||
        resultStr === 'YES';
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
        errorCode: ErrorCodeValues.GENERAL,
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Internal synchronous string evaluation (called by both sync and async methods)
   * @private
   */
  private resolveStringEvaluationSync(
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
          errorCode: ErrorCodeValues.FLAG_NOT_FOUND,
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
      // For multivariate flags with serve rules, the result might be a variation name (e.g., "BLUE")
      // or a variation value (e.g., "blue"). We prefer to return the variation name if it looks like one.
      const resultStr = String(result);
      const variationName = this.getVariationName(flagIndex, result);
      // If we detected a variation name, use it; otherwise use the result as-is
      const stringValue = variationName !== undefined ? variationName : resultStr;
      // Determine variant for multivariate flags
      const variant = variationName;
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
        errorCode: ErrorCodeValues.GENERAL,
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Internal synchronous number evaluation (called by both sync and async methods)
   * @private
   */
  private resolveNumberEvaluationSync(
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
          errorCode: ErrorCodeValues.FLAG_NOT_FOUND,
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
          errorCode: ErrorCodeValues.TYPE_MISMATCH,
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
        errorCode: ErrorCodeValues.GENERAL,
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Internal synchronous object evaluation (called by both sync and async methods)
   * @private
   */
  private resolveObjectEvaluationSync<T extends Record<string, unknown>>(
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
          errorCode: ErrorCodeValues.FLAG_NOT_FOUND,
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
            errorCode: ErrorCodeValues.TYPE_MISMATCH,
          };
          this.setCachedResult(flagKey, evalContext, details);
          return details;
        }
      } else {
        const details: ResolutionDetails<T> = {
          value: defaultValue,
          reason: 'DEFAULT',
          errorCode: ErrorCodeValues.TYPE_MISMATCH,
        };
        this.setCachedResult(flagKey, evalContext, details);
        return details;
      }

      // Determine variant for multivariate flags
      const variant = this.getVariationName(flagIndex, result);
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
        errorCode: ErrorCodeValues.GENERAL,
        errorMessage: error instanceof Error ? error.message : String(error),
      };
    }
  }
}
