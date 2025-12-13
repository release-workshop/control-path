# Control Path

Git-native feature flags with type-safe SDKs.

## ⚠️ Development Status

**⚠️ WARNING: This project is currently under active development and is NOT ready for production use.**

- The API and features are subject to change
- Breaking changes may occur without notice
- Documentation may be incomplete
- Some features may not be fully implemented or tested

**Do not use this software in production environments.** Use at your own risk.

**DISCLAIMER OF LIABILITY:** This software is provided "AS IS" without warranty of any kind. Release Workshop Ltd and its contributors shall not be liable for any damages, losses, or liabilities arising from the use of this software, including but not limited to direct, indirect, incidental, special, consequential, or punitive damages. By using this software, you agree that Release Workshop Ltd is not responsible for any issues, bugs, data loss, security vulnerabilities, or other problems that may occur.

## Development Setup

This is a monorepo managed with **pnpm** and **Turborepo**.

### Prerequisites

- **Node.js 24 LTS** or higher (install from [nodejs.org](https://nodejs.org/) or use [nvm](https://github.com/nvm-sh/nvm))
  - Required for the compiler package and build tooling
  - The project includes a `.nvmrc` file for automatic version switching with nvm
- **Deno** (install from [deno.land](https://deno.land/) or use [dvm](https://github.com/justjavac/dvm))
  - Required for the CLI tool runtime
- **pnpm 8+** (install with `npm install -g pnpm`)
  - Required for package management and workspace resolution
- **Turborepo** (installed automatically via pnpm, or install globally with `npm install -g turbo`)

**Note**: 
- The project requires Node.js 24 LTS for the compiler package and build tooling
- The CLI tool runs on Deno runtime (not Node.js)
- The preinstall hook will verify you're using the correct versions
- If using nvm, run `nvm use` to automatically switch to the correct Node.js version
- **Use Turborepo commands** (`turbo run <task>`) instead of pnpm scripts for optimal performance and caching

### About Turborepo

This project uses [Turborepo](https://turbo.build/) for build orchestration. **Always use Turborepo commands** (`turbo run <task>`) instead of pnpm scripts for the best experience.

**Turborepo provides:**
- **Parallel execution** - Tasks run in parallel across packages for faster builds
- **Intelligent caching** - Skips tasks when inputs haven't changed
- **Task dependencies** - Ensures correct execution order (lint → build → test)
- **Incremental builds** - Only rebuilds what changed
- **Remote caching** - Share cache across team and CI/CD (optional)

**Recommended Commands (use these directly):**
- `turbo run build` - Build all packages ⚡ (faster than `pnpm build`)
- `turbo run test` - Run all tests ⚡ (faster than `pnpm test`)
- `turbo run lint` - Lint all packages ⚡ (faster than `pnpm lint`)
- `turbo run format:check` - Check formatting ⚡ (faster than `pnpm format:check`)

**Why use Turborepo directly?**
- Better caching - Turborepo caches results and skips unchanged work
- Parallel execution - Tasks run in parallel across packages
- Faster builds - Only rebuilds what changed
- Better CI/CD integration - Remote caching support

All build, test, lint, and format:check commands should be run through Turborepo for optimal performance.

### Important: Requirements

**This project requires:**
- **Node.js 24 LTS** or higher - Required for compiler package and build tooling
- **Deno** - Required for CLI tool runtime (CLI runs on Deno, not Node.js)
- **pnpm as the package manager** - Using npm or yarn will fail during installation

The preinstall hook automatically checks Node.js LTS and pnpm requirements and provides helpful error messages if they're not met.

### Initial Setup

```bash
# Install Node.js LTS (if not installed)
# Using nvm (recommended):
nvm install
nvm use
# Or manually:
nvm install --lts
nvm use --lts

# Or download from https://nodejs.org/

# Verify Node.js LTS is installed (should be 24.x or higher)
node --version

# Install Deno (if not installed)
# Using installer:
curl -fsSL https://deno.land/install.sh | sh

# Or using dvm (Deno Version Manager):
dvm install
dvm use

# Or download from https://deno.land/

# Verify Deno is installed
deno --version

# Install pnpm (if not installed)
npm install -g pnpm

# Verify pnpm is installed and version 8+
pnpm --version

# Install dependencies (will automatically check for Node.js LTS and pnpm)
# This will also install Turborepo as a dev dependency
pnpm install

# Verify Turborepo is available
npx turbo --version

# Build all packages (runs lint and format:check first, then builds)
# Use turbo commands for optimal performance and caching
turbo run build

# Run tests
turbo run test

# Lint code
turbo run lint

# Format code (still uses pnpm for root-level formatting)
pnpm format

# Check formatting
turbo run format:check
```

### Build Pipeline (Turborepo)

Turborepo orchestrates the build pipeline with the following task dependencies:

1. **Lint** - Code quality checks (runs first, in parallel across packages)
2. **format:check** - Prettier formatting checks (runs in parallel with lint)
3. **Build** - TypeScript compilation (depends on lint passing, runs after dependencies build)
4. **Test** - Test execution (depends on build completing)

**Task Execution Order:**
```
lint ──┐
       ├──> build ──> test
format:check ──┘
```

This ensures:
- Code quality checks run before building
- Formatting is verified before compilation
- Tests only run after successful builds
- Tasks run in parallel when possible for faster execution
- Turborepo caches results to skip unchanged work

### Project Structure

```
control-path/
├── packages/
│   ├── compiler/     # AST compiler (TypeScript library, Node.js runtime)
│   │   ├── src/
│   │   ├── package.json
│   │   └── tsconfig.json
│   └── cli/          # CLI tool (TypeScript, Deno runtime)
│       ├── src/
│       ├── package.json
│       ├── deno.json
│       └── tsconfig.json
├── runtime/           # Runtime SDKs (future)
├── schemas/           # JSON schemas
├── scripts/
│   └── ensure-pnpm.js  # Preinstall hook to enforce pnpm usage
├── .eslintrc.json     # ESLint configuration
├── .prettierrc.json   # Prettier configuration
└── turbo.json         # Turborepo configuration
```

### Runtime Architecture

- **Compiler Package**: Runs on Node.js (uses Node.js APIs for file system, etc.)
- **CLI Tool**: Runs on Deno runtime (uses Deno APIs, can be compiled to native binary)
- Both packages use TypeScript and share code through workspace dependencies

### Code Quality

- **ESLint**: Configured with TypeScript rules, Prettier integration (compiler package)
- **Deno Lint**: Built-in linting for CLI package (Deno runtime)
- **Prettier/Deno Fmt**: Code formatting with consistent style
- **TypeScript**: Strict mode enabled for type safety

## License

This project is licensed under the **Elastic License 2.0**. See the [LICENSE](LICENSE) file for details.

### Licensing Overview

Control Path uses a two-layer SDK architecture with different licensing considerations:

- **Layer 1 (Low-Level Runtime SDK)**: The core runtime SDK (`@controlpath/runtime`) is owned and distributed by Release Workshop Ltd. It cannot be redistributed separately, but can be used as a dependency in your projects.

- **Layer 2 (Generated Type-Safe SDK)**: Generated SDKs created by the CLI tool can be included and redistributed with your application code. These are generated from your flag definitions and are specific to your project.

For more details, see the [LICENSE](LICENSE) file and [CONTRIBUTING.md](CONTRIBUTING.md).

### Contributing

We welcome contributions! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines. By contributing, you agree to our [Contributor License Agreement](CONTRIBUTOR_LICENSE_AGREEMENT.md), which grants Release Workshop Ltd ownership of your contributions.

