# Control Path GitHub Action

A GitHub Action for validating and compiling Control Path feature flag definitions in CI/CD pipelines.

## Features

- ✅ **Automatic CLI Installation**: Downloads the Control Path CLI binary for your platform
- ✅ **Flag Validation**: Validates flag definitions and deployment files against schemas
- ✅ **AST Compilation**: Compiles deployment files to AST artifacts for runtime use
- ✅ **Multi-Platform Support**: Works on Linux, macOS, and Windows runners
- ✅ **Flexible Configuration**: Supports environment-based or file-based workflows

## Usage

### Basic Example

Validate and compile flags for a production environment:

```yaml
name: Validate and Compile Flags

on:
  push:
    branches: [main]
    paths:
      - 'flags.definitions.yaml'
      - '.controlpath/**/*.deployment.yaml'

jobs:
  compile-flags:
    runs-on: ubuntu-latest  # Linux only
    steps:
      - uses: actions/checkout@v4
      
      - name: Validate and Compile Flags
        uses: releaseworkshop/control-path/actions/controlpath-action@main
        with:
          environment: production
```

### Validate Only

Skip compilation and only validate flag definitions:

```yaml
- name: Validate Flags
  uses: releaseworkshop/control-path/actions/controlpath-action@main
  with:
    environment: production
    skip-compilation: true
```

### Compile Only

Skip validation and only compile (useful if validation runs in a separate step):

```yaml
- name: Compile Flags
  uses: releaseworkshop/control-path/actions/controlpath-action@main
  with:
    environment: production
    skip-validation: true
```

### Using Explicit File Paths

Instead of environment names, specify file paths directly:

```yaml
- name: Validate and Compile
  uses: releaseworkshop/control-path/actions/controlpath-action@main
  with:
    definitions-file: flags.definitions.yaml
    deployment-file: .controlpath/production.deployment.yaml
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
        uses: releaseworkshop/control-path/actions/controlpath-action@main
        with:
          environment: ${{ matrix.environment }}
      
      - name: Upload Artifact
        uses: actions/upload-artifact@v4
        with:
          name: ast-${{ matrix.environment }}
          path: .controlpath/${{ matrix.environment }}.ast
```

### Using Specific CLI Version

Pin to a specific Control Path CLI version:

```yaml
- name: Validate and Compile
  uses: releaseworkshop/control-path/actions/controlpath-action@main
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
      - 'flags.definitions.yaml'
      - '.controlpath/**'
  pull_request:
    paths:
      - 'flags.definitions.yaml'
      - '.controlpath/**'

jobs:
  validate:
    runs-on: ubuntu-latest  # Linux only
    steps:
      - uses: actions/checkout@v4
      
      - name: Validate Flags
        uses: releaseworkshop/control-path/actions/controlpath-action@main
        with:
          skip-compilation: true

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
        uses: releaseworkshop/control-path/actions/controlpath-action@main
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

### Notes on Inputs

- **Either `deployment-file` or `environment` must be provided** for compilation (unless `skip-compilation` is `true`)
- If neither `deployment-file` nor `environment` is provided for validation, the action will auto-detect files using `--all` flag
- The `version` input accepts version tags (e.g., `v1.0.0`) or `latest` for the most recent release

## Outputs

| Output | Description |
|-------|-------------|
| `compiled-artifact-path` | Path to the compiled AST artifact file |

### Example: Using Output

```yaml
- name: Compile Flags
  id: compile
  uses: releaseworkshop/control-path/actions/controlpath-action@main
  with:
    environment: production

- name: Use Compiled Artifact
  run: |
    echo "Artifact path: ${{ steps.compile.outputs.compiled-artifact-path }}"
    ls -la ${{ steps.compile.outputs.compiled-artifact-path }}
```

## Platform Support

The action supports Linux runners only:

- **Linux**: `linux-x86_64`, `linux-aarch64` (automatically detected)

## Project Structure

The action expects your project to follow the standard Control Path structure:

```
.
├── flags.definitions.yaml          # Flag definitions
└── .controlpath/
    ├── production.deployment.yaml  # Production rules
    ├── staging.deployment.yaml     # Staging rules
    ├── production.ast              # Compiled artifacts (generated)
    └── staging.ast                 # Compiled artifacts (generated)
```

## Error Handling

The action will fail if:

- Validation fails (invalid flag definitions or deployment files)
- Compilation fails (syntax errors, missing flags, etc.)
- CLI binary cannot be downloaded
- Required files are missing

All errors include clear messages to help diagnose issues.

## Best Practices

1. **Validate in PRs**: Run validation on pull requests to catch errors early
2. **Compile in Main**: Only compile artifacts on the main branch to avoid unnecessary builds
3. **Upload Artifacts**: Upload compiled AST files as artifacts for deployment
4. **Pin Versions**: Use specific CLI versions in production workflows for reproducibility
5. **Separate Jobs**: Consider separating validation and compilation into different jobs for better visibility

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
- `flags.definitions.yaml` exists (or specify custom path)
- `.controlpath/*.deployment.yaml` files exist (or specify `deployment-file`)

## License

This action is licensed under the Elastic License 2.0, same as the Control Path project.

## Support

For issues, questions, or contributions:
- [GitHub Issues](https://github.com/releaseworkshop/control-path/issues)
- [Documentation](https://github.com/releaseworkshop/control-path)
