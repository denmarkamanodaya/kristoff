# Kristoff contract (v1)

The single source for every route, wire shape, error code, storage key, env key and SSM name. Code imports or is
tested against this document's fixtures in `contracts/`; nothing restates it by hand (`core/dry.md`).
Plan: `~/.claude/plans/hey-claude-we-have-gentle-ripple.md`. Stage: `prod` only. Region: `ap-southeast-2`.

## Principal

Single owner. The session token's `sub` is the constant owner id `kristoff` (from config `OWNER_ID`, not a literal
in code). Every storage key is scoped by it.

## Auth flow

1. Page load: the web shows the TOTP prompt. No token is persisted (memory only); a reload asks again.
2. `POST /v1/auth/verify` `{ "code": "123456" }` → `200 { "token": "<session>", "expiresAt": "<ISO instant>" }`.
3. Every other request sends `Authorization: Bearer <session>`.
   - HTTP API: a Lambda REQUEST authorizer (simple response) verifies the token, passes `sub` in the context.
   - Stream URL: the `ask` Lambda verifies the same token with the same use case (`verify-session`).
4. Session token: `base64url(payloadJson) + "." + base64url(HMAC-SHA256(key, payloadPart))`,
   payload `{ "sub": string, "iat": epochSeconds, "exp": epochSeconds }`, lifetime `SESSION_TTL_SECONDS` (43200).
   Compare the MAC in constant time.
5. TOTP: RFC 6238, SHA-1, 6 digits, 30 s step, window ±1 step. Replay guard: conditional update
   `lastStep < :step`. Lockout: 5 consecutive failures set `lockedUntil = now + 15 min`; success resets the count.

## Routes (HTTP API, base `API_BASE_URL`)

All bodies JSON. All strings bounded. Ids are 16 lowercase hex chars (`^[a-f0-9]{16}$`) unless stated.

| Method + path | Auth | Request | 200 response |
|---|---|---|---|
| `POST /v1/auth/verify` | none | `{ code: /^\d{6}$/ }` | `{ token, expiresAt }` |
| `GET /v1/config` | bearer | — | `{ model, maxAttachedDocs, dailyQuestionCap }` |
| `GET /v1/docs` | bearer | `?cursor` (opaque, ≤ 512) | `{ items: Doc[], nextCursor: string \| null }` (page 100) |
| `POST /v1/docs` | bearer | `{ fileName: string ≤ 255, size: int 1..33554432 }` | `{ doc: Doc, uploadUrl, uploadHeaders: { "Content-Type": "application/pdf" } }` (doc status `pending`) |
| `POST /v1/docs/{id}/confirm` | bearer | — | `{ doc: Doc }` status `ready`; checks object exists, size matches, first 5 bytes `%PDF-` |
| `PATCH /v1/docs/{id}` | bearer | `{ title: string 1..200 }` | `{ doc: Doc }` |
| `GET /v1/docs/{id}/link` | bearer | — | `{ url, expiresAt }` presigned GET, 300 s, `ResponseContentType=application/pdf` |
| `GET /v1/chats` | bearer | `?cursor` | `{ items: ChatSummary[], nextCursor }` newest `updatedAt` first, page 50 |
| `POST /v1/chats` | bearer | `{ docIds?: Id[] ≤ maxAttachedDocs }` | `{ chat: Chat }` |
| `GET /v1/chats/{id}` | bearer | — | `{ chat: Chat, messages: Message[] }` (oldest first, ≤ 200) |
| `PATCH /v1/chats/{id}` | bearer | `{ title?: string 1..120, docIds?: Id[] ≤ maxAttachedDocs }` (≥ 1 field) | `{ chat: Chat }`; unknown or non-ready docIds → `DOC_NOT_FOUND` |
| `GET /v1/usage` | bearer | — | `{ today: Usage, total: Usage }` |

### Stream (Lambda function URL, `STREAM_URL`, `RESPONSE_STREAM`)

`POST {STREAM_URL}/v1/chats/{id}/ask` bearer, `{ text: string 1..8000 }`. Response `text/event-stream`:

