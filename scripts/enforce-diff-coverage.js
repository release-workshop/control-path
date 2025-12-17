#!/usr/bin/env node

/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Differential coverage enforcement.
 *
 * Usage (from repo root):
 *   DIFF_COVERAGE_LCOV=packages/compiler/coverage/lcov.info \
 *   DIFF_COVERAGE_BASE=main \
 *   node scripts/enforce-diff-coverage.js
 *
 * The script:
 * - Computes changed lines between <base> (default: main) and HEAD
 * - Parses an lcov.info coverage report
 * - Ensures that all changed lines in tracked files are covered (hit count > 0)
 *
 * If any changed line is not covered, the script exits with code 1.
 */

const { execSync } = require('child_process');
const { existsSync, readFileSync } = require('fs');
const { resolve, join } = require('path');

function run(cmd) {
  return execSync(cmd, { encoding: 'utf-8' });
}

function getBaseRef() {
  return process.env.DIFF_COVERAGE_BASE || 'main';
}

function resolveBaseRef(baseRef) {
  // Check if baseRef exists as a local branch or tag
  try {
    execSync(`git rev-parse --verify ${baseRef}`, { encoding: 'utf-8', stdio: 'ignore' });
    return baseRef;
  } catch (e) {
    // If not, try origin/baseRef (e.g., origin/main)
    try {
      const remoteRef = `origin/${baseRef}`;
      execSync(`git rev-parse --verify ${remoteRef}`, { encoding: 'utf-8', stdio: 'ignore' });
      return remoteRef;
    } catch (e2) {
      // If that also fails, return the original and let git diff fail with a clearer error
      return baseRef;
    }
  }
}

function getDiffHunks(resolvedRef) {
  const output = run(`git diff --unified=0 ${resolvedRef}...HEAD`);
  return output.split('\n');
}

/**
 * Load ignore configuration from .diff-coverage-ignore.json
 * Returns an object with 'patterns' and 'files' arrays.
 */
function loadIgnoreConfig() {
  const repoRoot = execSync('git rev-parse --show-toplevel', { encoding: 'utf-8' }).trim();
  const configPath = join(repoRoot, '.diff-coverage-ignore.json');

  if (!existsSync(configPath)) {
    // Return default empty config if file doesn't exist
    return { patterns: [], files: [] };
  }

  try {
    const content = readFileSync(configPath, 'utf-8');
    const config = JSON.parse(content);
    return {
      patterns: config.patterns || [],
      files: config.files || [],
    };
  } catch (e) {
    console.warn(`⚠️  Warning: Failed to parse .diff-coverage-ignore.json: ${e.message}`);
    return { patterns: [], files: [] };
  }
}

/**
 * Simple glob pattern matcher for common patterns.
 * Supports:
 * - ** for matching any number of directories
 * - * for matching any characters except /
 * - Exact matches
 */
function matchesGlob(filePath, pattern) {
  // Normalize paths to use forward slashes
  const normalizedPath = filePath.replace(/\\/g, '/');
  const normalizedPattern = pattern.replace(/\\/g, '/');

  // Convert glob pattern to regex
  // Escape special regex characters except * and **
  let regexStr = normalizedPattern
    .replace(/[.+^${}()|[\]\\]/g, '\\$&')
    .replace(/\*\*/g, '___DOUBLE_STAR___')
    .replace(/\*/g, '[^/]*')
    .replace(/___DOUBLE_STAR___/g, '.*');

  const regex = new RegExp(`^${regexStr}$`);
  return regex.test(normalizedPath);
}

/**
 * Check if a file should be ignored based on the ignore config.
 */
function shouldIgnoreFile(filePath, ignoreConfig) {
  // Check specific file paths (treat as glob if they contain * or **)
  for (const file of ignoreConfig.files) {
    // If the file pattern contains glob characters, use glob matching
    if (file.includes('*')) {
      if (matchesGlob(filePath, file)) {
        return true;
      }
    } else {
      // Otherwise, treat as literal path
      if (filePath === file || filePath.startsWith(file + '/')) {
        return true;
      }
    }
  }

  // Check glob patterns
  for (const pattern of ignoreConfig.patterns) {
    if (matchesGlob(filePath, pattern)) {
      return true;
    }
  }

  return false;
}

function isSourceFile(filePath, ignoreConfig) {
  // Only enforce coverage for TypeScript source files in packages/*/src/
  // Exclude: test files, docs, CI configs, build configs, etc.
  if (!filePath.startsWith('packages/')) {
    return false;
  }

  if (!filePath.includes('/src/')) {
    return false;
  }

  // Must be a .ts file
  if (!filePath.endsWith('.ts')) {
    return false;
  }

  // Check ignore config (patterns and specific files)
  if (shouldIgnoreFile(filePath, ignoreConfig)) {
    return false;
  }

  return true;
}

