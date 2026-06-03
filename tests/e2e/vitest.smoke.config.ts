import { defineConfig } from 'vitest/config';

/** Pre-merge smoke only — do not merge with vitest.config.ts (base excludes src/smoke). */
export default defineConfig({
  test: {
    pool: 'forks',
    isolate: true,
    globals: true,
    include: ['src/smoke/**/*.test.ts'],
    exclude: ['**/node_modules/**'],
  },
});
