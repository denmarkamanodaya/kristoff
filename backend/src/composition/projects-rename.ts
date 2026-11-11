// Composition root of the `projects-rename` Lambda (docs/contract.md "v2 Projects"): reads env once,
// wires adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { systemClock } from '../adapters/system-clock/index.js';
import { renameProject } from '../usecases/rename-project/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the projects-rename handler.
 * @returns Handler plumbing and the rename-project use case.
 * @throws Error when the env is invalid.
 */
export const projectsRenameDeps = async () => {
  const env = parseEnv('projects-rename', process.env);
  return Promise.resolve({
    http: baseDeps('projects-rename', env.STAGE).http,
    renameProject: renameProject({
      projects: adapters.projects(env.TABLE_NAME),
      clock: systemClock,
    }),
  });
};
