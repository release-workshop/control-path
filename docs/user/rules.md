# Environment rules

Environment rules decide which boolean value a flag serves in a given **environment** (for example `staging` or `production`). They live in the **compiled artifact** at runtime; in local mode you author them in `control-path.yaml` (or via CLI commands that edit the same file).

See also:

- Catalog structure and modes: [`configuration.md`](configuration.md)
- CLI commands: [`cli.md`](cli.md)
- Command reference for `explain`: [`cli.md`](cli.md#explain)

## How evaluation works

For each flag, rules are an **ordered list**. Evaluation walks the list top to bottom; the **first matching rule wins**. If no rule matches, the flag’s catalog **default** applies.

Runtime order for the final boolean (see [`sdk-typescript.md`](sdk-typescript.md)):

1. **Kill switch file** (if the flag is listed)
2. **Compiled artifact** rules
3. Catalog **default**

### Rule types

Each rule is a YAML object. Provide **`serve`** or **`rollout`**, not both.

| Shape | Meaning |
| --- | --- |
| `serve: true` / `serve: false` | Always serve this value when the rule is eligible (no `when`), or when `when` is true. |
| `rollout:` with `percentage` and `serve` | When eligible, serve `serve` to a stable fraction of identities (0–100). Requires a stable `id` on evaluation attributes (see below). |
| `when: "<expression>"` (optional) | Expression must be true for the rule to be eligible. Omitted `when` means always eligible. |
| `reason: "..."` (optional) | Audit note stored in the catalog; shown in `explain` output. |

**Kill switch flags** (`kind: kill_switch`) may only use plain `serve` rules—no `when` or `rollout`. Use the kill switch file for incident overrides ([`kill-switches.md`](kill-switches.md)).

### Example (YAML)

```yaml
environments:
  production:
    rules:
      new_dashboard:
        - when: "segment('beta_users')"
          serve: true
        - rollout:
            percentage: 10
            serve: true
          reason: Gradual rollout after beta validation
```

Full catalog examples: [`schemas/examples/local-only.control-path.yaml`](../../schemas/examples/local-only.control-path.yaml).

## Where rules are authored

| Mode | Authoring |
| --- | --- |
| **local** | `environments.<env>.rules` in `control-path.yaml`, or CLI commands that append to the same structure. Compile with `controlpath compile` / `controlpath deploy`. |
| **saas** | Do **not** put `environments`, `segments`, `artifacts`, or `kill_switches` in the service catalog (validation fails). Rules are owned by the **SaaS project**; use platform UI and `controlpath sync` so `.controlpath/<env>.ast` is available for SDK generation and `explain`. |

Rule **semantics** (ordering, `when`, `rollout`) are the same in both modes; only the **authoring surface** changes.

SaaS catalog example: [`schemas/examples/saas.control-path.yaml`](../../schemas/examples/saas.control-path.yaml).

## Editing rules in Git (local mode)

### Add or change rules by hand

1. Edit `environments.<env>.rules.<flag_name>`—an array of rule objects.
2. Use **local flag names only** (not `import.namespace.flag` on imported flags).
3. Run `controlpath validate`, then `controlpath deploy --env <env>` (or `controlpath compile --env <env>`).

Imported flags are ruled in their **source** catalog. Putting rules for them in a consumer catalog fails validation. See [`configuration.md`](configuration.md#imports-and-shared-catalogs).

### CLI equivalent (`flag enable`)

`controlpath flag enable` **appends** one rule per environment—it does not replace the whole rule list. It only creates `serve` rules (no `rollout`); use YAML for percentage rollouts.

**Catch-all enable** (no `when`):

```bash
controlpath flag enable new_dashboard --env staging --value true
```

Equivalent YAML (new entry at the end of the flag’s rule list):

```yaml
environments:
  staging:
    rules:
      new_dashboard:
        # ... existing rules ...
        - serve: true
```

**Conditional enable**:

```bash
controlpath flag enable new_dashboard --env staging --rule "role == 'admin'"
```

Equivalent YAML:

```yaml
        - when: "role == 'admin'"
          serve: true
```

`--all` omits `when` (catch-all). `--value false` sets `serve: false`.

After CLI edits, the tool saves `control-path.yaml` and compiles unless `--no-compile` is set.

### Segments (local mode)

Reusable predicates live under `segments`:

```yaml
segments:
  beta_users:
    when: "plan == 'beta'"
```

Reference them in rules with `segment('beta_users')` or `IN_SEGMENT('beta_users')` (equivalent).

## Evaluation attributes

Rules read fields from a **single attributes object** passed into the SDK (and into `explain`). Typical fields include `id`, `role`, `environment`, and your own properties.

- In **application code**, pass one object to generated flag methods or `setAttributes()` (see [`sdk-typescript.md`](sdk-typescript.md)).
- In **`explain`**, pass evaluation attributes as JSON with `--attributes` (file path or inline JSON). Omit `--attributes` to evaluate with an empty object.

**Prefixes in rule strings:** `user.role` and `role` compile to the same property (`role` on the attributes object). `context.environment` and `environment` compile the same way. Prefer bare names (`role`, `plan`) in new rules.

**Rollout bucketing** uses a stable string `id` on that object (or a string user value). Without `id`, rollout rules may not bucket as expected; `explain` warns when `id` is missing.

Example `attributes.json` for explain:

```json
{
  "id": "user-42",
  "role": "admin",
  "plan": "beta"
}
```

```bash
controlpath explain --flag new_dashboard --attributes attributes.json --env production --trace
```

### Planned: catalog-driven attribute typing

Today the generated SDK exports a base `Attributes` interface plus `[key: string]: unknown`. The optional top-level `attributes:` section will declare extra fields so `generate-sdk` emits **base ∪ your fields**, giving compile-time checks that call sites pass a complete object.

## Expression language

`when` clauses and segment `when` strings use the same expression language. Expressions must parse at compile time (`controlpath validate` / `controlpath compile`).

**Not supported:** arithmetic operators such as `%` (use `rollout` or `HASHED_PARTITION` for percentage-style bucketing).

### Literals and properties

- Strings: `'admin'` or `"admin"` (escape `\'` inside single-quoted strings)
- Booleans: `true`, `false`
- Null: `null`
- Numbers: `42`, `3.14`
- Properties: `role`, `user.role`, `plan`, `app_version` (see [Evaluation attributes](#evaluation-attributes))

### Comparison operators

`==`, `!=`, `>`, `<`, `>=`, `<=`

Comparisons are typed at evaluation time (strings, numbers, booleans).

### Logical operators

`AND`, `OR`, `NOT` (keywords, case-sensitive as shown)

Use parentheses for grouping:

```text
(role == 'admin' AND environment == 'production') OR plan == 'beta'
```

### Membership

```text
role IN ['admin', 'moderator']
```

`IN` is also available as `IN(value, array)`.

### Functions

Function names are case-insensitive at parse time. Arguments are expressions.

| Function | Description |
| --- | --- |
| `STARTS_WITH(str, prefix)` | String prefix test |
| `ENDS_WITH(str, suffix)` | String suffix test |
| `CONTAINS(container, value)` | Substring or array/list containment |
| `MATCHES(str, pattern)` | Regular expression match |
| `UPPER(str)` / `LOWER(str)` | Case conversion |
| `LENGTH(value)` | String length or array size |
| `INTERSECTS(array1, array2)` | True if arrays share an element |
| `IN(value, array)` | Membership (see [Membership](#membership)) |
| `SEMVER_EQ(v1, v2)` | Semantic version equal |
| `SEMVER_GT(v1, v2)` | Greater than |
| `SEMVER_GTE(v1, v2)` | Greater than or equal |
| `SEMVER_LT(v1, v2)` | Less than |
| `SEMVER_LTE(v1, v2)` | Less than or equal |
| `HASHED_PARTITION(id, buckets)` | Stable bucket in `0 .. buckets-1` (use in comparisons, e.g. `HASHED_PARTITION(id, 100) < 10`) |
| `COALESCE(a, b, ...)` | First non-null value |
| `IS_BETWEEN(start, end)` | Current UTC time within RFC3339 range (inclusive) |
| `IS_AFTER(timestamp)` / `IS_BEFORE(timestamp)` | Compare current UTC to RFC3339 timestamp |
| `CURRENT_TIMESTAMP()` | Current UTC time as RFC3339 string |
| `CURRENT_HOUR_UTC()` | Hour 0–23 (UTC) |
| `CURRENT_DAY_OF_WEEK_UTC()` | `MONDAY`, `TUESDAY`, … (UTC) |
| `CURRENT_DAY_OF_MONTH_UTC()` | Day of month 1–31 (UTC) |
| `CURRENT_MONTH_UTC()` | Month 1–12 (UTC) |
| `segment('name')` / `IN_SEGMENT('name')` | True when the named segment’s `when` matches |

For percentage rollouts, prefer a **`rollout`** rule over hand-rolled `HASHED_PARTITION` comparisons when possible.

### Example expressions

```text
role == 'admin'
plan == 'beta' AND app_version != '1.0.0'
STARTS_WITH(id, 'employee_')
SEMVER_GTE(app_version, '2.0.0')
segment('beta_users')
HASHED_PARTITION(id, 100) < 10
```

## Percentage rollouts

A **`rollout`** rule serves its `serve` value to a percentage of identities that pass `when` (or to all identities if `when` is omitted). Bucketing is stable per `id`—the same `id` always lands in the same bucket for a given percentage.

```yaml
        - rollout:
            percentage: 10
            serve: true
```

`percentage` must be between 0 and 100. Use `explain --trace` to see rollout bucket information when debugging.

CLI `flag enable` does not create `rollout` rules; add them in YAML, then run `controlpath deploy --env <env>`.

## Validate and debug

```bash
controlpath validate
controlpath compile --env staging
controlpath explain --flag new_dashboard --attributes attributes.json --env staging --trace
```

See [`troubleshooting.md`](troubleshooting.md) for common compile and expression errors.
