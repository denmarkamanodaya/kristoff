// Composition root of the `chats-delete` Lambda (docs/contract.md "Thread deletion"): reads env
// once, wires adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { deleteChat } from '../usecases/delete-chat/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the chats-delete handler.
 * @returns Handler plumbing and the delete-chat use case.
 * @throws Error when the env is invalid.
 */
export const chatsDeleteDeps = async () => {
  const env = parseEnv('chats-delete', process.env);
  return Promise.resolve({
    http: baseDeps('chats-delete', env.STAGE).http,
    deleteChat: deleteChat({ chats: adapters.chats(env.TABLE_NAME) }),
  });
};
