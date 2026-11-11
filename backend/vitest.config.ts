// Vitest config for the backend (stack/testing.md). Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
// test/it/ (DynamoDB Local) runs only through vitest.it.config.ts (`npm run test:it`).
// Coverage gate: 90 % lines on domain + usecases, 80 % overall; thresholds only ratchet up.
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['src/**/*.test.ts', 'test/**/*.test.ts', 'scripts/**/*.test.mjs'],
    exclude: ['test/it/**', 'node_modules/**'],
    environment: 'node',
    coverage: {
      provider: 'v8',
      include: ['src/**/*.ts'],
      exclude: ['src/**/*.test.ts', 'src/**/__tests__/**'],
      reporter: ['text-summary'],
      thresholds: {
        lines: 80,
        'src/domain/**': { lines: 90 },
        'src/usecases/**': { lines: 90 },
      },
    },
  },
});
