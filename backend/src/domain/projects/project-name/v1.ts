// The project-name rule (docs/contract.md "v2 Projects": name 1..80): trimmed, never blank. One
// rule for create-project and rename-project. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { MAX_PROJECT_NAME } from '../../limits.js';
import { appError, type AppError } from '../../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../../shared/result/result.js';

/**
 * Normalises a project name.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param raw - The name as sent (the request schema already bounds it to 80 characters).
 * @returns The trimmed name, or `VALIDATION_FAILED` when it is blank or longer than 80.
 */
export const projectName = (raw: string): Result<string, AppError> => {
  const name = raw.trim();
  return name === '' || name.length > MAX_PROJECT_NAME
    ? err(appError('VALIDATION_FAILED', 'name must be 1 to 80 characters'))
    : ok(name);
};
