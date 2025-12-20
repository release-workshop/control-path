import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov'],
      reportsDirectory: './coverage',
      // Enforce minimum coverage thresholds
      // Vitest will fail if coverage falls below these thresholds
      thresholds: {
        lines: 80, // Minimum 80% line coverage
        functions: 80, // Minimum 80% function coverage
        branches: 80, // Minimum 80% branch coverage
        statements: 80, // Minimum 80% statement coverage
        // You can also set per-file thresholds or auto-update thresholds
        // autoUpdate: true, // Auto-update thresholds based on current coverage
        // 100: true, // Require 100% coverage (strict mode)
      },
      // Exclude files from coverage calculation
      exclude: [
        '**/*.test.ts',
        '**/*.spec.ts',
        '**/*.d.ts',
        '**/node_modules/**',
        '**/dist/**',
        '**/coverage/**',
        '**/__tests__/**',
      ],
    },
  },
});
