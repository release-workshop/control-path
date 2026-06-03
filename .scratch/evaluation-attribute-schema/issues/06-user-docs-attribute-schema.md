Status: ready-for-agent

## What to build

User-facing documentation for **attribute schema** aligned with **`CONTEXT.md`** and ADR 0002:

- **`docs/user/configuration.md`** — **`attributes:`** syntax, opt-in semantics, scalar types, base-name reservation, import namespace nesting
- **`docs/user/rules.md`** — replace “Planned” section; document strict local validation, SaaS validation scope, **`explain --attributes`**, bare vs namespaced runtime JSON
- **`docs/user/sdk-typescript.md`** — **`BaseAttributes`**, closed **`Attributes`**, **per-flag attribute types**, superset calling convention

Cross-link ADR 0002 from developer docs if appropriate (`docs/developer/` or README — follow existing ADR linking patterns).

## Acceptance criteria

- [ ] No “planned” language remains for catalog-driven typing in user docs
- [ ] Examples use **`attributes.json`** and **`--attributes`**, not **`--context`** / **`--user`**
- [ ] Documents explicitly state SDK types do not track **environment rule** changes (artifact / SaaS velocity)
- [ ] Import example shows shared catalog bare names vs service runtime nested JSON

## Blocked by

- `.scratch/evaluation-attribute-schema/issues/05-typed-sdk-generation.md`
