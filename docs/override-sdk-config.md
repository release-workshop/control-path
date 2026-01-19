# SDK Configuration for Overrides

This guide explains how to configure the Control Path SDK to load and poll override files (kill switches).

## Quick Start

```typescript
import { Provider } from '@controlpath/runtime';

// Override file is loaded and polled automatically during initialization
const provider = new Provider({
  overrideUrl: 'https://cdn.example.com/overrides.json',
});

// Load AST artifact (required for flag evaluation)
await provider.loadArtifact('./flags/production.ast');
```

That's it! The override file is loaded automatically when the Provider is created, and polling starts in the background.

## Configuration Options

### Basic Configuration

```typescript
const provider = new Provider({
  // Override file URL (HTTP/HTTPS, file://, or file path)
  overrideUrl: 'https://cdn.example.com/overrides.json',
  
  // Polling interval in milliseconds (default: 3000ms / 3 seconds)
  pollingInterval: 3000,
  
  // Enable/disable polling (default: true when overrideUrl is set)
  enablePolling: true,
});
```

### URL Types

The SDK supports three types of URLs:

#### 1. HTTP/HTTPS URLs (Production)

```typescript
const provider = new Provider({
  overrideUrl: 'https://cdn.example.com/overrides.json',
  // Polling starts automatically for HTTP/HTTPS URLs
});
```

**Features:**
- Automatic polling (checks for updates every 3 seconds by default)
- ETag support (only fetches if file changed)
- Works with CDN, web server, S3 public URLs, etc.

#### 2. File URLs (Local Development)

```typescript
const provider = new Provider({
  overrideUrl: 'file:///absolute/path/to/overrides.json',
  // Or: overrideUrl: './overrides.json' (Node.js only)
});
```

**Features:**
- Loads once during initialization
- No polling (file changes require reload or restart)
- Useful for local development

#### 3. Direct File Paths (Node.js Only)

```typescript
const provider = new Provider({
  overrideUrl: './overrides.json', // Relative to current working directory
  // Or: overrideUrl: '/absolute/path/to/overrides.json'
});
```

**Features:**
- Same as file:// URLs (no polling)
- Only works in Node.js (not browsers)
- Useful for local development

## Polling Configuration

### Automatic Polling

Polling starts automatically when you provide an HTTP/HTTPS URL:

```typescript
const provider = new Provider({
  overrideUrl: 'https://cdn.example.com/overrides.json',
  // Polling starts automatically (no manual call needed)
});
```

### Custom Polling Interval

Adjust the polling interval (1-5 seconds recommended):

```typescript
const provider = new Provider({
  overrideUrl: 'https://cdn.example.com/overrides.json',
  pollingInterval: 5000, // Poll every 5 seconds
});
```

### Disable Polling

Disable automatic polling if you want manual control:

```typescript
const provider = new Provider({
  overrideUrl: 'https://cdn.example.com/overrides.json',
  enablePolling: false, // Disable automatic polling
});

// Manually start polling later
provider.startPolling();

// Or manually reload override file
await provider.reloadOverrideFile();
```

### Manual Polling Control

Start and stop polling programmatically:

```typescript
const provider = new Provider({
  overrideUrl: 'https://cdn.example.com/overrides.json',
  enablePolling: false, // Don't start automatically
});

// Start polling
provider.startPolling();

// Later, stop polling
provider.stopPolling();
```

## Error Handling

The SDK handles errors gracefully (never throws during evaluation):

```typescript
const provider = new Provider({
  overrideUrl: 'https://cdn.example.com/overrides.json',
  logger: {
    // Optional: Custom logger for override loading errors
    error: (message) => console.error('Override error:', message),
    warn: (message) => console.warn('Override warning:', message),
    debug: (message) => console.debug('Override debug:', message),
  },
});
```

**Error Scenarios:**
- **Network errors**: Logged as warnings, polling continues
- **Invalid JSON**: Logged as warnings, falls back to AST evaluation
- **File not found**: Logged as warnings, falls back to AST evaluation
- **Invalid override values**: Logged as warnings, falls back to AST evaluation

