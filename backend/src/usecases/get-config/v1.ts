// GET /v1/config use case (docs/contract.md "Routes"): the client-visible settings, all from
// configuration values the composition root passes in. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.

/** Settings the web shows or enforces in the UI (the server enforces them on its own). */
export interface ClientConfig {
  readonly model: string;
  readonly maxAttachedDocs: number;
  readonly dailyQuestionCap: number;
}

/**
 * Builds the get-config use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param config - Values from the env (`MODEL_ID`, `MAX_ATTACHED_DOCS`, `DAILY_QUESTION_CAP`).
 * @returns A function returning those settings; it cannot fail.
 */
export const getConfig = (config: ClientConfig) => (): ClientConfig => ({ ...config });
