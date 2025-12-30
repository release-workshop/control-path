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
 * This helper function creates the flagNameMap required by the Provider class.
 *
 * @param flags - Array of flag definitions with name property
 * @returns Record mapping flag names to their indices
 *
 * @example
 * ```typescript
 * import { buildFlagNameMap } from '@controlpath/runtime';
 * import { parseDefinitions } from '@controlpath/compiler';
 *
 * const definitions = parseDefinitions('flags.definitions.yaml');
 * const flagNameMap = buildFlagNameMap(definitions.flags);
 *
 * const provider = new Provider({ flagNameMap });
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
 * Build a flag name to index map from an AST artifact.
 * This extracts flag names from the string table if available.
 *
 * Note: This requires flag names to be present in the AST string table.
 * For best results, use buildFlagNameMap with flag definitions instead.
 *
 * @param artifact - The AST artifact
 * @param flagNames - Array of flag names in the same order as flags in the artifact
 * @returns Record mapping flag names to their indices
 *
 * @example
 * ```typescript
 * import { buildFlagNameMapFromArtifact } from '@controlpath/runtime';
 *
 * const artifact = await loadFromFile('production.ast');
 * const flagNames = ['new_dashboard', 'enable_analytics', 'theme_color'];
 * const flagNameMap = buildFlagNameMapFromArtifact(artifact, flagNames);
 *
 * const provider = new Provider({ flagNameMap });
 * ```
 */
export function buildFlagNameMapFromArtifact(
  artifact: { flags: unknown[][] },
  flagNames: string[]
): Record<string, number> {
  const map: Record<string, number> = {};
  flagNames.forEach((name, index) => {
    if (index < artifact.flags.length) {
      map[name] = index;
    }
  });
  return map;
}
