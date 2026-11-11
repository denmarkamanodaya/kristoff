// Composition root of the `chats-get` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { getChat } from '../usecases/get-chat/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the chats-get handler.
 * @returns Handler plumbing and the get-chat use case.
 * @throws Error when the env is invalid.
 */
export const chatsGetDeps = async () => {
  const env = parseEnv('chats-get', process.env);
  return Promise.resolve({
    http: baseDeps('chats-get', env.STAGE).http,
    getChat: getChat({ chats: adapters.chats(env.TABLE_NAME) }),
  });
};