The application continues running even if override file is unavailable.

## Complete Example

```typescript
import { Provider } from '@controlpath/runtime';

// Configure provider with override file
const provider = new Provider({
  // Override file URL
  overrideUrl: process.env.OVERRIDE_URL || 'https://cdn.example.com/overrides.json',
  
  // Polling configuration
  pollingInterval: 3000, // 3 seconds (default)
  enablePolling: true, // Start polling automatically (default)
  
  // Optional: Custom logger
  logger: {
    error: (msg) => console.error('[Override]', msg),
    warn: (msg) => console.warn('[Override]', msg),
    debug: (msg) => console.debug('[Override]', msg),
  },
});

// Load AST artifact (required for flag evaluation)
await provider.loadArtifact('./flags/production.ast');

// Evaluate flags (override takes precedence over AST)
const context = { id: 'user123', role: 'admin' };
const result = provider.resolveBooleanEvaluation('new_dashboard', false, context);

if (result.value) {
  console.log('New dashboard enabled');
}
```

## Multi-Environment Setup

Use different override URLs per environment:

```typescript
// Production
const prodProvider = new Provider({
  overrideUrl: 'https://flags.example.com/production/overrides.json',
});

// Staging
const stagingProvider = new Provider({
  overrideUrl: 'https://flags.example.com/staging/overrides.json',
});

// Development
const devProvider = new Provider({
  overrideUrl: './overrides.json', // Local file
});
```

Or use environment variables:

```typescript
const provider = new Provider({
  overrideUrl: process.env.OVERRIDE_URL || './overrides.json',
  pollingInterval: parseInt(process.env.OVERRIDE_POLLING_INTERVAL || '3000', 10),
  enablePolling: process.env.OVERRIDE_ENABLE_POLLING !== 'false',
});
```

## Evaluation Priority

Override values take precedence over AST evaluation:

1. **Override** (if present in override file)
2. **AST** (from compiled deployment file)
3. **Default** (from flag definitions)

```typescript
// Override file: { "new_dashboard": "OFF" }
// AST: new_dashboard = true (for user123)
// Result: false (override takes precedence)
const result = provider.resolveBooleanEvaluation('new_dashboard', false, context);
// result.value = false (override wins)
```

## Performance Considerations

### Polling Impact

- **Default interval**: 3 seconds (configurable)
- **ETag support**: Only fetches if file changed (reduces bandwidth)
- **Background polling**: Non-blocking, doesn't affect evaluation performance
- **Cache**: Override values are cached in memory (no re-parsing on each evaluation)

### Bandwidth Usage

With ETag support, polling is very efficient:
- **First request**: Full file download (~1KB typical)
- **Subsequent requests**: 304 Not Modified (no body, just headers)
- **On change**: Full file download again

Typical bandwidth: < 1KB per 3 seconds (only when file changes).

## Troubleshooting

### Override not loading

```typescript
// Check if override file loaded
console.log('Override state:', provider.getOverrideState());

// Check logs for errors
const provider = new Provider({
  overrideUrl: 'https://cdn.example.com/overrides.json',
  logger: {
    error: (msg) => console.error('Error:', msg),
    warn: (msg) => console.warn('Warning:', msg),
  },
});
```

### Polling not working

- **Check URL type**: Polling only works for HTTP/HTTPS URLs (not `file://` or direct paths)
- **Check enablePolling**: Ensure `enablePolling: true` (default when overrideUrl is set)
- **Check network**: Verify URL is accessible (try in browser)

### Override not taking effect

- **Check priority**: Override → AST → Default (override should win)
- **Check flag name**: Ensure flag name matches exactly (case-sensitive)
- **Check value format**: Boolean flags use `"ON"`/`"OFF"`, multivariate use variation name
- **Check logs**: SDK logs warnings for invalid overrides

## Next Steps

- [Storage Setup Guide](./override-setup.md) - Set up override file storage
- [CLI Usage Guide](./override-cli-usage.md) - Manage override files with CLI
- [Use Cases and Examples](./override-examples.md) - Real-world scenarios
