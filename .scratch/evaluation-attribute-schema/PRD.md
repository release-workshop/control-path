# Evaluation attribute schema

Status: ready-for-agent

## Problem

Flag targeting reads fields from a single **evaluation attributes** object, but catalogs cannot declare those fields, generated TypeScript stays loose, and rule typos are only caught at runtime. Imported shared catalogs need the same namespacing model as qualified flag names without splitting runtime into separate user/context bags.

## Goal

Opt-in catalog **`attributes:`** (scalar types, namespaced by **import namespace**) drives:

- Closed generated SDK types extending **`BaseAttributes`** from `@controlpath/runtime`
- **Per-flag attribute types** from catalog ownership (not **environment rules**)
- Strict property validation where rules are authored in Git (**local mode**)
- Compile rewrite of imported rule property paths to `namespace.field`
- **`explain --attributes`** using the same JSON shape as production

## Non-goals (v1)

- Nested object types in **attribute schema**
- Required-field markers on catalog entries
- Validating SaaS-remote **environment rules** from the service repo
- Rule-derived TypeScript minimums
- Scaffolding **`attributes:`** in **`controlpath init`**

## References

- Domain glossary: `CONTEXT.md` (**evaluation attributes**, **attribute schema**, **namespaced attributes**, **per-flag attribute type**, **base attributes**)
- ADR: `docs/adr/0002-evaluation-attribute-schema.md`

## Issues

1. `01-base-attributes-in-runtime.md` — runtime export + legacy SDK path
2. `02-attribute-schema-parse-and-validate.md` — schema, model, opt-in validation
3. `03-local-mode-rule-property-validation.md` — strict refs in rules/segments
4. `04-imported-namespace-compile-rewrite.md` — artifact property paths + eval
5. `05-typed-sdk-generation.md` — closed types + per-flag signatures
6. `06-user-docs-attribute-schema.md` — configuration + rules docs
