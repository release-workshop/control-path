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
const { resolve } = require('path');

function run(cmd) {
  return execSync(cmd, { encoding: 'utf-8' });
}

function getBaseRef() {
  return process.env.DIFF_COVERAGE_BASE || 'main';
}

function getDiffHunks(baseRef) {
  const output = run(`git diff --unified=0 ${baseRef}...HEAD`);
  return output.split('\n');
}

function collectChangedLines(diffLines) {
  const fileChanges = new Map(); // file -> Set<lineNumber>
  let currentFile = null;

  for (const line of diffLines) {
    if (line.startsWith('+++ b/')) {
      currentFile = line.substring('+++ b/'.length).trim();
      if (!fileChanges.has(currentFile)) {
        fileChanges.set(currentFile, new Set());
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

  const baseRef = getBaseRef();

  console.log(`Differential coverage base: ${baseRef}`);
  console.log(`Using coverage report: ${lcovPath}`);

  const diffLines = getDiffHunks(baseRef);
  const changed = collectChangedLines(diffLines);

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


