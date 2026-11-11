// Composition root of the `docs-rename` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { renameDoc } from '../usecases/rename-doc/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the docs-rename handler.
 * @returns Handler plumbing and the rename-doc use case.
 * @throws Error when the env is invalid.
 */
export const docsRenameDeps = async () => {
  const env = parseEnv('docs-rename', process.env);
  return Promise.resolve({
    http: baseDeps('docs-rename', env.STAGE).http,
    renameDoc: renameDoc({ docs: adapters.docs(env.TABLE_NAME) }),
  });
};