```
event: text      data: { "text": "..." }
event: citation  data: Citation
event: done      data: { "message": Message, "usage": Usage }
event: error     data: { "error": { "code", "message", "requestId" } }
```
Pre-stream failures (auth, validation, cap, kill switch) return the JSON error envelope with a status instead.
The user message and the assistant message are both persisted, also when the stream fails midway (assistant text
then ends with a note and `stopReason: "error"`).

## Shapes

```
Doc         { id, title ≤ 200, size, status: "pending"|"ready", createdAt: ISO }
ChatSummary { id, title ≤ 120, docIds: Id[], updatedAt: ISO }
Chat        ChatSummary + { createdAt: ISO }
Message     { id: ULID (26 chars), role: "user"|"assistant", text ≤ 200000, citations: Citation[] ≤ 500,
              usage: Usage | null, model: string | null, stopReason: string | null, at: ISO }
Citation    { n: int ≥ 1, docId: Id | null, docTitle ≤ 200, page: int | null, endPage: int | null, quote ≤ 4000 }
Usage       { input, output, cacheRead, cacheWrite }   (non-negative ints)
```
Optional values are `null`, never omitted. Instants are UTC ISO-8601 with `Z` and milliseconds.

## Error envelope and codes

`{ "error": { "code": "UPPER_SNAKE", "message": "human text", "requestId": "..." } }`

| Code | Status | Client action |
|---|---|---|
| `VALIDATION_FAILED` | 400 | fix the request |
| `OTP_INVALID` | 401 | tell the user, retry with a new code |
| `OTP_REPLAYED` | 401 | wait for the next code |
| `OTP_LOCKED` | 429 + `Retry-After` | tell the user to wait |
| `UNAUTHENTICATED` | 401 | show the TOTP prompt |
| `DOC_NOT_FOUND` | 404 | resync the library |
| `CHAT_NOT_FOUND` | 404 | resync the chat list |
| `UPLOAD_INVALID` | 422 | tell the user (not a PDF, or size mismatch) |
| `DAILY_CAP_REACHED` | 429 + `Retry-After` | tell the user, try tomorrow |
| `AI_DISABLED` | 503 + `Retry-After` | tell the user |
| `UPSTREAM_UNAVAILABLE` | 503 | retry later |
| `INTERNAL` | 500 | retry later |

## Storage

DynamoDB table `kristoff-prod-core`, keys `PK`/`SK` (strings), GSI `GSI1` on `GSI1PK`/`GSI1SK`. Built only through
`backend/src/domain/keys`.

| Item | PK | SK | GSI1 | Attributes |
|---|---|---|---|---|
| Doc | `OWNER#<uid>` | `DOC#<id>` | — | title, size, status, s3Key, createdAt |
| Chat | `OWNER#<uid>` | `CHAT#<id>` | `GSI1PK=OWNER#<uid>#CHATS`, `GSI1SK=<updatedAt ISO>#<id>` | title, docIds, createdAt, updatedAt |
| Message | `CHAT#<id>` | `MSG#<ulid>` | — | the Message fields |
| Usage day | `OWNER#<uid>` | `USAGE#<yyyy-mm-dd UTC>` | — | questions, input, output, cacheRead, cacheWrite |
| Usage total | `OWNER#<uid>` | `USAGE#TOTAL` | — | same counters |
| TOTP state | `AUTH#<uid>` | `TOTP` | — | lastStep, failCount, lockedUntil |

Every item carries `schemaVersion: 1` and `version` (int, optimistic lock on updates).
Daily cap: one conditional `UpdateItem ADD questions :1` with `questions < :cap` (or not exists) **before** the model
call. Tokens are added after the call.

S3 bucket `kristoff-prod-docs-<account_id>`: key `docs/<uid>/<docId>.pdf`, server-built only. Versioned, SSE-S3,
Block Public Access, no expiry; CORS `PUT`/`GET` from the web origins only.

## Config

Lambda env (Zod in `backend/src/composition/env.ts`): `STAGE`, `OWNER_ID`, `TABLE_NAME`, `DOCS_BUCKET`,
`WEB_ORIGINS` (comma list), `MODEL_IDS`, `MAX_TOKENS`, `MAX_ATTACHED_DOCS` (5), `MAX_HISTORY_TURNS` (20),
`DAILY_QUESTION_CAP`, `SESSION_TTL_SECONDS`, `ANTHROPIC_TIMEOUT_MS`, `SSM_ANTHROPIC_KEY`, `SSM_TOTP_SECRET`,
`SSM_SESSION_KEY`, `SSM_AI_ENABLED`.

