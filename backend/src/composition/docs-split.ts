// Composition root of the `docs-split` Lambda (docs/contract.md "v2 Large PDFs"): reads env once,
// wires adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { pdfLibReader } from '../adapters/pdf-lib-reader/index.js';
import { splitDoc } from '../usecases/split-doc/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the docs-split handler.
 * @returns Logger, clock and the split-doc use case.
 * @throws Error when the env is invalid.
 */
export const docsSplitDeps = async () => {
  const env = parseEnv('docs-split', process.env);
  const { log, http } = baseDeps('docs-split', env.STAGE);
  return Promise.resolve({
    log,
    now: http.now,
    splitDoc: splitDoc({
      docs: adapters.docs(env.TABLE_NAME),
      files: adapters.bulkFiles(env.DOCS_BUCKET),
      pdfs: pdfLibReader(),
    }),
  });
};
