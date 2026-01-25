# Control Path Compile Action

A GitHub Action for **compiling** Control Path feature flag definitions to AST artifacts for runtime use. This action can optionally validate flag definitions before compilation, but its primary purpose is to produce compiled AST artifacts.

> **Note**: For validation-only workflows (e.g., in pull requests), consider using `controlpath-validate-action` instead when available.

## Features

- ✅ **AST Compilation**: Primary purpose - compiles deployment files to AST artifacts for runtime use
- ✅ **Optional Validation**: Can validate flag definitions before compilation (enabled by default)
- ✅ **Automatic CLI Installation**: Downloads the Control Path CLI binary for Linux runners
- ✅ **Flexible Configuration**: Supports environment-based or file-based workflows
- ✅ **Artifact Output**: Provides path to compiled AST artifact for downstream steps

## Usage

### Basic Example

Validate and compile flags for a production environment:

```yaml
name: Validate and Compile Flags

on:
  push:
    branches: [main]
    paths:
      - 'control-path.yaml'
      - '.controlpath/**/*.ast'

jobs:
  compile-flags:
    runs-on: ubuntu-latest  # Linux only
    steps:
      - uses: actions/checkout@v4
      
      - name: Compile Flags
        uses: releaseworkshop/control-path/actions/controlpath-compile-action@main
        with:
          environment: production
```

### Validate Only (Not Recommended)

> **Note**: For validation-only workflows, use `controlpath-validate-action` when available. This action is optimized for compilation.

If you need to use this action for validation only, you can skip compilation:

```yaml
- name: Validate Flags
  uses: releaseworkshop/control-path/actions/controlpath-compile-action@main
  with:
    environment: production
    skip-compilation: true
```

### Compile Only

Skip validation and only compile (useful if validation runs in a separate step or using `controlpath-validate-action`):

```yaml
- name: Compile Flags
  uses: releaseworkshop/control-path/actions/controlpath-compile-action@main
  with:
    environment: production
    skip-validation: true
```

### Using Explicit File Paths

Instead of environment names, specify file paths directly:

```yaml
- name: Compile Flags
  uses: releaseworkshop/control-path/actions/controlpath-compile-action@main
  with:
    # Using config (recommended)
    environment: production
    # Or legacy files:
    # definitions-file: flags.definitions.yaml
    # deployment-file: .controlpath/production.deployment.yaml
```

### Multiple Environments

Compile flags for multiple environments:

```yaml
jobs:
  compile-flags:
    runs-on: ubuntu-latest  # Linux only
    strategy:
      matrix:
        environment: [production, staging, development]
    steps:
      - uses: actions/checkout@v4
      
      - name: Compile ${{ matrix.environment }}
        uses: releaseworkshop/control-path/actions/controlpath-compile-action@main
        with:
          environment: ${{ matrix.environment }}
      
      - name: Upload Artifact
        uses: actions/upload-artifact@v4
        with:
          name: ast-${{ matrix.environment }}
          path: .controlpath/${{ matrix.environment }}.ast
```

### Working Directory Support

For projects where Control Path files are in a subdirectory, use the `working-directory` input:

```yaml
jobs:
  compile-flags:
    runs-on: ubuntu-latest  # Linux only
    strategy:
      matrix:
        service: [service-a, service-b, service-c]
    steps:
      - uses: actions/checkout@v4
      
      - name: Compile Flags for ${{ matrix.service }}
        id: compile
        uses: releaseworkshop/control-path/actions/controlpath-compile-action@main
        with:
          working-directory: packages/${{ matrix.service }}
          environment: production
      
      - name: Upload Artifact
        uses: actions/upload-artifact@v4
        with:
          name: ast-${{ matrix.service }}-production
          path: packages/${{ matrix.service }}/${{ steps.compile.outputs.compiled-artifact-path }}
```

**Subdirectory Structure Example:**
```
.
└── packages/
    ├── service-a/
    │   ├── control-path.yaml
    │   └── .controlpath/
    │       └── production.ast (generated)
    ├── service-b/
    │   ├── control-path.yaml
    │   └── .controlpath/
    │       └── production.ast (generated)
    └── service-c/
        ├── control-path.yaml
        └── .controlpath/
            └── production.ast (generated)
```

**Notes:**
- The `working-directory` path is relative to the repository root
- All file paths (`definitions-file`, `deployment-file`) are relative to the working directory
- The `compiled-artifact-path` output is relative to the working directory

### Using Specific CLI Version

Pin to a specific Control Path CLI version:

```yaml
- name: Compile Flags
  uses: releaseworkshop/control-path/actions/controlpath-compile-action@main
  with:
    environment: production
    version: v1.0.0
```

### Complete CI/CD Workflow

Example workflow that validates, compiles, and uploads artifacts:

