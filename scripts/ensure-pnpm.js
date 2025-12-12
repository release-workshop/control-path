#!/usr/bin/env node

/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Ensures that pnpm is being used instead of npm/yarn, and that Node.js LTS is installed.
 * This script is run as a preinstall hook to prevent accidental use of other package managers
 * and ensure the correct Node.js version.
 */

const { execSync } = require('child_process');
const { existsSync } = require('fs');
const { join } = require('path');

// Check if pnpm-lock.yaml exists
const lockFile = join(process.cwd(), 'pnpm-lock.yaml');
const hasPnpmLock = existsSync(lockFile);

// Check if npm/yarn lock files exist (should not exist in this repo)
const npmLock = join(process.cwd(), 'package-lock.json');
const yarnLock = join(process.cwd(), 'yarn.lock');

// Get the package manager from environment or detect from lock files
const userAgent = process.env.npm_config_user_agent || '';
const isPnpm = userAgent.includes('pnpm') || hasPnpmLock;
const isNpm = userAgent.includes('npm') || existsSync(npmLock);
const isYarn = userAgent.includes('yarn') || existsSync(yarnLock);

if (isNpm && !isPnpm) {
  console.error('\n❌ Error: This project requires pnpm as the package manager.');
  console.error('\nPlease use pnpm instead of npm:');
  console.error('  npm install -g pnpm');
  console.error('  pnpm install\n');
  process.exit(1);
}

if (isYarn && !isPnpm) {
  console.error('\n❌ Error: This project requires pnpm as the package manager.');
  console.error('\nPlease use pnpm instead of yarn:');
  console.error('  npm install -g pnpm');
  console.error('  pnpm install\n');
  process.exit(1);
}

// Check Node.js version (must be LTS - 24.x or higher)
const nodeVersion = process.version;
const nodeMajorVersion = parseInt(nodeVersion.slice(1).split('.')[0], 10);

if (nodeMajorVersion < 24) {
  console.error(`\n❌ Error: Node.js ${nodeVersion} is installed, but Node.js 24 LTS or higher is required.`);
  console.error('\nPlease install Node.js LTS:');
  console.error('  - Using nvm: nvm install --lts && nvm use --lts');
  console.error('  - Or download from: https://nodejs.org/\n');
  process.exit(1);
}

// Check if pnpm is installed
try {
  execSync('pnpm --version', { stdio: 'ignore' });
} catch (error) {
  console.error('\n❌ Error: pnpm is not installed.');
  console.error('\nPlease install pnpm:');
  console.error('  npm install -g pnpm\n');
  process.exit(1);
}

// Check pnpm version
try {
  const version = execSync('pnpm --version', { encoding: 'utf-8' }).trim();
  const majorVersion = parseInt(version.split('.')[0], 10);
  
  if (majorVersion < 8) {
    console.error(`\n❌ Error: pnpm version ${version} is installed, but version 8+ is required.`);
    console.error('\nPlease update pnpm:');
    console.error('  npm install -g pnpm@latest\n');
    process.exit(1);
  }
} catch (error) {
  // Version check failed, but pnpm exists, so continue
  console.warn('⚠️  Warning: Could not verify pnpm version, but pnpm appears to be installed.');
}

// Check if Deno is installed (for CLI)
try {
  execSync('deno --version', { stdio: 'ignore' });
  const denoVersion = execSync('deno --version', { encoding: 'utf-8' }).trim().split('\n')[0];
  
  // Success
  if (process.env.npm_config_user_agent) {
    console.log(`✓ Node.js ${nodeVersion} (LTS)`);
    console.log(`✓ ${denoVersion}`);
    console.log('✓ Using pnpm as package manager');
  }
} catch (error) {
  // Deno not installed - warn but don't fail (compiler package doesn't need it)
  if (process.env.npm_config_user_agent) {
    console.log(`✓ Node.js ${nodeVersion} (LTS)`);
    console.log('✓ Using pnpm as package manager');
    console.warn('⚠️  Warning: Deno is not installed. Required for CLI tool.');
    console.warn('   Install with: curl -fsSL https://deno.land/install.sh | sh');
  }
}

