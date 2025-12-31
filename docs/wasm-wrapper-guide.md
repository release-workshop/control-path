# WASM Wrapper Guide

This guide explains how to create a WebAssembly (WASM) wrapper for the Control Path Rust compiler for use in Cloudflare Workers, browsers, and other WASM environments.

## Overview

The `controlpath-compiler` crate is designed to be WASM-compatible, but it needs a wrapper crate to provide JavaScript bindings. This wrapper will be created in a **separate repository** to keep concerns separated.

## Architecture

```
┌─────────────────────────────────────┐
│     controlpath-compiler            │
│     (Pure Rust, WASM-compatible)    │
│  - No file I/O                      │
│  - No WASM dependencies             │
│  - Strings in, bytes out            │
└──────────────┬──────────────────────┘
               │
               │ (dependency)
               │
┌──────────────▼──────────────────────┐
│     control-path-wasm                │
│     (WASM Wrapper - Separate Repo)  │
│  - wasm-bindgen bindings             │
│  - JavaScript interop                │
│  - Cloudflare Workers support        │
└──────────────────────────────────────┘
```

## Prerequisites

- Rust toolchain (stable)
- `wasm-pack` tool
- Node.js (for testing)

## Setup

### 1. Create New Repository

Create a new repository for the WASM wrapper:

```bash
mkdir control-path-wasm
cd control-path-wasm
cargo init --lib
```

### 2. Configure Cargo.toml

```toml
[package]
name = "control-path-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
# Control Path compiler (from git or published crate)
controlpath-compiler = { git = "https://github.com/controlpath/control-path.git", path = "crates/compiler" }
# Or when published:
# controlpath-compiler = "0.1.0"

# WASM bindings
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"  # If async support needed

# Error handling
thiserror = "1.0"

[dev-dependencies]
wasm-bindgen-test = "0.3"

[profile.release]
opt-level = "z"  # Optimize for size
lto = true       # Link-time optimization
```

### 3. Configure wasm-pack

Create `wasm-pack.toml`:

```toml
[package]
name = "control-path-wasm"
version = "0.1.0"
description = "Control Path compiler WASM bindings for Cloudflare Workers"
license = "Elastic-2.0"
repository = "https://github.com/controlpath/control-path-wasm"

[build]
target = "wasm32-unknown-unknown"
```

## Implementation

### Basic WASM Bindings

Create `src/lib.rs`:

```rust
use wasm_bindgen::prelude::*;
use controlpath_compiler::{
    parse_definitions, parse_deployment,
    validate_definitions, validate_deployment,
    compile, serialize, CompilerError
};

// Convert Rust errors to JavaScript errors
impl From<CompilerError> for JsValue {
    fn from(err: CompilerError) -> Self {
        JsValue::from_str(&format!("{}", err))
    }
}

/// Parse flag definitions from YAML/JSON string
#[wasm_bindgen]
pub fn parse_definitions_wasm(content: &str) -> Result<JsValue, JsValue> {
    let definitions = parse_definitions(content)
        .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    
    // Convert to JavaScript object
    serde_wasm_bindgen::to_value(&definitions)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Parse deployment from YAML/JSON string
#[wasm_bindgen]
pub fn parse_deployment_wasm(content: &str) -> Result<JsValue, JsValue> {
    let deployment = parse_deployment(content)
        .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    
    serde_wasm_bindgen::to_value(&deployment)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Validate flag definitions
#[wasm_bindgen]
pub fn validate_definitions_wasm(definitions: &JsValue) -> Result<(), JsValue> {
    let definitions: serde_json::Value = serde_wasm_bindgen::from_value(definitions.clone())
        .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
    
    validate_definitions(&definitions)
        .map_err(|e| JsValue::from_str(&format!("{}", e)))
}

/// Validate deployment
#[wasm_bindgen]
pub fn validate_deployment_wasm(deployment: &JsValue) -> Result<(), JsValue> {
    let deployment: serde_json::Value = serde_wasm_bindgen::from_value(deployment.clone())
        .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
    
    validate_deployment(&deployment)
        .map_err(|e| JsValue::from_str(&format!("{}", e)))
}

/// Compile deployment and definitions to MessagePack bytes
#[wasm_bindgen]
pub fn compile_wasm(definitions: &JsValue, deployment: &JsValue) -> Result<Vec<u8>, JsValue> {
    let definitions: serde_json::Value = serde_wasm_bindgen::from_value(definitions.clone())
        .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
    
    let deployment: serde_json::Value = serde_wasm_bindgen::from_value(deployment.clone())
        .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
    
    let artifact = compile(&deployment, &definitions)
        .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    
    serialize(&artifact)
        .map_err(|e| JsValue::from_str(&format!("{}", e)))
}

/// Compile from YAML strings (convenience function)
#[wasm_bindgen]
pub fn compile_from_strings(definitions_yaml: &str, deployment_yaml: &str) -> Result<Vec<u8>, JsValue> {
    let definitions = parse_definitions(definitions_yaml)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;
    
    let deployment = parse_deployment(deployment_yaml)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;
    
    validate_definitions(&definitions)
        .map_err(|e| JsValue::from_str(&format!("Validation error: {}", e)))?;
    
    validate_deployment(&deployment)
        .map_err(|e| JsValue::from_str(&format!("Validation error: {}", e)))?;
    
    let artifact = compile(&deployment, &definitions)
        .map_err(|e| JsValue::from_str(&format!("Compilation error: {}", e)))?;
    
    serialize(&artifact)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}
```

