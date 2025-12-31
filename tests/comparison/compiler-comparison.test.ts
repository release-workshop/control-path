/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import {
  compileTypeScript,
  compileRust,
  compareBytes,
  formatBytesHex,
} from './test-helpers';

/**
 * Helper function to compare TypeScript and Rust compiler output with detailed error messages
 */
function expectIdenticalOutput(
  definitions: string,
  deployment: string,
  testDescription?: string
): void {
  const tsBytes = compileTypeScript(definitions, deployment);
  const rustBytes = compileRust(definitions, deployment);

  const comparison = compareBytes(tsBytes, rustBytes);
  
  if (!comparison.equal) {
    console.error('TypeScript bytes:', formatBytesHex(tsBytes));
    console.error('Rust bytes:', formatBytesHex(rustBytes));
    console.error('Differences:', comparison.differences);
  }
  
  // Include comparison details in assertion message for better debugging
  const diffSummary = comparison.differences
    ?.slice(0, 5)
    .map(d => d.offset === -1 
      ? `length mismatch: TS=${d.a} Rust=${d.b}`
      : `offset ${d.offset}: TS=0x${d.a.toString(16).padStart(2, '0')} Rust=0x${d.b.toString(16).padStart(2, '0')}`
    )
    .join(', ') || 'unknown differences';
  
  expect(comparison.equal).toBe(
    true,
    testDescription 
      ? `${testDescription}: Outputs differ: ${diffSummary}`
      : `Outputs differ: ${diffSummary}`
  );
}

/**
 * Comparison tests for TypeScript and Rust compiler implementations.
 * These tests ensure both implementations produce identical MessagePack output.
 */

