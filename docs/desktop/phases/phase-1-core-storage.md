# Phase 1: core, storage and every command except ask

Goal: the real `desktop/` workspace, with the existing UI running inside it. Library, projects,
threads, rename, delete and the TOTP gate all work against local SQLite and disk. Asking shows
`AI_DISABLED` until Phase 2.

## 1. Workspace scaffold

```
desktop/
  Cargo.toml            resolver 3; edition 2024; rust-version 1.99; [workspace.dependencies] pinned with =
  rust-toolchain.toml   1.99.1 + rustfmt + clippy
  rustfmt.toml          max_width = 100
  clippy.toml           too-many-lines 40, too-many-arguments 4, cognitive-complexity 10
  deny.toml             licence allow-list as DexFS; target aarch64-apple-darwin; yanked = deny
  crates/kristoff-core/        clippy.toml bans SystemTime::now, env::var, fs::*, Command::new, TcpStream
  crates/kristoff-contracts/
  crates/kristoff-adapters/
  app/src-tauri/        tauri.conf.json, build.rs, capabilities/default.json, src/
  scripts/ci-local.sh
```

- Copy the workspace lints from `dexfs/Cargo.toml:67-81`: `unsafe_code` deny, `missing_docs` warn,
  clippy pedantic, and deny on `unwrap_used`, `expect_used`, `panic` and `print_*`.
- `tauri.conf.json`:
  - Identifier `app.kristoff.desktop`, productName `Kristoff`, window 1280×820 (minimum 960×600).
  - `frontendDist` is `../../../web/dist-desktop`.
  - `bundle.macOS`: `signingIdentity "-"`, `hardenedRuntime false`, `minimumSystemVersion "14.0"`.
  - CSP: `default-src 'self'; script-src 'self'; style-src 'self'; connect-src ipc: http://ipc.localhost; frame-src kristoff:`.
- `build.rs` reads `KRISTOFF_ANTHROPIC_KEY` and `KRISTOFF_TOTP_SECRET`, fails the build when either
  is missing or empty, and passes them on with `cargo:rustc-env`. Tests and `cargo tauri dev` read
  dev values from an untracked `desktop/.env.local` (gitignored; add it to `.gitignore` in this PR).
- Generate one Tauri permission per command from a single command list in `kristoff-contracts`
  (pattern: `dexfs/app/src-tauri/build.rs`).

## 2. Contracts (`kristoff-contracts`)

- Write serde structs for every shape in `backend/src/contracts/shapes.ts` and `requests.ts`:
  Doc, Part, ChatSummary, Chat, Message, Citation, Usage, Project, Config, the list envelopes and
  the error envelope.
- Use `#[serde(rename_all = "camelCase", deny_unknown_fields)]` on requests. Keep the same bounds
  as `domain/limits.ts`, checked in `validate()` methods.
- Error codes: one `ErrorCode` enum, serialised as `UPPER_SNAKE`, with the same list as
  `contracts/errors.ts:7-67`. It never includes HTTP statuses.
- Additive contract change: `Citation.kind` (`"doc"|"law"`) and `Citation.sourceUrl` (`string|null`).
  Update `docs/contract.md` and add fixtures `contracts/fixtures/citation-doc.json` and
  `citation-law.json`.
- Fixture-parity test: every `contracts/fixtures/*.json` shape deserialises and serialises back
  byte-for-byte (after key-sorted normalisation).

## 3. Core (`kristoff-core`)

Ports (plain traits, no versions): `DocStore`, `ChatStore`, `ProjectStore`, `UsageMeter`,
`AuthStateStore`, `FileStore`, `Clock`, `IdGenerator`.

Port these use cases as behaviour from the TS sources:

| Use case | TS source |
|---|---|
| verify OTP + lockout | `usecases/verify-otp/v1.ts`, `domain/auth/verify-totp/v1.ts`, `totp-lockout/v1.ts` |
| create doc / confirm doc | `usecases/create-doc-upload/v1.ts`, `confirm-doc/v1.ts` (`%PDF-` check, size match) |
| rename, list, delete doc | `usecases/delete-doc/v1.ts` (also strips the doc id and part ids from every chat) |
| attachment rules | `domain/docs/attachment-target/v1.ts` |
| chats CRUD, projects CRUD (cap 50) | `usecases/*chat*`, `usecases/*project*` |
| usage get | `usecases/get-usage` |

