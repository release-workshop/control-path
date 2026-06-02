# Architecture

This document describes the current architecture after the v1->v2 migration and platform-spine work.

## High-level model

Control Path has three execution layers:

1. **Catalog and compile layer (Rust)**  
   Parses and validates `control-path.yaml`, then compiles per-environment artifacts.
2. **CLI orchestration layer (Rust)**  
   Exposes workflows (`setup`, `new-flag`, `deploy`, `ci`, `explain`, etc.).
3. **Runtime and generated SDK layer (TypeScript)**  
   Loads artifacts/kill-switch files, evaluates flags, and exposes typed generated methods.

## Ownership boundaries

- `crates/compiler`: schema/semantic validation and compile pipeline.
- `crates/cli`: user workflows, project IO, command UX, and integrations.
- `runtime/typescript`: loader utilities, polling coordinators, evaluator runtime contract.
- generated SDK output: thin layer that delegates to runtime internals.

## Core runtime contract

Evaluation precedence:

1. kill switch file
2. compiled artifact rules
3. catalog default

Additional runtime constraints:

- artifact and kill switch polling are independent loops
- failed refresh never destroys last known-good loaded state
- init/re-init behavior preserves expected runtime state semantics

## Catalog vs deploy velocities

- Catalog changes (flag definitions/defaults/imports): SDK regeneration + app deploy.
- Environment rules changes: artifact publish and runtime poll update.
- Incident kill switch changes: kill switch file update and faster poll propagation.

## Related references

- `CONTEXT.md` for domain language
- `docs/adr/0001-compiled-artifact-runtime-delivery.md` for delivery decisions
