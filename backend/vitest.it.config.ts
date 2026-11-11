// Vitest config for the DynamoDB Local contract run (`DYNAMODB_LOCAL_URL=... npm run test:it`).
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: { include: ['test/it/**/*.test.ts'], environment: 'node' },
});
