# Configuration

Control Path uses a single catalog file: `control-path.yaml`.

## What the catalog contains

- Catalog identity (`catalog.id`, optional namespace)
- Flag definitions (boolean, defaults, kind, metadata)
- Local-mode environments and ordered rules
- Optional shared catalog imports
- Optional local-mode remote URLs for artifacts and kill switches

## Mental model

- **Flag catalog changes** (new flags/defaults/kinds/imports): regenerate SDK and redeploy app.
- **Environment rule changes** only: publish a new compiled artifact for that environment.
- **Kill switch changes**: update kill switch file, picked up by runtime poll.

## Environments and rules

Rules are ordered and first-match wins. If no rule matches, evaluation falls back to catalog default.

For local mode:

- environments are declared in `control-path.yaml`
- compile output is `.controlpath/<env>.ast`

For SaaS mode:

- environment rules are owned by platform control plane
- local catalog remains source for definitions, defaults, metadata, and imports

## Validation and compile

Validate after edits:

```bash
controlpath validate
```

Compile one environment:

```bash
controlpath compile --env production
```

Compile via workflow command:

```bash
controlpath deploy --env production
```

## Imports

Imported shared catalogs are namespace-qualified in generated SDK surfaces and reporting.
Environment rules for imported flags are owned by their source catalog.
