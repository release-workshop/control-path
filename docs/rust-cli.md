# Rust CLI Usage Documentation

This document describes how to use the Control Path Rust CLI (`controlpath`).

## Installation

### From Source

Build from the repository:

```bash
cd control-path
cargo build --release --bin controlpath
```

The binary will be located at `target/release/controlpath`.

### Distribution

The CLI is distributed as a native binary for:
- Linux (x86_64)
- macOS (x86_64, ARM64)
- Windows (x86_64)

## Commands

### `validate`

Validate `control-path.yaml` and environment rules against schemas.

#### Usage

```bash
controlpath validate [OPTIONS]
```

#### Options

- `--env <ENV>`: Validate specific environment rules
- `--all`: Validate all environments in the configuration file

#### Examples

Validate the configuration file:

```bash
controlpath validate
```

Validate specific environment:

```bash
controlpath validate --env production
```

Validate all environments:

```bash
controlpath validate --all
```

#### Exit Codes

- `0`: Validation passed
- `1`: Validation failed or no files found

### `compile`

Compile environment rules from `control-path.yaml` into AST artifacts.

#### Usage

```bash
controlpath compile [OPTIONS]
```

#### Options

- `--env <ENV>`: Environment name
- `--output <FILE>`: Output path for AST file (default: `.controlpath/<env>.ast`)

#### Examples

Compile using environment name:

```bash
controlpath compile --env production
```

This will:
1. Read `control-path.yaml`
2. Extract production environment rules
3. Compile to `.controlpath/production.ast`

Compile with custom output path:

```bash
controlpath compile --env production --output dist/production.ast
```

#### Exit Codes

- `0`: Compilation succeeded
- `1`: Compilation failed

### `setup`

One-command setup for new projects. Creates project structure, sample flags, compiles ASTs, installs runtime SDK, and generates type-safe SDKs.

#### Usage

```bash
controlpath setup [OPTIONS]
```

#### Options

- `--lang <LANGUAGE>`: Language for SDK generation (auto-detected if not provided)
- `--skip-install`: Skip installing runtime SDK package
- `--no-examples`: Skip creating example flags/usage files

#### Examples

Auto-detect language and setup:

```bash
controlpath setup
```

Setup with specific language:

```bash
controlpath setup --lang typescript
```

Setup without installing runtime SDK:

```bash
controlpath setup --lang typescript --skip-install
```

#### Exit Codes

- `0`: Setup successful
- `1`: Setup failed

### `watch`

Watches files and auto-regenerates SDK/AST on changes.

#### Usage

```bash
controlpath watch [OPTIONS]
```

#### Options

- `--lang <LANGUAGE>`: Language for SDK generation (default: typescript; used when SDK regeneration runs)
- `--definitions`: Regenerate SDK only when `control-path.yaml` changes (skip AST recompilation)
- `--deployments`: Recompile AST artifacts only when `control-path.yaml` changes (skip SDK regeneration)

#### Examples

Watch everything (SDK + AST recompilation):

```bash
controlpath watch --lang typescript
```

Regenerate SDK only on catalog changes:

```bash
controlpath watch --definitions --lang typescript
```

Recompile ASTs only on catalog changes:

```bash
controlpath watch --deployments
```

#### Behavior

- Validates `control-path.yaml` exists before watching
- Watches `control-path.yaml` for changes
- Default (no flags): regenerates SDK and recompiles all environment ASTs
- `--definitions`: SDK regeneration only
- `--deployments`: AST recompilation only
- Shows output when the catalog changes
- Runs until interrupted (Ctrl+C)

#### Exit Codes

- `0`: Normal exit
- `1`: Error (file missing, permission error, etc.)

### `explain`

Explains flag evaluation for a given user/context.

#### Usage

```bash
controlpath explain [OPTIONS]
```

#### Options

- `--flag <NAME>`: Flag name (required)
- `--user <FILE|JSON>`: User JSON file or JSON string
- `--context <FILE|JSON>`: Optional context JSON file or JSON string
- `--env <ENV>`: Environment name (uses `.controlpath/<env>.ast`)
- `--ast <FILE>`: Path to AST file (alternative to `--env`)
- `--trace`: Show detailed trace of evaluation

#### Examples

Explain with user file:

```bash
controlpath explain --flag new_dashboard --user user.json --env production
```

Explain with detailed trace:

```bash
controlpath explain --flag new_dashboard --user user.json --env production --trace
```

Explain with JSON string:

```bash
controlpath explain --flag new_dashboard --user '{"id":"123","role":"admin"}' --context '{"environment":"production"}' --env production
```

#### Output

Shows:
- Flag value
- Which rule matched (if any)
- Why rule matched/didn't match
- Expression evaluation details (if `--trace`)

