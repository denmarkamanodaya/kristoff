// PATCH /v1/projects/{id} use case (docs/contract.md "v2 Projects"): renames a project.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { Principal, Project } from '../../domain/model.js';
import type { Clock } from '../../ports/clock.js';
import type { ProjectStore } from '../../ports/project-store.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { projectName } from '../../domain/projects/project-name/index.js';
import type { Result } from '../../shared/result/result.js';

/**
 * Builds the rename-project use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Project store and clock.
 * @returns A function from the principal, project id and name to the project, or
 *   `VALIDATION_FAILED`, `PROJECT_NOT_FOUND` and `VERSION_CONFLICT`.
 */
export const renameProject =
  (deps: { readonly projects: ProjectStore; readonly clock: Clock }) =>
  async (
    principal: Principal,
    projectId: string,
    name: string,
  ): Promise<Result<Project, AppError>> => {
    const checked = projectName(name);
    if (!checked.ok) return checked;
    const found = await deps.projects.get(principal.uid, projectId);
    if (!found.ok) return found;
    const updatedAt = new Date(deps.clock.now()).toISOString();
    return deps.projects.rename(principal.uid, projectId, {
      name: checked.value,
      updatedAt,
      version: found.value.version,
    });
  };
