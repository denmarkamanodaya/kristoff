// ProjectStore port: the owner's projects (docs/contract.md "v2 Projects").
import type { AppError } from '../shared/errors/app-error.js';
import type { Result } from '../shared/result/result.js';
import type { Project, Versioned } from '../domain/model.js';

/** Stores an owner's projects. Every method is scoped by `uid`. */
export interface ProjectStore {
  /**
   * Creates a project and counts it in one transaction; `PROJECT_LIMIT_REACHED` when the owner
   * already has `cap` projects.
   */
  create(uid: string, project: Project, cap: number): Promise<Result<void, AppError>>;
  /** Reads one project; `PROJECT_NOT_FOUND` when absent. */
  get(uid: string, projectId: string): Promise<Result<Versioned<Project>, AppError>>;
  /** Every project of the owner (at most the cap), in any order. */
  list(uid: string): Promise<Result<Project[], AppError>>;
  /** Renames if the stored version still equals `version`; else `VERSION_CONFLICT`. */
  rename(
    uid: string,
    projectId: string,
    change: { readonly name: string; readonly updatedAt: string; readonly version: number },
  ): Promise<Result<Project, AppError>>;
  /** Deletes the project and uncounts it in one transaction; `PROJECT_NOT_FOUND` when absent. */
  delete(uid: string, projectId: string): Promise<Result<void, AppError>>;
}
