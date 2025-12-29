# @controlpath/runtime

Low-level runtime SDK for Control Path. This package provides AST artifact loading and flag evaluation capabilities with OpenFeature compliance.

## Installation

```bash
pnpm add @controlpath/runtime
```

## Usage

### Basic Usage

```typescript
import { Provider } from '@controlpath/runtime';

// Create provider instance
const provider = new Provider();

// Load AST artifact from file
await provider.loadArtifact('./flags/production.ast');

// Or load from URL
await provider.loadArtifact('https://cdn.example.com/flags/production.ast');

// Evaluate flags using OpenFeature interface
const result = provider.resolveBooleanEvaluation('my_flag', false, context);
```

### Loading AST Artifacts

```typescript
import { loadFromFile, loadFromURL, loadFromBuffer } from '@controlpath/runtime';

// Load from file
const artifact = await loadFromFile('./flags/production.ast');

// Load from URL
const artifact = await loadFromURL('https://cdn.example.com/flags/production.ast');

// Load from Buffer
const buffer = Buffer.from(/* ... */);
const artifact = loadFromBuffer(buffer);
```

### Hot Reloading

```typescript
const provider = new Provider();
await provider.loadArtifact('./flags/production.ast');

// Later, reload updated artifact
await provider.reloadArtifact('./flags/production.ast');
```

## API Reference

### Provider

OpenFeature-compliant provider for flag evaluation.

#### Methods

- `loadArtifact(artifact: string | URL)`: Load AST artifact from file path or URL
- `reloadArtifact(artifact: string | URL)`: Reload AST artifact (replaces cached AST)
- `resolveBooleanEvaluation(flagKey, defaultValue, context)`: Evaluate boolean flag
- `resolveStringEvaluation(flagKey, defaultValue, context)`: Evaluate string flag
- `resolveNumberEvaluation(flagKey, defaultValue, context)`: Evaluate number flag
- `resolveObjectEvaluation(flagKey, defaultValue, context)`: Evaluate object flag

#### Properties

- `metadata`: Provider metadata (`{ name: 'controlpath' }`)
- `hooks`: Array of OpenFeature hooks (optional)

## Development

```bash
# Build
pnpm build

# Test
pnpm test

# Type check
pnpm typecheck

# Lint
pnpm lint
```

## License

Elastic License 2.0

