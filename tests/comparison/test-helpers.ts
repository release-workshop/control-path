/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { execSync, spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdirSync, rmSync, mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
// Import from compiler package
import { compileAndSerialize, parseDefinitionsFromString, parseDeploymentFromString } from '@controlpath/compiler';
import type { FlagDefinitions, Deployment } from '@controlpath/compiler';

/**
 * Get the path to the Rust CLI binary
 */
function getRustCliPath(): string {
  // Try release build first (faster for repeated runs)
  const releasePath = join(__dirname, '../../target/release/controlpath');
  try {
    readFileSync(releasePath);
    return releasePath;
  } catch {
    // Fall back to debug build
    const debugPath = join(__dirname, '../../target/debug/controlpath');
    try {
      readFileSync(debugPath);
      return debugPath;
    } catch {
      throw new Error(
        'Rust CLI binary not found. Please build it first: cargo build --release --bin controlpath'
      );
    }
  }
}

/**
 * Compile using TypeScript implementation
 */
export function compileTypeScript(
  definitionsYaml: string,
  deploymentYaml: string
): Uint8Array {
  const definitions = parseDefinitionsFromString(definitionsYaml);
  const deployment = parseDeploymentFromString(deploymentYaml);
  return compileAndSerialize(deployment, definitions);
}

/**
 * Compile using Rust implementation via library API
 * This uses the Rust compiler library directly (if we had Node bindings)
 * For now, we'll use the CLI as a workaround
 */
export function compileRust(
  definitionsYaml: string,
  deploymentYaml: string
): Uint8Array {
  // Create temporary directory using mkdtempSync for better security
  const tempDir = mkdtempSync(join(tmpdir(), 'controlpath-test-'));

  try {
    // Write temporary files
    const definitionsPath = join(tempDir, 'flags.definitions.yaml');
    const deploymentPath = join(tempDir, 'deployment.yaml');
    const outputPath = join(tempDir, 'output.ast');

    writeFileSync(definitionsPath, definitionsYaml, 'utf-8');
    writeFileSync(deploymentPath, deploymentYaml, 'utf-8');

    // Run Rust CLI using spawnSync with array-based arguments for better security
    const rustCli = getRustCliPath();
    const result = spawnSync(rustCli, [
      'compile',
      '--definitions', definitionsPath,
      '--deployment', deploymentPath,
      '--output', outputPath,
    ], {
      cwd: tempDir,
      stdio: 'pipe',
      encoding: 'utf-8',
    });

    if (result.error) {
      throw new Error(`Failed to run Rust CLI: ${result.error.message}`);
    }

    if (result.status !== 0) {
      const errorMsg = result.stderr?.toString() || result.stdout?.toString() || 'Unknown error';
      throw new Error(`Rust CLI failed with exit code ${result.status}: ${errorMsg}`);
    }

    // Read output
    const output = readFileSync(outputPath);
    return new Uint8Array(output);
  } finally {
    // Clean up
    rmSync(tempDir, { recursive: true, force: true });
  }
}

/**
 * Compare two Uint8Arrays byte-for-byte
 */
export function compareBytes(a: Uint8Array, b: Uint8Array): {
  equal: boolean;
  differences?: Array<{ offset: number; a: number; b: number }>;
} {
  if (a.length !== b.length) {
    return {
      equal: false,
      differences: [{ offset: -1, a: a.length, b: b.length }],
    };
  }

  const differences: Array<{ offset: number; a: number; b: number }> = [];
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) {
      differences.push({ offset: i, a: a[i], b: b[i] });
      // Limit to first 10 differences for readability
      if (differences.length >= 10) {
        break;
      }
    }
  }

  return {
    equal: differences.length === 0,
    differences: differences.length > 0 ? differences : undefined,
  };
}

/**
 * Format bytes as hex string for debugging
 */
export function formatBytesHex(bytes: Uint8Array, maxLength = 100): string {
  const hex = Array.from(bytes.slice(0, maxLength))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join(' ');
  return bytes.length > maxLength ? `${hex}... (${bytes.length} bytes)` : `${hex} (${bytes.length} bytes)`;
}

/**
 * Run TypeScript CLI command and return output
 * The TypeScript CLI uses Deno, so we use deno run
 */
export function runTypeScriptCli(
  args: string[],
  input?: string,
  options?: { cwd?: string }
): {
  stdout: string;
  stderr: string;
  exitCode: number;
} {
  try {
    // Use environment variable or default path for more robust path resolution
    const cliPath = process.env.CONTROLPATH_CLI_PATH || 
      join(__dirname, '../../packages/cli/src/index.ts');
    
    // Use spawnSync with array-based arguments for better security
    const result = spawnSync('deno', [
      'run',
      '--allow-read',
      '--allow-write',
      '--allow-net',
      cliPath,
      ...args,
    ], {
      input,
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'pipe'],
      cwd: options?.cwd,
    });

    if (result.error) {
      return {
        stdout: '',
        stderr: result.error.message,
        exitCode: 1,
      };
    }

    return {
      stdout: result.stdout?.toString() || '',
      stderr: result.stderr?.toString() || '',
      exitCode: result.status || 0,
    };
  } catch (error: any) {
    return {
      stdout: '',
      stderr: error.message || 'Unknown error',
      exitCode: 1,
    };
  }
}

/**
 * Run Rust CLI command and return output
 */
export function runRustCli(
  args: string[],
  input?: string,
  options?: { cwd?: string }
): {
  stdout: string;
  stderr: string;
  exitCode: number;
} {
  try {
    const rustCli = getRustCliPath();
    // Use spawnSync with array-based arguments for better security
    const result = spawnSync(rustCli, args, {
      input,
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'pipe'],
      cwd: options?.cwd,
    });

    if (result.error) {
      return {
        stdout: '',
        stderr: result.error.message,
        exitCode: 1,
      };
    }

    return {
      stdout: result.stdout?.toString() || '',
      stderr: result.stderr?.toString() || '',
      exitCode: result.status || 0,
    };
  } catch (error: any) {
    return {
      stdout: '',
      stderr: error.message || 'Unknown error',
      exitCode: 1,
    };
  }
}

/**
 * Create a temporary test directory with files
 */
export function createTempTestDir(files: Record<string, string>): {
  path: string;
  cleanup: () => void;
} {
  // Use mkdtempSync for better security and to avoid race conditions
  const tempDir = mkdtempSync(join(tmpdir(), 'controlpath-test-'));

  for (const [filename, content] of Object.entries(files)) {
    const filePath = join(tempDir, filename);
    const dir = join(filePath, '..');
    mkdirSync(dir, { recursive: true });
    writeFileSync(filePath, content, 'utf-8');
  }

  return {
    path: tempDir,
    cleanup: () => {
      rmSync(tempDir, { recursive: true, force: true });
    },
  };
}

