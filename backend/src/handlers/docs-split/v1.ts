// docs-split handler (docs/contract.md "v2 Large PDFs"): an asynchronous Lambda invoked by confirm.
// Parses the event, runs split-doc, logs exactly one outcome line. It never throws on a typed
// failure: split-doc has already marked the doc `failed`, and a Lambda retry would only be skipped.
import { SplitEvent } from '../../contracts/requests.js';
import type { SplitOutcome } from '../../usecases/split-doc/index.js';
import { docsSplitDeps } from '../../composition/docs-split.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import type { Logger } from '../../shared/log/logger.js';
import type { Result } from '../../shared/result/result.js';

/** What the docs-split handler needs. */
export interface DocsSplitHandlerDeps {
  readonly log: Logger;
  readonly now: () => number;
  readonly splitDoc: (uid: string, docId: string) => Promise<Result<SplitOutcome, AppError>>;
}

/**
 * Builds the docs-split handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Logger, clock and the split-doc use case.
 * @returns The handler; resolves for every event (an invalid event is logged as an error).
 */
export const createDocsSplitHandler =
  (deps: DocsSplitHandlerDeps) =>
  async (event: unknown): Promise<void> => {
    const started = deps.now();
    const parsed = SplitEvent.safeParse(event);
    if (!parsed.success) {
      deps.log.error('split.request', { operation: 'docs-split', outcome: 'VALIDATION_FAILED' });
      return;
    }
    const result = await deps.splitDoc(parsed.data.uid, parsed.data.docId);
    const outcome = result.ok ? result.value : result.error.code;
    const fields = { operation: 'docs-split', outcome, durationMs: deps.now() - started };
    if (result.ok) deps.log.info('split.request', fields);
    else deps.log.warn('split.request', fields);
  };

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createDocsSplitHandler(await docsSplitDeps()));
