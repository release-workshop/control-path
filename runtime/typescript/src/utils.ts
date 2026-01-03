/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Utility functions for the Control Path runtime SDK.
 */

/**
 * Build a flag name to index map from flag definitions.
 * This helper function is primarily useful for testing or tooling purposes.
 *
 * **Note**: The Provider automatically builds the flag name map from the artifact
 * when `loadArtifact()` is called. You typically don't need this function for
 * normal usage.
 *
 * @param flags - Array of flag definitions with name property
 * @returns Record mapping flag names to their indices
 *
 * @example
 * ```typescript
 * import { buildFlagNameMap } from '@controlpath/runtime';
 *
 * // For testing or tooling
 * const flags = [{ name: 'flag1' }, { name: 'flag2' }];
 * const flagNameMap = buildFlagNameMap(flags);
 * ```
 */
export function buildFlagNameMap(flags: Array<{ name: string }>): Record<string, number> {
  const map: Record<string, number> = {};
  flags.forEach((flag, index) => {
    map[flag.name] = index;
  });
  return map;
}

/**
 * Build a flag name to index map from an AST artifact's flagNames array.
 * This extracts flag names from the artifact's flagNames array and string table.
 *
 * **Note**: The Provider automatically does this when `loadArtifact()` is called.
 * You only need this function for testing or tooling purposes.
 *
 * @param artifact - The AST artifact with flagNames array
 * @returns Record mapping flag names to their indices
 *
 * @example
 * ```typescript
 * import { buildFlagNameMapFromArtifact } from '@controlpath/runtime';
 * import { loadFromFile } from '@controlpath/runtime';
 *
 * // For testing or tooling
 * const artifact = await loadFromFile('production.ast');
 * const flagNameMap = buildFlagNameMapFromArtifact(artifact);
 * ```
 */
export function buildFlagNameMapFromArtifact(artifact: {
  flags: unknown[][];
  flagNames: number[];
  strs: string[];
}): Record<string, number> {
  const map: Record<string, number> = {};
  artifact.flagNames.forEach((nameIndex, flagIndex) => {
    const flagName = artifact.strs[nameIndex];
    if (flagName) {
      map[flagName] = flagIndex;
    }
  });
  return map;
}
