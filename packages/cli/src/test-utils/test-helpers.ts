/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { join } from 'https://deno.land/std@0.208.0/path/mod.ts';

/**
 * Test directory setup result with path and cleanup function.
 */
export interface TestDir {
  path: string;
  cleanup: () => Promise<void>;
}

/**
 * Setup a temporary test directory.
 * Uses system temp directory for isolation.
 *
 * @param suffix - Optional suffix for the temp directory name (e.g., '-validate', '-init')
 * @returns Test directory path and cleanup function
 */
export async function setupTestDir(suffix = ''): Promise<TestDir> {
  const testDir = await Deno.makeTempDir({
    prefix: 'controlpath-test-',
    suffix,
  });

  return {
    path: testDir,
    cleanup: async () => {
      try {
        await Deno.remove(testDir, { recursive: true });
      } catch {
        // Ignore cleanup errors
      }
    },
  };
}

/**
 * Create a test file with content in the current working directory.
 * Creates parent directories if they don't exist.
 *
 * @param path - File path relative to current working directory
 * @param content - File content
 */
export async function createTestFile(path: string, content: string): Promise<void> {
  const dir = join(path, '..');
  if (dir !== '.' && dir !== path) {
    try {
      await Deno.mkdir(dir, { recursive: true });
    } catch {
      // Directory might already exist
    }
  }
  await Deno.writeTextFile(path, content);
}

/**
 * Create multiple test files at once.
 *
 * @param files - Array of { path, content } objects
 */
export async function createTestFiles(
  files: Array<{ path: string; content: string }>,
): Promise<void> {
  for (const file of files) {
    await createTestFile(file.path, file.content);
  }
}

/**
 * Check if a file or directory exists.
 *
 * @param path - Path to check
 * @returns True if the path exists, false otherwise
 */
export async function pathExists(path: string): Promise<boolean> {
  try {
    await Deno.stat(path);
    return true;
  } catch {
    return false;
  }
}

/**
 * Read a test file's content.
 *
 * @param path - File path
 * @returns File content as string
 */
export async function readTestFile(path: string): Promise<string> {
  return await Deno.readTextFile(path);
}

/**
 * Change to a test directory and return a function to restore the original directory.
 *
 * @param testDir - Test directory path
 * @returns Function to restore the original working directory
 */
export function changeToTestDir(testDir: string): () => void {
  const originalCwd = Deno.cwd();
  Deno.chdir(testDir);
  return () => {
    Deno.chdir(originalCwd);
  };
}

/**
 * Setup test environment: create temp directory and change to it.
 * Returns cleanup function that restores original directory and removes temp dir.
 *
 * @param suffix - Optional suffix for the temp directory name
 * @returns Test directory info and cleanup function
 */
export async function setupTestEnvironment(
  suffix = '',
): Promise<{ testDir: string; restore: () => void; cleanup: () => Promise<void> }> {
  const { path: testDir, cleanup: cleanupDir } = await setupTestDir(suffix);
  const restore = changeToTestDir(testDir);

  return {
    testDir,
    restore,
    cleanup: async () => {
      restore();
      await cleanupDir();
    },
  };
}