Ids: 16-hex ids sortable by time (12 hex of epoch ms + 4 random hex) and ULIDs for messages, both
from `IdGenerator`. Same formats as `adapters/crypto-id-generator/v1.ts`.

## 4. Adapters (`kristoff-adapters`)

- **`SqliteStore`** (rusqlite bundled):
  - Database at `<app_data_dir>/kristoff.db`. WAL, `foreign_keys=ON`, busy timeout 2 s, writes in
    `BEGIN IMMEDIATE`, migrations through `PRAGMA user_version` (pattern:
    `dexfs/crates/footage-adapters/src/sqlite_setup.rs`).
  - Tables: `docs`, `doc_parts`, `chats`, `chat_docs`, `messages`, `projects`, `usage_days`,
    `usage_total`, `auth_state`. Every mutable row has an integer `version` and writes check it.
  - The daily question cap is one conditional `UPDATE ... WHERE questions < ?cap` that runs before
    the work.
  - Cursors are opaque base64url of `(sort key, id)`.
- **`DiskFileStore`**: `<app_data_dir>/docs/<id>.pdf` and `docs/<id>/part-<n>.pdf`. Uploads are
  written to `<id>.pdf.partial`, chunk by chunk at an explicit offset, then renamed on confirm.
  Delete removes the file, the parts folder and any `.partial` file.
- `SystemClock` and `RandomIdGenerator` (ring `SystemRandom`).
- Port contract suites: one suite per port, run against an in-memory fake and the real adapter
  (SQLite in a temp dir).

## 5. Commands (`app/src-tauri/src/commands/`)

Each command does three things: check the gate (except `verify_otp`), parse the args, and call one
use case on `spawn_blocking`. It returns `Result<T, ErrorDto>`, where `ErrorDto` =
`{code, message, retryAfter?}`.

| Command | Web function in `api-desktop.js` |
|---|---|
| `verify_otp {code}` | `verifyCode` → `{token:"local", expiresAt}`. The gate unlocks in Rust memory; the token is cosmetic |
| `get_config` | `getConfig` |
| `list_docs {cursor}` / `create_doc {fileName,size}` / `upload_chunk` (raw body, headers `x-doc-id`, `x-offset`) / `confirm_doc {docId}` / `rename_doc` / `delete_doc` | `listDocs`, `createDoc`, `uploadFile`, `confirmDoc`, `renameDoc`, `deleteDoc` |
| `get_doc_link {docId, part}` | `getDocLink` → `{url:"kristoff://doc/…", expiresAt}` |
| projects ×4, chats ×5, `get_usage` | same names as `api.js` |

The gate is `AtomicBool` unlocked state in `Composition`, plus an expiry of 12 h (the same TTL as
the session token). It locks again on expiry and on app quit.

## 6. Web

- `web/services/api-desktop.js`:
  - Exports exactly the names `api.js` exports: `ApiError`, the 18 functions, `uploadFile`,
    `parseSse` (stub) and `askStream` (returns `AI_DISABLED` in this phase).
  - Maps `ErrorDto` to `ApiError` with the same fields.
  - `uploadFile` slices the `File` into 8 MiB `ArrayBuffer`s and calls `invoke('upload_chunk', buf,
    {headers})`, reporting progress after each chunk.
- `web/scripts/build.mjs --target desktop` writes `web/dist-desktop/` with `services/api.js`
  replaced by `api-desktop.js`, and with no `config.json`.
- `web/scripts/check.mjs` gains a parity check: the export names of the two modules must be equal.

## 7. `scripts/ci-local.sh`

Runs, in order:
1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic`
3. The 300-line cap per `.rs` file
4. `cargo test --workspace`
5. `cargo deny check licenses advisories`
6. `npm --prefix ../web run check`
7. The dry-gate (`~/Projects/.claude/skills/dk-coding/scripts/dry-gate.mjs` with a Rust format
   config, as DexFS does)

## Exit

- `cargo tauri dev` opens Kristoff. A live TOTP code unlocks it. Five wrong codes lock it with a
  countdown.
- Upload a 5 MB and a 120 MB PDF with progress. Rename, create a project, move a chat into it, pin
  it, and delete the doc. The doc disappears from disk and from the chat's attachments.
- `ci-local.sh` is green. The PR lists these guard → test pairs, each mutation-checked:
  - Gate locked → `UNAUTHENTICATED`
  - Replayed code → `OTP_REPLAYED`
  - Lockout
  - Project cap
  - Attachment rules
  - `%PDF-` check
  - Size mismatch
