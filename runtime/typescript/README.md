# @controlpath/runtime

Low-level runtime SDK for Control Path. This package provides AST artifact loading and flag evaluation capabilities with OpenFeature compliance.

## Installation

```bash
pnpm add @controlpath/runtime
```

## Usage

### Basic Usage

```typescript
import { Provider, buildFlagNameMap } from '@controlpath/runtime';
import { parseDefinitions } from '@controlpath/compiler';

// Parse flag definitions to build flag name map
const definitions = parseDefinitions('flags.definitions.yaml');
const flagNameMap = buildFlagNameMap(definitions.flags);

// Create provider instance with flag name map
const provider = new Provider({ flagNameMap });

// Load AST artifact from file
await provider.loadArtifact('./flags/production.ast');

// Evaluate flags using OpenFeature interface
const context = { id: 'user123', role: 'admin' };
const result = provider.resolveBooleanEvaluation('new_dashboard', false, context);

if (result.value) {
  console.log('New dashboard enabled');
}
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

### Building Flag Name Map

The Provider requires a `flagNameMap` to map flag names to their indices in the AST. Use the helper function:

```typescript
import { buildFlagNameMap } from '@controlpath/runtime';
import { parseDefinitions } from '@controlpath/compiler';

// From flag definitions (recommended)
const definitions = parseDefinitions('flags.definitions.yaml');
const flagNameMap = buildFlagNameMap(definitions.flags);

// Or manually
const flagNameMap = {
  'new_dashboard': 0,
  'enable_analytics': 1,
  'theme_color': 2
};

const provider = new Provider({ flagNameMap });
```

### Signature Verification

Verify Ed25519 signatures when loading artifacts from untrusted sources:

```typescript
import { Provider } from '@controlpath/runtime';

// Public key (base64 or hex encoded)
const publicKey = 'base64-encoded-public-key-here';

const provider = new Provider({
  flagNameMap,
  publicKey,
  requireSignature: true, // Require signature (optional, default: false)
});

await provider.loadArtifact('https://cdn.example.com/flags/production.ast');
```

### Hot Reloading

```typescript
const provider = new Provider({ flagNameMap });
await provider.loadArtifact('./flags/production.ast');

// Later, reload updated artifact (clears cache automatically)
await provider.reloadArtifact('./flags/production.ast');
```

### Result Caching

The Provider caches evaluation results by default (5 minute TTL):

```typescript
const provider = new Provider({
  flagNameMap,
  enableCache: true, // Default: true
  cacheTTL: 5 * 60 * 1000, // 5 minutes (default)
});

// Clear cache manually if needed
provider.clearCache();
```

### Error Handling

The Provider follows a "Never Throws" policy - evaluation methods always return `ResolutionDetails`:

```typescript
const result = provider.resolveBooleanEvaluation('my_flag', false, context);

// Check for errors
if (result.errorCode) {
  switch (result.errorCode) {
    case 'FLAG_NOT_FOUND':
      console.warn('Flag not found in flag name map');
      break;
    case 'GENERAL':
      console.error('Evaluation error:', result.errorMessage);
      break;
  }
}

// Use the value (always safe)
const flagValue = result.value; // Guaranteed to be boolean
```

### Using with OpenFeature SDK

The Provider is fully compatible with `@openfeature/server-sdk`.

```typescript
import { OpenFeature } from '@openfeature/server-sdk';
import { Provider, buildFlagNameMap } from '@controlpath/runtime';
import { parseDefinitions } from '@controlpath/compiler';

// Build flag name map
const definitions = parseDefinitions('flags.definitions.yaml');
const flagNameMap = buildFlagNameMap(definitions.flags);

// Create and register provider
const provider = new Provider({ flagNameMap });
await provider.loadArtifact('./flags/production.ast');

OpenFeature.setProvider(provider);

// Use OpenFeature client
const client = OpenFeature.getClient();
const showNewDashboard = await client.getBooleanValue('new_dashboard', false, {
  role: 'admin'
});
```

**Note**: The Provider supports both synchronous (for direct usage) and asynchronous (for OpenFeature SDK) method signatures via TypeScript method overloading. The OpenFeature SDK will automatically use the async signature.

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

