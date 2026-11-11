// Composition root of the `docs-link` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { getDocLink } from '../usecases/get-doc-link/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the docs-link handler.
 * @returns Handler plumbing and the get-doc-link use case.
 * @throws Error when the env is invalid.
 */
export const docsLinkDeps = async () => {
  const env = parseEnv('docs-link', process.env);
  return Promise.resolve({
    http: baseDeps('docs-link', env.STAGE).http,
    getDocLink: getDocLink({
      docs: adapters.docs(env.TABLE_NAME),
      files: adapters.files(env.DOCS_BUCKET),
    }),
  });
};