Note: User/context should include all properties used in flag rules (e.g., `role`, `environment`, `id`, etc.)

#### Exit Codes

- `0`: Success
- `1`: Error

### `debug`

Starts interactive debug UI.

#### Usage

```bash
controlpath debug [OPTIONS]
```

#### Options

- `--port <PORT>`: Port for web server (default: 8080)
- `--env <ENV>`: Environment name (uses `.controlpath/<env>.ast`)
- `--ast <FILE>`: Path to AST file (alternative to `--env`)
- `--open`: Open browser automatically

#### Examples

Start debug UI with default settings:

```bash
controlpath debug
```

Start on custom port:

```bash
controlpath debug --port 3000
```

Start and open browser automatically:

```bash
controlpath debug --open
```

#### Behavior

- Starts web server at http://localhost:8080 (or specified port)
- Provides UI for flag evaluation
- Shows rule matching details
- Allows testing different users/contexts
- Shows all flags and their current values
- Runs until interrupted (Ctrl+C)

#### Exit Codes

- `0`: Normal exit
- `1`: Error

### `flag`

Manage flags (add, list, show, remove).

#### `flag add`

Adds a new boolean flag to `control-path.yaml` and optionally seeds environment rules.

##### Usage

```bash
controlpath flag add [OPTIONS]
```

##### Options

- `--name <NAME>`: Flag name (required, snake_case format)
- `--type <TYPE>`: Flag type (`boolean` only in v2 catalogs)
- `--default <VALUE>`: Default value (`true` or `false`)
- `--description <TEXT>`: Description
- `--lang <LANGUAGE>`: Language for SDK regeneration
- `--sync`: Add default serve rules for all environments in the catalog
- `--no-interactive`: Disable interactive mode

##### Examples

Interactive mode (prompts for values):

```bash
controlpath flag add
```

Add with all options:

```bash
controlpath flag add --name my_feature --type boolean --default false --description "My feature flag"
```

Add and seed rules in every environment:

```bash
controlpath flag add --name my_feature --sync
```

#### `flag list`

Lists flags from the catalog or from a specific environment's rules.

##### Usage

```bash
controlpath flag list [OPTIONS]
```

##### Options

- `--definitions`: List flag definitions from the catalog (default when `--deployment` is omitted)
- `--deployment <ENV>`: List flags configured in an environment's rules
- `--format <FORMAT>`: Output format (table, json, yaml, default: table)

##### Examples

List all flags in the catalog:

```bash
controlpath flag list
```

List flags with rules in production:

```bash
controlpath flag list --deployment production
```

List as JSON:

```bash
controlpath flag list --format json
```

#### `flag show`

Shows detailed information about a flag.

##### Usage

```bash
controlpath flag show [OPTIONS]
```

##### Options

- `--name <NAME>`: Flag name (required)
- `--deployment <ENV>`: Show environment rules for the given environment
- `--format <FORMAT>`: Output format (table, json, yaml)

##### Examples

Show flag details:

```bash
controlpath flag show --name my_feature
```

Show flag rules in production:

```bash
controlpath flag show --name my_feature --deployment production
```

#### `flag remove`

Removes a flag from the catalog or removes its rules from one environment.

##### Usage

```bash
controlpath flag remove [OPTIONS]
```

##### Options

- `--name <NAME>`: Flag name (required)
- `--env <ENV>`: Remove rules from this environment only (flag definition remains)
- `--force`: Force removal without confirmation

##### Examples

Remove a flag entirely:

```bash
controlpath flag remove --name my_feature
```

Remove production rules only:

```bash
controlpath flag remove --name my_feature --env production
```

Force removal without confirmation:

```bash
controlpath flag remove --name my_feature --force
```

### `env`

Manage environments (add, sync, list, remove).

#### `env add`

Adds a new environment.

##### Usage

```bash
controlpath env add [OPTIONS]
```

##### Options

- `--name <NAME>`: Environment name
- `--template <ENV>`: Template environment to copy from
- `--interactive`: Interactive mode (prompts for missing values)

##### Examples

Add new environment (interactive):

```bash
controlpath env add
```

Add with name:

```bash
controlpath env add --name staging
```

Add with template:

```bash
controlpath env add --name staging --template production
```

#### `env sync`

Validates catalog rules for one or all environments in `control-path.yaml`.

##### Usage

```bash
controlpath env sync [OPTIONS]
```

##### Options

- `--env <ENV>`: Environment to validate (validates all if not specified)
- `--dry-run`: Report status without writing artifacts

##### Examples

Validate all environments:

```bash
controlpath env sync
```

Validate a specific environment:

```bash
controlpath env sync --env staging
```

