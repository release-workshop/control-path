# E2E Tests for SDK Generator

End-to-end tests that verify the SDK generator works correctly by:

1. Generating TypeScript SDKs from flag definitions
2. Compiling AST artifacts from deployment files
3. Verifying the generated SDK structure and functionality
4. Testing various rule combinations (simple, conditional, etc.)

## Running Tests

```bash
# Install dependencies
cd tests/e2e
npm install

# Run tests
npm test

# Watch mode
npm run test:watch

# Type check
npm run typecheck
```

## Prerequisites

- Rust CLI must be built: `cargo build --release --bin controlpath`
- The CLI binary should be available at `target/release/controlpath` or `target/debug/controlpath`

## Test Structure

Tests are organized by scenario:
- **Simple Rules**: Basic serve rules without conditions
- **Conditional Rules**: Rules with `when` clauses
- **Default Values**: Verification of embedded defaults
- **Batch Evaluation**: Type-safe batch evaluation methods
- **Context Management**: setContext/clearContext methods
- **Method Overloads**: TypeScript overload signatures
- **Error Handling**: Never-throws policy verification
- **Runtime SDK Integration**: Provider and OpenFeature integration

