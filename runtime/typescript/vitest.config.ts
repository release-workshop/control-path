import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    // Use file pool for better file I/O isolation
    // This helps prevent race conditions when tests access the file system
    pool: 'forks',
    // Use isolation to prevent race conditions when tests access the file system
    isolate: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov'],
      reportsDirectory: './coverage',
      thresholds: {
        lines: 80,
        functions: 80,
        branches: 80,
        statements: 80,
      },
      exclude: [
        '**/*.test.ts',
        '**/*.spec.ts',
        '**/*.d.ts',
        'node_modules/**',
        'dist/**',
        'coverage/**',
        '__tests__/**',
        'src/index.ts', // Re-export file, no need to test
      ],
    },
  },
});

