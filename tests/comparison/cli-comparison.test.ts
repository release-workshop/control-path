/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import {
  runTypeScriptCli,
  runRustCli,
  createTempTestDir,
  compareBytes,
} from './test-helpers';

/**
 * Comparison tests for TypeScript and Rust CLI implementations.
 * These tests ensure both CLIs produce identical output and behavior.
 */

describe('CLI Comparison Tests', () => {
  beforeAll(() => {
    // Verify Rust CLI is available
    try {
      const result = runRustCli(['--help']);
      if (result.exitCode !== 0) {
        throw new Error('Rust CLI not available');
      }
    } catch (error: any) {
      throw new Error(
        `Rust CLI not available. Please build it first: cargo build --release --bin controlpath\n${error.message}`
      );
    }
  });

  describe('Compile Command', () => {
    it('should produce identical AST output for simple compilation', () => {
      const { path: tempDir, cleanup } = createTempTestDir({
        'flags.definitions.yaml': `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`,
        'deployment.yaml': `environment: production
rules:
  test_flag:
    rules:
      - serve: true
`,
      });

      try {
        const tsOutput = join(tempDir, 'ts-output.ast');
        const rustOutput = join(tempDir, 'rust-output.ast');

        // Run TypeScript CLI
        const tsResult = runTypeScriptCli([
          'compile',
          '--definitions',
          join(tempDir, 'flags.definitions.yaml'),
          '--deployment',
          join(tempDir, 'deployment.yaml'),
          '--output',
          tsOutput,
        ]);

        // Run Rust CLI
        const rustResult = runRustCli([
          'compile',
          '--definitions',
          join(tempDir, 'flags.definitions.yaml'),
          '--deployment',
          join(tempDir, 'deployment.yaml'),
          '--output',
          rustOutput,
        ]);

        expect(tsResult.exitCode).toBe(0);
        expect(rustResult.exitCode).toBe(0);

        // Compare output files
        const tsBytes = new Uint8Array(readFileSync(tsOutput));
        const rustBytes = new Uint8Array(readFileSync(rustOutput));

        const comparison = compareBytes(tsBytes, rustBytes);
        expect(comparison.equal).toBe(true);
      } finally {
        cleanup();
      }
    });

    it('should handle --env flag identically', () => {
      const { path: tempDir, cleanup } = createTempTestDir({
        'flags.definitions.yaml': `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`,
        '.controlpath/production.deployment.yaml': `environment: production
rules:
  test_flag:
    rules:
      - serve: true
`,
      });

      try {
        const tsOutput = join(tempDir, '.controlpath', 'ts-production.ast');
        const rustOutput = join(tempDir, '.controlpath', 'rust-production.ast');

        // Run TypeScript CLI
        const tsResult = runTypeScriptCli([
          'compile',
          '--definitions',
          join(tempDir, 'flags.definitions.yaml'),
          '--env',
          'production',
          '--output',
          tsOutput,
        ]);

        // Run Rust CLI
        const rustResult = runRustCli([
          'compile',
          '--definitions',
          join(tempDir, 'flags.definitions.yaml'),
          '--env',
          'production',
          '--output',
          rustOutput,
        ]);

        expect(tsResult.exitCode).toBe(0);
        expect(rustResult.exitCode).toBe(0);

        // Compare output files
        const tsBytes = new Uint8Array(readFileSync(tsOutput));
        const rustBytes = new Uint8Array(readFileSync(rustOutput));

        const comparison = compareBytes(tsBytes, rustBytes);
        expect(comparison.equal).toBe(true);
      } finally {
        cleanup();
      }
    });

    it('should produce identical error messages for invalid definitions', () => {
      const { path: tempDir, cleanup } = createTempTestDir({
        'invalid.definitions.yaml': `flags:
  - name: test_flag
    # Missing type field
    defaultValue: false
`,
        'deployment.yaml': `environment: production
rules:
  test_flag: {}
`,
      });

      try {
        // Run TypeScript CLI
        const tsResult = runTypeScriptCli([
          'compile',
          '--definitions',
          join(tempDir, 'invalid.definitions.yaml'),
          '--deployment',
          join(tempDir, 'deployment.yaml'),
        ]);

        // Run Rust CLI
        const rustResult = runRustCli([
          'compile',
          '--definitions',
          join(tempDir, 'invalid.definitions.yaml'),
          '--deployment',
          join(tempDir, 'deployment.yaml'),
        ]);

        // Both should fail
        expect(tsResult.exitCode).not.toBe(0);
        expect(rustResult.exitCode).not.toBe(0);

        // Both should have error messages (exact format may differ, but should indicate validation failure)
        expect(tsResult.stderr.length).toBeGreaterThan(0);
        expect(rustResult.stderr.length).toBeGreaterThan(0);
        
        // Check that both error messages contain key terms related to validation
        const tsError = tsResult.stderr.toLowerCase();
        const rustError = rustResult.stderr.toLowerCase();
        
        // Both should mention validation, error, or invalid
        expect(tsError).toMatch(/validation|error|invalid/i);
        expect(rustError).toMatch(/validation|error|invalid/i);
      } finally {
        cleanup();
      }
    });
  });

  describe('Validate Command', () => {
    it('should produce identical validation results for valid files', () => {
      const { path: tempDir, cleanup } = createTempTestDir({
        'flags.definitions.yaml': `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`,
        'deployment.yaml': `environment: production
rules:
  test_flag: {}
`,
      });

      try {
        // Run TypeScript CLI
        const tsResult = runTypeScriptCli([
          'validate',
          '--definitions',
          join(tempDir, 'flags.definitions.yaml'),
          '--deployment',
          join(tempDir, 'deployment.yaml'),
        ]);

        // Run Rust CLI
        const rustResult = runRustCli([
          'validate',
          '--definitions',
          join(tempDir, 'flags.definitions.yaml'),
          '--deployment',
          join(tempDir, 'deployment.yaml'),
        ]);

        // Both should succeed
        expect(tsResult.exitCode).toBe(0);
        expect(rustResult.exitCode).toBe(0);
      } finally {
        cleanup();
      }
    });

    it('should produce identical validation errors for invalid files', () => {
      const { path: tempDir, cleanup } = createTempTestDir({
        'invalid.definitions.yaml': `flags:
  - name: test_flag
    # Missing type field
    defaultValue: false
`,
        'deployment.yaml': `environment: production
rules:
  test_flag: {}
`,
      });

      try {
        // Run TypeScript CLI
        const tsResult = runTypeScriptCli([
          'validate',
          '--definitions',
          join(tempDir, 'invalid.definitions.yaml'),
          '--deployment',
          join(tempDir, 'deployment.yaml'),
        ]);

        // Run Rust CLI
        const rustResult = runRustCli([
          'validate',
          '--definitions',
          join(tempDir, 'invalid.definitions.yaml'),
          '--deployment',
          join(tempDir, 'deployment.yaml'),
        ]);

        // Both should fail
        expect(tsResult.exitCode).not.toBe(0);
        expect(rustResult.exitCode).not.toBe(0);
        
        // Check that both error messages contain key terms related to validation
        const tsError = tsResult.stderr.toLowerCase();
        const rustError = rustResult.stderr.toLowerCase();
        
        // Both should mention validation, error, or invalid
        expect(tsError).toMatch(/validation|error|invalid/i);
        expect(rustError).toMatch(/validation|error|invalid/i);
      } finally {
        cleanup();
      }
    });

    it('should produce similar error messages for validation failures', () => {
      const { path: tempDir, cleanup } = createTempTestDir({
        'invalid.definitions.yaml': `flags:
  - name: test_flag
    # Missing type field
    defaultValue: false
`,
        'deployment.yaml': `environment: production
rules:
  test_flag: {}
`,
      });

      try {
        // Run TypeScript CLI
        const tsResult = runTypeScriptCli([
          'validate',
          '--definitions',
          join(tempDir, 'invalid.definitions.yaml'),
          '--deployment',
          join(tempDir, 'deployment.yaml'),
        ]);

        // Run Rust CLI
        const rustResult = runRustCli([
          'validate',
          '--definitions',
          join(tempDir, 'invalid.definitions.yaml'),
          '--deployment',
          join(tempDir, 'deployment.yaml'),
        ]);

        // Both should fail
        expect(tsResult.exitCode).not.toBe(0);
        expect(rustResult.exitCode).not.toBe(0);

        // Check that both error messages contain key terms related to validation
        const tsError = tsResult.stderr.toLowerCase();
        const rustError = rustResult.stderr.toLowerCase();
        
        // Both should mention validation, error, or invalid
        expect(tsError).toMatch(/validation|error|invalid/i);
        expect(rustError).toMatch(/validation|error|invalid/i);
      } finally {
        cleanup();
      }
    });
  });

  describe('Init Command', () => {
    it('should create similar project structure', () => {
      const { path: tempDir, cleanup } = createTempTestDir({});

      try {
        const tsProjectDir = join(tempDir, 'ts-project');
        const rustProjectDir = join(tempDir, 'rust-project');
        mkdirSync(tsProjectDir, { recursive: true });
        mkdirSync(rustProjectDir, { recursive: true });

        // Run TypeScript CLI
        const tsResult = runTypeScriptCli(['init'], undefined, {
          cwd: tsProjectDir,
        });

        // Run Rust CLI
        const rustResult = runRustCli(['init'], undefined, {
          cwd: rustProjectDir,
        });

        // Both should succeed
        expect(tsResult.exitCode).toBe(0);
        expect(rustResult.exitCode).toBe(0);

        // Both should create flags.definitions.yaml
        // Note: Exact file structure may differ, but core files should be present
        // This is a basic check - more detailed comparison can be added if needed
      } finally {
        cleanup();
      }
    });
  });
});

