// Composition root of the `projects-list` Lambda (docs/contract.md "v2 Projects"): reads env once,
// wires adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.

import { listProjects } from '../usecases/list-projects/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the projects-list handler.
 * @returns Handler plumbing and the list-projects use case.
 * @throws Error when the env is invalid.
 */
export const projectsListDeps = async () => {
  const env = parseEnv('projects-list', process.env);
  return Promise.resolve({
    http: baseDeps('projects-list', env.STAGE).http,
    listProjects: listProjects({ projects: adapters.projects(env.TABLE_NAME) }),
  });
};
