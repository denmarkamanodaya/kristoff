// Composition root of the `projects-create` Lambda (docs/contract.md "v2 Projects"): reads env once,
// wires adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { cryptoIdGenerator } from '../adapters/crypto-id-generator/index.js';
import { systemClock } from '../adapters/system-clock/index.js';
import { createProject } from '../usecases/create-project/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the projects-create handler.
 * @returns Handler plumbing and the create-project use case.
 * @throws Error when the env is invalid.
 */
export const projectsCreateDeps = async () => {
  const env = parseEnv('projects-create', process.env);
  return Promise.resolve({
    http: baseDeps('projects-create', env.STAGE).http,
    createProject: createProject({
      projects: adapters.projects(env.TABLE_NAME),
      ids: cryptoIdGenerator(() => systemClock.now()),
      clock: systemClock,
    }),
  });
};
