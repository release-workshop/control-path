/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { Validator } from './validator';

describe('Validator', () => {
  let validator: Validator;

  beforeEach(() => {
    validator = new Validator();
  });

  describe('validateDefinitions', () => {
    it('should validate a valid flag definitions file', () => {
      const validData = {
        flags: [
          {
            name: 'new_dashboard',
            type: 'boolean',
            defaultValue: false,
            description: 'New dashboard UI feature',
          },
        ],
      };

      const result = validator.validateDefinitions('test.yaml', validData);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should reject definitions with missing required fields', () => {
      const invalidData = {
        flags: [
          {
            name: 'new_dashboard',
            // missing type and defaultValue
          },
        ],
      };

      const result = validator.validateDefinitions('test.yaml', invalidData);
      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
      expect(result.errors.some((e) => e.message.includes('required'))).toBe(true);
    });

    it('should reject duplicate flag names', () => {
      const invalidData = {
        flags: [
          {
            name: 'duplicate_flag',
            type: 'boolean',
            defaultValue: false,
          },
          {
            name: 'duplicate_flag',
            type: 'boolean',
            defaultValue: true,
          },
        ],
      };

      const result = validator.validateDefinitions('test.yaml', invalidData);
      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.message.includes('Duplicate flag name'))).toBe(true);
    });

    it('should reject multivariate flags without variations', () => {
      const invalidData = {
        flags: [
          {
            name: 'multivariate_flag',
            type: 'multivariate',
            defaultValue: 'variation_a',
            // missing variations
          },
        ],
      };

      const result = validator.validateDefinitions('test.yaml', invalidData);
      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.message.includes('variations'))).toBe(true);
    });

    it('should reject multivariate flags with duplicate variation names', () => {
      const invalidData = {
        flags: [
          {
            name: 'multivariate_flag',
            type: 'multivariate',
            defaultValue: 'variation_a',
            variations: [
              { name: 'VARIATION_A', value: 'a' },
              { name: 'VARIATION_A', value: 'b' }, // duplicate
            ],
          },
        ],
      };

      const result = validator.validateDefinitions('test.yaml', invalidData);
      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.message.includes('Duplicate variation name'))).toBe(true);
    });

    it('should accept valid multivariate flags with variations', () => {
      const validData = {
        flags: [
          {
            name: 'multivariate_flag',
            type: 'multivariate',
            defaultValue: 'variation_a',
            variations: [
              { name: 'VARIATION_A', value: 'a' },
              { name: 'VARIATION_B', value: 'b' },
            ],
          },
        ],
      };

      const result = validator.validateDefinitions('test.yaml', validData);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should reject invalid flag name patterns', () => {
      const invalidData = {
        flags: [
          {
            name: 'InvalidFlagName', // should be snake_case
            type: 'boolean',
            defaultValue: false,
          },
        ],
      };

      const result = validator.validateDefinitions('test.yaml', invalidData);
      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
    });
  });

  describe('validateDeployment', () => {
    it('should validate a valid deployment file', () => {
      const validData = {
        environment: 'production',
        rules: {
          new_dashboard: {
            default: false,
            rules: [
              {
                name: 'Admin Users',
                when: "user.role == 'admin'",
                serve: true,
              },
            ],
          },
        },
      };

      const result = validator.validateDeployment('test.yaml', validData);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should reject deployments with missing required fields', () => {
      const invalidData = {
        // missing environment and rules
      };

      const result = validator.validateDeployment('test.yaml', invalidData);
      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
      expect(result.errors.some((e) => e.message.includes('required'))).toBe(true);
    });

    it('should reject rules without serve, variations, or rollout', () => {
      const invalidData = {
        environment: 'production',
        rules: {
          new_dashboard: {
            default: false,
            rules: [
              {
                name: 'Invalid Rule',
                when: "user.role == 'admin'",
                // missing serve, variations, and rollout
              },
            ],
          },
        },
      };

      const result = validator.validateDeployment('test.yaml', invalidData);
      expect(result.valid).toBe(false);
      expect(
        result.errors.some(
          (e) =>
            e.message.includes('serve') ||
            e.message.includes('variations') ||
            e.message.includes('rollout')
        )
      ).toBe(true);
    });

    it('should reject variation weights exceeding 100%', () => {
      const invalidData = {
        environment: 'production',
        rules: {
          multivariate_flag: {
            default: 'variation_a',
            rules: [
              {
                variations: [
                  { variation: 'VARIATION_A', weight: 60 },
                  { variation: 'VARIATION_B', weight: 50 }, // total > 100
                ],
              },
            ],
          },
        },
      };

      const result = validator.validateDeployment('test.yaml', invalidData);
      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.message.includes('exceed 100%'))).toBe(true);
    });

    it('should reject rollout percentage outside 0-100', () => {
      const invalidData = {
        environment: 'production',
        rules: {
          new_dashboard: {
            default: false,
            rules: [
              {
                rollout: {
                  variation: 'variation_a',
                  percentage: 150, // invalid
                },
              },
            ],
          },
        },
      };

      const result = validator.validateDeployment('test.yaml', invalidData);
      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.message.includes('between 0 and 100'))).toBe(true);
    });

    it('should accept valid deployment with variations', () => {
      const validData = {
        environment: 'production',
        rules: {
          multivariate_flag: {
            default: 'variation_a',
            rules: [
              {
                variations: [
                  { variation: 'VARIATION_A', weight: 50 },
                  { variation: 'VARIATION_B', weight: 50 },
                ],
              },
            ],
          },
        },
      };

      const result = validator.validateDeployment('test.yaml', validData);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should accept valid deployment with rollout', () => {
      const validData = {
        environment: 'production',
        rules: {
          new_dashboard: {
            default: false,
            rules: [
              {
                rollout: {
                  variation: 'variation_a',
                  percentage: 50,
                },
              },
            ],
          },
        },
      };

      const result = validator.validateDeployment('test.yaml', validData);
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('formatErrors', () => {
    it('should format errors with file path and line numbers', () => {
      const errors = [
        {
          file: 'test.yaml',
          line: 12,
          column: 5,
          message: 'Missing required field',
          path: '/flags/0/type',
          suggestion: "Add 'type: boolean'",
        },
      ];

      const formatted = validator.formatErrors(errors);
      expect(formatted).toContain('test.yaml:12:5');
      expect(formatted).toContain('Missing required field');
      expect(formatted).toContain("Add 'type: boolean'");
    });

    it('should format errors without line numbers', () => {
      const errors = [
        {
          file: 'test.yaml',
          message: 'Validation error',
        },
      ];

      const formatted = validator.formatErrors(errors);
      expect(formatted).toContain('test.yaml');
      expect(formatted).toContain('Validation error');
    });

    it('should return empty string for no errors', () => {
      const formatted = validator.formatErrors([]);
      expect(formatted).toBe('');
    });
  });
});
