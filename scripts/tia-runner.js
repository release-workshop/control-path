#!/usr/bin/env node

/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Test Impact Analysis helper.
 *
 * Usage:
 *   node scripts/tia-runner.js --base main
 *
 * The script:
 * - Computes changed files between <base> and HEAD
 * - For any file under packages/*:
 *   - If it is a *.test.ts file, include it directly
 *   - If it is a *.ts source file, look for a sibling *.test.ts in the same directory
 * - Writes a space-separated list of test paths to the GitHub Actions step output as `affected_tests`
 *
 * If no specific tests can be determined, it leaves `affected_tests` empty, allowing
 * the workflow to fall back to a broader/full test run.
 */

const { execSync } = require('child_process');
const { existsSync, readdirSync, statSync, readFileSync } = require('fs');
const { join, sep, resolve, dirname, relative } = require('path');

function parseArgs(argv) {
  const args = {};
  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--base' && argv[i + 1]) {
      args.base = argv[i + 1];
      i += 1;
    }
  }
  return args;
}

function run(cmd) {
  return execSync(cmd, { encoding: 'utf-8' }).trim();
}

function getGitRoot() {
  return run('git rev-parse --show-toplevel');
}

function getChangedFiles(baseRef) {
  const cmd = `git diff --name-only ${baseRef}...HEAD`;
  const output = run(cmd);
  if (!output) return [];
  return output.split('\n').map((f) => f.trim()).filter(Boolean);
}

function isMonorepoPackageFile(filePath) {
  // Any file directly under packages/* is considered part of a package.
  // We ignore dist and node_modules outputs.
  return (
    filePath.startsWith(`packages${sep}`) &&
    !filePath.includes(`${sep}dist${sep}`) &&
    !filePath.includes(`${sep}node_modules${sep}`)
  );
}

function isTestFile(filePath) {
  return isMonorepoPackageFile(filePath) && filePath.endsWith('.test.ts');
}

function isSourceFile(filePath) {
  return (
    isMonorepoPackageFile(filePath) &&
    filePath.endsWith('.ts') &&
    !filePath.endsWith('.test.ts') &&
    !filePath.endsWith('.d.ts')
  );
}

function listAllTsFiles(rootDirAbs, gitRoot) {
  const results = [];

  function walk(dir) {
    const entries = readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const absPath = join(dir, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === 'node_modules' || entry.name === 'dist') continue;
        walk(absPath);
      } else if (entry.isFile()) {
        if (!absPath.endsWith('.ts') || absPath.endsWith('.d.ts')) continue;
        const relPath = relative(gitRoot, absPath);
        if (isMonorepoPackageFile(relPath)) {
          results.push(relPath);
        }
      }
    }
  }

  walk(rootDirAbs);
  return results;
}

function extractImportSpecifiers(sourceText) {
  const specs = new Set();
  const importRegex =
    /\bimport\s+[^'"]*['"]([^'"]+)['"]|import\(['"]([^'"]+)['"]\)|require\(\s*['"]([^'"]+)['"]\s*\)|export\s+[^'"]*from\s+['"]([^'"]+)['"]/g;
  let match = importRegex.exec(sourceText);
  while (match) {
    const spec = match[1] || match[2] || match[3] || match[4];
    if (spec) specs.add(spec);
    match = importRegex.exec(sourceText);
  }
  return Array.from(specs);
}

function resolveImport(fromFileRel, spec, gitRoot) {
  // Only handle relative imports here; non-relative ones are ignored for TIA purposes.
  if (!spec.startsWith('.')) return null;

  const fromAbs = resolve(gitRoot, fromFileRel);
  const base = resolve(dirname(fromAbs), spec);

  const candidates = [
    base,
    `${base}.ts`,
    join(base, 'index.ts'),
  ];

  // Find the first existing candidate and return its repo-relative path.
  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      const rel = relative(gitRoot, candidate);
      return rel.split(sep).join(sep);
    }
  }

  return null;
}

function buildReverseDependencyGraph(allFiles, gitRoot) {
  const reverse = new Map(); // file -> Set<dependents>

  for (const fileRel of allFiles) {
    const absPath = resolve(gitRoot, fileRel);
    const source = readFileSync(absPath, 'utf-8');
    const specs = extractImportSpecifiers(source);

    for (const spec of specs) {
      const importedRel = resolveImport(fileRel, spec, gitRoot);
      if (!importedRel) continue;

      if (!reverse.has(importedRel)) {
        reverse.set(importedRel, new Set());
      }
      reverse.get(importedRel).add(fileRel);
    }
  }

  return reverse;
}

function resolveAffectedTestsFromGraph(changedFiles, gitRoot) {
  const packagesRootAbs = resolve(gitRoot, 'packages');
  const allFiles = listAllTsFiles(packagesRootAbs, gitRoot);
  const reverse = buildReverseDependencyGraph(allFiles, gitRoot);

  const tests = new Set();
  const visited = new Set();
  const stack = [];

  for (const file of changedFiles) {
    if (!isMonorepoPackageFile(file)) continue;
    stack.push(file);
  }

  while (stack.length > 0) {
    const current = stack.pop();
    if (visited.has(current)) continue;
    visited.add(current);

    if (isTestFile(current)) {
      tests.add(current);
    }

    const dependents = reverse.get(current);
    if (!dependents) continue;

    for (const dep of dependents) {
      if (!visited.has(dep)) {
        stack.push(dep);
      }
    }
  }

  return Array.from(tests);
}

function writeGithubOutput(name, value) {
  const outputPath = process.env.GITHUB_OUTPUT;
  if (!outputPath) {
    // Local run or not in GitHub Actions
    // eslint-disable-next-line no-console
    console.log(`${name}=${value}`);
    return;
  }

  const fs = require('fs');
  fs.appendFileSync(outputPath, `${name}=${value}\n`);
}

function main() {
  const { base = 'main' } = parseArgs(process.argv);

  try {
    const gitRoot = getGitRoot();
    const changedFiles = getChangedFiles(base);

    if (changedFiles.length === 0) {
      console.log('No changed files detected; leaving affected_tests empty.');
      writeGithubOutput('affected_tests', '');
      return;
    }

    const affectedTests = resolveAffectedTestsFromGraph(changedFiles, gitRoot);

    if (affectedTests.length === 0) {
      console.log(
        'TIA could not determine specific tests to run; leaving affected_tests empty so the workflow can fall back to a broader run.'
      );
      writeGithubOutput('affected_tests', '');
      return;
    }

    const list = affectedTests.join(' ');
    console.log(`TIA determined affected tests: ${list}`);
    writeGithubOutput('affected_tests', list);
  } catch (err) {
    console.error('TIA failed; leaving affected_tests empty.', err);
    writeGithubOutput('affected_tests', '');
  }
}

if (require.main === module) {
  main();
}

