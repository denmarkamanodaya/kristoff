// Composition root of the `docs-create` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { cryptoIdGenerator } from '../adapters/crypto-id-generator/index.js';
import { systemClock } from '../adapters/system-clock/index.js';
import { createDocUpload } from '../usecases/create-doc-upload/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the docs-create handler.
 * @returns Handler plumbing and the create-doc-upload use case.
 * @throws Error when the env is invalid.
 */
export const docsCreateDeps = async () => {
  const env = parseEnv('docs-create', process.env);
  return Promise.resolve({
    http: baseDeps('docs-create', env.STAGE).http,
    createDocUpload: createDocUpload({
      docs: adapters.docs(env.TABLE_NAME),
      files: adapters.files(env.DOCS_BUCKET),
      ids: cryptoIdGenerator(() => systemClock.now()),
      clock: systemClock,
    }),
  });
};
