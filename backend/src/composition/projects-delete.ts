// Composition root of the `projects-delete` Lambda (docs/contract.md "v2 Projects"): reads env once,
// wires adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.

import { deleteProject } from '../usecases/delete-project/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the projects-delete handler.
 * @returns Handler plumbing and the delete-project use case.
 * @throws Error when the env is invalid.
 */
export const projectsDeleteDeps = async () => {
  const env = parseEnv('projects-delete', process.env);
  return Promise.resolve({
    http: baseDeps('projects-delete', env.STAGE).http,
    deleteProject: deleteProject({
      projects: adapters.projects(env.TABLE_NAME),
      chats: adapters.chats(env.TABLE_NAME),
    }),
  });
};
