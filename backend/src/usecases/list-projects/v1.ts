// GET /v1/projects use case (docs/contract.md "v2 Projects"): every project of the owner (≤ 50),
// in name order (locale-insensitive, tie-break on id so the order is total).
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { Principal, Project } from '../../domain/model.js';
import type { ProjectStore } from '../../ports/project-store.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { ok, type Result } from '../../shared/result/result.js';

const byName = (a: Project, b: Project): number =>
  a.name < b.name ? -1 : a.name > b.name ? 1 : a.id < b.id ? -1 : a.id > b.id ? 1 : 0;

/**
 * Builds the list-projects use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - The project store.
 * @returns A function from the principal to the projects in name order; store failures pass.
 */
export const listProjects =
  (deps: { readonly projects: ProjectStore }) =>
  async (principal: Principal): Promise<Result<Project[], AppError>> => {
    const all = await deps.projects.list(principal.uid);
    return all.ok ? ok([...all.value].sort(byName)) : all;
  };
