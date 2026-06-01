/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import { resolveBooleanFlag } from './resolve-flag';
import type { Artifact, Attributes, KillSwitchFile } from './types';
import { RuleType, ExpressionType, BinaryOp } from './types';

describe('resolveBooleanFlag', () => {
  const artifact: Artifact = {
    v: '1.0',
    env: 'production',
    strs: ['ON', 'OFF', 'role', 'admin'],
    flags: [
      [[RuleType.SERVE, undefined, 0]],
      [
        [
          RuleType.SERVE,
          [
            ExpressionType.BINARY_OP,
            BinaryOp.EQ,
            [ExpressionType.PROPERTY, 2],
            [ExpressionType.LITERAL, 3],
          ],
          0,
        ],
      ],
    ],
    flagNames: [0, 1],
  };

  const attributes: Attributes = { id: 'user1', role: 'admin' };

  it('returns kill switch value and skips AST when flag is listed', () => {
    const killSwitchFile: KillSwitchFile = {
      version: '2.0',
      flags: { new_dashboard: false },
    };

    const value = resolveBooleanFlag({
      qualifiedName: 'new_dashboard',
      flagIndex: 0,
      artifact,
      catalogDefault: true,
      killSwitchFile,
      attributes,
    });

    expect(value).toBe(false);
  });

  it('evaluates AST rules when flag is not in kill switch file', () => {
    const killSwitchFile: KillSwitchFile = { version: '2.0', flags: {} };

    const value = resolveBooleanFlag({
      qualifiedName: 'new_dashboard',
      flagIndex: 0,
      artifact,
      catalogDefault: false,
      killSwitchFile,
      attributes,
    });

    expect(value).toBe(true);
  });

  it('falls back to catalog default when AST has no match', () => {
    const value = resolveBooleanFlag({
      qualifiedName: 'beta_feature',
      flagIndex: 999,
      artifact,
      catalogDefault: true,
      attributes,
    });

    expect(value).toBe(true);
  });

  it('applies kill switch override for imported qualified flag names', () => {
    const killSwitchFile: KillSwitchFile = {
      version: '2.0',
      flags: { 'platform.emergency_kill_switch': false },
    };

    const value = resolveBooleanFlag({
      qualifiedName: 'platform.emergency_kill_switch',
      flagIndex: 0,
      artifact,
      catalogDefault: true,
      killSwitchFile,
      attributes,
    });

    expect(value).toBe(false);
  });

  it('evaluates imported-namespace flags via AST using qualified flag index', () => {
    const importedArtifact: Artifact = {
      ...artifact,
      strs: [...artifact.strs, 'platform.emergency_kill_switch'],
      flags: [...artifact.flags, [[RuleType.SERVE, undefined, 1]]],
      flagNames: [...artifact.flagNames, 4],
    };

    const value = resolveBooleanFlag({
      qualifiedName: 'platform.emergency_kill_switch',
      flagIndex: 2,
      artifact: importedArtifact,
      catalogDefault: true,
      attributes,
    });

    expect(value).toBe(false);
  });
});
