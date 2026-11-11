// GET /v1/config use case (docs/contract.md "Routes", "Model choice"): the client-visible settings,
// all from configuration values the composition root passes in.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.

/** One selectable model: its API id and the label the web shows. */
interface ModelOption {
  readonly id: string;
  readonly label: string;
}

/** Settings the web shows or enforces in the UI (the server enforces them on its own). */
export interface ClientConfig {
  /** The default model id (the first allowed one). */
  readonly model: string;
  /** The allowed models in allowlist order. */
  readonly models: readonly ModelOption[];
  readonly maxAttachedDocs: number;
  readonly dailyQuestionCap: number;
}

/** What the composition root passes in. */
export interface ConfigSource {
  /** Allowed model ids, first = default (env `MODEL_IDS`, at least one). */
  readonly modelIds: readonly [string, ...string[]];
  /** Labels by model id (src/contracts/model-config.ts); an id without a label shows the id. */
  readonly labels: Readonly<Record<string, string>>;
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
 * @param source - Allowed models, labels and limits from configuration.
 * @returns A function returning the settings; it cannot fail.
 */
export const getConfig = (source: ConfigSource) => (): ClientConfig => ({
  model: source.modelIds[0],
  models: source.modelIds.map((id) => ({ id, label: source.labels[id] ?? id })),
  maxAttachedDocs: source.maxAttachedDocs,
  dailyQuestionCap: source.dailyQuestionCap,
});
