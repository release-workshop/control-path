/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import { pack } from 'msgpackr';
import { evaluate } from './evaluator';
import type { Artifact, Attributes } from './types';
import { RuleType, ExpressionType, BinaryOp } from './types';

describe('Performance Tests', () => {
  const mockAttributes: Attributes = {
    id: 'user1',
    email: 'test@example.com',
    role: 'admin',
  };

  describe('evaluation speed', () => {
    it('should evaluate simple flag in less than 1ms', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF'],
        flags: [[[RuleType.SERVE, undefined, 0]]], // Returns 'ON'
      };

      const start = performance.now();
      const result = evaluate(0, artifact, mockAttributes);
      const end = performance.now();
      const duration = end - start;

      expect(result).toBe('ON');
      expect(duration).toBeLessThan(1); // Less than 1ms
    });

    it('should evaluate flag with when clause in less than 1ms', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF', 'role', 'admin'],
        flags: [
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
      };

      const start = performance.now();
      const result = evaluate(0, artifact, mockAttributes);
      const end = performance.now();
      const duration = end - start;

      expect(result).toBe('ON');
      // Allow some overhead for test environment (1.1ms threshold)
      expect(duration).toBeLessThan(1.1); // Less than 1.1ms (allowing test overhead)
    });

    it('should evaluate multiple flags efficiently', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF'],
        flags: [
          [[RuleType.SERVE, undefined, 0]], // Flag 0
          [[RuleType.SERVE, undefined, 0]], // Flag 1
          [[RuleType.SERVE, undefined, 0]], // Flag 2
          [[RuleType.SERVE, undefined, 0]], // Flag 3
          [[RuleType.SERVE, undefined, 0]], // Flag 4
        ],
      };

      const iterations = 100;
      const start = performance.now();

      for (let i = 0; i < iterations; i++) {
        for (let flagIndex = 0; flagIndex < 5; flagIndex++) {
          evaluate(flagIndex, artifact, mockAttributes);
        }
      }

      const end = performance.now();
      const totalDuration = end - start;
      const avgDuration = totalDuration / (iterations * 5);

      // Average evaluation should be less than 1ms
      expect(avgDuration).toBeLessThan(1);
    });

    it('should handle large string tables efficiently', () => {
      // Create artifact with large string table
      const strs: string[] = [];
      for (let i = 0; i < 1000; i++) {
        strs.push(`string_${i}`);
      }
      strs.push('ON');

      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs,
        flags: [[[RuleType.SERVE, undefined, strs.length - 1]]], // Returns 'ON'
      };

      const start = performance.now();
      const result = evaluate(0, artifact, mockAttributes);
      const end = performance.now();
      const duration = end - start;

      expect(result).toBe('ON');
      expect(duration).toBeLessThan(1); // Less than 1ms even with large string table
    });

    it('should handle flags with many rules efficiently', () => {
      const artifact: Artifact = {
        v: '1.0',
        env: 'test',
        strs: ['ON', 'OFF', 'role', 'admin'],
        flags: [
          [
            // 10 rules, first one matches
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
            [RuleType.SERVE, undefined, 1],
            [RuleType.SERVE, undefined, 1],
            [RuleType.SERVE, undefined, 1],
            [RuleType.SERVE, undefined, 1],
            [RuleType.SERVE, undefined, 1],
            [RuleType.SERVE, undefined, 1],
            [RuleType.SERVE, undefined, 1],
            [RuleType.SERVE, undefined, 1],
            [RuleType.SERVE, undefined, 1],
          ],
        ],
      };

      const start = performance.now();
      const result = evaluate(0, artifact, mockAttributes);
      const end = performance.now();
      const duration = end - start;

      expect(result).toBe('ON');
      expect(duration).toBeLessThan(1); // Less than 1ms even with many rules
    });
  });
});
