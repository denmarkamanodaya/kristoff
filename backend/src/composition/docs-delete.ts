// Composition root of the `docs-delete` Lambda (docs/contract.md "v2 File delete"): reads env once,
// wires adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { deleteDoc } from '../usecases/delete-doc/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the docs-delete handler.
 * @returns Handler plumbing and the delete-doc use case.
 * @throws Error when the env is invalid.
 */
export const docsDeleteDeps = async () => {
  const env = parseEnv('docs-delete', process.env);
  return Promise.resolve({
    http: baseDeps('docs-delete', env.STAGE).http,
    deleteDoc: deleteDoc({
      docs: adapters.docs(env.TABLE_NAME),
      chats: adapters.chats(env.TABLE_NAME),
      files: adapters.files(env.DOCS_BUCKET),
    }),
  });
};
