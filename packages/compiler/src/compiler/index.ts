/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { pack } from 'msgpackr';
import { Artifact, Rule, RuleType, Variation, Expression } from '../ast';
import type { FlagDefinitions, FlagDefinition, Deployment, DeploymentRule } from '../parser/types';
import { parseExpression } from './expressions';
import { StringTable } from './string-table';

/**
 * Compile a deployment to an AST artifact.
 *
 * @param deployment - Parsed deployment
 * @param definitions - Parsed flag definitions (for flag order and validation)
 * @returns Compiled AST artifact
 */
export function compile(deployment: Deployment, definitions: FlagDefinitions): Artifact {
  const stringTable = new StringTable();
  const flags: Rule[][] = [];
  const segments: [number, Expression][] = [];

  // Build flag index map from definitions (order matters)
  const flagIndexMap = new Map<string, number>();
  definitions.flags.forEach((flag, index) => {
    flagIndexMap.set(flag.name, index);
  });

  // Initialize flags array (one entry per flag definition)
  definitions.flags.forEach(() => {
    flags.push([]);
  });

  // Compile segments if present
  if (deployment.segments) {
    for (const [segmentName, segmentDef] of Object.entries(deployment.segments)) {
      const segmentExpr = parseExpression(segmentDef.when);
      const processedExpr = stringTable.processExpression(segmentExpr);
      const nameIndex = stringTable.add(segmentName);
      segments.push([nameIndex, processedExpr]);
    }
  }

  // Compile flag rules
  for (const [flagName, flagRules] of Object.entries(deployment.rules)) {
    const flagIndex = flagIndexMap.get(flagName);
    if (flagIndex === undefined) {
      throw new Error(`Flag "${flagName}" not found in flag definitions`);
    }

    const rules: Rule[] = [];

    // Compile each rule
    if (flagRules.rules) {
      for (const rule of flagRules.rules) {
        const compiledRule = compileRule(rule, flagName, definitions.flags[flagIndex], stringTable);
        if (compiledRule) {
          rules.push(compiledRule);
        }
      }
    }

    flags[flagIndex] = rules;
  }

  // Append default serve rule for every flag using its definition defaultValue.
  definitions.flags.forEach((flagDef, flagIndex) => {
    const defaultValue = normalizeValue(flagDef.defaultValue, flagDef);
    const defaultIndex = stringTable.add(String(defaultValue));
    flags[flagIndex].push([RuleType.SERVE, undefined, defaultIndex]);
  });

  // Build flag names array (string table indices) for automatic flag name map inference
  // This allows the runtime SDK to automatically build the flagNameMap without requiring
  // the flag definitions file at runtime.
  const flagNames: number[] = [];
  definitions.flags.forEach((flagDef) => {
    const nameIndex = stringTable.add(flagDef.name);
    flagNames.push(nameIndex);
  });

  // Build artifact
  const artifact: Artifact = {
    v: '1.0',
    env: deployment.environment,
    strs: stringTable.toArray(),
    flags,
    flagNames,
  };

  // Add segments if present
  if (segments.length > 0) {
    artifact.segments = segments;
  }

  return artifact;
}

/**
 * Compile a single deployment rule to an AST rule.
 */
function compileRule(
  rule: DeploymentRule,
  flagName: string,
  flagDef: FlagDefinition,
  stringTable: StringTable
): Rule | null {
  // Parse when clause if present
  let whenExpr: Expression | undefined;
  if (rule.when) {
    const parsedExpr = parseExpression(rule.when);
    whenExpr = stringTable.processExpression(parsedExpr);
  }

  // Compile serve rule
  if (rule.serve !== undefined) {
    const value = normalizeValue(rule.serve, flagDef);
    const valueIndex = stringTable.add(String(value));
    return whenExpr !== undefined
      ? [RuleType.SERVE, whenExpr, valueIndex]
      : [RuleType.SERVE, undefined, valueIndex];
  }

  // Compile variations rule
  if (rule.variations && rule.variations.length > 0) {
    const variations: Variation[] = [];
    for (const variation of rule.variations) {
      if (!flagDef.variations) {
        throw new Error(
          `Flag "${flagName}" does not have variations defined, but rule uses variations`
        );
      }

      const varDef = flagDef.variations.find((v) => v.name === variation.variation);
      if (!varDef) {
        throw new Error(`Variation "${variation.variation}" not found in flag "${flagName}"`);
      }

      const varValue = String(varDef.value);
      const varIndex = stringTable.add(varValue);
      variations.push([varIndex, Math.round(variation.weight)]);
    }

    return whenExpr !== undefined
      ? [RuleType.VARIATIONS, whenExpr, variations]
      : [RuleType.VARIATIONS, undefined, variations];
  }

  // Compile rollout rule
  if (rule.rollout) {
    let valueIndex: number;

    if (flagDef.type === 'boolean') {
      // For boolean flags, rollout.variation is the value (ON/OFF), not a variation name
      const value = normalizeValue(rule.rollout.variation, flagDef);
      valueIndex = stringTable.add(value);
    } else {
      // For multivariate flags, rollout.variation is a variation name
      if (!flagDef.variations) {
        throw new Error(
          `Flag "${flagName}" does not have variations defined, but rule uses rollout`
        );
      }

      const varDef = flagDef.variations.find((v) => v.name === rule.rollout!.variation);
      if (!varDef) {
        throw new Error(`Variation "${rule.rollout.variation}" not found in flag "${flagName}"`);
      }

      const varValue = String(varDef.value);
      valueIndex = stringTable.add(varValue);
    }

    const percentage = Math.max(0, Math.min(100, Math.round(rule.rollout.percentage)));

    return whenExpr !== undefined
      ? [RuleType.ROLLOUT, whenExpr, [valueIndex, percentage]]
      : [RuleType.ROLLOUT, undefined, [valueIndex, percentage]];
  }

  // No valid rule type found
  return null;
}

/**
 * Normalize a flag value to a string representation.
 * For boolean flags, converts boolean to string.
 */
function normalizeValue(value: unknown, flagDef: FlagDefinition): string {
  if (flagDef.type === 'boolean') {
    // For boolean flags, normalize to string representation
    if (typeof value === 'boolean') {
      return value ? 'ON' : 'OFF';
    }
    if (typeof value === 'string') {
      const upper = value.toUpperCase();
      if (upper === 'ON' || upper === 'TRUE' || upper === '1') {
        return 'ON';
      }
      if (upper === 'OFF' || upper === 'FALSE' || upper === '0') {
        return 'OFF';
      }
    }
  }

  return String(value);
}

/**
 * Serialize an artifact to MessagePack bytes.
 *
 * @param artifact - Compiled AST artifact
 * @returns MessagePack-encoded bytes
 */
export function serialize(artifact: Artifact): Uint8Array {
  return pack(artifact);
}

/**
 * Compile and serialize a deployment to MessagePack bytes.
 *
 * @param deployment - Parsed deployment
 * @param definitions - Parsed flag definitions
 * @returns MessagePack-encoded AST artifact
 */
export function compileAndSerialize(
  deployment: Deployment,
  definitions: FlagDefinitions
): Uint8Array {
  const artifact = compile(deployment, definitions);
  return serialize(artifact);
}
