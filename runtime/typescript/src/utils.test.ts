/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import { buildFlagNameMap, buildFlagNameMapFromArtifact } from './utils';
import type { Artifact } from './types';

describe('utils', () => {
  describe('buildFlagNameMap', () => {
    it('should build flag name map from flag definitions', () => {
      const flags = [{ name: 'flag1' }, { name: 'flag2' }, { name: 'flag3' }];
      const map = buildFlagNameMap(flags);

      expect(map).toEqual({
        flag1: 0,
        flag2: 1,
        flag3: 2,
      });
    });

    it('should handle empty array', () => {
      const flags: Array<{ name: string }> = [];
      const map = buildFlagNameMap(flags);

      expect(map).toEqual({});
    });

    it('should handle single flag', () => {
      const flags = [{ name: 'single-flag' }];
      const map = buildFlagNameMap(flags);

      expect(map).toEqual({
        'single-flag': 0,
      });
    });

    it('should handle flags with duplicate names (last one wins)', () => {
      const flags = [{ name: 'flag1' }, { name: 'flag1' }];
      const map = buildFlagNameMap(flags);

      expect(map).toEqual({
        flag1: 1, // Last index wins
      });
    });
  });

  describe('buildFlagNameMapFromArtifact', () => {
    it('should build flag name map from artifact', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1', 'flag2', 'flag3'],
        flags: [[], [], []],
        flagNames: [0, 1, 2], // flag1, flag2, flag3
      };

      const map = buildFlagNameMapFromArtifact(artifact);

      expect(map).toEqual({
        flag1: 0,
        flag2: 1,
        flag3: 2,
      });
    });

    it('should handle empty flagNames array', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: [],
        flags: [],
        flagNames: [],
      };

      const map = buildFlagNameMapFromArtifact(artifact);

      expect(map).toEqual({});
    });

    it('should handle missing string table entries', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['flag1'],
        flags: [[], []],
        flagNames: [0, 999], // flag1 exists, but index 999 doesn't
      };

      const map = buildFlagNameMapFromArtifact(artifact);

      expect(map).toEqual({
        flag1: 0,
        // flag at index 1 is skipped because strs[999] is undefined
      });
    });

    it('should handle single flag', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['single-flag'],
        flags: [[]],
        flagNames: [0],
      };

      const map = buildFlagNameMapFromArtifact(artifact);

      expect(map).toEqual({
        'single-flag': 0,
      });
    });
  });
});

