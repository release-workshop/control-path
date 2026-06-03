# Release Process

This doc captures the practical release/update loop for Control Path components.

## What can ship independently

- Rust CLI binary (`controlpath`)
- TypeScript runtime package (`@controlpath/runtime`)
- Generated SDK output in consuming apps

## Change classes

1. **Catalog contract changes**  
   Require coordinated CLI/compiler/runtime compatibility checks and docs updates.
2. **CLI workflow changes**  
   Require command docs updates and regression tests for affected workflows.
3. **Runtime contract changes**  
   Require runtime tests, changelog updates, and generated SDK compatibility checks.

## Recommended release checklist

1. Run required quality gates ([Testing](testing.md); checklist: [testing-and-quality-gates.md](testing-and-quality-gates.md)).
2. Update impacted docs (`README.md`, `DEVELOPING.md`, user/developer pages).
3. Confirm runtime package changelog for behavior changes.
4. Validate representative end-to-end flow:
   - `setup`
   - `new-flag`
   - `flag enable`
   - `deploy`
   - `generate-sdk`
   - app-side evaluate path
5. Tag and publish through repository's standard release automation.

## Backward-compatibility posture

This codebase is actively evolving. Breaks can be introduced intentionally, but must be:

- explicit in docs/changelog
- covered by tests for new contract semantics
- reflected in examples and onboarding guides