SSM parameters (names passed by Terraform):
`/kristoff/prod/anthropic-api-key` (SecureString), `/kristoff/prod/totp-secret` (SecureString, base32),
`/kristoff/prod/session-hmac-key` (SecureString, ≥ 32 random bytes base64), `/kristoff/prod/ai-enabled`
(String `true|false`, kill switch, read with a 60 s cache).

Web build config `web/config.json` (generated at deploy from Terraform outputs): `{ "apiBaseUrl", "streamUrl" }`.

## Model call

Model id, system prompt and inference parameters live in `backend/src/contracts/model-config.ts` (prompt text copied
from the original `server.js` SYSTEM_PROMPT). Attached ready docs go as `document` blocks with
`source: { type: "url", url: <presigned GET, 900 s> }`, `title`, `citations: { enabled: true }`, `cache_control`
ephemeral on the last doc, in the first user turn. History: last `MAX_HISTORY_TURNS` messages as text. Never log
prompts or outputs.

## Build artefacts (backend ↔ Terraform seam)

`cd backend && npm run build` writes one zip per Lambda: `backend/dist/<name>.zip`, containing `index.mjs`
exporting `handler`; runtime `nodejs24.x`, arch `arm64`. Names and routes:

| Lambda name | Trigger |
|---|---|
| `authorizer` | HTTP API REQUEST authorizer (simple responses, identity source `$request.header.Authorization`, cache 0) |
| `auth-verify` | `POST /v1/auth/verify` (no authorizer) |
| `config-get` | `GET /v1/config` |
| `docs-list` | `GET /v1/docs` |
| `docs-create` | `POST /v1/docs` |
| `docs-confirm` | `POST /v1/docs/{id}/confirm` |
| `docs-rename` | `PATCH /v1/docs/{id}` |
| `docs-link` | `GET /v1/docs/{id}/link` |
| `chats-list` | `GET /v1/chats` |
| `chats-create` | `POST /v1/chats` |
| `chats-get` | `GET /v1/chats/{id}` |
| `chats-update` | `PATCH /v1/chats/{id}` |
| `usage-get` | `GET /v1/usage` |
| `ask-stream` | Lambda function URL, auth NONE (token checked in code), `invoke_mode = RESPONSE_STREAM`, timeout 300 s, memory 512 MB |

Every other Lambda: timeout 10 s, memory 256 MB. HTTP API stage throttling: burst 20, rate 10.
IAM per Lambda: only the table/index actions, bucket prefix and SSM parameters it uses (env-scope list in
`backend/lambda-scope.json`, written by the backend, read by Terraform).

## Clarifications (2026-11-12, from the web smoke)

- CORS (HTTP API, function URL): allow headers `authorization`, `content-type`; expose `retry-after`. S3 bucket CORS:
  `PUT` with `Content-Type` from the web origins, `GET` for presigned links.
- Persisted assistant `Message.text` contains the ` [n]` citation markers appended where each citation arrived (as the
  original `server.js` did); `n` matches `Citation.n`.
- `ask` sets the chat title to the first 60 characters of the question when the chat has no messages yet.
- `GET /v1/docs` returns newest first (`createdAt` descending, tie-break on `id`).
- Hosting (owner ruling 2026-11-12): no custom domain. The web is served only at the Amplify default domain
  `https://main.<app_id>.amplifyapp.com`; that exact origin is the only CORS origin (`WEB_ORIGINS`).
- `MAX_TOKENS` and the model allowlist live in Lambda env (`MAX_TOKENS`, `MODEL_IDS`, see "Model choice"); `model-config.ts` holds the
  system prompt, retry count and model labels only.
