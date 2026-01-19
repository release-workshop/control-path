# CLI Usage Guide

This guide explains how to use the Control Path CLI with a focus on the core mental model and common workflows.

## Mental Model: Three Core Concepts

Control Path is built around three core concepts:

1. **Flags** → Flag definitions (`flags.definitions.yaml`)
   - What flags you have and their types/defaults
   - Declare flags here

2. **Environments** → Deployment files (`.controlpath/<env>.deployment.yaml`)
   - How flags behave per environment (rollout rules, targeting)
   - Configure rollouts here

3. **SDK** → Generated code (`./flags/`)
   - Type-safe SDK that your application code imports and uses
   - Generated automatically from your flag definitions

Everything else (AST artifacts, compiler details) is handled automatically by the CLI as part of higher-level workflows. You don't need to think about them in day-to-day use.

## Quick Start

### First Time Setup

```bash
# One command to get started (auto-detects language)
controlpath setup
```

This creates:
- `flags.definitions.yaml` with an example flag
- `.controlpath/production.deployment.yaml` (and optionally staging)
- Generated SDK in `./flags/` for your application code
- Installs runtime SDK package
- Compiles ASTs automatically

### Add Your First Flag

```bash
# Add a flag and enable it in staging
controlpath new-flag my_feature --enable staging
```

This:
- Adds the flag to `flags.definitions.yaml`
- Syncs it to all environment deployment files
- Enables it in staging
- Regenerates the SDK automatically
- Compiles the staging AST automatically

### Use the Generated SDK

Import and use the generated SDK in your application code:

```typescript
// TypeScript example
import { evaluator } from './flags';

const user = { id: '123', role: 'admin' };
const context = {};

if (evaluator.myFeature(user, context)) {
  // Feature is enabled for this user
}
```

### Deploy When Ready

```bash
# Deploy to staging and production
controlpath deploy --env staging,production
```

This validates everything and compiles ASTs for the specified environments.

## Common Workflows

### Daily Development Workflow

```bash
# Start dev mode (watches files, auto-regenerates SDK, auto-compiles ASTs)
controlpath dev

# In another terminal: add flags, enable them, test
controlpath new-flag my_feature --enable staging
controlpath test my_feature --env staging
```

### Adding a New Flag

```bash
# Complete workflow: add, enable, and deploy in one command
controlpath new-flag my_feature --enable staging --deploy staging

# Or step by step
controlpath new-flag my_feature
controlpath enable my_feature --env staging
controlpath deploy --env staging
```

### Enabling a Flag

```bash
# Enable for all users in staging
controlpath enable my_feature --env staging --all

# Enable with a rule (e.g., admins only)
controlpath enable my_feature --env staging --rule "user.role == 'admin'"
```

### Testing Flags

```bash
# Test a flag with different users
controlpath test my_feature --env staging

# Test with specific user JSON
controlpath test my_feature --user user.json --env staging
```

### CI/CD Integration

The `ci` command is designed for CI/CD pipelines. It validates, compiles, and regenerates SDKs as needed.

```bash
# Basic usage - validates, compiles, and regenerates SDK
controlpath ci

# Validate and compile specific environments only
controlpath ci --env production --env staging

# Skip SDK regeneration (faster, if SDK is already up to date)
controlpath ci --no-sdk

# Skip validation (faster, but less safe - use with caution)
controlpath ci --no-validate
```

#### GitHub Actions Example

```yaml
name: CI

on: [push, pull_request]

jobs:
  control-path-checks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Control Path CLI
        run: |
          # Install controlpath CLI (adjust for your installation method)
          # Example: cargo install --path . or download from releases
      
      - name: Control Path checks
        run: controlpath ci
```

The `ci` command will:
- ✅ Validate `flags.definitions.yaml` and all deployment files
- ✅ Compile ASTs for all environments (or specified with `--env`)
- ✅ Regenerate the SDK (unless `--no-sdk` is used)
- ❌ Exit with non-zero status if validation or compilation fails

## Command Reference

### Workflow Commands (Start Here)

- **`setup`** - One-command bootstrap for new projects
- **`new-flag`** - Add a new flag (optionally enable and deploy)
- **`enable`** / **`disable`** - Turn flags on/off per environment
- **`deploy`** - Validate and compile for deployment
- **`test`** - Test flag evaluation with different users

### Core Commands

- **`validate`** - Validate definitions and deployment files
- **`compile`** - Compile deployment → AST (usually automatic)
- **`generate-sdk`** - Generate SDK from definitions (usually automatic)

### Management Commands

- **`flag`** - Manage flags (add, list, show, remove)
- **`env`** - Manage environments (add, sync, list, remove)
- **`services`** - Manage services in monorepo

### Debug Commands

- **`explain`** - Explain flag evaluation for a user
- **`debug`** - Interactive debug UI
- **`status`** - Show project health

### Development Commands

- **`dev`** - Development mode (watch files, auto-regenerate)
- **`watch`** - Watch mode for file changes

### CI Commands

- **`ci`** - Single command for CI pipelines

## File Structure

```
your-project/
├── flags.definitions.yaml          # Flag definitions (what flags you have)
├── .controlpath/
│   ├── config.yaml                 # Optional config (language, defaults)
│   ├── production.deployment.yaml  # Production rollout config
│   ├── staging.deployment.yaml     # Staging rollout config
│   ├── production.ast              # Compiled AST (auto-generated)
│   └── staging.ast                 # Compiled AST (auto-generated)
└── flags/                          # Generated SDK (import this in your code)
    ├── index.ts                    # TypeScript SDK
    └── ...
```

## Key Points

1. **You edit**: `flags.definitions.yaml` and `.controlpath/<env>.deployment.yaml`
2. **You import**: The generated SDK from `./flags/` in your application code
3. **You rarely touch**: `.controlpath/<env>.ast` files (they're auto-generated)

## Next Steps

- See [CLI Usage Examples](./cli-usage-examples.md) for detailed examples
- See [specs/cli-commands.md](../../control-path-planning/specs/cli-commands.md) for complete command reference
- Run `controlpath help` for command-specific help
