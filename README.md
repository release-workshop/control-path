# Control Path

Developer-first feature flags for safe releases. Git-native, type-safe, zero network calls.

**Learn more:** [releaseworkshop.com](https://releaseworkshop.com)

## ⚠️ Development Status

**⚠️ WARNING: This project is currently under active development and is NOT ready for production use.**

- The API and features are subject to change
- Breaking changes may occur without notice
- Documentation may be incomplete
- Some features may not be fully implemented or tested

**Do not use this software in production environments.** Use at your own risk.

**DISCLAIMER OF LIABILITY:** This software is provided "AS IS" without warranty of any kind. Release Workshop Ltd and its contributors shall not be liable for any damages, losses, or liabilities arising from the use of this software, including but not limited to direct, indirect, incidental, special, consequential, or punitive damages. By using this software, you agree that Release Workshop Ltd is not responsible for any issues, bugs, data loss, security vulnerabilities, or other problems that may occur.

## Why Control Path?

Feature flags are essential for safe software releases, enabling gradual rollouts, deployment guardrails, and release safety mechanisms. However, traditional feature flag systems come with significant drawbacks:

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

- Flag configuration lives outside your codebase
- Changes require coordination across teams and tools
- No single source of truth for flag definitions and rules

### The Control Path Solution

Control Path solves these problems with a **developer-first, Git-native approach**:

- ✅ **Zero Network Calls** - Flags are evaluated locally in your application (< 1ms per evaluation)
- ✅ **Type-Safe SDKs** - Generated from your flag definitions, catching typos at compile-time
- ✅ **Git-Native Workflow** - Flag definitions and deployment rules live in your repository
- ✅ **OpenFeature Compatible** - Works with industry-standard OpenFeature SDKs
- ✅ **Fast & Reliable** - No external dependencies, works offline, no single point of failure

## Who Control Path is For

**Control Path is designed for engineering teams focused on release safety:**

- ✅ **Engineering teams** who need release guardrails and safety mechanisms
- ✅ **Teams deploying frequently** who need confidence in their releases
- ✅ **Organizations where engineering owns** the safety infrastructure
- ✅ **Developers who want** Git-native workflows and type-safe SDKs

**Control Path is NOT designed for:**

- ❌ **Product teams** running A/B tests and experimentation campaigns
- ❌ **Teams primarily using** feature flags for product optimization
- ❌ **Organizations where experimentation** is the primary use case

**When to Use Control Path:**

- **Use Control Path** if: Engineering owns release safety, you need Git-native workflows, you want type-safe SDKs, you value zero network calls, you need release guardrails
- **Consider other tools** if: Product team runs experiments, you need A/B testing analytics, experimentation is your primary use case

*Note: Control Path focuses on release safety for engineering teams, not product experimentation. We serve engineering teams who need release guardrails, not product teams who need experimentation platforms.*

## What is Control Path?

Control Path is a **Git-native feature flag system** that generates **type-safe SDKs** from your flag definitions. It uses a two-layer architecture.

For more information about Control Path, visit [releaseworkshop.com](https://releaseworkshop.com).

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

**🛡️ Release Safety (Coming Soon)**

- Advanced release safety features that don't exist in other feature flag tools
- Engineering-focused release guardrails
- Enhanced safety mechanisms for confident deployments

## How to Use Control Path

### Quick Start

**1. Install the CLI**

Download the pre-built binary from the [latest release](https://github.com/controlpath/control-path/releases/latest), or build from source:

```bash
# Build from source
cargo build --release --bin controlpath
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

## Roadmap

Control Path is actively being developed with a focus on release safety for engineering teams:

**Current Phase: Developer-First Feature Flags**
- ✅ Git-native workflows
- ✅ Type-safe SDKs
- ✅ Zero network calls
- ✅ File-based kill switches

**Coming Soon: Release Safety Features**
- 🔜 Advanced release safety capabilities - Features that don't exist in other feature flag tools
- 🔜 Enhanced safety mechanisms - Additional guardrails for confident deployments
- 🔜 Engineering-focused release infrastructure - Complete release safety tooling

*Control Path's mission is to improve release safety for engineering teams. We're building unique release safety features that will provide capabilities not available in other feature flag tools, giving engineering teams unprecedented confidence in their deployments.*

## Development

This is a monorepo with **Rust** (for compiler/CLI) and **TypeScript** (for runtime SDK), managed with **npm** for the TypeScript package.

### Prerequisites

- **Rust** (install from [rustup.rs](https://rustup.rs/))
  - Required for building the CLI tool and compiler
- **Node.js 24 LTS** or higher (install from [nodejs.org](https://nodejs.org/) or use [nvm](https://github.com/nvm-sh/nvm))
  - Required for the runtime SDK and build tooling
  - The project includes a `.nvmrc` file for automatic version switching with nvm
- **npm** (comes with Node.js)
  - Required for the TypeScript runtime SDK package management

**Note**:

- The project requires Rust for the CLI tool and compiler
- The project requires Node.js 24 LTS for the TypeScript runtime SDK
- If using nvm, run `nvm use` to automatically switch to the correct Node.js version

### Important: Requirements

**This project requires:**

- **Rust** - Required for CLI tool and compiler
- **Node.js 24 LTS** or higher - Required for runtime SDK and build tooling
- **npm as the package manager** - Standard npm is used for the TypeScript runtime SDK

Make sure you have Node.js 24 LTS installed before working with the TypeScript runtime SDK.

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

# Install Rust (if not installed)
# Using rustup (recommended):
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Or download from https://rustup.rs/

# Verify Rust is installed
rustc --version

# Verify npm is installed (comes with Node.js)
npm --version

# Setup git hooks (optional but recommended)
# This installs pre-commit checks, commit message validation, and git aliases
bash scripts/setup-git-aliases.sh

# Build Rust components
cargo build --release

# Install dependencies and build TypeScript runtime SDK
cd runtime/typescript
npm install
npm run build
cd ../..

# Run Rust tests
cargo test --workspace

# Run TypeScript runtime SDK tests
cd runtime/typescript
npm test
cd ../..

# Lint TypeScript runtime SDK
cd runtime/typescript
npm run lint
cd ../..

# Typecheck TypeScript runtime SDK
cd runtime/typescript
npm run typecheck
cd ../..

# Format TypeScript runtime SDK
cd runtime/typescript
npm run format
cd ../..

# Check formatting
cd runtime/typescript
npm run format:check
cd ../..
```

### Build Pipeline

The build process is straightforward:

1. **Rust components** (compiler and CLI) - Built with Cargo
2. **TypeScript runtime SDK** - Built with TypeScript compiler (tsc)

**Build Commands:**

- `cargo build --release` - Build Rust compiler and CLI
- `cd runtime/typescript && npm run build` - Build TypeScript runtime SDK
- `cargo test --workspace` - Run Rust tests
- `cd runtime/typescript && npm test` - Run TypeScript runtime SDK tests

### Project Structure

```
control-path/
├── crates/
│   ├── compiler/     # AST compiler (Rust library)
│   │   └── src/
│   └── cli/          # CLI tool (Rust binary)
│       └── src/
├── runtime/           # Runtime SDKs
│   └── typescript/   # TypeScript runtime SDK
├── schemas/           # JSON schemas
└── schemas/           # JSON schemas
```

### Runtime Architecture

- **Compiler**: Rust library that compiles flag definitions and deployments to AST artifacts
- **CLI Tool**: Rust binary that provides command-line interface for validation and compilation
- **Runtime SDK**: TypeScript package for loading and evaluating AST artifacts in applications

### Code Quality

- **Rust**: Uses standard Rust tooling (cargo, rustfmt, clippy)
- **ESLint**: Configured with TypeScript rules, Prettier integration (runtime SDK)
- **Prettier**: Code formatting with consistent style
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

**Learn more:** Visit [releaseworkshop.com](https://releaseworkshop.com) for product information, documentation, and updates.
