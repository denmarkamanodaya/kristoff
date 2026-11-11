// Composition root of the `docs-list` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { listDocs } from '../usecases/list-docs/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the docs-list handler.
 * @returns Handler plumbing and the list-docs use case.
 * @throws Error when the env is invalid.
 */
export const docsListDeps = async () => {
  const env = parseEnv('docs-list', process.env);
  return Promise.resolve({
    http: baseDeps('docs-list', env.STAGE).http,
    listDocs: listDocs({ docs: adapters.docs(env.TABLE_NAME) }),
  });
};
