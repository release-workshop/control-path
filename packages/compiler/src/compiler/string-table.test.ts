/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import { StringTable } from './string-table';
import { ExpressionType } from '../ast';

describe('StringTable', () => {
  describe('add', () => {
    it('should add strings and return indices', () => {
      const table = new StringTable();
      expect(table.add('ON')).toBe(0);
      expect(table.add('OFF')).toBe(1);
      expect(table.add('admin')).toBe(2);
    });

    it('should deduplicate strings', () => {
      const table = new StringTable();
      const index1 = table.add('ON');
      const index2 = table.add('ON');
      expect(index1).toBe(index2);
      expect(table.size()).toBe(1);
    });

    it('should handle empty strings', () => {
      const table = new StringTable();
      expect(table.add('')).toBe(0);
      expect(table.size()).toBe(1);
    });
  });

  describe('get', () => {
    it('should retrieve strings by index', () => {
      const table = new StringTable();
      const index = table.add('test');
      expect(table.get(index)).toBe('test');
    });

    it('should retrieve multiple strings', () => {
      const table = new StringTable();
      const index1 = table.add('first');
      const index2 = table.add('second');
      expect(table.get(index1)).toBe('first');
      expect(table.get(index2)).toBe('second');
    });
  });

  describe('toArray', () => {
    it('should return all strings as array', () => {
      const table = new StringTable();
      table.add('ON');
      table.add('OFF');
      table.add('admin');
      expect(table.toArray()).toEqual(['ON', 'OFF', 'admin']);
    });

    it('should return empty array when no strings added', () => {
      const table = new StringTable();
      expect(table.toArray()).toEqual([]);
    });
  });

  describe('size', () => {
    it('should return correct size', () => {
      const table = new StringTable();
      expect(table.size()).toBe(0);
      table.add('first');
      expect(table.size()).toBe(1);
      table.add('second');
      expect(table.size()).toBe(2);
      table.add('first'); // duplicate
      expect(table.size()).toBe(2);
    });
  });

  describe('processExpression', () => {
    it('should convert property strings to indices', () => {
      const table = new StringTable();
      const expr: any = [ExpressionType.PROPERTY, 'user.role'];
      const processed = table.processExpression(expr);
      expect(processed).toEqual([ExpressionType.PROPERTY, 0]);
      expect(table.get(0)).toBe('user.role');
    });

    it('should convert string literals to indices', () => {
      const table = new StringTable();
      const expr: any = [ExpressionType.LITERAL, 'admin'];
      const processed = table.processExpression(expr);
      expect(processed).toEqual([ExpressionType.LITERAL, 0]);
      expect(table.get(0)).toBe('admin');
    });

    it('should preserve number literals', () => {
      const table = new StringTable();
      const expr: any = [ExpressionType.LITERAL, 42];
      const processed = table.processExpression(expr);
      expect(processed).toEqual([ExpressionType.LITERAL, 42]);
    });

    it('should preserve boolean literals', () => {
      const table = new StringTable();
      const expr: any = [ExpressionType.LITERAL, true];
      const processed = table.processExpression(expr);
      expect(processed).toEqual([ExpressionType.LITERAL, true]);
    });

    it('should process binary op expressions', () => {
      const table = new StringTable();
      const expr: any = [
        ExpressionType.BINARY_OP,
        0, // EQ
        [ExpressionType.PROPERTY, 'user.role'],
        [ExpressionType.LITERAL, 'admin'],
      ];
      const processed = table.processExpression(expr);
      expect(processed[0]).toBe(ExpressionType.BINARY_OP);
      expect(processed[1]).toBe(0);
      expect(processed[2]).toEqual([ExpressionType.PROPERTY, 0]);
      expect(processed[3]).toEqual([ExpressionType.LITERAL, 1]);
      expect(table.get(0)).toBe('user.role');
      expect(table.get(1)).toBe('admin');
    });

    it('should process logical op expressions', () => {
      const table = new StringTable();
      const expr: any = [
        ExpressionType.LOGICAL_OP,
        6, // AND
        [ExpressionType.PROPERTY, 'user.role'],
        [ExpressionType.PROPERTY, 'user.active'],
      ];
      const processed = table.processExpression(expr);
      expect(processed[0]).toBe(ExpressionType.LOGICAL_OP);
      expect(processed[1]).toBe(6);
      expect(processed[2]).toEqual([ExpressionType.PROPERTY, 0]);
      expect(processed[3]).toEqual([ExpressionType.PROPERTY, 1]);
      expect(table.get(0)).toBe('user.role');
      expect(table.get(1)).toBe('user.active');
    });

    it('should process NOT expressions (no right operand)', () => {
      const table = new StringTable();
      const expr: any = [
        ExpressionType.LOGICAL_OP,
        8, // NOT
        [ExpressionType.PROPERTY, 'user.guest'],
      ];
      const processed = table.processExpression(expr);
      expect(processed[0]).toBe(ExpressionType.LOGICAL_OP);
      expect(processed[1]).toBe(8);
      expect(processed[2]).toEqual([ExpressionType.PROPERTY, 0]);
      expect(processed.length).toBe(3); // No right operand
      expect(table.get(0)).toBe('user.guest');
    });

    it('should process nested expressions', () => {
      const table = new StringTable();
      const expr: any = [
        ExpressionType.LOGICAL_OP,
        6, // AND
        [
          ExpressionType.BINARY_OP,
          0, // EQ
          [ExpressionType.PROPERTY, 'user.role'],
          [ExpressionType.LITERAL, 'admin'],
        ],
        [
          ExpressionType.BINARY_OP,
          0, // EQ
          [ExpressionType.PROPERTY, 'user.subscription_tier'],
          [ExpressionType.LITERAL, 'premium'],
        ],
      ];
      const processed = table.processExpression(expr);
      expect(processed[0]).toBe(ExpressionType.LOGICAL_OP);
      // Check that all strings were converted
      expect(table.toArray()).toContain('user.role');
      expect(table.toArray()).toContain('admin');
      expect(table.toArray()).toContain('user.subscription_tier');
      expect(table.toArray()).toContain('premium');
    });

    it('should handle already-processed expressions (with indices)', () => {
      const table = new StringTable();
      // Expression already has indices
      const expr: any = [ExpressionType.PROPERTY, 0];
      const processed = table.processExpression(expr);
      expect(processed).toEqual([ExpressionType.PROPERTY, 0]);
    });

    it('should deduplicate strings across expression', () => {
      const table = new StringTable();
      const expr: any = [
        ExpressionType.BINARY_OP,
        0, // EQ
        [ExpressionType.PROPERTY, 'user.role'],
        [ExpressionType.LITERAL, 'user.role'], // Same string as property
      ];
      const processed = table.processExpression(expr);
      // Both should reference the same index
      expect(processed[2]).toEqual([ExpressionType.PROPERTY, 0]);
      expect(processed[3]).toEqual([ExpressionType.LITERAL, 0]);
      expect(table.size()).toBe(1);
      expect(table.get(0)).toBe('user.role');
    });
  });
});
