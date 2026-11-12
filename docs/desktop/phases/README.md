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
| S1 | Ad-hoc signed DMG opens on a second macOS account after quarantine | **Yes, on this account only** (2026-11-12) | Signature is ad-hoc. A quarantined copy shows macOS 26 "Not Opened" with only Move to Trash / Done; right-click > Open no longer bypasses it. It launches after System Settings > Privacy & Security > Open Anyway, or after `xattr -dr com.apple.quarantine`. App 10 MiB before resources. Still to do: a second-account test and an actual Open Anyway click, before handover (Phase 5) |
| S2 | `webkitSpeechRecognition` works in the Tauri WKWebView | **Partial** (2026-11-12) | `webkitSpeechRecognition` exists, the Info.plist keys merge, and `start()` raises the macOS Speech Recognition prompt. No speech has been tested yet (needs a human: press Allow, then speak). Phase 2 must handle the pending/denied permission case, because the existing feature check alone won't hide the mic button |
| S3 | A PDF opens at `#page=N` from `window.open` through the custom scheme | **Partial → redesigned** (2026-11-12) | A `WebviewWindow` that Rust opens at `kristoff://doc/x#page=7` shows page 7. `window.open` returns null in WKWebView (popups are blocked), and `open file://…#page=N` drops the fragment. Hence D-12 |
| S4 | Files API document + `citations.enabled` returns page locations | **Yes** (2026-11-12) | `page_location` with correct `start_page_number` (3, 4) on a 5-page PDF. The citation also carries `file_id`; `cited_text` contains `\r\n`, so normalise it. Fixtures: `contracts/fixtures/anthropic-file-doc-*` |
| S5 | `search_result` tool content streams `citations_delta` | **Yes** (2026-11-12) | `search_result_location` with `source`, `title` and `search_result_index`. Sonnet 5.5 called the tool unprompted with a `law_number`. Fixtures: `contracts/fixtures/anthropic-tool-*` |
| S6 | candle e5-small matches the reference vectors (cosine ≥ 0.999) | **Yes** (2026-11-12) | Min cosine 0.9999999. Warm query 4.3 ms on Metal, 15.5 ms on CPU. Model `intfloat/multilingual-e5-small@614241f6`, which loads with candle's `bert` module unchanged; Rust tokens match Python. candle 0.11.1 (metal), tokenizers 0.23.2 (default features off, `onig`). Golden file: `contracts/fixtures/e5-small-golden.json`. Hence D-13 |

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
| D-11 | Raw HTTP uses `anthropic-version: 2026-10-21`. The Files API is GA, so there is no beta header (this supersedes the beta header in Phase 2). Never send `temperature`/`top_p`/`top_k`. Never put `text` blocks with `citations` in a `user` turn. Send assistant `content` back unchanged in the tool loop, including any `thinking` blocks | Verified live in S4/S5 and against the Claude API skill's migration notes for 2026-10-21 |
| D-12 | PDF citations call a new command `open_doc {docId, part, page}`; Rust opens its own `WebviewWindow` at `kristoff://doc/<id>[/part-n]#page=N`. `openDoc` is added to both API modules (the web version keeps today's window.open flow). Supersedes D-9's `window.open` handling; `getDocLink` stays for the web | Spike S3 |
| D-13 | Ship e5 weights as **f16 on disk (235 MB) and upcast to f32 at load**; never compute in f16, because candle's bert mask turns it into NaN. Ingest runs on Metal at batch 8: about 62 min for ~93k chunks, once per dataset snapshot, on the owner's Mac. The app ships the precomputed vectors and embeds only queries | Spike S6 |
| D-14 | The dataset holds only Republic Acts and the 1987 Constitution (no Revised Penal Code, Family Code, Labor Code or Rules of Court). A starter pack of code PDFs was researched, but the only bot-reachable free copies of the RPC and Labor Code are outdated, so the owner chose to **skip the starter pack** (2026-11-12). The system prompt and the Library must say that statute search covers RAs and the Constitution only, and that he can upload any code as a PDF | Phase 3 data finding; owner ruling |
| D-15 | `jscpd` 5.4.2 is a pinned web devDependency (owner approved 2026-11-12). The DRY gate and the web check never skip | Gate integrity |
| D-16 | Phase 5 eval accepted by the owner after run 2 (2026-11-12): look-ups 20/20 in the top 8; concept 10/10; mixed targets 9/10; mixed doc page 10/10; US$0.03 per statute question. Uncited-sentence bar not met: 75 flagged (48 name a law in a verify/scope note, 21 restate a retrieved provision outside its cited paragraph, 6 from memory). Shipped as is; the in-app disclaimer stays. Reports in desktop/target/eval/ | Owner ruling |
| D-17 | The desktop app has **no authentication**: the TOTP gate, `verify_otp`, the TOTP state and the build-time `KRISTOFF_TOTP_SECRET` are removed (2026-11-12). The `auth_state` table stays in existing databases but is unused. Supersedes D-6 and the gate half of D-4 for the desktop; the web (AWS) build keeps its gate | Owner ruling: the brother's Mac is personal and secured, and his authenticator never matched |
| D-18 | Phase 6 done 2026-11-12, at the owner's call (earlier than the one-week wait): compute (176 resources), Amplify, data (table, docs bucket, SSM, SNS) and bootstrap destroyed, each with its own yes; the account kristoff-prod was closed (SUSPENDED, removed for good after 90 days); OU kristoff and its 3 SCPs deleted. Code kept in git (owner ruling); no code-removal PR. Backup: ~/Backups/kristoff-aws-final-2026-11-12. The Claude key is saved at ~/.kristoff-anthropic-key for future DMG builds | Owner ruling |