describe('Compiler Comparison Tests', () => {
  beforeAll(() => {
    // Verify Rust CLI is available
    try {
      compileRust('flags: []', 'environment: test\nrules: {}');
    } catch (error: any) {
      throw new Error(
        `Rust CLI not available. Please build it first: cargo build --release --bin controlpath\n${error.message}`
      );
    }
  });

  describe('Basic Compilation', () => {
    it('should produce identical output for simple boolean flag with no rules', () => {
      const definitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  test_flag: {}
`;

      expectIdenticalOutput(definitions, deployment);
    });

    it('should produce identical output for multiple flags', () => {
      const definitions = `flags:
  - name: flag1
    type: boolean
    defaultValue: false
  - name: flag2
    type: boolean
    defaultValue: true
  - name: flag3
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: staging
rules:
  flag1: {}
  flag2: {}
  flag3: {}
`;

      expectIdenticalOutput(definitions, deployment);
    });
  });

  describe('Serve Rules', () => {
    it('should produce identical output for serve rule without when clause', () => {
      const definitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  test_flag:
    rules:
      - serve: true
`;

      expectIdenticalOutput(definitions, deployment);
    });

    it('should produce identical output for serve rule with when clause', () => {
      const definitions = `context:
  user:
    role: 'string'

flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  test_flag:
    rules:
      - when: user.role == 'admin'
        serve: true
`;

      expectIdenticalOutput(definitions, deployment);
    });

    it('should produce identical output for serve rule with complex expression', () => {
      const definitions = `context:
  user:
    age: 'number'
    role: 'string'

flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  test_flag:
    rules:
      - when: user.role == 'admin' AND user.age >= 18
        serve: true
`;

      expectIdenticalOutput(definitions, deployment);
    });
  });

  describe('Variations Rules', () => {
    it('should produce identical output for variations rule without when clause', () => {
      const definitions = `flags:
  - name: theme_color
    type: multivariate
    defaultValue: blue
    variations:
      - name: BLUE
        value: "blue"
      - name: GREEN
        value: "green"
      - name: RED
        value: "red"
`;

      const deployment = `environment: production
rules:
  theme_color:
    rules:
      - variations:
          - variation: BLUE
            weight: 50
          - variation: GREEN
            weight: 50
`;

      expectIdenticalOutput(definitions, deployment);
    });

    it('should produce identical output for variations rule with when clause', () => {
      const definitions = `context:
  user:
    plan: 'string'

flags:
  - name: theme_color
    type: multivariate
    defaultValue: blue
    variations:
      - name: BLUE
        value: "blue"
      - name: GREEN
        value: "green"
`;

      const deployment = `environment: production
rules:
  theme_color:
    rules:
      - when: user.plan == 'premium'
        variations:
          - variation: GREEN
            weight: 100
`;

      expectIdenticalOutput(definitions, deployment);
    });
  });

  describe('Rollout Rules', () => {
    it('should produce identical output for rollout rule on boolean flag', () => {
      const definitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  test_flag:
    rules:
      - rollout:
          variation: true
          percentage: 25
`;

      expectIdenticalOutput(definitions, deployment);
    });

    it('should produce identical output for rollout rule on multivariate flag', () => {
      const definitions = `flags:
  - name: theme_color
    type: multivariate
    defaultValue: blue
    variations:
      - name: BLUE
        value: "blue"
      - name: GREEN
        value: "green"
`;

      const deployment = `environment: production
rules:
  theme_color:
    rules:
      - rollout:
          variation: GREEN
          percentage: 50
`;

      expectIdenticalOutput(definitions, deployment);
    });

    it('should produce identical output for rollout rule with when clause', () => {
      const definitions = `context:
  user:
    country: 'string'

flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  test_flag:
    rules:
      - when: user.country == 'US'
        rollout:
          variation: true
          percentage: 75
`;

      expectIdenticalOutput(definitions, deployment);
    });
  });

  describe('Segments', () => {
    it('should produce identical output for deployment with segments', () => {
      const definitions = `context:
  user:
    plan: 'string'

flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
segments:
  premium_users:
    when: user.plan == 'premium'
rules:
  test_flag:
    rules:
      - when: IN_SEGMENT('premium_users')
        serve: true
`;

      expectIdenticalOutput(definitions, deployment);
    });

    it('should produce identical output for multiple segments', () => {
      const definitions = `context:
  user:
    role: 'string'
    plan: 'string'

flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
segments:
  admins:
    when: user.role == 'admin'
  premium_users:
    when: user.plan == 'premium'
rules:
  test_flag:
    rules:
      - when: IN_SEGMENT('admins') OR IN_SEGMENT('premium_users')
        serve: true
`;

      expectIdenticalOutput(definitions, deployment);
    });
  });

  describe('Expression Functions', () => {
    it('should produce identical output for STARTS_WITH function', () => {
      const definitions = `context:
  user:
    email: 'string'

flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  test_flag:
    rules:
      - when: STARTS_WITH(user.email, 'admin@')
        serve: true
`;

      expectIdenticalOutput(definitions, deployment);
    });

    it('should produce identical output for IN function', () => {
      const definitions = `context:
  user:
    country: 'string'

flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  test_flag:
    rules:
      - when: user.country IN ['US', 'CA', 'UK']
        serve: true
`;

      expectIdenticalOutput(definitions, deployment);
    });

    it('should produce identical output for complex nested expressions', () => {
      const definitions = `context:
  user:
    age: 'number'
    role: 'string'
    country: 'string'

flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  test_flag:
    rules:
      - when: (user.role == 'admin' OR user.role == 'moderator') AND user.age >= 18 AND user.country IN ['US', 'CA']
        serve: true
`;

      expectIdenticalOutput(definitions, deployment);
    });
  });

  describe('String Table Deduplication', () => {
    it('should produce identical output when strings are reused', () => {
      const definitions = `context:
  user:
    role: 'string'

flags:
  - name: flag1
    type: boolean
    defaultValue: false
  - name: flag2
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  flag1:
    rules:
      - when: user.role == 'admin'
        serve: true
  flag2:
    rules:
      - when: user.role == 'admin'
        serve: true
`;

      expectIdenticalOutput(definitions, deployment);
    });
  });

  describe('Flag Ordering', () => {
    it('should produce identical output respecting flag definition order', () => {
      const definitions = `flags:
  - name: flag1
    type: boolean
    defaultValue: false
  - name: flag2
    type: boolean
    defaultValue: false
  - name: flag3
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  flag3: {}
  flag1: {}
  flag2: {}
`;

      expectIdenticalOutput(definitions, deployment);
    });
  });

  describe('Edge Cases', () => {
    it('should produce identical output for empty rules', () => {
      const definitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  test_flag: {}
`;

      expectIdenticalOutput(definitions, deployment);
    });

    it('should produce identical output for multiple rules on same flag', () => {
      const definitions = `context:
  user:
    role: 'string'
    plan: 'string'

flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

      const deployment = `environment: production
rules:
  test_flag:
    rules:
      - when: user.role == 'admin'
        serve: true
      - when: user.plan == 'premium'
        serve: true
      - serve: false
`;

      expectIdenticalOutput(definitions, deployment);
    });
  });
});

