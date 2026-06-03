Status: done

## What to build

Add optional top-level **`attributes:`** to the v2 catalog JSON Schema and **`CatalogDocument`** model.

Validation when the key is present:

- Values are scalar type names only: `string`, `number`, `boolean`
- Keys are valid identifiers
- Keys must not collide with **base attributes** names
- Empty map `{}` is valid (opts in without declaring service fields yet)

**`controlpath validate`** surfaces errors for invalid shapes. Catalogs omitting **`attributes:`** behave exactly as today.

Document the YAML shape in **`docs/user/configuration.md`** (brief; full narrative lands in issue 06).

## Acceptance criteria

- [x] v2 schema accepts optional **`attributes:`**; rejects invalid types and base-name collisions
- [x] Compiler catalog model deserializes **`attributes:`**; validation runs in **`ValidationMode::SdkGenerate`** and **`Compile`**
- [x] Integration test: validate passes for declared scalars; fails for `role: string`, unknown type, or invalid key
- [x] Imported catalog fixture can declare its own **`attributes:`** map independently of the service catalog

## Blocked by

None — can start immediately (may land in parallel with issue 01)

## Comments

Imported-catalog **`attributes:`** are validated when that catalog file is validated; merging into the service SDK is issue 05.
