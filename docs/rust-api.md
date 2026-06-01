# Rust Compiler API Documentation

This document describes the public API of the Control Path Rust compiler library (`controlpath-compiler`).

## Overview

The `controlpath-compiler` crate provides a pure Rust implementation of the Control Path compiler. **New integrations should use the v2 catalog API** with a single `control-path.yaml` boolean catalog (`parse_catalog`, `load_and_validate_catalog`, `compile_catalog`).

Legacy split-file helpers (`parse_definitions`, `parse_deployment`, `compile`) remain for WASM compatibility and existing artifacts but are not the primary workflow.

The library is designed to be WASM-compatible and works only with in-memory data (no file I/O).

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
controlpath-compiler = { path = "../compiler" }  # For local development
# Or when published:
# controlpath-compiler = "0.1.0"
```

## Core API (v2 catalog — preferred)

Start with [Complete Example (v2 catalog)](#complete-example-v2-catalog) and the validation functions below. Jump to [Legacy split-file API](#legacy-split-file-api) only when you must consume v1 YAML shapes.

### Validation Functions (v2 catalog)

#### `load_and_validate_catalog`

Parse and validate a v2 `control-path.yaml` catalog document.

```rust
pub fn load_and_validate_catalog(
    content: &str,
    file_path: &str,
    ctx: &CatalogValidationContext,
    mode: ValidationMode,
) -> Result<(CatalogDocument, CatalogValidationResult), ParseError>
```

#### `ValidationMode`

Controls which validation phases run (`Authoring`, `SdkGenerate`, `Compile`). User-facing compile and SDK paths use `Compile` or `SdkGenerate`.

#### `validate_catalog`

Semantically validate an already-parsed catalog (including imports when provided in context and mode).

```rust
pub fn validate_catalog(
    file_path: &str,
    catalog: &CatalogDocument,
    ctx: &CatalogValidationContext,
    mode: ValidationMode,
) -> CatalogValidationResult
```

**Example:**
```rust
use controlpath_compiler::{
    load_and_validate_catalog, validate_catalog, CatalogValidationContext, ValidationMode,
};

let (catalog, initial) = load_and_validate_catalog(
    yaml,
    "control-path.yaml",
    &CatalogValidationContext::default(),
    ValidationMode::Compile,
)?;
if !initial.is_ok() {
    return Err(/* handle validation errors */);
}
let result = validate_catalog(
    "control-path.yaml",
    &catalog,
    &CatalogValidationContext::default(),
    ValidationMode::Compile,
);
if !result.is_ok() {
    return Err(/* handle semantic errors */);
}
```

Legacy v1 `validate_definitions` / `validate_deployment` / `validate_unified_config` entry points were removed. Use the catalog validators above for v2 boolean catalogs.

## Legacy split-file API

### Parsing Functions

#### `parse_definitions`

Parse legacy flag definitions from a YAML/JSON string.

```rust
pub fn parse_definitions(content: &str) -> Result<serde_json::Value, CompilerError>
```

#### `parse_deployment`

Parse legacy deployment configuration from a YAML/JSON string.

```rust
pub fn parse_deployment(content: &str) -> Result<serde_json::Value, CompilerError>
```

### Compilation Function

#### `compile`

Compile deployment and definitions into an AST artifact.

```rust
pub fn compile(
    deployment: &serde_json::Value,
    definitions: &serde_json::Value,
) -> Result<Artifact, CompilerError>
```

**Parameters:**
- `deployment`: Parsed deployment (from `parse_deployment`)
- `definitions`: Parsed flag definitions (from `parse_definitions`)

**Returns:**
- `Ok(Artifact)`: Compiled AST artifact
- `Err(CompilerError::Compilation)`: If compilation fails

**Example:**
```rust
use controlpath_compiler::{parse_definitions, parse_deployment, compile};

let definitions = parse_definitions(definitions_yaml)?;
let deployment = parse_deployment(deployment_yaml)?;
let artifact = compile(&deployment, &definitions)?;
```

### Serialization Function

#### `serialize`

Serialize an AST artifact to MessagePack bytes.

```rust
pub fn serialize(artifact: &Artifact) -> Result<Vec<u8>, CompilerError>
```

**Parameters:**
- `artifact`: Compiled AST artifact (from `compile`)

**Returns:**
- `Ok(Vec<u8>)`: MessagePack-encoded bytes
- `Err(CompilerError::Serialization)`: If serialization fails

**Example:**
```rust
use controlpath_compiler::{compile, serialize};

let artifact = compile(&deployment, &definitions)?;
let bytes = serialize(&artifact)?;
```

## Complete Example (v2 catalog)

```rust
use controlpath_compiler::{
    compile_catalog, load_and_validate_catalog, serialize, CatalogValidationContext,
    CompilerError, ValidationMode,
};