- Function-URL CORS is owned by Terraform (the function URL config); the `ask-stream` handler sets no CORS headers.
- Presigned upload PUT: 300 s, signs `Content-Type: application/pdf` and the content length.
- `GET /v1/chats/{id}` returns the newest 200 messages, oldest first.
- Message items are capped at 390 KB by the adapter (DynamoDB 400 KB limit); a larger answer fails with `INTERNAL`.
- Follow-ups (not in v1): a `CONFLICT` (409) wire code for optimistic-lock misses (today `INTERNAL`); lower
  per-message bounds.
- The HTTP API authorizer's own denials (401 `{"message":"Unauthorized"}` with no token, 403 `{"message":"Forbidden"}`
  for a bad or expired token) carry no envelope; HTTP APIs cannot reshape them. Clients treat a 401/403 without an
  envelope as `UNAUTHENTICATED`.

## Model choice (2026-11-12, owner request)

- Lambda env `MODEL_IDS`: comma list of allowed model ids, the FIRST is the default
  (prod: `claude-sonnet-5-5,claude-opus-5-5,claude-haiku-5-5`). Replaces `MODEL_ID`. Labels live in
  `backend/src/contracts/model-config.ts` keyed by id (`Sonnet 5.5`, `Opus 5.5`, `Haiku 5.5`); an id without a
  label shows the id.
- `GET /v1/config` → `{ model: <default id>, models: [{ id, label }] (1..5, allowlist order), maxAttachedDocs,
  dailyQuestionCap }`.
- `POST {STREAM_URL}/v1/chats/{id}/ask` body `{ text: 1..8000, model?: string ≤ 64 }`. Omitted → default. Not in
  `MODEL_IDS` → `VALIDATION_FAILED` (400) before the meter. The used id is stored in `Message.model` of the
  assistant message.
- The web shows a select in the composer (where the model label was), default = `config.model`, choice kept in
  memory only for the page session, sent with every ask.

## Thread deletion + dark mode (2026-11-12, owner request)

