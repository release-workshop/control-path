/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { defineConfig } from 'vitest/config';
import { resolve } from 'node:path';

export default defineConfig({
  test: {
    name: 'comparison-tests',
    include: ['**/*.test.ts'],
    globals: true,
    environment: 'node',
    // Allow longer timeout for comparison tests (they may need to build Rust CLI)
    testTimeout: 30000,
  },
  resolve: {
    alias: {
      // Allow importing from packages - use src for source files (tests run against source)
      '@controlpath/compiler': resolve(__dirname, '../../packages/compiler/src'),
    },
  },
});

