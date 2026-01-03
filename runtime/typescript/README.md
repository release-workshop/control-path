# @controlpath/runtime

Low-level runtime SDK for Control Path. This package provides AST artifact loading and flag evaluation capabilities with OpenFeature compliance.

## Installation

```bash
npm install @controlpath/runtime
```

## Usage

### Basic Usage

```typescript
import { Provider } from '@controlpath/runtime';

// Create provider instance (flag name map is optional)
const provider = new Provider();

// Load AST artifact from file (flag name map is automatically built)
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

### Flag Name Map

The Provider automatically builds the flag name map from the artifact when you call `loadArtifact()` or `reloadArtifact()`. The artifact includes flag names, so no manual configuration is needed.

The flag name map is built automatically from the `flagNames` array in the artifact, which contains string table indices for each flag name. This allows the Provider to look up flags by name without requiring the flag definitions file at runtime.

### Signature Verification

Verify Ed25519 signatures when loading artifacts from untrusted sources:

```typescript
import { Provider } from '@controlpath/runtime';

// Public key (base64 or hex encoded)
const publicKey = 'base64-encoded-public-key-here';

const provider = new Provider({
  publicKey,
  requireSignature: true, // Require signature (optional, default: false)
});

await provider.loadArtifact('https://cdn.example.com/flags/production.ast');
```

### Hot Reloading

```typescript
const provider = new Provider();
await provider.loadArtifact('./flags/production.ast');

// Later, reload updated artifact (clears cache automatically)
await provider.reloadArtifact('./flags/production.ast');
```

### Result Caching

The Provider caches evaluation results by default (5 minute TTL):

```typescript
const provider = new Provider({
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
import { Provider } from '@controlpath/runtime';

// Create and register provider (flag name map is automatically inferred)
const provider = new Provider();
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
npm run build

# Test
npm test

# Type check
npm run typecheck

# Lint
npm run lint
```

## License

Elastic License 2.0

