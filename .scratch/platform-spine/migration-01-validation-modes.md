# Migration: validation modes (issue 01)

## CLI breaking changes

| Removed | Replacement |
|---------|-------------|
| `controlpath ci --no-validate` | Removed — CI always validates (compile uses `ValidationMode::Compile`). |
| `controlpath deploy --skip-validation` | Removed — deploy always runs `validate` first. |
| `generate-sdk` unchecked paths | Removed — generation always uses full validation. |

## GitHub Actions breaking changes

| Action | Removed input |
|--------|----------------|
| `controlpath-action` | `skip-validation` |
| `controlpath-compile-action` | `skip-validation` |

Validation always runs before compile. Remove `skip-validation: true` from workflow YAML; delete the input line if present.

## Library (`controlpath-compiler`)

- `load_and_validate_catalog` and `validate_catalog` now require a `ValidationMode` argument.
- Use `ValidationMode::Compile` for compile pipelines and `ValidationMode::SdkGenerate` for SDK projection loads.