```yaml
name: Feature Flags CI/CD

on:
  push:
    branches: [main]
    paths:
      - 'control-path.yaml'
      - '.controlpath/**'
  pull_request:
    paths:
      - 'control-path.yaml'
      - '.controlpath/**'

jobs:
  validate:
    runs-on: ubuntu-latest  # Linux only
    steps:
      - uses: actions/checkout@v4
      
      - name: Validate Flags
        uses: releaseworkshop/control-path/actions/controlpath-validate-action@main
        # Note: When controlpath-validate-action is available, use it here
        # For now, you can use controlpath-compile-action with skip-compilation: true

  compile:
    needs: validate
    runs-on: ubuntu-latest  # Linux only
    strategy:
      matrix:
        environment: [production, staging]
    steps:
      - uses: actions/checkout@v4
      
      - name: Compile Flags
        id: compile
        uses: releaseworkshop/control-path/actions/controlpath-compile-action@main
        with:
          environment: ${{ matrix.environment }}
          skip-validation: true
      
      - name: Upload AST Artifact
        uses: actions/upload-artifact@v4
        with:
          name: ast-${{ matrix.environment }}
          path: ${{ steps.compile.outputs.compiled-artifact-path }}
```

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `definitions-file` | Path to flag definitions file | No | `flags.definitions.yaml` |
| `deployment-file` | Path to deployment file (optional if environment is provided) | No | - |
| `environment` | Environment name (uses `.controlpath/<env>.deployment.yaml`) | No | - |
| `version` | Control Path CLI version to use | No | `latest` |
| `skip-validation` | Skip validation step | No | `false` |
| `skip-compilation` | Skip compilation step | No | `false` |
| `working-directory` | Working directory for the action (for monorepos) | No | Repository root |

### Notes on Inputs

- **Either `deployment-file` or `environment` must be provided** for compilation (unless `skip-compilation` is `true`)
- If neither `deployment-file` nor `environment` is provided for validation, the action will auto-detect files using `--all` flag
- The `version` input accepts version tags (e.g., `v1.0.0`) or `latest` for the most recent release
- **At least one of `skip-validation` or `skip-compilation` must be `false`** - the action will fail if both are `true`
- **`working-directory`** is useful for monorepos where each service has its own Control Path setup. Paths are relative to the repository root. All file paths are then relative to the working directory.

## Outputs

| Output | Description |
|-------|-------------|
| `compiled-artifact-path` | Path to the compiled AST artifact file |

### Example: Using Output

```yaml
- name: Compile Flags
  id: compile
  uses: releaseworkshop/control-path/actions/controlpath-compile-action@main
  with:
    environment: production

- name: Use Compiled Artifact
  run: |
    echo "Artifact path: ${{ steps.compile.outputs.compiled-artifact-path }}"
    ls -la ${{ steps.compile.outputs.compiled-artifact-path }}
```

## Related Actions

- **`controlpath-validate-action`** (coming soon): Validation-only action for checking flag definitions without compilation. Ideal for pull request workflows.

## Platform Support

The action supports Linux runners only:

- **Linux**: `linux-x86_64`, `linux-aarch64` (automatically detected)

## Project Structure

The action expects your project to follow the standard Control Path structure:

**Single Service:**
```
.
├── flags.definitions.yaml          # Flag definitions
└── .controlpath/
    ├── production.deployment.yaml  # Production rules
    ├── staging.deployment.yaml     # Staging rules
    ├── production.ast              # Compiled artifacts (generated)
    └── staging.ast                 # Compiled artifacts (generated)
```

**Monorepo (Multiple Services):**
```
.
└── packages/
    ├── service-a/
    │   ├── control-path.yaml
    │   └── .controlpath/
    │       └── production.ast
    └── service-b/
        ├── control-path.yaml
        └── .controlpath/
            └── production.ast
```

Use the `working-directory` input to specify which service directory to use.

## Error Handling

The action will fail if:

- Both `skip-validation` and `skip-compilation` are set to `true` (at least one step must run)
- Validation fails (invalid flag definitions or deployment files)
- Compilation fails (syntax errors, missing flags, etc.)
- Compiled artifact is not found at expected path after compilation
- CLI binary cannot be downloaded
- Required files are missing
- Latest version cannot be determined (when using `version: latest`)

All errors include clear messages to help diagnose issues. Debug logging is available when `ACTIONS_STEP_DEBUG` is set to `true`.

## Best Practices

1. **Use for Compilation**: This action is optimized for compiling AST artifacts. Use `controlpath-validate-action` for validation-only workflows when available.
2. **Compile in Main**: Only compile artifacts on the main branch or release branches to avoid unnecessary builds
3. **Upload Artifacts**: Upload compiled AST files as artifacts for deployment
4. **Pin Versions**: Use specific CLI versions in production workflows for reproducibility
5. **Separate Jobs**: Consider separating validation and compilation into different jobs for better visibility and faster feedback

## Troubleshooting

### "CLI binary not found"

The action downloads binaries from GitHub Releases. Ensure:
- The repository has published releases
- The version specified exists (if using a specific version)
- Your runner has internet access

### "Validation failed"

Check your flag definitions and deployment files:
- Ensure YAML syntax is correct
- Verify schema compliance
- Check for missing required fields

### "Compilation failed"

Common causes:
- Invalid expressions in `when` clauses
- Missing flag definitions referenced in deployment files
- Type mismatches between definitions and rules

### "No files to validate"

Ensure your project structure matches the expected layout:
- `control-path.yaml` exists (config, recommended), OR
- Legacy files: `flags.definitions.yaml` and `.controlpath/*.deployment.yaml` (or specify custom paths)

## License

This action is licensed under the Elastic License 2.0, same as the Control Path project.

## Support

For issues, questions, or contributions:
- [GitHub Issues](https://github.com/releaseworkshop/control-path/issues)
- [Documentation](https://github.com/releaseworkshop/control-path)
