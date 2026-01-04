#!/bin/bash
# Helper script to run E2E tests and keep generated SDKs for inspection

set -e

cd "$(dirname "$0")/.."

echo "Building CLI..."
cargo build --release --bin controlpath

echo "Building runtime SDK..."
cd runtime/typescript
npm run build
cd ../..

echo "Running E2E tests..."
cd tests/e2e

# Set environment variable to keep temp directories
export KEEP_TEMP_SDKS=true

# Run tests and capture output
npm test 2>&1 | tee /tmp/e2e-test-output.log

echo ""
echo "Test output saved to /tmp/e2e-test-output.log"
echo ""
echo "To find generated SDKs, check:"
echo "  ls -la /tmp/controlpath-e2e/"

