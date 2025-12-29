/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * OpenFeature Provider implementation for Control Path.
 * This provider directly implements the OpenFeature Provider interface.
 */

import type { Artifact, ResolutionDetails, Logger } from './types';
import { loadFromFile, loadFromURL } from './ast-loader';

/**
 * Options for Provider constructor
 */
export interface ProviderOptions {
  /** Optional logger for error and debug logging */
  logger?: Logger;
}

/**
 * Control Path Provider for OpenFeature.
 * Implements the OpenFeature Provider interface directly.
 */
export class Provider {
  private artifact: Artifact | null = null;
  private logger?: Logger;

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
   * @param _flagKey - The name of the flag to evaluate
   * @param defaultValue - Default boolean value to return if evaluation fails
   * @param _context - OpenFeature EvaluationContext (optional)
   * @returns ResolutionDetails with the evaluated value or default
   */
  resolveBooleanEvaluation(
    _flagKey: string,
    defaultValue: boolean,
    _context?: unknown
  ): ResolutionDetails<boolean> {
    // TODO: Implement OpenFeature boolean evaluation
    // For now, return default value
    return {
      value: defaultValue,
      reason: 'DEFAULT',
    };
  }

  /**
   * Resolve string flag evaluation (OpenFeature interface)
   * @param _flagKey - The name of the flag to evaluate
   * @param defaultValue - Default string value to return if evaluation fails
   * @param _context - OpenFeature EvaluationContext (optional)
   * @returns ResolutionDetails with the evaluated value or default
   */
  resolveStringEvaluation(
    _flagKey: string,
    defaultValue: string,
    _context?: unknown
  ): ResolutionDetails<string> {
    // TODO: Implement OpenFeature string evaluation
    // For now, return default value
    return {
      value: defaultValue,
      reason: 'DEFAULT',
    };
  }

  /**
   * Resolve number flag evaluation (OpenFeature interface)
   * @param _flagKey - The name of the flag to evaluate
   * @param defaultValue - Default number value to return if evaluation fails
   * @param _context - OpenFeature EvaluationContext (optional)
   * @returns ResolutionDetails with the evaluated value or default
   */
  resolveNumberEvaluation(
    _flagKey: string,
    defaultValue: number,
    _context?: unknown
  ): ResolutionDetails<number> {
    // TODO: Implement OpenFeature number evaluation
    // For now, return default value
    return {
      value: defaultValue,
      reason: 'DEFAULT',
    };
  }

  /**
   * Resolve object flag evaluation (OpenFeature interface)
   * @param _flagKey - The name of the flag to evaluate
   * @param defaultValue - Default object value to return if evaluation fails
   * @param _context - OpenFeature EvaluationContext (optional)
   * @returns ResolutionDetails with the evaluated value or default
   */
  resolveObjectEvaluation<T extends Record<string, unknown>>(
    _flagKey: string,
    defaultValue: T,
    _context?: unknown
  ): ResolutionDetails<T> {
    // TODO: Implement OpenFeature object evaluation
    // For now, return default value
    return {
      value: defaultValue,
      reason: 'DEFAULT',
    };
  }
}
