Status: done

## What to build

End-to-end support for **namespaced attributes** from imported catalogs:

1. Shared catalog declares **`attributes:`** and uses bare names in its **environment rules** (e.g. `org_tier == 'gold'`).
2. **`compile`** on a consuming service merges imported rules into the artifact and rewrites property paths from those rules to **`import_namespace.field`** in the artifact string table (e.g. `platform.org_tier`).
3. Service-local rules keep bare top-level paths (`plan`).
4. Evaluation (**Rust** shared evaluator, **`explain --attributes`**, TypeScript runtime) resolves nested runtime JSON: `{ plan: 'beta', platform: { org_tier: 'gold' } }`.

Strict property validation on the **imported catalog file** uses that catalog’s **`attributes:`** map (bare names in source rules).

## Acceptance criteria

- [x] Compile rewrite: imported rule referencing `org_tier` matches runtime object with `platform.org_tier` nested shape
- [x] Service-local rule referencing `plan` still matches top-level `plan`
- [x] **`controlpath explain --attributes`** returns correct flag value for an imported flag using namespaced JSON
- [x] TypeScript **`evaluate`** (runtime package test or e2e) agrees with Rust for the same fixture
- [x] Validation fails on unknown bare property in imported catalog rules when that catalog opts in

## Blocked by

- `.scratch/evaluation-attribute-schema/issues/02-attribute-schema-parse-and-validate.md`

## Comments

Decision shape from ADR 0002:

```yaml
# platform/control-path.yaml
attributes:
  org_tier: string
environments:
  production:
    rules:
      emergency_kill_switch:
        - when: "org_tier == 'gold'"
          serve: true
```

Runtime / explain JSON:

```json
{ "id": "u1", "platform": { "org_tier": "gold" } }
```