### Add serde-wasm-bindgen

Add to `Cargo.toml`:

```toml
[dependencies]
serde-wasm-bindgen = "0.6"
```

## Building

### Build WASM Package

```bash
wasm-pack build --target web
```

This creates a `pkg/` directory with:
- `control_path_wasm.js` - JavaScript bindings
- `control_path_wasm_bg.wasm` - WASM binary
- TypeScript definitions

### Build for Cloudflare Workers

```bash
wasm-pack build --target no-modules
```

Or use `wasm-pack build --target bundler` for bundler compatibility.

## Usage

### JavaScript/TypeScript

```typescript
import init, { compile_from_strings } from './pkg/control_path_wasm';

// Initialize WASM module
await init();

// Compile from YAML strings
const definitionsYaml = `
flags:
  - name: my_flag
    type: boolean
    defaultValue: false
`;

const deploymentYaml = `
environment: production
rules:
  my_flag:
    rules:
      - serve: true
`;

try {
  const bytes = compile_from_strings(definitionsYaml, deploymentYaml);
  // bytes is Uint8Array
  console.log('Compiled successfully:', bytes.length, 'bytes');
} catch (error) {
  console.error('Compilation failed:', error);
}
```

### Cloudflare Workers

```typescript
// worker.ts
import init, { compile_from_strings } from './pkg/control_path_wasm';

export default {
  async fetch(request: Request): Promise<Response> {
    // Initialize WASM (only once)
    await init();
    
    // Get YAML from request
    const { definitions, deployment } = await request.json();
    
    try {
      const bytes = compile_from_strings(definitions, deployment);
      return new Response(bytes, {
        headers: { 'Content-Type': 'application/octet-stream' }
      });
    } catch (error) {
      return new Response(JSON.stringify({ error: error.message }), {
        status: 400,
        headers: { 'Content-Type': 'application/json' }
      });
    }
  }
};
```

## Configuration

### getrandom Configuration

The `jsonschema` crate uses `getrandom` which needs WASM configuration. Add to `Cargo.toml`:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.2", features = ["js"] }
```

### Size Optimization

Optimize for smaller WASM binary:

```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
panic = "abort"      # Smaller binary (no panic unwinding)
```

## Testing

### Unit Tests

```rust
// src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_compile_from_strings() {
        let definitions = r#"
flags:
  - name: test_flag
    type: boolean
    defaultValue: false
"#;
        
        let deployment = r#"
environment: test
rules:
  test_flag:
    rules:
      - serve: true
"#;
        
        let result = compile_from_strings(definitions, deployment);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }
}
```

Run tests:

```bash
wasm-pack test --headless --firefox
```

### Integration Tests

Create JavaScript/TypeScript tests:

```typescript
// tests/integration.test.ts
import init, { compile_from_strings } from '../pkg/control_path_wasm';

describe('WASM Integration', () => {
  beforeAll(async () => {
    await init();
  });

  it('should compile from YAML strings', () => {
    const definitions = `
flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;
    
    const deployment = `
environment: test
rules:
  test_flag:
    rules:
      - serve: true
`;
    
    const bytes = compile_from_strings(definitions, deployment);
    expect(bytes).toBeInstanceOf(Uint8Array);
    expect(bytes.length).toBeGreaterThan(0);
  });
});
```

## Distribution

### npm Package

Publish to npm:

```bash
wasm-pack publish
```

This creates an npm package that can be installed:

```bash
npm install @controlpath/compiler-wasm
```

### CDN Distribution

Upload `pkg/` contents to CDN for direct browser usage:

```html
<script type="module">
  import init, { compile_from_strings } from 'https://cdn.example.com/control-path-wasm.js';
  
  await init();
  // Use compile_from_strings...
</script>
```

## Performance Considerations

### Initialization

WASM module initialization is a one-time cost. Consider:
- Lazy initialization
- Pre-initialization in workers
- Caching initialized modules

### Memory Management

WASM has limited memory. Consider:
- Streaming compilation for large inputs
- Chunked processing
- Memory cleanup after use

## Troubleshooting

### Issue: "getrandom: target wasm32-unknown-unknown is not supported"

**Solution**: Add `getrandom` with `js` feature (see Configuration section).

### Issue: "Module not found" in Cloudflare Workers

**Solution**: Ensure you're using the correct target:
```bash
wasm-pack build --target no-modules
```

### Issue: Large WASM binary size

**Solution**: 
1. Enable size optimizations (see Configuration)
2. Use `wasm-opt` for additional optimization:
   ```bash
   wasm-opt -Oz pkg/control_path_wasm_bg.wasm -o pkg/control_path_wasm_bg_opt.wasm
   ```

## Best Practices

1. **Error Handling**: Always handle errors from WASM functions
2. **Initialization**: Initialize WASM module before use
3. **Memory**: Be mindful of WASM memory limits
4. **Size**: Optimize for size in production builds
5. **Testing**: Test in target environment (browser/workers)

## See Also

- [Rust API Documentation](./rust-api.md)
- [wasm-pack Documentation](https://rustwasm.github.io/wasm-pack/)
- [wasm-bindgen Documentation](https://rustwasm.github.io/wasm-bindgen/)
- [Cloudflare Workers Documentation](https://developers.cloudflare.com/workers/)