fn compile_catalog_yaml(catalog_yaml: &str, env: &str) -> Result<Vec<u8>, CompilerError> {
    let (catalog, initial) = load_and_validate_catalog(
        catalog_yaml,
        "control-path.yaml",
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    )
    .map_err(|e| CompilerError::Parse(e.into()))?;

    if !initial.is_ok() {
        return Err(CompilerError::Validation(
            controlpath_compiler::validator::error::ValidationError::SchemaValidation(
                "catalog validation failed".into(),
            ),
        ));
    }

    let artifact = compile_catalog(&catalog, env)?;
    serialize(&artifact)
}
```

## Error Types

### `CompilerError`

The main error type for all compiler operations.

```rust
pub enum CompilerError {
    Parse(ParseError),
    Validation(ValidationError),
    Compilation(CompilationError),
    Serialization(SerializationError),
}
```

### `ParseError`

Errors that occur during YAML/JSON parsing.

```rust
pub enum ParseError {
    InvalidYaml(String),
    InvalidJson(String),
    MissingField(String),
    InvalidFieldType(String),
}
```

### `ValidationError`

Errors that occur during schema validation.

```rust
pub enum ValidationError {
    SchemaValidation(String),
    InvalidFlagDefinition(String),
    InvalidDeployment(String),
    FlagNotFound(String),
    TypeMismatch(String),
}
```

### `CompilationError`

Errors that occur during AST compilation.

```rust
pub enum CompilationError {
    ExpressionParsing(String),
    InvalidExpression(String),
    StringTable(String),
    InvalidRule(String),
    InvalidSegment(String),
}
```

### `SerializationError`

Errors that occur during MessagePack serialization.

```rust
pub enum SerializationError {
    MessagePack(String),
    InvalidArtifact(String),
}
```

## Data Types

### `Artifact`

The compiled AST artifact structure.

```rust
pub struct Artifact {
    pub version: String,
    pub environment: String,
    pub string_table: Vec<String>,
    pub flags: Vec<Vec<Rule>>,
    pub flag_names: Vec<u16>,
    pub segments: Option<Vec<(u16, Expression)>>,
    pub signature: Option<Vec<u8>>,
}
```

**Fields:**
- `version`: Format version (e.g., "1.0")
- `environment`: Environment name
- `string_table`: All strings referenced by index (deduplicated)
- `flags`: Array of flag rule arrays, indexed by flag definition order
- `flag_names`: Flag names as string table indices
- `segments`: Optional segment definitions as `[name_index, expression]` tuples
- `signature`: Optional Ed25519 signature

### `Rule`

A rule in the AST artifact.

```rust
pub enum Rule {
    ServeWithoutWhen(ServePayload),
    ServeWithWhen(Expression, ServePayload),
    VariationsWithoutWhen(Vec<Variation>),
    VariationsWithWhen(Expression, Vec<Variation>),
    RolloutWithoutWhen(RolloutPayload),
    RolloutWithWhen(Expression, RolloutPayload),
}
```

### `Expression`

An expression AST node (for `when` clauses).

```rust
pub enum Expression {
    Literal { value: serde_json::Value },
    Property { prop_index: u16 },
    BinaryOp { op_code: u8, left: Box<Expression>, right: Box<Expression> },
    UnaryOp { op_code: u8, operand: Box<Expression> },
    FunctionCall { func_index: u16, args: Vec<Expression> },
    ArrayLiteral { elements: Vec<Expression> },
}
```

## WASM Compatibility

The compiler library is designed to be WASM-compatible:

- **No File I/O**: All functions work on in-memory strings and data structures
- **WASM-Compatible Dependencies**: All dependencies compile to `wasm32-unknown-unknown`
- **Embedded Schemas**: JSON schemas are embedded at compile time using `include_str!`
- **Minimal API Surface**: Strings in, bytes out - no file paths or file system access

For WASM usage, see the [WASM Wrapper Guide](./wasm-wrapper-guide.md).

## Performance

The Rust compiler is optimized for performance:

- **Fast Parsing**: Efficient YAML/JSON parsing using `yaml-rust` and `serde_json`
- **Compact Output**: MessagePack serialization produces compact binary artifacts
- **Memory Efficient**: String table deduplication reduces memory usage
- **Zero-Copy Where Possible**: Uses references and slices to minimize allocations

## Thread Safety

All public API functions are thread-safe:

- Functions take immutable references (`&str`, `&serde_json::Value`, `&Artifact`)
- No shared mutable state
- Safe to call from multiple threads concurrently

## Error Handling

The API uses Rust's `Result` type for error handling:

- All functions return `Result<T, CompilerError>`
- Errors are descriptive and include context
- Use `?` operator for error propagation
- Use `match` or `if let` for error handling

## Examples

See the `tests/` directory in the compiler crate for more examples of API usage.

## See Also

- [CLI Usage Documentation](./rust-cli.md)
- [Migration Guide](./migration-typescript-to-rust.md)
- [WASM Wrapper Guide](./wasm-wrapper-guide.md)
- [Architecture Documentation](../control-path-next/ARCHITECTURE.md)

