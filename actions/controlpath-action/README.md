# Control Path GitHub Action

Validate and compile v2 `control-path.yaml` catalogs in CI/CD pipelines.

## Usage

```yaml
name: Validate and Compile Flags

on:
  push:
    branches: [main]
    paths:
      - 'control-path.yaml'
      - 'control-path.workspace.yaml'
      - '.controlpath/**'

jobs:
  compile-flags:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Validate and Compile Flags
        uses: releaseworkshop/control-path/actions/controlpath-action@main
        with:
          environment: production
```

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `environment` | Environment name from `control-path.yaml` | No | - |
| `version` | Control Path CLI version | No | `latest` |
| `skip-compilation` | Skip compilation | No | `false` |

## Outputs

| Output | Description |
|--------|-------------|
| `compiled-artifact-path` | Path to `.controlpath/<env>.ast` |

## Notes

- Requires `control-path.yaml` in the working directory (or set `working-directory` on the compile action variant).
- `environment` is required for compilation; validation alone can use `--all` semantics when omitted.
- Legacy `flags.definitions.yaml` and `.controlpath/*.deployment.yaml` split files are no longer supported.
