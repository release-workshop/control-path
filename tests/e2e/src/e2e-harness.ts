/**
 * Shared helpers for SDK generator E2E tests (full suite and pre-merge smoke).
 */

import { writeFile, mkdir } from 'fs/promises';
import { join, dirname } from 'path';
import { spawnSync, execSync } from 'child_process';
import { fileURLToPath } from 'url';
import { readFileSync } from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
let runtimeBuiltForE2E = false;

export const SMOKE_SIMPLE_RULES = `      new_dashboard:
        - serve: true
      enable_analytics:
        - serve: false
      beta_ui:
        - serve: true`;

function getRustCliPath(): string {
  const releasePath = join(__dirname, '../../../target/release/controlpath');
  try {
    readFileSync(releasePath);
    return releasePath;
  } catch {
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

function runCliCommand(
  args: string[],
  cwd: string
): { success: boolean; stdout: string; stderr: string } {
  const rustCli = getRustCliPath();
  const result = spawnSync(rustCli, args, {
    encoding: 'utf-8',
    stdio: 'pipe',
    cwd,
  });

  return {
    success: result.status === 0,
    stdout: result.stdout?.toString() || '',
    stderr: result.stderr?.toString() || '',
  };
}

function buildCatalog(productionRules: string): string {
  return `catalog:
  id: test-service
mode: local
flags:
  new_dashboard:
    default: false
    kind: release
    description: "New dashboard UI feature"
  enable_analytics:
    default: false
    kind: release
    description: "Enable analytics tracking"
  beta_ui:
    default: false
    kind: release
    description: "Beta UI rollout"
environments:
  production:
    rules:
${productionRules}
`;
}

export async function writeCatalog(
  productionRules: string,
  catalogPath: string
): Promise<void> {
  await writeFile(catalogPath, buildCatalog(productionRules));
}

export async function compileAst(catalogDir: string, astFile: string): Promise<void> {
  const result = runCliCommand(
    ['compile', '--env', 'production', '--output', astFile],
    catalogDir
  );

  if (!result.success) {
    throw new Error(`Compilation failed: ${result.stderr || result.stdout}`);
  }
}

export async function generateSdk(catalogDir: string, outputDir: string): Promise<void> {
  const result = runCliCommand(
    ['generate-sdk', '--output', outputDir, '--lang', 'typescript'],
    catalogDir
  );

  if (!result.success) {
    throw new Error(`SDK generation failed: ${result.stderr || result.stdout}`);
  }
}

export async function setupGeneratedSdk(sdkDir: string): Promise<void> {
  const runtimePath = join(__dirname, '../../../runtime/typescript');
  if (!runtimeBuiltForE2E) {
    execSync('npm run build', {
      cwd: runtimePath,
      stdio: 'pipe',
    });
    runtimeBuiltForE2E = true;
  }
  const sdkNodeModules = join(sdkDir, 'node_modules', '@controlpath');
  await mkdir(sdkNodeModules, { recursive: true });

  try {
    execSync(`ln -sf "${runtimePath}" "${join(sdkNodeModules, 'runtime')}"`, {
      stdio: 'pipe',
    });
  } catch {
    // Symlink may fail on Windows; runtime may resolve via parent node_modules.
  }

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

  const tscCommand = resolveTscCommand();

  try {
    execSync(`${tscCommand} --skipLibCheck`, {
      cwd: sdkDir,
      stdio: 'pipe',
    });
  } catch (tscError: unknown) {
    const err = tscError as { stdout?: Buffer; stderr?: Buffer; message?: string };
    const errorOutput =
      err.stdout?.toString() || err.stderr?.toString() || err.message || 'Unknown error';
    throw new Error(`TypeScript compilation failed: ${errorOutput}`);
  }
}

function resolveTscCommand(): string {
  const e2eTypescriptPath = join(__dirname, '../node_modules/.bin/tsc');
  try {
    readFileSync(e2eTypescriptPath);
    return `node "${e2eTypescriptPath}"`;
  } catch {
    return 'npx tsc';
  }
}

/** Typecheck an extra TypeScript file against a generated SDK directory. */
export function typecheckSdkSource(
  sdkDir: string,
  relativePath: string
): { success: boolean; output: string } {
  const tscCommand = resolveTscCommand();
  try {
    execSync(`${tscCommand} --noEmit --skipLibCheck ${relativePath}`, {
      cwd: sdkDir,
      stdio: 'pipe',
    });
    return { success: true, output: '' };
  } catch (tscError: unknown) {
    const err = tscError as { stdout?: Buffer; stderr?: Buffer; message?: string };
    const output =
      err.stdout?.toString() || err.stderr?.toString() || err.message || 'Unknown error';
    return { success: false, output };
  }
}

export async function loadGeneratedSdkModule(sdkDir: string) {
  const sdkPath = `file://${join(sdkDir, 'dist', 'index.js')}`;
  return import(sdkPath);
}

export async function loadGeneratedSdk(sdkDir: string, astFile: string) {
  const sdkModule = await loadGeneratedSdkModule(sdkDir);
  const { Evaluator } = sdkModule;
  const evaluator = new Evaluator();
  await evaluator.init({ artifact: astFile });
  return evaluator;
}