- `DELETE /v1/chats/{id}` (bearer) → `200 { "deleted": true }`; unknown id → `CHAT_NOT_FOUND`. Deletes the chat item
  and every `MSG#` item under `CHAT#<id>` (paginated Query, then `DeleteItem` per message with bounded concurrency 8;
  `BatchWriteItem` is outside the Lambda boundary's action ceiling, so it is not used). Messages are deleted first, the chat item last, so a failure midway leaves the thread
  visible and the delete can be repeated (idempotent; a repeat after success returns `CHAT_NOT_FOUND`). Attached
  documents are NOT deleted (library stays). Usage counters are untouched. Recovery window = DynamoDB PITR (35 d).
  New Lambda `chats-delete` (timeout 30 s, memory 256 MB); route `DELETE /v1/chats/{id}`; CORS methods add `DELETE`.
- Dark mode is web-only: a theme toggle (Light / Dark), default Light. The choice is a per-viewer convenience kept in
  `localStorage` key `kristoff.theme` (values `light|dark`), read/written only by `web/services/prefs.js`, wrapped in
  try/catch (falls back to Light). Dark mode redefines the colour tokens under `:root[data-theme="dark"]`; no
  selector duplication. Session tokens are still never stored.

## v2 (2026-11-12, owner request): projects, pin, file delete, follow-ups, timing, large-PDF parts

### Chats: pin + project
- `ChatSummary`/`Chat` gain `pinned: boolean` (default false) and `projectId: Id | null`.
- `PATCH /v1/chats/{id}` accepts `pinned?: boolean`, `projectId?: Id | null` (with title/docIds; ≥ 1 field). Unknown
  project → `PROJECT_NOT_FOUND` (404, resync).
- `GET /v1/chats` order unchanged (newest first); the web groups Pinned / Projects / Recents.

### Projects
`Project { id, name 1..80, createdAt, updatedAt }`. Cap 50 per owner (`PROJECT_LIMIT_REACHED`, 409).
| Method + path | Request | 200 |
|---|---|---|
| `GET /v1/projects` | — | `{ items: Project[] }` (name order, ≤ 50) |
| `POST /v1/projects` | `{ name }` | `{ project }` |
| `PATCH /v1/projects/{id}` | `{ name }` | `{ project }` |
| `DELETE /v1/projects/{id}` | — | `{ deleted: true }` — its chats get `projectId = null` first (GSI1 query of the owner's chats, filter in code, conditional UpdateItem each), then the project item is deleted |
Key `OWNER#<uid>` / `PROJECT#<id>`. Lambdas `projects-list|create|rename|delete`.

### File delete (real)
- `DELETE /v1/docs/{id}` → `{ deleted: true }`: removes the docId (and its part ids) from every owner chat's `docIds`,
  deletes **every S3 version** of the doc's objects (original + parts; `ListObjectVersions` on the prefix, then
  `DeleteObject` with `VersionId`), then the doc item. Old citations keep `docTitle`; their link → `DOC_NOT_FOUND`
  (web shows "file deleted"). Lambda `docs-delete` (timeout 60 s). Needs `s3:ListBucketVersions` on the bucket
  (prefix-conditioned) and `s3:DeleteObject`, `s3:DeleteObjectVersion` on `docs/<uid>/*` — boundary change.

### Answer extras
- `Message.durationMs: int | null` (assistant: ms from request start to final event).
- `Message.followUps: string[]` (0..5, each 1..120). After the main answer, one Haiku call (`FOLLOWUP_MODEL_ID` env,
  prod `claude-haiku-5-5`, `max_tokens` 400, timeout 15 s) with the question + first 2,000 chars of the answer,
  structured JSON output validated by Zod. New SSE event `followups` `{ followUps }` sent before `done`. Any
  failure → `[]`, logged as `warn`, never an error event. Not counted in the daily cap; its tokens are added to usage.

### Large PDFs (Phase 6)
- Upload limit becomes 209,715,200 bytes (200 MB). `POST /v1/docs` size cap updated.
- `confirm`: a PDF > 30 MB or > 100 pages gets `status: "processing"` and an async invoke of Lambda `docs-split`
  (memory 3008 MB, timeout 300 s, `pdf-lib` 1.17.1). It writes parts `docs/<uid>/<docId>/part-<n>.pdf` of ≤ 100 pages
  and ≤ 30 MB each, then sets `status: "ready"` and `parts: [{ n, startPage, endPage, size }]`. Failure →
  `status: "failed"` (web offers delete). Small PDFs stay as today (`parts: []`).
- `Doc` gains `pageCount: int | null`, `parts: Part[]`, status `pending|processing|ready|failed`.
- Attachment ids in `docIds`: `<docId>` (whole doc; allowed only when `parts` is empty) or `<docId>:p<n>` (one part).
  Regex `^[a-f0-9]{16}(:p[0-9]{1,3})?$`. The ask Lambda sends each attached part as its own document block titled
  `"<title> (pp. a–b)"`; citation pages are mapped back to original page numbers (`startPage + page − 1`).
- `GET /v1/docs/{id}/link?part=n` (optional) links a part; the web opens the original-page link via the part that
  holds that page.

## Desktop (2026-11-12, Kristoff Desktop Phase 1 §6)

Plan: `docs/desktop/plan.md`, `docs/desktop/phases/` (decisions D-8, D-9, D-10).

### Citation: `kind` and `sourceUrl` (additive)

```
Citation    + { kind: "doc"|"law", sourceUrl: string | null }
```
- `kind: "doc"` cites an attached PDF (every field as before). `kind: "law"` cites a statute section
  (Phase 4): `docId`, `page` and `endPage` are `null`, `docTitle` is the section title (e.g. "RA 9262 · Sec. 5"),
  `quote` is the cited text, and `sourceUrl` is an `https:` link to the official source.
- Writers always send both fields (`kind: "doc"`, `sourceUrl: null` for doc citations). Readers treat a citation
  stored before this change, with neither field, as `kind: "doc"`, `sourceUrl: null`.
- The web opens `sourceUrl` with `window.open`; the desktop app opens it through `open_external`, which accepts only
  `https:` URLs on allowlisted hosts. The web renders a law citation whose `sourceUrl` is not `https:` as plain text.

### Law status line (Phase 4)

A `kind: "law"` citation's `quote` may begin with exactly one status label when the cited provision is not in
force as written: `[<Status>]` followed by a space or newline, then the quote text. `<Status>` is 1..40 characters of
letters, spaces and hyphens, starting with a letter (regex `^\s*\[([A-Za-z][A-Za-z -]{0,39})\]\s*`). The server uses
these values: `Repealed`, `Amended`, `Partially repealed`, `Declared unconstitutional`; clients show any label that
matches the regex. A provision in force has no label. Clients strip the label from the shown quote and show it as a
badge. `doc` citations never carry a label (the web shows a doc quote unchanged even when it starts with `[`).

### Chat: `statutes` and Config: `features` (Phase 4, desktop)

```
Chat        + { statutes: boolean }          (absent → true)
Config      + { features: { statutes: boolean, lawCount?: int ≥ 0, lawReady: boolean } }   (absent → no statute search; the web build has none)
```
- `statutes` turns statute search (the law tools) on or off for that chat's future questions. Readers treat a chat
  without the field as `true`.
- It is set only through `update_chat {chatId, patch: {statutes}}` (desktop). `create_chat` is unchanged: the web
  creates the chat, then sends `update_chat` with `statutes: false` before the first `ask` when the user switched
  it off on the home screen. The HTTP `PATCH /v1/chats/{id}` does not accept `statutes`; the web sends it only when
  `config.features.statutes` is true.
- `get_config` returns `features: { statutes: true, lawCount }` on desktop. The web shows the Statutes toggle only then.
- `lawReady` is true once the law index has warmed (vectors and model loaded), and true at once when there is no
  index, so a client never waits forever. The shell also emits the Tauri event `law-ready` (no payload) at that
  moment. The desktop web shows a launch screen until then (event, or `get_config` every 500 ms, at most 60 s).
- `lawCount` is the number of laws in the bundled statute index, read from the `phlaws.db` meta table (the one
  source). The web renders "Searches {lawCount, locale-formatted} Republic Acts and the 1987 Constitution on this
  Mac" next to the toggle, and hides that note (keeping the toggle) when `lawCount` is absent.

### Desktop IPC

The desktop app has no HTTP API: `web/services/api-desktop.js` (served as `services/api.js` in `web/dist-desktop/`)
calls Tauri commands with `window.__TAURI__.core.invoke`. Until the AWS stack is retired, the HTTP routes above
remain the source for every request and response shape; each command returns exactly the 200 body of the route
it replaces.

- Arguments are camelCase. Absent optionals are `null`, never omitted.
- A failed command rejects with `ErrorDto { code, message, retryAfter: int | null, requestId?: string }`, using the
  codes in "Error envelope and codes". The web maps it to `ApiError` (`retryAfter` in seconds). A non-object
  rejection (a Tauri failure such as an unknown command) maps to `INTERNAL`.
- The desktop app has no authentication (decision D-17): no gate, no `verify_otp`, and no command returns
  `UNAUTHENTICATED` or `OTP_*`. Those codes stay in the shared code list for the web backend. `api-desktop.js`
  exports `requiresUnlock = false` and a `verifyCode` stub that rejects without calling the shell, so its exports
  still equal `api.js`'s.

| Command | Request | Response | Errors (besides `INTERNAL`) | Replaces |
|---|---|---|---|---|
| `get_config` | `{}` | `{ model, models, maxAttachedDocs, dailyQuestionCap }` | — | `GET /v1/config` |
| `list_docs` | `{ cursor: string \| null }` | `{ items: Doc[], nextCursor }` | `VALIDATION_FAILED` | `GET /v1/docs` |
| `create_doc` | `{ fileName, size }` | `{ doc, uploadUrl: "kristoff-upload://<docId>", uploadHeaders }` | `VALIDATION_FAILED` | `POST /v1/docs` |
| `upload_chunk` | raw body: one `ArrayBuffer` (≤ 8 MiB); headers `x-doc-id: <docId>`, `x-offset: <byte offset, decimal>` | `{ written }` | `VALIDATION_FAILED`, `DOC_NOT_FOUND`, `UPLOAD_INVALID` | the S3 PUT |
| `confirm_doc` | `{ docId }` | `{ doc }` | `DOC_NOT_FOUND`, `UPLOAD_INVALID` | `POST /v1/docs/{id}/confirm` |
| `rename_doc` | `{ docId, title }` | `{ doc }` | `VALIDATION_FAILED`, `DOC_NOT_FOUND` | `PATCH /v1/docs/{id}` |
| `get_doc_link` | `{ docId, part: int \| null }` | `{ url: "kristoff://doc/<id>[/part-<n>]", expiresAt }` | `DOC_NOT_FOUND` | `GET /v1/docs/{id}/link` |
| `open_doc` | `{ docId, part: int \| null, page: int \| null }` (`page` is 1-based inside that file or part) | `null`; the shell opens its PDF viewer window at that page (D-12: WKWebView refuses `window.open`) | `DOC_NOT_FOUND` | (web: `window.open` + `get_doc_link`) |
| `delete_doc` | `{ docId }` | `{ deleted: true }` | `DOC_NOT_FOUND` | `DELETE /v1/docs/{id}` |
| `list_projects` | `{}` | `{ items: Project[] }` | — | `GET /v1/projects` |
| `create_project` | `{ name }` | `{ project }` | `VALIDATION_FAILED` | `POST /v1/projects` |
| `rename_project` | `{ projectId, name }` | `{ project }` | `VALIDATION_FAILED`, `PROJECT_NOT_FOUND` | `PATCH /v1/projects/{id}` |
| `delete_project` | `{ projectId }` | `{ deleted: true }` | `PROJECT_NOT_FOUND` | `DELETE /v1/projects/{id}` |
| `list_chats` | `{ cursor: string \| null }` | `{ items: ChatSummary[], nextCursor }` | `VALIDATION_FAILED` | `GET /v1/chats` |
| `create_chat` | `{ docIds }` | `{ chat }` | `VALIDATION_FAILED`, `DOC_NOT_FOUND` | `POST /v1/chats` |
| `get_chat` | `{ chatId }` | `{ chat, messages }` | `CHAT_NOT_FOUND` | `GET /v1/chats/{id}` |
| `update_chat` | `{ chatId, patch: { title?, docIds?, pinned?, projectId? } }` | `{ chat }` | `VALIDATION_FAILED`, `CHAT_NOT_FOUND`, `DOC_NOT_FOUND`, `PROJECT_NOT_FOUND` | `PATCH /v1/chats/{id}` |
| `delete_chat` | `{ chatId }` | `{ deleted: true }` | `CHAT_NOT_FOUND` | `DELETE /v1/chats/{id}` |
| `get_usage` | `{}` | `{ today: Usage, total: Usage }` | — | `GET /v1/usage` |
| `ask` | `{ chatId, text, model: string \| null, onEvent: Channel<AskEvent> }` | `null`, after the last event is sent | pre-stream: `VALIDATION_FAILED`, `CHAT_NOT_FOUND`, `DOC_NOT_FOUND`, `DAILY_CAP_REACHED`, `AI_DISABLED` | `POST {STREAM_URL}/v1/chats/{id}/ask` |
| `cancel_ask` | `{ chatId }` | `null` | — | (client abort) |
| `open_external` | `{ url }` | `null` | `VALIDATION_FAILED` (not `https:` or host not allowlisted) | (`window.open`) |

`AskEvent` is `{ event, data }`, where `event` and `data` are exactly the SSE event name and its JSON data from
"Stream": `text { text }`, `citation Citation`, `followups { followUps }`, `done { message, usage }`,
`error { error: { code, message, requestId } }`, and (Phase 4, additive) `search { query, count }`. `done` or
`error` is the last event.

`search { query: string ≤ 500, count: int ≥ 0 }` is sent once per statute search, before the text that uses it:
`query` is the search string the model sent, `count` the number of articles returned. It is display-only and not
persisted on the Message. Builds without statute search never send it; clients ignore unknown events. The web's
`askStream` takes an optional `onSearch({ query, count })` callback on both transports (the HTTP stream may carry the
same event; the AWS backend does not send it). `model: null` means the
default model. Channel messages are not ordered against the command's reply, so the web waits 2 s after `ask`
resolves for a final event before it reports `UPSTREAM_UNAVAILABLE`. On `cancel_ask` the shell stops the stream and
stores the partial answer with `stopReason: "error"`; it sends no further events.
