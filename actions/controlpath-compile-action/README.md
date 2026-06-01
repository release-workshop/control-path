# Control Path Compile Action

Compile v2 `control-path.yaml` catalogs to AST artifacts. Optionally validates before compilation.

## Usage

```yaml
- uses: actions/checkout@v4

- name: Compile production flags
  uses: releaseworkshop/control-path/actions/controlpath-compile-action@main
  with:
    environment: production
    working-directory: packages/my-service  # optional, for monorepos
```

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `environment` | Environment name from `control-path.yaml` | Yes (for compile) | - |
| `version` | Control Path CLI version | No | `latest` |
| `skip-validation` | Skip validation | No | `false` |
| `skip-compilation` | Skip compilation | No | `false` |
| `working-directory` | Monorepo subdirectory containing `control-path.yaml` | No | - |

## Outputs

| Output | Description |
|--------|-------------|
| `compiled-artifact-path` | Path to `.controlpath/<env>.ast` |

## Project layout

```
control-path.yaml          # v2 catalog (flags + environment rules)
.controlpath/
  production.ast             # compiled artifact (generated)
  production.kill-switches.json
```

Legacy split-file layouts (`flags.definitions.yaml`, `.controlpath/*.deployment.yaml`) are no longer supported.
