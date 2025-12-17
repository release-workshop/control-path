/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Control Path CLI
 * Main entry point for the CLI tool (Deno runtime)
 */

import { validateCommand } from './commands/validate.ts';
import { compileCommand } from './commands/compile.ts';
import { initCommand } from './commands/init.ts';

interface ParsedArgs {
  command?: string;
  flags: Record<string, string | boolean>;
  args: string[];
}

/**
 * Parse command line arguments.
 */
function parseArgs(args: string[]): ParsedArgs {
  const result: ParsedArgs = {
    flags: {},
    args: [],
  };

  let i = 0;
  while (i < args.length) {
    const arg = args[i];

    if (arg.startsWith('--')) {
      const flagName = arg.slice(2);
      const nextArg = args[i + 1];

      // Check if next argument is a value (not a flag)
      if (nextArg && !nextArg.startsWith('-') && i + 1 < args.length) {
        result.flags[flagName] = nextArg;
        i += 2;
      } else {
        // Boolean flag
        result.flags[flagName] = true;
        i += 1;
      }
    } else if (arg.startsWith('-') && arg.length > 1 && arg[1] !== '-') {
      // Short flag (single dash)
      const flagName = arg.slice(1);
      const nextArg = args[i + 1];

      if (nextArg && !nextArg.startsWith('-') && i + 1 < args.length) {
        result.flags[flagName] = nextArg;
        i += 2;
      } else {
        result.flags[flagName] = true;
        i += 1;
      }
    } else {
      // Positional argument
      if (!result.command && !arg.startsWith('-')) {
        result.command = arg;
      } else {
        result.args.push(arg);
      }
      i += 1;
    }
  }

  return result;
}

/**
 * Show help message.
 */
function showHelp(): void {
  console.log(`Control Path CLI

Usage:
  controlpath <command> [flags]

Commands:
  validate    Validate flag definitions and deployment files
  compile     Compile deployment files to AST artifacts
  init        Initialize a new Control Path project

Examples:
  controlpath validate --definitions flags.definitions.yaml
  controlpath validate --deployment .controlpath/production.deployment.yaml
  controlpath compile --env production
  controlpath compile --deployment .controlpath/production.deployment.yaml --output .controlpath/production.ast
  controlpath init

For more information, see: https://github.com/controlpath/control-path
`);
}

/**
 * Show version.
 */
function showVersion(): void {
  console.log('0.1.0');
}

/**
 * Main CLI entry point.
 */
async function main(): Promise<number> {
  const args = Deno.args;
  const parsed = parseArgs(args);

  // Handle global flags
  if (parsed.flags.help || parsed.flags.h) {
    showHelp();
    return 0;
  }

  if (parsed.flags.version || parsed.flags.v) {
    showVersion();
    return 0;
  }

  // Handle commands
  const command = parsed.command;

  if (!command) {
    showHelp();
    return 1;
  }

  switch (command) {
    case 'validate': {
      return await validateCommand({
        definitions: parsed.flags.definitions as string | undefined,
        deployment: parsed.flags.deployment as string | undefined,
        env: parsed.flags.env as string | undefined,
        all: parsed.flags.all === true,
      });
    }

    case 'compile': {
      return await compileCommand({
        deployment: parsed.flags.deployment as string | undefined,
        env: parsed.flags.env as string | undefined,
        output: parsed.flags.output as string | undefined,
        definitions: parsed.flags.definitions as string | undefined,
      });
    }

    case 'init': {
      return await initCommand({
        force: parsed.flags.force === true,
        exampleFlags: parsed.flags['example-flags'] === true,
        noExamples: parsed.flags['no-examples'] === true,
      });
    }

    default: {
      console.error(`✗ Unknown command: ${command}`);
      console.error(`  Run 'controlpath --help' for usage information`);
      return 1;
    }
  }
}

// Run CLI if this is the main module
if (import.meta.main) {
  const exitCode = await main();
  Deno.exit(exitCode);
}
