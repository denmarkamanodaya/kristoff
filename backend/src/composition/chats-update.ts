// Composition root of the `chats-update` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { systemClock } from '../adapters/system-clock/index.js';
import { resolveAttachedDocs } from '../usecases/resolve-attached-docs/index.js';
import { updateChat } from '../usecases/update-chat/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the chats-update handler.
 * @returns Handler plumbing and the update-chat use case.
 * @throws Error when the env is invalid.
 */
export const chatsUpdateDeps = async () => {
  const env = parseEnv('chats-update', process.env);
  const docs = adapters.docs(env.TABLE_NAME);
  return Promise.resolve({
    http: baseDeps('chats-update', env.STAGE).http,
    updateChat: updateChat({
      chats: adapters.chats(env.TABLE_NAME),
      clock: systemClock,
      resolveDocs: resolveAttachedDocs({ docs, maxAttachedDocs: env.MAX_ATTACHED_DOCS }),
    }),
  });
};
