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
`WEB_ORIGINS` (comma list), `MODEL_ID`, `MAX_TOKENS`, `MAX_ATTACHED_DOCS` (5), `MAX_HISTORY_TURNS` (20),
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
- Model id and `MAX_TOKENS` live in Lambda env (`MODEL_ID`, `MAX_TOKENS`, set in tfvars); `model-config.ts` holds the
  system prompt and retry count only.
- Function-URL CORS is owned by Terraform (the function URL config); the `ask-stream` handler sets no CORS headers.
- Presigned upload PUT: 300 s, signs `Content-Type: application/pdf` and the content length.
- `GET /v1/chats/{id}` returns the newest 200 messages, oldest first.
- Message items are capped at 390 KB by the adapter (DynamoDB 400 KB limit); a larger answer fails with `INTERNAL`.
- Follow-ups (not in v1): a `CONFLICT` (409) wire code for optimistic-lock misses (today `INTERNAL`); lower
  per-message bounds.
- The HTTP API authorizer's own denials (401 `{"message":"Unauthorized"}` with no token, 403 `{"message":"Forbidden"}`
  for a bad or expired token) carry no envelope; HTTP APIs cannot reshape them. Clients treat a 401/403 without an
  envelope as `UNAUTHENTICATED`.
