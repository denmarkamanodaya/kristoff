// Composition root of the `docs-confirm` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { confirmDoc } from '../usecases/confirm-doc/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the docs-confirm handler.
 * @returns Handler plumbing and the confirm-doc use case.
 * @throws Error when the env is invalid.
 */
export const docsConfirmDeps = async () => {
  const env = parseEnv('docs-confirm', process.env);
  return Promise.resolve({
    http: baseDeps('docs-confirm', env.STAGE).http,
    confirmDoc: confirmDoc({
      docs: adapters.docs(env.TABLE_NAME),
      files: adapters.files(env.DOCS_BUCKET),
      splits: adapters.splitTrigger(env.DOCS_SPLIT_FUNCTION),
    }),
  });
};