function collectChangedLines(diffLines, ignoreConfig) {
  const fileChanges = new Map(); // file -> Set<lineNumber>
  let currentFile = null;

  for (const line of diffLines) {
    if (line.startsWith('+++ b/')) {
      currentFile = line.substring('+++ b/'.length).trim();
      // Only track source files
      if (isSourceFile(currentFile, ignoreConfig)) {
        if (!fileChanges.has(currentFile)) {
          fileChanges.set(currentFile, new Set());
        }
      } else {
        currentFile = null; // Skip this file
      }
      continue;
    }

    if (!currentFile) {
      continue;
    }

    if (line.startsWith('@@')) {
      // Example: @@ -10,0 +11,5 @@
      const match = /@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@/.exec(line);
      if (!match) continue;
      const start = parseInt(match[1], 10);
      const count = match[2] ? parseInt(match[2], 10) : 1;
      const changed = fileChanges.get(currentFile);
      for (let i = 0; i < count; i += 1) {
        changed.add(start + i);
      }
    }
  }

  return fileChanges;
}

function parseLcov(lcovPath) {
  const content = readFileSync(lcovPath, 'utf-8');
  const lines = content.split('\n');

  const coverage = new Map(); // file -> Map<lineNumber, hits>
  let currentFile = null;

  for (const line of lines) {
    if (line.startsWith('SF:')) {
      currentFile = line.substring(3).trim();
      if (!coverage.has(currentFile)) {
        coverage.set(currentFile, new Map());
      }
    } else if (line.startsWith('DA:') && currentFile) {
      const [, lineNo, hits] = /DA:(\d+),(\d+)/.exec(line) || [];
      if (!lineNo) continue;
      const fileMap = coverage.get(currentFile);
      fileMap.set(parseInt(lineNo, 10), parseInt(hits, 10));
    } else if (line === 'end_of_record') {
      currentFile = null;
    }
  }

  return coverage;
}

function normalizePath(pathStr) {
  // lcov may store absolute or relative paths; we normalize to relative-from-repo-root
  // by stripping the repo root prefix if present.
  const repoRoot = execSync('git rev-parse --show-toplevel', { encoding: 'utf-8' })
    .trim()
    .replace(/\\/g, '/');
  const normalized = pathStr.replace(/\\/g, '/');
  if (normalized.startsWith(repoRoot + '/')) {
    return normalized.slice(repoRoot.length + 1);
  }
  return normalized;
}

function buildCoverageIndex(coverage) {
  const index = new Map(); // normalizedFile -> Map<lineNumber, hits>
  for (const [filePath, lines] of coverage.entries()) {
    const key = normalizePath(filePath);
    index.set(key, lines);
  }
  return index;
}

function main() {
  const lcovPathEnv = process.env.DIFF_COVERAGE_LCOV;
  const lcovPath = lcovPathEnv ? resolve(lcovPathEnv) : resolve('coverage/lcov.info');

  if (!existsSync(lcovPath)) {
    console.error(`❌ Differential coverage: lcov file not found at ${lcovPath}`);
    process.exit(1);
  }

  // Load ignore configuration
  const ignoreConfig = loadIgnoreConfig();
  if (ignoreConfig.patterns.length > 0 || ignoreConfig.files.length > 0) {
    console.log(`Using ignore config: ${ignoreConfig.patterns.length} pattern(s), ${ignoreConfig.files.length} file(s)`);
  }

  const baseRef = getBaseRef();
  const resolvedRef = resolveBaseRef(baseRef);

  console.log(`Differential coverage base: ${baseRef}${resolvedRef !== baseRef ? ` (resolved to ${resolvedRef})` : ''}`);
  console.log(`Using coverage report: ${lcovPath}`);

  const diffLines = getDiffHunks(resolvedRef);
  const changed = collectChangedLines(diffLines, ignoreConfig);

  if (changed.size === 0) {
    console.log('No changed lines detected; nothing to enforce for differential coverage.');
    return;
  }

  const coverage = parseLcov(lcovPath);
  const coverageIndex = buildCoverageIndex(coverage);

  const uncovered = [];

  for (const [file, lines] of changed.entries()) {
    // Only enforce for files that appear in coverage; files not in coverage are considered uncovered.
    const fileCoverage = coverageIndex.get(file);

    for (const lineNo of lines) {
      const hits = fileCoverage ? fileCoverage.get(lineNo) : 0;
      if (!hits || hits <= 0) {
        uncovered.push({ file, line: lineNo });
      }
    }
  }

  if (uncovered.length > 0) {
    console.error('❌ Differential coverage requirement not met. The following lines are not covered:');
    for (const { file, line } of uncovered) {
      console.error(`  - ${file}:${line}`);
    }
    process.exit(1);
  }

  console.log('✅ 100% differential coverage achieved for changed lines.');
}

if (require.main === module) {
  main();
}


