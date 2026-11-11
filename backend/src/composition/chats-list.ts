// Composition root of the `chats-list` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { listChats } from '../usecases/list-chats/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the chats-list handler.
 * @returns Handler plumbing and the list-chats use case.
 * @throws Error when the env is invalid.
 */
export const chatsListDeps = async () => {
  const env = parseEnv('chats-list', process.env);
  return Promise.resolve({
    http: baseDeps('chats-list', env.STAGE).http,
    listChats: listChats({ chats: adapters.chats(env.TABLE_NAME) }),
  });
};
