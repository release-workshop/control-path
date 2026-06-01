/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Runtime orchestration for generated TypeScript SDKs: init, polling, and flag evaluation.
 */

import { loadFromFile, loadFromURL } from './ast-loader';
import {
  assertArtifactAccepted,
  ArtifactRefreshCoordinator,
  resolveExpectedArtifactEnv,
  shouldValidateArtifactAtInit,
} from './artifact-polling';
import {
  KillSwitchRefreshCoordinator,
  killSwitchInitDelayMs,
  pollInitDelayMs,
  startJitteredPoll,
  startKillSwitchPoll,
} from './kill-switch-polling';
import { resolveBooleanFlag } from './resolve-flag';
import { buildFlagNameMapFromArtifact } from './utils';
import type { Artifact, Attributes, Logger } from './types';

export const DEFAULT_GENERATED_KILL_SWITCH_POLL_MS = 30_000;
/** Spread deploy-time fetches across pods (0..5s before first background refresh). */
export const DEFAULT_GENERATED_KILL_SWITCH_INIT_JITTER_MS = 5_000;
/** Added to each poll interval (30s + 0..10s) to avoid aligned CDN requests. */
export const DEFAULT_GENERATED_KILL_SWITCH_POLL_JITTER_MS = 10_000;

export const DEFAULT_GENERATED_ARTIFACT_POLL_MS = 60_000;
export const DEFAULT_GENERATED_ARTIFACT_INIT_JITTER_MS = 10_000;
export const DEFAULT_GENERATED_ARTIFACT_POLL_JITTER_MS = 15_000;

export interface GeneratedEvaluatorRuntimeConfig {
  killSwitchUrls: Readonly<Record<string, string>>;
  artifactUrls: Readonly<Record<string, string>>;
  sdkQualifiedFlagNames: ReadonlySet<string>;
  killSwitchPollMs?: number;
  killSwitchInitJitterMs?: number;
  killSwitchPollJitterMs?: number;
  artifactPollMs?: number;
  artifactInitJitterMs?: number;
  artifactPollJitterMs?: number;
}

export interface GeneratedEvaluatorInitOptions {
  artifact?: string;
  logger?: Logger;
}

export interface EvaluateBooleanFlagOptions {
  qualifiedName: string;
  catalogDefault: boolean;
  attributes?: Attributes;
}

/**
 * Owns compiled-artifact and kill-switch refresh state for a generated SDK instance.
 */
export class GeneratedEvaluatorRuntime {
  private readonly config: Required<
    Pick<
      GeneratedEvaluatorRuntimeConfig,
      | 'killSwitchPollMs'
      | 'killSwitchInitJitterMs'
      | 'killSwitchPollJitterMs'
      | 'artifactPollMs'
      | 'artifactInitJitterMs'
      | 'artifactPollJitterMs'
    >
  > &
    GeneratedEvaluatorRuntimeConfig;

  private artifact: Artifact | null = null;
  private artifactEnv: string | null = null;
  private flagNameMap: Record<string, number> = {};
  private initialized = false;
  private storedAttributes?: Attributes;
  private killSwitchCoordinator = new KillSwitchRefreshCoordinator();
  private artifactCoordinator = new ArtifactRefreshCoordinator();
  private killSwitchLogger?: Logger;
  private artifactLogger?: Logger;
  private stopKillSwitchPoll?: () => void;
  private stopArtifactPoll?: () => void;

  constructor(config: GeneratedEvaluatorRuntimeConfig) {
    this.config = {
      killSwitchPollMs: DEFAULT_GENERATED_KILL_SWITCH_POLL_MS,
      killSwitchInitJitterMs: DEFAULT_GENERATED_KILL_SWITCH_INIT_JITTER_MS,
      killSwitchPollJitterMs: DEFAULT_GENERATED_KILL_SWITCH_POLL_JITTER_MS,
      artifactPollMs: DEFAULT_GENERATED_ARTIFACT_POLL_MS,
      artifactInitJitterMs: DEFAULT_GENERATED_ARTIFACT_INIT_JITTER_MS,
      artifactPollJitterMs: DEFAULT_GENERATED_ARTIFACT_POLL_JITTER_MS,
      ...config,
    };
  }

