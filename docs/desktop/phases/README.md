# Kristoff Desktop: phased implementation

Approved by the owner on 2026-11-12, including decision D-1 (no `v<N>` units for Rust yet) and the
new crates (candle, tokenizers, lopdf, parquet). The design and the reasons behind it are in
[`../plan.md`](../plan.md). This folder says how to build it, one phase per file.

| Phase | File | Days | Branch | Exit (one line) |
|---|---|---|---|---|
| 0 | [phase-0-spikes.md](phase-0-spikes.md) | 0.5–1 | `spike/desktop-*` (thrown away) | Six spikes recorded as yes/no in this file |
| 1 | [phase-1-core-storage.md](phase-1-core-storage.md) | 1.5 | `feat/desktop-core` | The whole UI works on the desktop except asking |
| 2 | [phase-2-ask-flow.md](phase-2-ask-flow.md) | 1.5 | `feat/desktop-ask` | An answer cites a page of a 150-page, 60 MB PDF |
| 3 | [phase-3-law-index.md](phase-3-law-index.md) | 1 | `feat/desktop-law-index` | The 20-query smoke test hits the top 5 |
| 4 | [phase-4-statute-tools.md](phase-4-statute-tools.md) | 1 | `feat/desktop-statutes` | One answer cites a PDF page and RA 9262 Sec. 5 |
| 5 | [phase-5-eval-package.md](phase-5-eval-package.md) | 1 | `feat/desktop-package` | Eval passes; the DMG installs on a clean account |
| 6 | [phase-6-retire-aws.md](phase-6-retire-aws.md) | 0.5 | `chore/retire-aws` | AWS destroyed, with a yes for each apply |

## Spike results

Phase 0 fills this table. If a spike says no, change the plan before Phase 1.

| # | Spike | Result | Consequence |
|---|---|---|---|
| S1 | Ad-hoc signed DMG opens on a second macOS account after quarantine | – | – |
| S2 | `webkitSpeechRecognition` works in the Tauri WKWebView | – | – |
| S3 | A PDF opens at `#page=N` from `window.open` through the custom scheme | – | – |
| S4 | Files API document + `citations.enabled` returns page locations | – | – |
| S5 | `search_result` tool content streams `citations_delta` | – | – |
| S6 | candle e5-small matches the reference vectors (cosine ≥ 0.999) | – | – |

## Rules for every phase

- **Branching.** Branch from the previous phase's branch until PR #1 (`feat/app-dk-rebuild`)
  merges, then from `develop`. Use one PR per phase and add commits to the open PR. The owner
  merges. Never push to `develop` or `main`.
- **Gate before every push:** `desktop/scripts/ci-local.sh` (built in Phase 1). Paste its output
  in the PR's **What ran** section. List anything only traced separately.
- **Dependencies.** Add only the crates named in `../plan.md`. Pin exact versions (`=x.y.z`). Check
  that each release is at least 7 days old. Record the version and the date checked in a comment
  in `desktop/Cargo.toml`.
- **Secrets.** Secrets come from the env at build time only (`KRISTOFF_ANTHROPIC_KEY`,
  `KRISTOFF_TOTP_SECRET`). Never in git, fixtures, logs or test output. Tests use a fake key and
  the RFC 6238 test secret.
- **Seams.** Trace every hop before saying it works: web call → `invoke` payload → command →
  use case → adapter → storage → read back. Assert the outgoing payloads in tests.
- **The contract stays the source.** If a shape changes, update [`../../contract.md`](../../contract.md)
  and `contracts/fixtures/` in the same PR.
- **Owner testing.** The owner tests only the final DMG (Phase 5). Phases 1–4 are verified by
  Claude on this Mac with `cargo tauri dev`.

## Decisions made while writing the phases

| ID | Decision | Why |
|---|---|---|
| D-8 | Uploads keep the web flow `createDoc → uploadFile → confirmDoc`. On desktop, `uploadFile` sends the `File` in 8 MiB chunks with raw-body `invoke`. `tauri-plugin-dialog` is dropped | `web/ui/library.js` stays unchanged, upload progress keeps working, and one dependency fewer |
| D-9 | `getDocLink` returns `kristoff://doc/<id>[/part-<n>]`; the Rust shell handles `window.open` (spike S3) | `web/ui/citations.js` stays unchanged |
| D-10 | `parseSse` stays web-only. `api-desktop.js` exports a no-op stub with the same name, so the export-parity check holds | Desktop streams through `Channel`, not SSE |
