# Finalize boolean catalog schema contract

Status: done
Type: HITL

## What to build

Finalize the new `control-path.yaml` contract for the redesigned CLI before implementation begins. The schema should describe a boolean-only flag catalog, explicit catalog identity, namespaced catalog imports, flag kind and lifecycle, declared metadata, optional local environment rules, and SaaS mode rule authority.

This slice should produce a reviewed schema example and decision notes that AFK agents can implement without reopening product semantics.

## Acceptance criteria

- [x] The schema confirms boolean-only flags and removes variant/experiment/configuration semantics from the public product model.
- [x] The schema defines catalog identity, imports, flag keys, `kind`, `lifecycle`, metadata, local environments, and local rule shape.
- [x] The schema documents local mode versus SaaS mode rule authority.
- [x] The schema documents how Git-declared metadata differs from SaaS-observed telemetry.
- [x] The schema includes representative examples for local-only, SaaS, and imported/global catalog usage.

## Deliverables

- `schemas/control-path.schema.v2.json`
- `schemas/control-path.workspace.schema.v1.json`
- `schemas/examples/*.yaml`
- `.scratch/cli-salvage-redesign/schema-decisions.md`

## Unblocks

- `.scratch/cli-salvage-redesign/issues/02-parse-validate-new-catalog-schema.md`

## Blocked by

None — completed.