  async init(options?: GeneratedEvaluatorInitOptions): Promise<void> {
    this.stopKillSwitchPolling();
    this.stopArtifactPolling();
    this.killSwitchLogger = options?.logger;
    this.artifactLogger = options?.logger;
    this.killSwitchCoordinator.reset();
    this.artifactCoordinator.reset();

    if (options?.artifact) {
      const artifactSource = options.artifact;
      if (artifactSource.startsWith('http://') || artifactSource.startsWith('https://')) {
        const loaded = await loadFromURL(artifactSource, undefined, options.logger);
        this.applyInitialArtifact(loaded.artifact, artifactSource, loaded.etag);
      } else {
        const loadedArtifact = await loadFromFile(artifactSource);
        this.applyInitialArtifact(loadedArtifact, artifactSource);
      }

      setTimeout(
        () => void this.refreshKillSwitch(),
        killSwitchInitDelayMs(this.config.killSwitchInitJitterMs)
      );
      this.stopKillSwitchPoll = startKillSwitchPoll(
        () => this.refreshKillSwitch(),
        this.config.killSwitchPollMs,
        { jitterMs: this.config.killSwitchPollJitterMs }
      );

      if (this.artifactEnv !== null && this.config.artifactUrls[this.artifactEnv] !== undefined) {
        setTimeout(
          () => void this.refreshArtifact(),
          pollInitDelayMs(this.config.artifactInitJitterMs)
        );
        this.stopArtifactPoll = startJitteredPoll(
          () => this.refreshArtifact(),
          this.config.artifactPollMs,
          { jitterMs: this.config.artifactPollJitterMs }
        );
      }
    }

    this.initialized = true;
  }

  async ensureInitialized(): Promise<void> {
    if (!this.initialized) {
      await this.init();
    }
  }

  setAttributes(attributes: Attributes): void {
    this.storedAttributes = attributes;
  }

  clearAttributes(): void {
    this.storedAttributes = undefined;
  }

  stopKillSwitchPolling(): void {
    if (this.stopKillSwitchPoll) {
      this.stopKillSwitchPoll();
      this.stopKillSwitchPoll = undefined;
    }
  }

  stopArtifactPolling(): void {
    if (this.stopArtifactPoll) {
      this.stopArtifactPoll();
      this.stopArtifactPoll = undefined;
    }
  }

  getArtifact(): Artifact | null {
    return this.artifact;
  }

  getFlagNameMap(): Readonly<Record<string, number>> {
    return this.flagNameMap;
  }

  /**
   * Resolve a boolean flag using kill switch → AST → catalog default order.
   */
  evaluateBooleanFlag(options: EvaluateBooleanFlagOptions): boolean {
    const attributes = this.resolveAttributes(options.attributes);
    if (!attributes) {
      return options.catalogDefault;
    }

    if (!this.artifact) {
      return options.catalogDefault;
    }

    const flagIndex = this.flagNameMap[options.qualifiedName];
    if (flagIndex === undefined) {
      return options.catalogDefault;
    }

    try {
      return resolveBooleanFlag({
        qualifiedName: options.qualifiedName,
        flagIndex,
        artifact: this.artifact,
        catalogDefault: options.catalogDefault,
        killSwitchFile: this.killSwitchCoordinator.getState().file,
        attributes,
      });
    } catch {
      return options.catalogDefault;
    }
  }

  /** Refresh kill switch file for the current artifact environment (also used by poll loop). */
  async refreshKillSwitch(): Promise<void> {
    const env = this.artifact?.env;
    if (!env) {
      return;
    }
    const url = this.config.killSwitchUrls[env];
    if (!url) {
      return;
    }

    await this.killSwitchCoordinator.refresh(url, undefined, this.killSwitchLogger);
  }

  /** Refresh compiled artifact for the current environment (also used by poll loop). */
  async refreshArtifact(): Promise<void> {
    const env = this.artifact?.env;
    if (!env) {
      return;
    }
    const url = this.config.artifactUrls[env];
    if (!url || !this.artifact) {
      return;
    }

    const expectedEnv = resolveExpectedArtifactEnv(url, this.artifact, this.config.artifactUrls);
    const result = await this.artifactCoordinator.refresh(
      url,
      expectedEnv,
      this.config.sdkQualifiedFlagNames,
      undefined,
      this.artifactLogger
    );
    if (result.status === 'updated' && result.state.artifact !== null) {
      this.artifact = result.state.artifact;
      this.artifactEnv = result.state.artifact.env;
      this.flagNameMap = result.state.flagNameMap;
    }
  }

  private applyInitialArtifact(artifact: Artifact, artifactSource: string, etag?: string): void {
    const expectedEnv = resolveExpectedArtifactEnv(
      artifactSource,
      artifact,
      this.config.artifactUrls
    );
    if (shouldValidateArtifactAtInit(this.config.artifactUrls, expectedEnv)) {
      assertArtifactAccepted(artifact, expectedEnv, this.config.sdkQualifiedFlagNames);
    }
    this.artifact = artifact;
    this.artifactEnv = artifact.env;
    this.flagNameMap = buildFlagNameMapFromArtifact(artifact);
    this.artifactCoordinator.seed({
      artifact,
      etag,
      flagNameMap: this.flagNameMap,
    });
  }

  private resolveAttributes(attributes?: Attributes): Attributes | null {
    const resolvedAttributes = attributes ?? this.storedAttributes;

    if (
      !resolvedAttributes ||
      !resolvedAttributes.id ||
      (typeof resolvedAttributes.id === 'string' && resolvedAttributes.id.trim() === '')
    ) {
      return null;
    }

    return resolvedAttributes;
  }
}
