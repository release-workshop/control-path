/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * End-to-end tests for SDK generator.
 * These tests generate TypeScript SDKs and verify they work with various rule combinations.
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { writeFile, mkdir, rm, readFile, copyFile } from 'fs/promises';
import { join, dirname } from 'path';
import { tmpdir } from 'os';
import { spawnSync, execSync } from 'child_process';
import { fileURLToPath } from 'url';
import { readFileSync } from 'fs';
import { Provider } from '@controlpath/runtime';

// Get __dirname equivalent for ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * Get the path to the Rust CLI binary
 */
function getRustCliPath(): string {
  // Try release build first (faster for repeated runs)
  // From tests/e2e/src/, go up 3 levels to project root
  const releasePath = join(__dirname, '../../../target/release/controlpath');
  try {
    readFileSync(releasePath);
    return releasePath;
  } catch {
    // Fall back to debug build
    const debugPath = join(__dirname, '../../../target/debug/controlpath');
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
 * Run CLI command and return result
 */
function runCliCommand(args: string[]): { success: boolean; stdout: string; stderr: string } {
  const rustCli = getRustCliPath();
  const result = spawnSync(rustCli, args, {
    encoding: 'utf-8',
    stdio: 'pipe',
  });

  return {
    success: result.status === 0,
    stdout: result.stdout?.toString() || '',
    stderr: result.stderr?.toString() || '',
  };
}

/**
 * Compile AST using Rust CLI
 */
async function compileAst(
  definitionsFile: string,
  deploymentFile: string,
  outputFile: string
): Promise<void> {
  const result = runCliCommand([
    'compile',
    '--definitions',
    definitionsFile,
    '--deployment',
    deploymentFile,
    '--output',
    outputFile,
  ]);

  if (!result.success) {
    throw new Error(`Compilation failed: ${result.stderr || result.stdout}`);
  }
}

/**
 * Generate SDK using Rust CLI
 */
async function generateSdk(
  definitionsFile: string,
  outputDir: string,
  lang: string = 'typescript'
): Promise<void> {
  const result = runCliCommand([
    'generate-sdk',
    '--definitions',
    definitionsFile,
    '--output',
    outputDir,
    '--lang',
    lang,
  ]);

  if (!result.success) {
    throw new Error(`SDK generation failed: ${result.stderr || result.stdout}`);
  }
}

/**
 * Set up generated SDK for execution
 * Links runtime SDK and compiles TypeScript
 */
async function setupGeneratedSdk(sdkDir: string): Promise<void> {
  const runtimePath = join(__dirname, '../../../runtime/typescript');
  
  // Create node_modules structure and link runtime SDK
  const sdkNodeModules = join(sdkDir, 'node_modules', '@controlpath');
  await mkdir(sdkNodeModules, { recursive: true });
  
  // Create symlink to runtime SDK
  try {
    execSync(`ln -sf "${runtimePath}" "${join(sdkNodeModules, 'runtime')}"`, {
      stdio: 'pipe',
    });
  } catch {
    // If symlink fails (Windows), try copying or using npm link
    // For now, we'll rely on the runtime being in the parent node_modules
  }
  
  // Create tsconfig.json for compilation
  const tsconfig = {
    compilerOptions: {
      target: 'ES2020',
      module: 'commonjs',
      lib: ['ES2020'],
      outDir: './dist',
      rootDir: '.',
      strict: true,
      esModuleInterop: true,
      skipLibCheck: true,
      forceConsistentCasingInFileNames: true,
      resolveJsonModule: true,
      declaration: true,
      moduleResolution: 'node',
      baseUrl: '.',
      paths: {
        '@controlpath/runtime': [runtimePath],
      },
    },
    include: ['index.ts', 'types.ts'],
    exclude: ['node_modules', 'dist'],
  };

  await writeFile(join(sdkDir, 'tsconfig.json'), JSON.stringify(tsconfig, null, 2));

  // Install dependencies (except @controlpath/runtime which is symlinked)
  // No external dependencies needed - OpenFeature support has been removed

  // Use TypeScript from e2e test's node_modules (already installed)
  const e2eTypescriptPath = join(__dirname, '../node_modules/.bin/tsc');
  let tscCommand = 'npx tsc';
  
  // Check if TypeScript is available in e2e test's node_modules
  try {
    readFileSync(e2eTypescriptPath);
    // Use the e2e test's TypeScript
    tscCommand = `node "${e2eTypescriptPath}"`;
  } catch {
    // Fall back to npx (will fail if TypeScript not installed, but that's expected)
  }

  // Compile TypeScript
  try {
    execSync(`${tscCommand} --skipLibCheck`, {
      cwd: sdkDir,
      stdio: 'pipe',
    });
  } catch (tscError: any) {
    // Log the actual TypeScript errors
    const errorOutput = tscError.stdout?.toString() || tscError.stderr?.toString() || tscError.message || 'Unknown error';
    
    // Debug: Output the generated index.ts file around the error line
    try {
      const indexContent = await readFile(join(sdkDir, 'index.ts'), 'utf-8');
      const lines = indexContent.split('\n');
      console.error(`\n=== Generated index.ts has ${lines.length} lines ===`);
      if (lines.length >= 375) {
        console.error(`\n=== Lines 370-380 of generated index.ts ===`);
        console.error(lines.slice(369, 380).map((line, i) => `${370 + i}: ${line}`).join('\n'));
      } else {
        console.error(`\n=== Last 10 lines of generated index.ts ===`);
        console.error(lines.slice(-10).map((line, i) => `${lines.length - 10 + i}: ${line}`).join('\n'));
      }
    } catch (readError) {
      // Ignore read errors
    }
    
    throw new Error(`TypeScript compilation failed: ${errorOutput}`);
  }
}

/**
 * Load and use the generated SDK
 */
async function loadGeneratedSdk(sdkDir: string, astFile: string) {
  // Import the compiled SDK using file:// URL for absolute path
  const sdkPath = `file://${join(sdkDir, 'dist', 'index.js')}`;
  
  // Use dynamic import
  const sdkModule = await import(sdkPath);
  const { Evaluator } = sdkModule;
  
  // Create evaluator instance
  const evaluator = new Evaluator();
  
  // Initialize with AST
  await evaluator.init({ artifact: astFile });
  
  return evaluator;
}

describe('SDK Generator E2E Tests', () => {
  // Use OS temp directory for better isolation
  const testDir = join(
    tmpdir(),
    'controlpath-e2e',
    `test-${Date.now()}-${Math.random().toString(36).substring(7)}`
  );
  const definitionsFile = join(testDir, 'flags.definitions.yaml');
  const deploymentFile = join(testDir, 'production.deployment.yaml');
  const astFile = join(testDir, 'production.ast');
  const sdkDir = join(testDir, 'generated-sdk');

  const flagsDefinitions = `flags:
  - name: new_dashboard
    type: boolean
    defaultValue: false
    description: "New dashboard UI feature"
  
  - name: enable_analytics
    type: boolean
    defaultValue: false
    description: "Enable analytics tracking"
  
  - name: checkout_experiment
    type: multivariate
    defaultValue: CONTROL
    description: "Checkout flow experiment"
    variations:
      - name: CONTROL
        value: "control"
      - name: VARIANT_A
        value: "variant_a"
      - name: VARIANT_B
        value: "variant_b"
`;

  beforeAll(async () => {
    await mkdir(testDir, { recursive: true });
    await writeFile(definitionsFile, flagsDefinitions);
  });

  afterAll(async () => {
    try {
      await rm(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('Simple Rules', () => {
    const simpleDeployment = `environment: production
rules:
  new_dashboard:
    rules:
      - serve: true
  enable_analytics:
    rules:
      - serve: false
  checkout_experiment:
    rules:
      - serve: VARIANT_A
`;

    it('should generate SDK and evaluate flags with simple rules', async () => {
      // Write deployment file
      await writeFile(deploymentFile, simpleDeployment);

      // Generate SDK
      await generateSdk(definitionsFile, sdkDir);

      // Compile AST
      await compileAst(definitionsFile, deploymentFile, astFile);

      // Set up SDK for execution
      await setupGeneratedSdk(sdkDir);

      // Load and use the generated SDK
      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      // Test user
      const user = { id: 'user1', role: 'user' };

      // Evaluate flags using generated SDK methods
      const newDashboard = await evaluator.newDashboard(user);
      expect(newDashboard).toBe(true); // Rule says serve: true

      const enableAnalytics = await evaluator.enableAnalytics(user);
      expect(enableAnalytics).toBe(false); // Rule says serve: false

      const checkoutExperiment = await evaluator.checkoutExperiment(user);
      expect(checkoutExperiment).toBe('VARIANT_A'); // Rule says serve: VARIANT_A

      // Test method overloads
      // No parameters (uses setContext)
      evaluator.setContext(user);
      const newDashboardNoParams = await evaluator.newDashboard();
      expect(newDashboardNoParams).toBe(true);

      // User + Context
      const context = { environment: 'production' };
      const newDashboardWithContext = await evaluator.newDashboard(user, context);
      expect(newDashboardWithContext).toBe(true);
    });
  });

  describe('Conditional Rules', () => {
    const conditionalDeployment = `environment: production
rules:
  new_dashboard:
    rules:
      - when: "user.role == 'admin'"
        serve: true
      - serve: false
  enable_analytics:
    rules:
      - when: "user.plan == 'premium'"
        serve: true
      - serve: false
  checkout_experiment:
    rules:
      - when: "user.role == 'admin'"
        serve: VARIANT_B
      - serve: CONTROL
`;

    it('should generate SDK and evaluate flags with conditional rules', async () => {
      // Write deployment file
      await writeFile(deploymentFile, conditionalDeployment);

      // Generate SDK
      await generateSdk(definitionsFile, sdkDir);

      // Compile AST
      await compileAst(definitionsFile, deploymentFile, astFile);

      // Set up SDK for execution
      await setupGeneratedSdk(sdkDir);

      // Load and use the generated SDK
      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      // Test admin user (should match conditional rule)
      const adminUser = { id: 'admin1', role: 'admin' };
      const newDashboardAdmin = await evaluator.newDashboard(adminUser);
      expect(newDashboardAdmin).toBe(true); // Matches: user.role == 'admin'

      const checkoutAdmin = await evaluator.checkoutExperiment(adminUser);
      expect(checkoutAdmin).toBe('VARIANT_B'); // Matches: user.role == 'admin'

      // Test regular user (should get default/fallback)
      const regularUser = { id: 'user1', role: 'user' };
      const newDashboardRegular = await evaluator.newDashboard(regularUser);
      expect(newDashboardRegular).toBe(false); // Falls through to default

      const checkoutRegular = await evaluator.checkoutExperiment(regularUser);
      expect(checkoutRegular).toBe('CONTROL'); // Falls through to default

      // Test premium user (should match enable_analytics rule)
      const premiumUser = { id: 'premium1', plan: 'premium' };
      const enableAnalyticsPremium = await evaluator.enableAnalytics(premiumUser);
      expect(enableAnalyticsPremium).toBe(true); // Matches: user.plan == 'premium'

      // Test non-premium user
      const freeUser = { id: 'free1', plan: 'free' };
      const enableAnalyticsFree = await evaluator.enableAnalytics(freeUser);
      expect(enableAnalyticsFree).toBe(false); // Falls through to default
    });
  });

  describe('Default Values', () => {
    const defaultDeployment = `environment: production
rules:
  new_dashboard:
    rules:
      - serve: false
  enable_analytics:
    rules:
      - serve: false
  checkout_experiment:
    rules:
      - serve: CONTROL
`;

    it('should return default values when no context provided', async () => {
      // Write deployment file
      await writeFile(deploymentFile, defaultDeployment);

      // Generate SDK
      await generateSdk(definitionsFile, sdkDir);

      // Compile AST
      await compileAst(definitionsFile, deploymentFile, astFile);

      // Set up SDK for execution
      await setupGeneratedSdk(sdkDir);

      // Load and use the generated SDK
      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      // Test without user (should return defaults)
      const newDashboardNoUser = await evaluator.newDashboard();
      expect(newDashboardNoUser).toBe(false); // Default value

      // Test with user but no matching rules (should return defaults from definitions)
      const user = { id: 'user1' };
      const newDashboard = await evaluator.newDashboard(user);
      expect(newDashboard).toBe(false); // Rule says serve: false, which matches default

      const checkoutExperiment = await evaluator.checkoutExperiment(user);
      expect(checkoutExperiment).toBe('CONTROL'); // Rule says serve: CONTROL, which matches default
    });
  });

  describe('Batch Evaluation', () => {
    const batchDeployment = `environment: production
rules:
  new_dashboard:
    rules:
      - serve: true
  enable_analytics:
    rules:
      - serve: false
  checkout_experiment:
    rules:
      - serve: VARIANT_A
`;

    it('should generate and use type-safe batch evaluation methods', async () => {
      // Write deployment file
      await writeFile(deploymentFile, batchDeployment);

      // Generate SDK
      await generateSdk(definitionsFile, sdkDir);

      // Compile AST
      await compileAst(definitionsFile, deploymentFile, astFile);

      // Set up SDK for execution
      await setupGeneratedSdk(sdkDir);

      // Load and use the generated SDK
      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      const user = { id: 'user1' };

      // Test evaluateBatch with specific flags
      const batchResult = await evaluator.evaluateBatch(
        ['newDashboard', 'enableAnalytics'] as const,
        user
      );
      expect(batchResult.newDashboard).toBe(true); // Rule says serve: true
      expect(batchResult.enableAnalytics).toBe(false); // Rule says serve: false

      // Test evaluateAll (evaluates all flags)
      const allResult = await evaluator.evaluateAll(user);
      expect(allResult.newDashboard).toBe(true);
      expect(allResult.enableAnalytics).toBe(false);
      expect(allResult.checkoutExperiment).toBe('VARIANT_A'); // Rule says serve: VARIANT_A
    });
  });

  describe('Context Management', () => {
    const contextDeployment = `environment: production
rules:
  new_dashboard:
    rules:
      - serve: true
  enable_analytics:
    rules:
      - serve: false
  checkout_experiment:
    rules:
      - serve: CONTROL
`;

    it('should use context management methods correctly', async () => {
      // Write deployment file
      await writeFile(deploymentFile, contextDeployment);

      // Generate SDK
      await generateSdk(definitionsFile, sdkDir);

      // Compile AST
      await compileAst(definitionsFile, deploymentFile, astFile);

      // Set up SDK for execution
      await setupGeneratedSdk(sdkDir);

      // Load and use the generated SDK
      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      const user = { id: 'user1', role: 'user' };
      const context = { environment: 'production' };

      // Test setContext
      evaluator.setContext(user, context);

      // Test that methods use setContext when no parameters provided
      const newDashboardFromContext = await evaluator.newDashboard();
      expect(newDashboardFromContext).toBe(true); // Uses setContext

      // Test clearContext
      evaluator.clearContext();

      // After clearing, should return default (no user)
      const newDashboardAfterClear = await evaluator.newDashboard();
      expect(newDashboardAfterClear).toBe(false); // Default value

      // Test that explicit user overrides setContext
      const explicitUser = { id: 'user2' };
      const newDashboardExplicit = await evaluator.newDashboard(explicitUser);
      expect(newDashboardExplicit).toBe(true); // Uses explicit user
    });
  });

  describe('Observability Methods', () => {
    const observabilityDeployment = `environment: production
rules:
  new_dashboard:
    rules:
      - serve: true
`;

    it('should have setLogger, setTracer, and setMetrics methods', async () => {
      // Write deployment file
      await writeFile(deploymentFile, observabilityDeployment);

      // Generate SDK
      await generateSdk(definitionsFile, sdkDir);

      // Compile AST
      await compileAst(definitionsFile, deploymentFile, astFile);

      // Set up SDK for execution
      await setupGeneratedSdk(sdkDir);

      // Load and use the generated SDK
      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      // Verify methods exist
      expect(typeof evaluator.setLogger).toBe('function');
      expect(typeof evaluator.setTracer).toBe('function');
      expect(typeof evaluator.setMetrics).toBe('function');

      // Test setLogger - should not throw
      const mockLogger = {
        error: () => {},
        warn: () => {},
        info: () => {},
        debug: () => {},
      };
      expect(() => evaluator.setLogger(mockLogger)).not.toThrow();

      // Test setTracer - should not throw
      const mockTracer = {
        startSpan: (name: string) => ({
          setAttribute: () => {},
          addEvent: () => {},
          end: () => {},
        }),
      };
      expect(() => evaluator.setTracer(mockTracer)).not.toThrow();

      // Test setMetrics - should not throw
      const mockMetrics = {
        increment: () => {},
        gauge: () => {},
      };
      expect(() => evaluator.setMetrics(mockMetrics)).not.toThrow();

      // Verify hooks were added to provider
      // Access the provider through the evaluator (if possible) or verify hooks exist
      // Note: We can't directly access provider.hooks from the generated SDK,
      // but we can verify the methods work by checking they don't throw
    });
  });

  describe('Method Overloads', () => {
    const overloadDeployment = `environment: production
rules:
  new_dashboard:
    rules:
      - serve: true
  enable_analytics:
    rules:
      - serve: false
  checkout_experiment:
    rules:
      - serve: CONTROL
`;

    it('should work with all method overload variants', async () => {
      // Write deployment file
      await writeFile(deploymentFile, overloadDeployment);

      // Generate SDK
      await generateSdk(definitionsFile, sdkDir);

      // Compile AST
      await compileAst(definitionsFile, deploymentFile, astFile);

      // Set up SDK for execution
      await setupGeneratedSdk(sdkDir);

      // Load and use the generated SDK
      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      const user = { id: 'user1' };
      const context = { environment: 'production' };

      // Test overload 1: No parameters (uses setContext)
      evaluator.setContext(user);
      const result1 = await evaluator.newDashboard();
      expect(result1).toBe(true);

      // Test overload 2: User only
      const result2 = await evaluator.newDashboard(user);
      expect(result2).toBe(true);

      // Test overload 3: User + Context
      const result3 = await evaluator.newDashboard(user, context);
      expect(result3).toBe(true);

      // All should return the same value (rule says serve: true)
      expect(result1).toBe(result2);
      expect(result2).toBe(result3);
    });
  });

  describe('Error Handling', () => {
    const errorDeployment = `environment: production
rules:
  new_dashboard:
    rules:
      - serve: true
  enable_analytics:
    rules:
      - serve: false
  checkout_experiment:
    rules:
      - serve: CONTROL
`;

    it('should never throw errors, always return defaults', async () => {
      // Write deployment file
      await writeFile(deploymentFile, errorDeployment);

      // Generate SDK
      await generateSdk(definitionsFile, sdkDir);

      // Compile AST
      await compileAst(definitionsFile, deploymentFile, astFile);

      // Set up SDK for execution
      await setupGeneratedSdk(sdkDir);

      // Load and use the generated SDK
      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      // Test that methods never throw, even with invalid inputs
      // No user provided - should return default, not throw
      const result1 = await evaluator.newDashboard();
      expect(result1).toBe(false); // Default value, no error thrown

      // Invalid user (empty) - should return default, not throw
      const invalidUser = { id: '' };
      const result2 = await evaluator.newDashboard(invalidUser);
      expect(result2).toBe(false); // Default value, no error thrown

      // All methods should return values, never throw
      const result3 = await evaluator.enableAnalytics();
      expect(typeof result3).toBe('boolean');

      const result4 = await evaluator.checkoutExperiment();
      expect(typeof result4).toBe('string');
    });
  });

  describe('Runtime SDK Integration', () => {
    const integrationDeployment = `environment: production
rules:
  new_dashboard:
    rules:
      - serve: true
  enable_analytics:
    rules:
      - serve: false
  checkout_experiment:
    rules:
      - serve: VARIANT_A
`;

    it('should integrate with runtime SDK Provider correctly', async () => {
      // Write deployment file
      await writeFile(deploymentFile, integrationDeployment);

      // Generate SDK
      await generateSdk(definitionsFile, sdkDir);

      // Compile AST
      await compileAst(definitionsFile, deploymentFile, astFile);

      // Set up SDK for execution
      await setupGeneratedSdk(sdkDir);

      // Load and use the generated SDK
      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      const user = { id: 'user1' };

      // Test that the generated SDK uses Provider correctly
      // Boolean flag evaluation
      const newDashboard = await evaluator.newDashboard(user);
      expect(newDashboard).toBe(true); // Rule says serve: true

      // Multivariate flag evaluation
      const checkoutExperiment = await evaluator.checkoutExperiment(user);
      expect(checkoutExperiment).toBe('VARIANT_A'); // Rule says serve: VARIANT_A

      // Verify the SDK is actually using the Provider (not just returning defaults)
      // If Provider wasn't working, we'd get defaults instead of rule values
      expect(newDashboard).not.toBe(false); // Not the default
      expect(checkoutExperiment).not.toBe('CONTROL'); // Not the default
    });
  });

  describe('Generated SDK Execution', () => {
    const executionDeployment = `environment: production
rules:
  new_dashboard:
    rules:
      - serve: true
  enable_analytics:
    rules:
      - serve: false
  checkout_experiment:
    rules:
      - serve: VARIANT_A
`;

    it('should generate SDK that can be imported and used', async () => {
      // Write deployment file
      await writeFile(deploymentFile, executionDeployment);

      // Generate SDK
      await generateSdk(definitionsFile, sdkDir);

      // Compile AST
      await compileAst(definitionsFile, deploymentFile, astFile);

      // Verify package.json exists and has correct dependencies
      const packageJsonPath = join(sdkDir, 'package.json');
      const packageJson = JSON.parse(await readFile(packageJsonPath, 'utf-8'));
      
      expect(packageJson.dependencies).toHaveProperty('@controlpath/runtime');
      // OpenFeature support has been removed - should not be in dependencies
      expect(packageJson.dependencies).not.toHaveProperty('@openfeature/server-sdk');
      expect(packageJson.main).toBe('index.js');
      expect(packageJson.types).toBe('index.d.ts');

      // Verify all required files exist
      expect(await readFile(join(sdkDir, 'index.ts'), 'utf-8')).toBeTruthy();
      expect(await readFile(join(sdkDir, 'types.ts'), 'utf-8')).toBeTruthy();
      expect(await readFile(join(sdkDir, 'package.json'), 'utf-8')).toBeTruthy();

      // Set up and actually use the SDK
      await setupGeneratedSdk(sdkDir);
      const evaluator = await loadGeneratedSdk(sdkDir, astFile);

      // Verify SDK works end-to-end
      const user = { id: 'user1' };
      const newDashboard = await evaluator.newDashboard(user);
      expect(newDashboard).toBe(true); // Rule says serve: true

      const checkoutExperiment = await evaluator.checkoutExperiment(user);
      expect(checkoutExperiment).toBe('VARIANT_A'); // Rule says serve: VARIANT_A
    });
  });
});