Dry run:

```bash
controlpath env sync --dry-run
```

#### `env list`

Lists environments defined in `control-path.yaml`.

##### Usage

```bash
controlpath env list [OPTIONS]
```

##### Options

- `--format <FORMAT>`: Output format (table, json, yaml, default: table)

##### Examples

List as table (default):

```bash
controlpath env list
```

List as JSON:

```bash
controlpath env list --format json
```

#### `env remove`

Removes an environment.

##### Usage

```bash
controlpath env remove [OPTIONS]
```

##### Options

- `--name <NAME>`: Environment name (required)
- `--force`: Force removal without confirmation

##### Examples

Remove environment (with confirmation):

```bash
controlpath env remove --name staging
```

Force removal without confirmation:

```bash
controlpath env remove --name staging --force
```


## File Organization

### Standard Structure

```
project-root/
├── control-path.yaml               # Configuration (flags + environment rules)
├── .controlpath/                   # Compiled artifacts directory
│   ├── config.yaml                 # Optional config (language, defaults, mode)
│   ├── production.ast              # Compiled AST artifacts
│   └── staging.ast
└── flags/                          # Generated SDK (import this in your code)
    ├── index.ts
    └── ...
```

### Configuration File

Location: `control-path.yaml` in the project root

Contains:
- Flag definitions (boolean defaults, kind metadata)
- Environment-specific rollout rules
- Segment definitions
- Mode configuration (local or saas)

All flag definitions and environment rules are in a single file, simplifying the mental model and workflow.

### AST Artifacts

Location: `.controlpath/<env>.ast` (or custom path via `--output`)

Generated by: `controlpath compile`

Contains:
- Compiled MessagePack binary
- All flag rules
- Segment definitions
- Optional signature

## Workflow Examples

### Basic Workflow

1. **Setup project:**
   ```bash
   controlpath setup
   ```

2. **Add flags and configure rules:**
   Edit `control-path.yaml` to add new flags and environment rules.

3. **Validate:**
   ```bash
   controlpath validate
   ```

4. **Compile:**
   ```bash
   controlpath compile --env production
   ```

5. **Use AST artifact:**
   The compiled `.controlpath/production.ast` file can be used by the runtime SDK.

### Multi-Environment Workflow

1. **Configure environments in file:**
   Edit `control-path.yaml` to add environment rules for each flag.

2. **Compile each environment:**
   ```bash
   controlpath compile --env production
   controlpath compile --env staging
   ```

3. **Validate all environments:**
   ```bash
   controlpath validate --all
   ```

### CI/CD Integration

Example GitHub Actions workflow:

```yaml
name: Compile Flags

on:
  push:
    branches: [main]
    paths:
      - 'control-path.yaml'
      - '.controlpath/**/*.ast'

jobs:
  compile:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Validate
        run: cargo run --bin controlpath -- validate
      - name: Compile Production
        run: cargo run --bin controlpath -- compile --env production
      - name: Compile Staging
        run: cargo run --bin controlpath -- compile --env staging
      - name: Upload Artifacts
        uses: actions/upload-artifact@v3
        with:
          name: ast-artifacts
          path: .controlpath/*.ast
```

## Error Messages

### Validation Errors

When validation fails, the CLI provides clear error messages:

```
✗ Validation failed
  Error: v1 array "flags" is not supported; use map-keyed flags
```

### Compilation Errors

Compilation errors include context:

```
✗ Compilation failed
  Error: Expression parsing error: Expected expression after AND operator
    Expression: "role == 'admin' AND"
    Position: 28
```

### File Not Found Errors

Clear messages when files are missing:

```
✗ Compilation failed
  Error: Failed to read control-path.yaml: No such file or directory (os error 2)
```

## Performance

The Rust CLI is optimized for performance:

- **Fast Startup**: Native binary, no runtime overhead
- **Fast Compilation**: Efficient Rust implementation
- **Small Binary**: Optimized release builds
- **Low Memory**: Efficient memory usage

## Troubleshooting

### "No such file or directory"

Ensure files exist and paths are correct:

```bash
# Check if config exists
ls -la control-path.yaml
ls -la .controlpath/*.ast
```

### "Validation failed"

Check your YAML syntax and schema compliance:

```bash
# Validate with verbose output
controlpath validate --all
```

### "Compilation failed"

Check for:
- Invalid expressions in `when` clauses (remember: no `user.` or `context.` prefixes)
- Missing flags or environment rules in `control-path.yaml`
- Type mismatches
- Ensure attributes in expressions match the properties in your attributes object

## See Also

- [Rust API Documentation](./rust-api.md)
- [Architecture Documentation](../control-path-next/ARCHITECTURE.md)

