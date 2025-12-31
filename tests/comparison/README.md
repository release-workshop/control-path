# Comparison Tests

This directory contains comparison tests that verify the TypeScript and Rust compiler implementations produce identical output.

## Overview

The comparison tests ensure:
- **Byte-for-byte MessagePack output**: Both implementations produce identical serialized artifacts
- **CLI behavior parity**: Both CLIs handle commands and errors identically
- **Feature completeness**: All features work correctly in both implementations

## Prerequisites

1. **Build the Rust CLI**:
   ```bash
   cargo build --release --bin controlpath
   ```

2. **Build the TypeScript compiler**:
   ```bash
   pnpm build
   ```

3. **Install dependencies**:
   ```bash
   cd tests/comparison
   pnpm install
   ```

## Running Tests

From the repository root:
```bash
pnpm test:comparison
```

Or from the `tests/comparison` directory:
```bash
pnpm test
```

For watch mode:
```bash
pnpm test:comparison:watch
```

## Test Structure

- **`test-helpers.ts`**: Utilities for invoking both implementations and comparing outputs
- **`compiler-comparison.test.ts`**: Tests comparing compiler library output (MessagePack byte-for-byte)
- **`cli-comparison.test.ts`**: Tests comparing CLI command behavior and output
- **`performance.test.ts`**: Performance comparison tests (compilation time and artifact size)

## What Gets Tested

### Compiler Comparison Tests

- Basic compilation (simple flags, multiple flags)
- Serve rules (with and without `when` clauses)
- Variations rules (with and without `when` clauses)
- Rollout rules (boolean and multivariate flags)
- Segments (single and multiple)
- Expression functions (STARTS_WITH, IN, complex nested expressions)
- String table deduplication
- Flag ordering
- Edge cases (empty rules, multiple rules per flag)

### CLI Comparison Tests

- Compile command output
- Validate command behavior
- Error handling
- File path resolution

### Performance Comparison Tests

- Compilation time comparison (TypeScript vs Rust)
- Artifact size verification (< 13KB for 500 flags)
- Performance scaling analysis
- Size target verification for different flag counts

## Troubleshooting

### Rust CLI Not Found

If tests fail with "Rust CLI not found", ensure you've built the release binary:
```bash
cargo build --release --bin controlpath
```

### TypeScript Compiler Not Found

If tests fail with module resolution errors, ensure the compiler package is built:
```bash
cd packages/compiler
pnpm build
```

### Deno Not Found

The TypeScript CLI uses Deno. Ensure Deno is installed:
```bash
curl -fsSL https://deno.land/install.sh | sh
```

## Running Performance Benchmarks

### Rust Benchmarks

Run Rust benchmarks using Criterion:
```bash
cargo bench --bench compilation --package controlpath-compiler
```

This will run comprehensive benchmarks for:
- Compilation time (10, 50, 100, 250, 500 flags)
- Full pipeline (parse + compile + serialize)
- Artifact size (10, 50, 100, 250, 500, 1000 flags)
- Parsing performance

Results are saved to `target/criterion/` and include detailed statistics and plots.

### TypeScript Performance Tests

Run TypeScript performance comparison tests:
```bash
cd tests/comparison
pnpm test performance.test.ts
```

These tests compare TypeScript and Rust implementations for:
- Compilation time
- Artifact size
- Performance scaling

## Adding New Tests

When adding new features to either implementation:

1. Add a test case to `compiler-comparison.test.ts` if it's a compiler feature
2. Add a test case to `cli-comparison.test.ts` if it's a CLI feature
3. Add performance tests to `performance.test.ts` if it affects performance
4. Ensure both implementations pass the test
5. If outputs differ, investigate and fix the discrepancy

## CI/CD Integration

These tests should be run in CI/CD to catch regressions. The tests are designed to:
- Fail fast if implementations diverge
- Provide clear error messages showing differences
- Work in both local development and CI environments

