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

## Why Control Path?

Feature flags are essential for modern software development, enabling gradual rollouts, A/B testing, and safe deployments. However, traditional feature flag systems come with significant drawbacks:

### The Problems with Traditional Feature Flags

**🔴 Network Dependency & Latency**

- Every flag evaluation requires a network call to a SaaS service
- Adds latency to your application (often 50-200ms per evaluation)
- Creates a single point of failure - if the service is down, your app breaks
- Requires complex caching strategies that can lead to stale data

**🔴 String-Based APIs Lead to Bugs**

- Typo in a flag name? You'll only find out at runtime
- No IDE autocomplete means you're constantly checking documentation
- Refactoring flag names requires manual string searches across your codebase
- Easy to accidentally use the wrong flag name

**🔴 Lack of Type Safety**

- No compile-time validation of flag types or values
- Runtime errors when you expect a boolean but get a string
- No type checking for user attributes or context properties
- Bugs slip through to production

**🔴 Vendor Lock-In & Complexity**

- Your flag configuration lives in a third-party SaaS platform
- No Git history or audit trail for flag changes
- Requires separate tooling and workflows from your codebase
- Complex integrations and API dependencies

**🔴 Separation of Concerns**

- Engineering defines flags, but Product manages targeting rules
- Changes require coordination across teams and tools
- No single source of truth for flag definitions and rules

### The Control Path Solution

Control Path solves these problems with a **developer-first, Git-native approach**:

- ✅ **Zero Network Calls** - Flags are evaluated locally in your application (< 1ms per evaluation)
- ✅ **Type-Safe SDKs** - Generated from your flag definitions, catching typos at compile-time
- ✅ **Git-Native Workflow** - Flag definitions and deployment rules live in your repository
- ✅ **OpenFeature Compatible** - Works with industry-standard OpenFeature SDKs
- ✅ **Fast & Reliable** - No external dependencies, works offline, no single point of failure

## What is Control Path?

Control Path is a **Git-native feature flag system** that generates **type-safe SDKs** from your flag definitions. It uses a two-layer architecture.

### Two-Layer Architecture

```
┌─────────────────────────────────────────────┐
│  Your Application Code                      │
│  evaluator.newDashboard(context)            │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│  Layer 2: Generated Type-Safe SDK           │
│  • Type-safe methods per flag               │
│  • IDE autocomplete                         │
│  • Compile-time validation                  │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│  Layer 1: Low-Level Runtime SDK             │
│  • AST artifact loading                     │
│  • OpenFeature-compliant Provider           │
│  • Flag evaluation                          │
└─────────────────────────────────────────────┘
```

### Key Features

**🎯 Type-Safe SDKs**

- Generate type-safe methods for each flag (e.g., `evaluator.newDashboard()`)
- IDE autocomplete for all flags and their types
- Compile-time validation catches typos before deployment
- Type-safe user and context objects

**📝 Git-Native Workflow**

- Flag definitions (`flags.definitions.yaml`) live in your repository
- Deployment rules (`.controlpath/production.deployment.yaml`) are versioned in Git
- Complete audit trail through Git history
- Standard Git workflows: branches, PRs, reviews, rollbacks

**⚡ Zero Network Calls**

- Flags are compiled to compact AST artifacts (< 12KB for 500 flags)
- AST artifacts are evaluated locally in your application
- Sub-millisecond evaluation (< 1ms per flag)
- Works offline, no external service dependencies

**🔧 OpenFeature Compatible**

- Low-level SDK directly implements OpenFeature Provider interface
- Works with any OpenFeature SDK (no adapter needed)
- Industry-standard API for feature flag evaluation

**🚀 Fast & Reliable**

- AST artifacts are small and efficient (MessagePack format)
- In-memory evaluation with no I/O overhead
- Graceful fallback to embedded defaults if AST fails to load
- "Never Throws" policy ensures your app keeps running

**🎨 Flexible Deployment**

- Bundle AST artifacts with your application code
- Or load from CDN/object storage at runtime
- Support for multiple environments (production, staging, dev)
- Hot reloading support for development

## How to Use Control Path

### Quick Start

**1. Install the CLI**

```bash
# Using npm
npm install -g @controlpath/cli

# Using pnpm
pnpm add -g @controlpath/cli

# Using deno
deno install -A -n controlpath https://deno.land/x/controlpath/cli.ts
```

**2. Initialize Your Project**

```bash
controlpath init
```

This creates:

- `flags.definitions.yaml` - Define your flags here
- `.controlpath/` directory - Deployment rules per environment

**3. Define Your Flags**

Edit `flags.definitions.yaml`:

```yaml
flags:
  new_dashboard:
    type: boolean
    default: false
    description: 'Enable the new dashboard UI'

  welcome_message:
    type: string
    default: 'Welcome!'
    variations:
      - 'Welcome!'
      - 'Hello there!'
      - 'Greetings!'
```

**4. Generate Type-Safe SDK**

```bash
controlpath generate-sdk
```

This generates a type-safe SDK in `./flags/` directory.

**5. Configure Deployment Rules**

Edit `.controlpath/production.deployment.yaml`:

```yaml
flags:
  new_dashboard:
    rules:
      - when: "user.role == 'admin'"
        serve: ON
      - when: 'user.beta_tester == true'
        serve: ON
      - serve: OFF # Default
```

**6. Compile AST Artifact**

```bash
controlpath compile --env production
```

This creates `.controlpath/production.ast` - a compact binary artifact.

**7. Use in Your Application**

```typescript
import { Evaluator } from './flags';

const evaluator = new Evaluator();
await evaluator.loadArtifact('./.controlpath/production.ast');

// Type-safe flag evaluation
const showNewDashboard = await evaluator.newDashboard(user, context);
if (showNewDashboard) {
  // Render new dashboard
}
```

### Workflow Example

```bash
# 1. Add a new flag
controlpath flag add new_feature --type boolean

# 2. Configure deployment rules
# Edit .controlpath/production.deployment.yaml

# 3. Validate configuration
controlpath validate

# 4. Compile AST artifact
controlpath compile --env production

# 5. Generate/regenerate SDK
controlpath generate-sdk

# 6. Commit changes
git add flags.definitions.yaml .controlpath/production.deployment.yaml .controlpath/production.ast
git commit -m "Add new_feature flag"
```

### Advanced Usage

**Multiple Environments**

```bash
# Compile for different environments
controlpath compile --env production
controlpath compile --env staging
controlpath compile --env development
```

**Expression Language**

Control Path supports a powerful expression language for targeting:

```yaml
rules:
  - when: "user.role == 'admin' AND context.environment == 'production'"
    serve: ON
  - when: "IN_SEGMENT(user, 'beta_users') AND user.account_age_days > 30"
    serve: ON
  - when: 'HASHED_PARTITION(user.id, 100) < 25' # 25% rollout
    serve: ON
```

**Segments**

Define user segments in your deployment file:

```yaml
segments:
  beta_users:
    when: "user.role == 'beta' AND user.account_age_days > 30"
  premium_customers:
    when: "user.subscription_tier == 'premium' OR user.subscription_tier == 'pro'"
```

Then use them in flag rules:

```yaml
rules:
  - when: "IN_SEGMENT(user, 'beta_users')"
    serve: ON
```

**Hot Reloading**

For development, you can reload AST artifacts without restarting:

```typescript
// Reload artifact when it changes
await evaluator.reloadArtifact('./.controlpath/production.ast');
```

## Development

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
