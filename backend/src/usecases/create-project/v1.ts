// POST /v1/projects use case (docs/contract.md "v2 Projects"): a new project, at most 50 per
// owner (`PROJECT_LIMIT_REACHED`, enforced by the store's conditional counter).
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { MAX_PROJECTS } from '../../domain/limits.js';
import type { Principal, Project } from '../../domain/model.js';
import type { Clock } from '../../ports/clock.js';
import type { IdGenerator } from '../../ports/id-generator.js';
import type { ProjectStore } from '../../ports/project-store.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { projectName } from '../../domain/projects/project-name/index.js';
import { ok, type Result } from '../../shared/result/result.js';

/** What create-project needs. */
export interface CreateProjectDeps {
  readonly projects: ProjectStore;
  readonly ids: IdGenerator;
  readonly clock: Clock;
}

/**
 * Builds the create-project use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Project store, ids and clock.
 * @returns A function from the principal and name to the project, or `VALIDATION_FAILED` (blank
 *   name), `PROJECT_LIMIT_REACHED` and store failures.
 */
export const createProject =
  (deps: CreateProjectDeps) =>
  async (principal: Principal, name: string): Promise<Result<Project, AppError>> => {
    const checked = projectName(name);
    if (!checked.ok) return checked;
    const now = new Date(deps.clock.now()).toISOString();
    const project: Project = {
      id: deps.ids.newId(),
      name: checked.value,
      createdAt: now,
      updatedAt: now,
    };
    const created = await deps.projects.create(principal.uid, project, MAX_PROJECTS);
    return created.ok ? ok(project) : created;
  };
