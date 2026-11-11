// Layer rules (core/architecture-hexagonal.md): dependencies point inward.
// usecases -> ports -> domain; shared and contracts' error list are the kernel. Only adapters and
// composition may import SDKs; nothing but handlers imports composition.
/** @type {import('dependency-cruiser').IConfiguration} */
module.exports = {
  forbidden: [
    { name: 'no-circular', severity: 'error', from: {}, to: { circular: true } },
    {
      name: 'pure-layers-no-packages',
      comment: 'domain, ports, usecases and shared import no npm package (node built-ins only)',
      severity: 'error',
      from: { path: '^src/(domain|ports|usecases|shared)', pathNot: '^src/shared/http/' },
      to: { dependencyTypes: ['npm', 'npm-dev', 'npm-optional', 'npm-peer', 'npm-no-pkg'] },
    },
    {
      name: 'pure-layers-no-io-builtins',
      severity: 'error',
      from: { path: '^src/(domain|ports|usecases)' },
      to: { dependencyTypes: ['core'], pathNot: '^node:crypto$|^crypto$' },
    },
    {
      name: 'domain-is-innermost',
      severity: 'error',
      from: { path: '^src/domain' },
      to: { path: '^src/(ports|usecases|adapters|handlers|composition)' },
    },
    {
      name: 'ports-only-domain',
      severity: 'error',
      from: { path: '^src/ports' },
      to: { path: '^src/(usecases|adapters|handlers|composition)' },
    },
    {
      name: 'usecases-not-outer',
      severity: 'error',
      from: { path: '^src/usecases' },
      to: { path: '^src/(adapters|handlers|composition)' },
    },
    {
      name: 'core-only-error-contract',
      comment: 'the core may read the error-code list, never a wire schema',
      severity: 'error',
      from: { path: '^src/(domain|ports|usecases|shared)', pathNot: '^src/shared/http/' },
      to: { path: '^src/contracts/', pathNot: '^src/contracts/errors\\.ts$' },
    },
    {
      name: 'http-plumbing-only-for-handlers',
      comment: 'shared/http is driving-adapter plumbing (envelope, body parse): handlers only',
      severity: 'error',
      from: { pathNot: '^src/(handlers|shared/http)/' },
      to: { path: '^src/shared/http/' },
    },
    {
      name: 'adapters-not-driving',
      severity: 'error',
      from: { path: '^src/adapters' },
      to: { path: '^src/(usecases|handlers|composition)' },
    },
    {
      name: 'only-handlers-import-composition',
      severity: 'error',
      from: { pathNot: '^src/(handlers|composition)' },
      to: { path: '^src/composition' },
    },
    {
      name: 'sdk-only-in-adapters-and-composition',
      severity: 'error',
      from: { pathNot: '^src/(adapters|composition)' },
      to: { path: '@aws-sdk|@anthropic-ai' },
    },
  ],
  options: {
    tsConfig: { fileName: 'tsconfig.json' },
    doNotFollow: { path: 'node_modules' },
    exclude: { path: '\\.test\\.ts$|__tests__' },
  },
};
