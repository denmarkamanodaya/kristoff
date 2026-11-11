// Composition root of the `chats-create` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { cryptoIdGenerator } from '../adapters/crypto-id-generator/index.js';
import { systemClock } from '../adapters/system-clock/index.js';
import { createChat } from '../usecases/create-chat/index.js';
import { resolveAttachedDocs } from '../usecases/resolve-attached-docs/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the chats-create handler.
 * @returns Handler plumbing and the create-chat use case.
 * @throws Error when the env is invalid.
 */
export const chatsCreateDeps = async () => {
  const env = parseEnv('chats-create', process.env);
  const docs = adapters.docs(env.TABLE_NAME);
  return Promise.resolve({
    http: baseDeps('chats-create', env.STAGE).http,
    createChat: createChat({
      chats: adapters.chats(env.TABLE_NAME),
      ids: cryptoIdGenerator(() => systemClock.now()),
      clock: systemClock,
      resolveDocs: resolveAttachedDocs({ docs, maxAttachedDocs: env.MAX_ATTACHED_DOCS }),
    }),
  });
};
