# Kristoff Desktop plan

Status: approved by the owner 2026-11-12. Build steps: [`phases/`](phases/README.md).
Branch base: `feat/app-dk-rebuild` (PR #1). The desktop work needs the rebuilt `web/` UI, so it
stacks on that branch until PR #1 merges.

Related: [`docs/contract.md`](../contract.md) (wire shapes and error codes the app keeps) and the
Claude Doc "Kristoff PH Law RAG — Plan, Architecture & Cost" (the statute RAG design).

## Decisions

| ID | Decision | Date | By |
|---|---|---|---|
| D-1 | Rust code is not split into `v<N>` units until a rollback need appears (same as DexFS D13). Nothing has shipped. | 2026-11-12 | Owner (approved) |
| D-2 | Rust (Tauri 2), not Electron. | 2026-11-12 | Owner |
| D-3 | Apple Silicon only. Ad-hoc signed, never notarised. Private use by one person. | 2026-11-12 | Owner |
| D-4 | The Claude API key and the TOTP secret are baked in at build time. | 2026-11-12 | Owner |
| D-5 | Start fresh; no import of AWS data. | 2026-11-12 | Owner |
| D-6 | Keep the TOTP gate on every launch. | 2026-11-12 | Owner |
| D-7 | Retire the AWS stack after the app works, as a separate, owner-approved step. | 2026-11-12 | Owner |

## Context

Kristoff is live as a web app on AWS (Lambda, S3, DynamoDB) for the owner's brother, a law student.
The owner now wants:

- **One standalone Mac app**, used privately by the brother only, built in **Rust (Tauri 2), not Electron**.
- **The PH statute RAG** from the Claude Doc "Kristoff PH Law RAG — Plan, Architecture & Cost" built into it.
- **No notarisation.** The app is ad-hoc signed only.
- **The AWS stack retired** once the app works (a later, separately approved step).

Owner answers (2026-11-12):

| Question | Answer |
|---|---|
| Target Mac | Apple Silicon only (`aarch64-apple-darwin`) |
| Claude API key | Baked into the build |
| Existing AWS data | Start fresh (no import) |
| App lock | Keep the TOTP gate (the same 6-digit authenticator flow as today) |

What stays the same: the vanilla-JS `web/` UI (threads, projects, library, Sources cards, follow-ups,
dictation, dark default), the system prompt, the citation behaviour and the error-code vocabulary in
`docs/contract.md`.

> **Security note (baked key).** Anyone who gets the DMG can extract the API key and spend the
> owner's credits. Mitigation: a dedicated Console key in its own workspace with a monthly spend
> limit, set before the first build. Rotating the key means revoking it and shipping a new build.
> The TOTP secret is baked the same way, so the gate stops casual use only; it does not protect
> the key.

## Architecture

```
Kristoff.app (Tauri 2, arm64, ad-hoc signed)
├── WKWebView: web/ (unchanged UI) + services/api-desktop.js  ── invoke / Channel ──┐
└── Rust                                                                            │
    ├── src-tauri: composition root + #[tauri::command]s  ◄──────────────────────────┘
    ├── kristoff-core:      domain + use cases + ports (pure; clippy bans IO and clock)
    ├── kristoff-adapters:  SqliteStore · DiskFileStore · AnthropicModel (reqwest SSE)
    │                       · AnthropicFiles · LopdfSplitter · CandleEmbedder · LawIndex
    ├── kristoff-contracts: serde wire shapes = docs/contract.md; checked against contracts/fixtures
    └── kristoff-ingest (bin, build-time only): HF Parquet → phlaws.db (FTS5 + int8 vectors)

Data on his Mac: ~/Library/Application Support/<bundle id>/
    kristoff.db (user data, WAL) · docs/<id>.pdf · docs/<id>/part-<n>.pdf
Bundled resources: phlaws.db (read-only, ~150–200 MB) · e5-small f16 weights + tokenizer (~250 MB)
Only outside call: api.anthropic.com
```

Key choices, all decided:

| Area | Choice | Why |
|---|---|---|
| UI-to-Rust seam | Tauri `invoke` for JSON calls, `ipc::Channel` for the answer stream | No local port, no CORS, no token. The service layer is designed for this swap |
| Web change | New `web/services/api-desktop.js` exporting the **same function names** as `api.js`. The build picks one. A check asserts the two export lists match | Only one module changes. `api.js` stays until AWS is retired |
| Uploads | `tauri-plugin-dialog` returns file paths; Rust copies the files into `docs/` | Avoids pushing 200 MB through IPC |
| PDF viewing | Custom URI scheme `kristoff://doc/<id>[/part-n]` opened in a Tauri `WebviewWindow` at `#page=N` | WKWebView ignores `window.open`. Phase 0 spike verifies that `#page` works there; fallback is to open the file in the default browser |
| PDFs to Claude | Upload each PDF and part once to the **Files API** when the doc becomes ready, store `fileId`, send `{source:{type:"file"}}` | Avoids re-uploading up to 150 MB of base64 per question. Phase 0 verifies citations on file sources |
| Splitting | `lopdf`: same rules as `domain/docs/plan-parts` (100 pages / 30 MiB parts, max 200) | Keeps the 200 MB limit |
| Streaming | reqwest with the `stream` feature plus a small hand-written SSE parser in the adapter | No official Rust SDK exists |
| Embeddings | `multilingual-e5-small` through **candle** (pure Rust, Metal). Corpus vectors are built at ingest; the app embeds only the query | One embedder implementation, no Python or Node at runtime |
| Vector search | Brute-force cosine over int8 BLOBs (95k × 384, ~36 MB) + FTS5 BM25, merged by reciprocal rank fusion with K=60 | DexFS pattern (no sqlite-vec) |
| Gate | TOTP RFC 6238 (HMAC-SHA1 via `ring`, ±1 step, replay guard, 5 failures lock for 15 min), state in `kristoff.db`. Every command except `verify_otp` fails closed with `UNAUTHENTICATED` while the app is locked. Unlock lives in memory only | Same behaviour as `usecases/verify-otp`, minus HTTP tokens |
| Secrets | Read by `build.rs` from env (`KRISTOFF_ANTHROPIC_KEY`, `KRISTOFF_TOTP_SECRET`). The build fails if either is missing. Never in git | Owner chose a baked key. Reuse the TOTP secret already enrolled, so his authenticator keeps working |
| Signing | `bundle.macOS.signingIdentity: "-"` (ad-hoc), `hardenedRuntime: false`, no notarisation | Apple Silicon needs at least an ad-hoc signature to run |
| Versioning | Rust is not split into `v<N>` units until a rollback need appears (same ruling as DexFS D13) | Nothing has shipped. Recorded as decision D-1 in the plan doc |

Kept from the web backend as behaviour (ported, not copied): all 20 routes become commands with the
same request and response shapes and error codes; the answer flow order in
`backend/src/usecases/ask-question/v1.ts:303-336`; `build-model-turns`; `map-citation`; the
follow-up suggester (Haiku, JSON schema output); the daily question cap; usage counters; and delete
purging the doc id from chats.

Dropped: S3 presigned URLs, SSM, the kill switch, the authorizer, session tokens, polling backoff
tuned for Lambda, and the docs-split SQS queue/DLQ (splitting runs in a background task instead).

Contract change (additive): `Citation` gains `kind: "doc"|"law"` and `sourceUrl|null`, so statute
articles render as Sources cards ("RA 9262 · Sec. 5") linking to the Official Gazette or LRB
`source_url`.

## Repo layout (new top-level dir: needs owner OK, given by approving this plan)

```
kristoff/desktop/
  Cargo.toml  rust-toolchain.toml (1.99.1)  deny.toml  clippy.toml  rustfmt.toml
  crates/kristoff-core/  kristoff-adapters/  kristoff-contracts/  kristoff-ingest/
  app/src-tauri/  (tauri.conf.json, capabilities/, build.rs, src/commands/*.rs)
  scripts/ci-local.sh  scripts/build-dmg.sh  scripts/fetch-model.sh
web/services/api-desktop.js     web/scripts/build.mjs gains --target desktop
contracts/fixtures/             shared by Rust tests and web/scripts/mock-api.mjs
docs/desktop/plan.md            this plan, for review in VS Code before any code
docs/desktop/install-for-brother.md
```

DexFS prior art to reuse as patterns (not as code dependencies):

- Workspace lints and the pure-core clippy bans: `dexfs/Cargo.toml:67-81`, `dexfs/crates/footage-core/clippy.toml`.
- SQLite setup (WAL, `BEGIN IMMEDIATE`, `user_version` migrations): `dexfs/crates/footage-adapters/src/sqlite_setup.rs`.
- FTS5 with the porter/unicode61 tokenizer: `sqlite_catalogue/schema_v15.rs:52-67`.
- Brute-force vectors and rank fusion: `sqlite_catalogue/vectors.rs` and `footage-core/src/domain/fuse_ranks.rs`.
- Command and permission generation: `app/src-tauri/build.rs` and `commands/mod.rs:60-82`.
- CI chain: `dexfs/scripts/ci-local.sh`.

## New dependencies (each needs an owner yes; checked against the 7-day release-age rule when added)

| Crate | Version source | Purpose |
|---|---|---|
| tauri 2.12.3, tauri-build 2.7.2, tauri-plugin-dialog 2.8.2 | already approved in DexFS | Shell, file picker |
| rusqlite 0.41.1 (bundled, FTS5) | DexFS | Both databases |
| reqwest 0.13.5 (+ `stream`), tokio 1.53.3, serde, serde_json, ring 0.17.14 | DexFS (`stream` feature is new) | Claude API, async, TOTP HMAC |
| **candle-core / candle-nn / candle-transformers, tokenizers** | new | Query embeddings |
| **lopdf** | new | PDF page count and split |
| **parquet** (ingest bin only, not shipped in the app) | new | Read the HF dataset |

## Phases (one PR per batch on `feat/desktop-*` → `develop`; owner merges)

0. **Plan doc and spikes (0.5–1 day).**
   - Write `docs/desktop/plan.md` and open it in VS Code for owner review.
   - Spikes in a throwaway branch, each recorded as yes or no:
     1. An ad-hoc signed DMG installs and opens after quarantine on a second macOS user account.
     2. `webkitSpeechRecognition` exists in Tauri's WKWebView, with `NSMicrophoneUsageDescription` and `NSSpeechRecognitionUsageDescription` set.
     3. A PDF opens at `#page=N` through the custom scheme.
     4. The Files API with `citations.enabled` returns page locations.
     5. `search_result` tool content streams `citations_delta`.
     6. candle e5-small vectors match HF reference vectors (golden file, cosine ≥ 0.999).
   - Any "no" changes the plan before Phase 1.
1. **Core and storage (1.5 days).** Contracts plus fixtures; the SQLite schema for docs, chats,
   messages, projects, usage and TOTP; `DiskFileStore`; the TOTP gate; every command except ask;
   `api-desktop.js`; the build target. Exit: the whole UI works except asking (library, projects, threads).
2. **Ask flow (1.5 days).** Files API upload when a doc is ready, the lopdf split task, the
   `AnthropicModel` SSE stream through `Channel` (text, citation, followups, done or error), cancel
   on window close, the follow-up suggester, the PDF viewer window. Exit: one answer cites a page of
   an uploaded 150-page, 60 MB PDF.
3. **Law index (1 day).** `kristoff-ingest`: download Parquet (snapshot 2026-09-16), clean, chunk at
   article level with the law-title prefix, FTS5, e5 vectors as int8, write `phlaws.db`. A
   `LawIndex` adapter does hybrid search. Exit: the 20-query smoke test finds the right article in
   the top 5.
4. **Statute tools (1 day).** `search_ph_laws` and `get_law_article` tool loop (max 3 searches),
   `search_result` blocks, law citation cards, the composer "Statutes" toggle, `law_status` badges,
   and the system prompt cite-or-say-not-found rules. Exit: one answer cites both a decision page and RA 9262 Sec. 5.
5. **Eval and package (1 day).** A 40-question eval (≥ 90% of look-ups in the top 8, zero uncited
   statutory claims). `build-dmg.sh` builds an arm64 ad-hoc DMG of ~450 MB at
   `desktop/dist/Kristoff-<ver>-arm64.dmg`. `install-for-brother.md` covers dragging to
   Applications, then System Settings → Privacy & Security → Open Anyway, or
   `xattr -dr com.apple.quarantine /Applications/Kristoff.app`. README update.
6. **Retire AWS (later, separate owner yes per apply).** Run `terraform destroy` on the compute and
   data stacks, then the bootstrap, and close or keep the `kristoff-prod` account. Remove
   `backend/`, `web/services/api.js` and the Amplify app. Separate PR.

Total ≈ 6–7 working days.

## Verification

- Every PR runs `desktop/scripts/ci-local.sh`: `cargo fmt --check`, `clippy -D warnings -W
  clippy::pedantic`, `cargo test --workspace`, `cargo deny check licenses advisories`, the
  300-line file cap, dry-gate on Rust and web, `web` `npm run check`, and the fixture-parity test
  (Rust serialises each `contracts/fixtures` shape byte-for-byte).
- Wire tests: assert the exact outgoing Claude request JSON (document blocks, `cache_control` on
  the last document, tools, the beta header) against recorded fixtures, and parse recorded SSE byte
  streams into events. No live API calls in tests.
- Mutation checks on the guards: gate locked → `UNAUTHENTICATED`; replayed code; lockout; daily
  cap; attachment-target rules; max 3 tool calls. Record each `guard → test` pair in the PR.
- End to end, run by me on this Mac with `cargo tauri dev` and then the built DMG. Unlock with a
  live TOTP code, upload a large PDF, ask a mixed question, open a citation at its page, dictate,
  delete the file, and confirm it is gone from disk and from chats.
- The owner tests only the final DMG (memory: owner tests the final build), using a checklist in `install-for-brother.md`.
