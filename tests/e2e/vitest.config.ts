import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    pool: 'forks',
    isolate: true,
    globals: true,
  },
});

