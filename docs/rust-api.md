# Rust Compiler API Documentation

This document describes the public API of the Control Path Rust compiler library (`controlpath-compiler`).

## Overview

The `controlpath-compiler` crate provides a pure Rust implementation of the Control Path compiler. It compiles deployment YAML files into compact MessagePack AST artifacts. The library is designed to be WASM-compatible and works only with in-memory data (no file I/O).

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
controlpath-compiler = { path = "../compiler" }  # For local development
# Or when published:
# controlpath-compiler = "0.1.0"
```

## Core API

### Parsing Functions

#### `parse_definitions`

Parse flag definitions from a YAML/JSON string.

```rust
pub fn parse_definitions(content: &str) -> Result<serde_json::Value, CompilerError>
```

**Parameters:**
- `content`: YAML or JSON string containing flag definitions

**Returns:**
- `Ok(serde_json::Value)`: Parsed flag definitions as JSON value
- `Err(CompilerError::Parse)`: If parsing fails

**Example:**
```rust
use controlpath_compiler::parse_definitions;

let yaml = r#"
flags:
  - name: my_flag
    type: boolean
    defaultValue: false
"#;

let definitions = parse_definitions(yaml)?;
```

#### `parse_deployment`

Parse deployment configuration from a YAML/JSON string.

```rust
pub fn parse_deployment(content: &str) -> Result<serde_json::Value, CompilerError>
```

**Parameters:**
- `content`: YAML or JSON string containing deployment configuration

**Returns:**
- `Ok(serde_json::Value)`: Parsed deployment as JSON value
- `Err(CompilerError::Parse)`: If parsing fails

**Example:**
```rust
use controlpath_compiler::parse_deployment;

let yaml = r#"
environment: production
rules:
  my_flag:
    rules:
      - serve: true
"#;

let deployment = parse_deployment(yaml)?;
```

### Validation Functions

#### `validate_definitions`

Validate flag definitions against the JSON schema.

```rust
pub fn validate_definitions(definitions: &serde_json::Value) -> Result<(), CompilerError>
```

**Parameters:**
- `definitions`: Parsed flag definitions (from `parse_definitions`)

**Returns:**
- `Ok(())`: If validation passes
- `Err(CompilerError::Validation)`: If validation fails

**Example:**
```rust
use controlpath_compiler::{parse_definitions, validate_definitions};

let definitions = parse_definitions(yaml)?;
validate_definitions(&definitions)?;
```

#### `validate_deployment`

Validate deployment configuration against the JSON schema.

```rust
pub fn validate_deployment(deployment: &serde_json::Value) -> Result<(), CompilerError>
```

**Parameters:**
- `deployment`: Parsed deployment (from `parse_deployment`)

**Returns:**
- `Ok(())`: If validation passes
- `Err(CompilerError::Validation)`: If validation fails

**Example:**
```rust
use controlpath_compiler::{parse_deployment, validate_deployment};

let deployment = parse_deployment(yaml)?;
validate_deployment(&deployment)?;
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

## Complete Example

Here's a complete example showing the full compilation workflow:

```rust
use controlpath_compiler::{
    parse_definitions, parse_deployment, validate_definitions, validate_deployment,
    compile, serialize, CompilerError
};

fn compile_deployment(
    definitions_yaml: &str,
    deployment_yaml: &str,
) -> Result<Vec<u8>, CompilerError> {
    // Parse definitions
    let definitions = parse_definitions(definitions_yaml)?;
    
    // Validate definitions
    validate_definitions(&definitions)?;
    
    // Parse deployment
    let deployment = parse_deployment(deployment_yaml)?;
    
    // Validate deployment
    validate_deployment(&deployment)?;
    
    // Compile to AST
    let artifact = compile(&deployment, &definitions)?;
    
    // Serialize to MessagePack
    let bytes = serialize(&artifact)?;
    
    Ok(bytes)
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

