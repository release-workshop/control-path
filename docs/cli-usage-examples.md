# CLI Usage Examples

This document provides comprehensive examples for using the Control Path CLI, with a focus on Phase 5 developer tooling commands.

## Table of Contents

- [Quick Start](#quick-start)
- [Watch Mode](#watch-mode)
- [Debug Commands](#debug-commands)
- [Flag Management](#flag-management)
- [Environment Management](#environment-management)
- [Setup Command](#setup-command)
- [Complete Workflows](#complete-workflows)

## Quick Start

### Initial Setup

```bash
# One-command setup (auto-detects language)
controlpath setup

# Setup with specific language
controlpath setup --lang typescript

# Setup without installing runtime SDK
controlpath setup --lang typescript --skip-install
```

### Basic Project Initialization

```bash
# Initialize project structure
controlpath init

# Initialize with example flags
controlpath init --example-flags

# Initialize without examples
controlpath init --no-examples
```

## Watch Mode

### Watch Everything

Watch both definitions and deployment files:

```bash
# Watch definitions and deployments, regenerate SDK on changes
controlpath watch --lang typescript
```

Output:
```
Watching for changes...
  ✓ flags.definitions.yaml (SDK generation)
  ✓ .controlpath/production.deployment.yaml (AST compilation)
  ✓ .controlpath/staging.deployment.yaml (AST compilation)

Press Ctrl+C to stop
```

### Watch Definitions Only

Regenerate SDK when flag definitions change:

```bash
controlpath watch --definitions --lang typescript
```

### Watch Deployments Only

Recompile ASTs when deployment files change:

```bash
controlpath watch --deployments
```

### Use Cases

**Development Workflow:**
```bash
# Terminal 1: Start watch mode
controlpath watch --lang typescript

# Terminal 2: Edit files
# - Edit flags.definitions.yaml → SDK regenerates automatically
# - Edit .controlpath/production.deployment.yaml → AST recompiles automatically
```

## Debug Commands

### Explain Command

#### Basic Usage

Explain how a flag evaluates for a specific user:

```bash
controlpath explain --flag new_dashboard --user user.json --env production
```

Example `user.json`:
```json
{
  "id": "123",
  "role": "admin",
  "email": "admin@example.com"
}
```

Output:
```
Flag: new_dashboard
Value: true

Rule Matched: "Admin Users"
  Condition: user.role == "admin"
  Evaluation: "admin" == "admin" ✓

Default: false (not used)
```

#### With Detailed Trace

Show step-by-step evaluation:

```bash
controlpath explain --flag new_dashboard --user user.json --env production --trace
```

#### With JSON String

Use inline JSON instead of a file:

```bash
controlpath explain --flag new_dashboard \
  --user '{"id":"123","role":"admin"}' \
  --env production
```

#### With Context

Include context object:

```bash
controlpath explain --flag new_dashboard \
  --user user.json \
  --context context.json \
  --env production
```

Example `context.json`:
```json
{
  "country": "US",
  "timezone": "America/New_York"
}
```

### Debug UI

#### Start Debug UI

Launch the interactive web-based debug UI:

```bash
# Default port (8080)
controlpath debug

# Custom port
controlpath debug --port 3000

# Open browser automatically
controlpath debug --open

# Use specific environment
controlpath debug --env staging

# Use specific AST file
controlpath debug --ast .controlpath/custom.ast
```

#### Using the Debug UI

1. Open http://localhost:8080 in your browser
2. Select a flag from the dropdown
3. Enter user JSON (or use the form)
4. Optionally enter context JSON
5. Click "Evaluate" to see results
6. View rule matching details and evaluation trace

## Flag Management

### Add Flags

#### Interactive Mode

Add a flag with interactive prompts:

```bash
controlpath flag add
```

Prompts for:
- Flag name
- Flag type (boolean/multivariate)
- Default value
- Description
- Whether to sync to deployments
- Whether to regenerate SDK

#### Non-Interactive Mode

Add a flag with all options specified:

```bash
# Boolean flag
controlpath flag add \
  --name my_feature \
  --type boolean \
  --default false \
  --description "My new feature flag"

# Multivariate flag
controlpath flag add \
  --name button_color \
  --type multivariate \
  --default blue \
  --description "Button color variation"

# Add and sync to deployments
controlpath flag add \
  --name my_feature \
  --type boolean \
  --sync

# Add and regenerate SDK
controlpath flag add \
  --name my_feature \
  --type boolean \
  --lang typescript
```

### List Flags

#### List from Definitions

```bash
# Table format (default)
controlpath flag list

# JSON format
controlpath flag list --format json

# YAML format
controlpath flag list --format yaml
```

#### List from Deployment

```bash
# List flags in production environment
controlpath flag list --deployment production

# List as JSON
controlpath flag list --deployment production --format json
```

### Show Flag Details

```bash
# Show flag definition
controlpath flag show --name my_feature

# Show flag in specific environment
controlpath flag show --name my_feature --deployment production

# Show as JSON
controlpath flag show --name my_feature --format json
```

### Remove Flags

```bash
# Remove from definitions only
controlpath flag remove --name my_feature --from-deployments false

# Remove from all deployments (default)
controlpath flag remove --name my_feature

# Remove from specific environment
controlpath flag remove --name my_feature --env staging

# Force removal without confirmation
controlpath flag remove --name my_feature --force
```

## Environment Management

### Add Environments

#### Basic Usage

```bash
# Interactive mode
controlpath env add

# With name
controlpath env add --name staging

# With template (copies flags from production)
controlpath env add --name staging --template production
```

### Sync Environments

#### Sync All Environments

Sync flags from definitions to all deployment files:

```bash
controlpath env sync
```

Output:
```
Syncing flags to all environments...

Production:
  + Added: new_feature (disabled)
  - Removed: old_feature

Staging:
  + Added: new_feature (disabled)
  ✓ Already synced: existing_feature

Sync complete!
```

#### Sync Specific Environment

```bash
controlpath env sync --env staging
```

#### Dry Run

Preview what would be synced:

```bash
controlpath env sync --dry-run
```

### List Environments

```bash
# Table format (default)
controlpath env list

# JSON format
controlpath env list --format json

# YAML format
controlpath env list --format yaml
```

### Remove Environments

```bash
# Remove with confirmation
controlpath env remove --name staging

# Force removal
controlpath env remove --name staging --force
```

## Setup Command

### Complete Setup

The `setup` command performs a complete project initialization:

```bash
# Auto-detect language
controlpath setup

# Specify language
controlpath setup --lang typescript
```

What it does:
1. Creates project structure (`init`)
2. Creates sample flag
3. Compiles AST for production environment
4. Installs runtime SDK (unless `--skip-install`)
5. Generates SDK
6. Creates example usage file

### Setup Output

```
✓ Project initialized
✓ Sample flag created
✓ AST compiled for production
✓ Runtime SDK installed
✓ SDK generated in ./flags
✓ Example usage file created: example_usage.ts

Setup complete!

Next steps:
  1. Add your first flag:    controlpath new-flag
  2. Enable a flag:          controlpath enable <flag> --env staging
  3. Test flags:             controlpath test
  4. Start watch mode:       controlpath watch
  5. Get help:               controlpath help
```

## Monorepo Support

Control Path CLI supports monorepo environments where multiple services each need their own Control Path setup.

### Working in a Monorepo

#### From Service Directory

```bash
# Navigate to service
cd services/service-a

# Commands work as normal (backward compatible)
controlpath compile --env production
controlpath generate-sdk --lang typescript
```

#### From Workspace Root

```bash
# Stay at workspace root
cd /path/to/monorepo

# Target specific service
controlpath compile --service service-a --env production
controlpath validate --service service-a

# Explicit workspace root
controlpath compile --service service-a --workspace-root . --env production
```

### Monorepo Configuration

Create `.controlpath/config.yaml` at workspace root:

```yaml
language: typescript
defaultEnv: production

monorepo:
  serviceDirectories:
    - services
    - packages
  discovery: auto
```

### Monorepo Examples

#### Standard Services Layout

```
monorepo/
├── .controlpath/config.yaml
├── services/
│   ├── api-service/
│   │   ├── flags.definitions.yaml
│   │   └── .controlpath/
│   └── web-service/
│       ├── flags.definitions.yaml
│       └── .controlpath/
```

#### Working with Multiple Services

```bash
# Compile all services (when --all-services is implemented)
controlpath compile --all-services --env production

# Validate specific service
controlpath validate --service api-service --all

# Generate SDK for specific service
controlpath generate-sdk --service web-service --lang typescript
```

## Complete Workflows

### Workflow 1: Adding a New Feature Flag

Complete workflow from creation to deployment:

```bash
# 1. Add flag to definitions
controlpath flag add --name new_dashboard --type boolean --default false

# 2. Sync to environments
controlpath env sync

# 3. Enable in staging with rule
controlpath enable new_dashboard --env staging --rule "user.role == 'admin'"

# 4. Test the flag
controlpath explain --flag new_dashboard --user admin.json --env staging

# 5. Compile for staging
controlpath compile --env staging

# 6. Deploy to production when ready
controlpath enable new_dashboard --env production --rule "user.role == 'admin'"
controlpath compile --env production
```

### Workflow 2: Development with Watch Mode

Development workflow using watch mode:

```bash
# Terminal 1: Start watch mode
controlpath watch --lang typescript

# Terminal 2: Make changes
# Edit flags.definitions.yaml → SDK regenerates automatically
# Edit .controlpath/staging.deployment.yaml → AST recompiles automatically

# Terminal 3: Test changes
controlpath explain --flag my_feature --user user.json --env staging
```

### Workflow 3: Debugging Flag Evaluation

Debug why a flag isn't working as expected:

```bash
# 1. Use explain command for quick check
controlpath explain --flag my_feature --user user.json --env production --trace

# 2. Use debug UI for interactive exploration
controlpath debug --env production

# 3. In browser:
#    - Select flag
#    - Try different user values
#    - See which rules match
#    - View evaluation trace
```

### Workflow 4: Multi-Environment Management

Managing flags across multiple environments:

```bash
# 1. Add new environment
controlpath env add --name staging --template production

# 2. Add new flag
controlpath flag add --name new_feature --type boolean

# 3. Sync to all environments
controlpath env sync

# 4. Enable in staging only
controlpath enable new_feature --env staging --rule "user.beta == true"

# 5. Test in staging
controlpath explain --flag new_feature --user beta_user.json --env staging

# 6. When ready, enable in production
controlpath enable new_feature --env production --rule "user.beta == true"

# 7. Deploy both environments
controlpath deploy --env staging,production
```

### Workflow 5: Flag Lifecycle Management

Complete flag lifecycle:

```bash
# 1. Create flag
controlpath flag add --name experimental_feature --type boolean

# 2. Enable in staging for testing
controlpath enable experimental_feature --env staging --rule "user.role == 'tester'"

# 3. Gradually roll out
controlpath enable experimental_feature --env production --rule "user.id IN ['user1', 'user2']"

# 4. Monitor with debug UI
controlpath debug --env production

# 5. When stable, enable for all
controlpath enable experimental_feature --env production --all

# 6. Eventually remove flag
controlpath flag remove --name experimental_feature
```

## Tips and Best Practices

### Watch Mode

- Use watch mode during active development
- Watch definitions only when you're only changing flag definitions
- Watch deployments only when you're only changing deployment rules
- Watch everything when working on both

### Debug Commands

- Use `explain` for quick command-line debugging
- Use `debug` UI for interactive exploration
- Use `--trace` flag to understand complex rule evaluation
- Test with different user/context combinations

### Flag Management

- Use interactive mode when unsure of options
- Use `--sync` to automatically sync new flags to all environments
- Use `--force` carefully (skips confirmation prompts)
- List flags regularly to see current state

### Environment Management

- Use templates when creating similar environments
- Sync regularly to keep environments in sync
- Use `--dry-run` to preview sync changes
- Remove unused environments to keep project clean

## See Also

- [Rust CLI Documentation](./rust-cli.md)
- [CLI Command Specifications](../../control-path-planning/specs/cli-commands.md)
- [Architecture Documentation](../../control-path-planning/ARCHITECTURE.md)

