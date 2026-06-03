/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Boolean flag resolution: kill switch file → AST → catalog default.
 */

import { evaluateBoolean } from './evaluator';
import type { Artifact, AttributesInput, KillSwitchFile } from './types';

export interface ResolveBooleanFlagOptions {
  /** Qualified flag name (`flag` or `namespace.flag`). */
  qualifiedName: string;
  /** Index in the artifact flags array. */
  flagIndex: number;
  artifact: Artifact;
  /** Catalog default when kill switch and AST do not apply. */
  catalogDefault: boolean;
  killSwitchFile?: KillSwitchFile | null;
  attributes?: AttributesInput;
}

function killSwitchValue(
  killSwitchFile: KillSwitchFile | null | undefined,
  flag: string
): boolean | undefined {
  if (!killSwitchFile) {
    return undefined;
  }
  if (Object.prototype.hasOwnProperty.call(killSwitchFile.flags, flag)) {
    return killSwitchFile.flags[flag];
  }
  return undefined;
}

/**
 * Resolve a boolean flag using product evaluation order.
 */
export function resolveBooleanFlag(options: ResolveBooleanFlagOptions): boolean {
  const killValue = killSwitchValue(options.killSwitchFile, options.qualifiedName);
  if (killValue !== undefined) {
    return killValue;
  }

  const astValue = evaluateBoolean(options.flagIndex, options.artifact, options.attributes);
  if (astValue !== undefined) {
    return astValue;
  }

  return options.catalogDefault;
}
