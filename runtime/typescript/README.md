# @controlpath/runtime

Low-level runtime SDK for Control Path. This package provides AST artifact loading and flag evaluation capabilities.

## Installation

```bash
npm install @controlpath/runtime
```

## Usage

### Basic Usage

```typescript
import { loadFromFile, evaluate, buildFlagNameMapFromArtifact } from '@controlpath/runtime';

// Load AST artifact from file
const artifact = await loadFromFile('./flags/production.ast');

// Build flag name map from artifact
const flagNameMap = buildFlagNameMapFromArtifact(artifact);

// Evaluate flags using attributes
const attributes = { id: 'user123', role: 'admin' };
const flagIndex = flagNameMap['new_dashboard'];
const result = evaluate(flagIndex, artifact, attributes);

if (result === 'ON') {
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

The flag name map is built from the `flagNames` array in the artifact, which contains string table indices for each flag name. This allows you to look up flags by name without requiring the flag definitions file at runtime.

```typescript
import { buildFlagNameMapFromArtifact } from '@controlpath/runtime';

const flagNameMap = buildFlagNameMapFromArtifact(artifact);
const flagIndex = flagNameMap['my_flag'];
```

### Signature Verification

Verify Ed25519 signatures when loading artifacts from untrusted sources:

```typescript
import { loadFromFile } from '@controlpath/runtime';

// Public key (base64 or hex encoded)
const publicKey = 'base64-encoded-public-key-here';

const artifact = await loadFromFile('./flags/production.ast', {
  publicKey,
  requireSignature: true, // Require signature (optional, default: false)
});
```

### Evaluation with Attributes

All attributes (user identity, user attributes, and environmental context) are provided in a single `Attributes` object:

```typescript
import { evaluate } from '@controlpath/runtime';

const attributes = {
  id: 'user123',
  role: 'admin',
  email: 'user@example.com',
  environment: 'production',
  device: 'desktop',
  app_version: '1.2.3'
};

const flagIndex = flagNameMap['my_flag'];
const result = evaluate(flagIndex, artifact, attributes);
```

### Error Handling

The `evaluate` function returns `undefined` if no rule matches. Always provide a default value:

```typescript
const result = evaluate(flagIndex, artifact, attributes);
const value = result ?? defaultValue;
```

## API Reference

### Loading Functions

- `loadFromFile(path: string, options?: LoadOptions): Promise<Artifact>` - Load AST artifact from file
- `loadFromURL(url: string | URL, timeout?: number, logger?: Logger, options?: LoadOptions): Promise<Artifact>` - Load AST artifact from URL
- `loadFromBuffer(buffer: Buffer | Uint8Array): Artifact` - Load AST artifact from buffer

### Evaluation Functions

- `evaluate(flagIndex: number, artifact: Artifact, attributes?: Attributes): unknown` - Evaluate a flag by index
- `evaluateRule(rule: Rule, artifact: Artifact, attributes?: Attributes): unknown` - Evaluate a single rule

### Utility Functions

- `buildFlagNameMapFromArtifact(artifact: Artifact): Record<string, number>` - Build flag name to index map from artifact

### Types

- `Artifact` - AST artifact structure
- `Rule` - Flag rule structure
- `Expression` - Expression node structure
- `Variation` - Variation structure
- `Attributes` - Attributes object for evaluation (consolidates user identity, attributes, and context)
- `Logger` - Logger interface
- `OverrideFile` - Override file format
- `OverrideValue` - Override value type

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
