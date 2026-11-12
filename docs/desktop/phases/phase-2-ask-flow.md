# Phase 2: ask flow, PDFs to Claude, split, viewer

Goal: asking works end to end on uploaded PDFs, with streaming text, page citations, follow-ups
and usage. This matches the web app's behaviour.

## 1. Split (`LopdfSplitter`, adapter; plan rules in core)

- Port `domain/docs/plan-parts/v1.ts` to a pure core function:
  - ≤ 100 pages and ≤ 30 MiB → no parts.
  - Otherwise cut 100-page ranges, halve any range over 31,457,280 bytes, at most 200 parts.
  - A single page over the limit → `UPLOAD_INVALID`.
- The adapter loads the PDF with lopdf, counts pages, and writes each part with only its pages
  kept. Measure each part's real size and re-plan if it is too big.
- `confirm_doc` sets `processing` and starts a background tokio task:
  split → Files API upload of the whole doc or each part → `ready`. Any failure → `failed`.
  - On app start, a doc left in `processing` is retried once, then marked `failed`.
  - The web keeps polling `list_docs` every 3 s (`web/ui/data.js`), with no change.

## 2. Claude adapters (reqwest `stream` + rustls; one client per app run)

- **`AnthropicFiles`**: `upload(path) -> fileId`, `delete(fileId)`. `fileId` is stored per doc and
  per part. Doc delete also deletes the remote files, best effort, and logs failures.
  - If S4 was "no": skip this adapter. `AnthropicModel` sends base64 sources read from disk, with
    the same `cache_control`.
- **`AnthropicModel`**: `POST /v1/messages` with `stream: true`.
  - Headers: `x-api-key` from the compile-time env and `anthropic-version: 2026-10-21`. No beta header: the Files API is GA (D-11).
    Verify each header value against the current docs while building.
  - Timeouts: connect 10 s, whole request 120 s (as `ANTHROPIC_TIMEOUT_MS`), one retry on
    408/429/5xx before the stream opens. Honour `retry-after`.
  - Request body, ported from `adapters/anthropic-answer-model/v1.ts:34-62` and
    `domain/chats/build-model-turns/v1.ts`:
    - `system` = the system prompt, moved to `kristoff-contracts` as one constant (the single source).
    - Documents only in the first user turn, `cache_control: {type:"ephemeral"}` on the last
      document, and part titles `"<title> (pp. a–b)"`.
    - History is the last 20 turns. Merge same-role turns, and use `"(no answer)"` for empty text.
  - The SSE parser is about 60 lines and hand-written. It splits on blank lines and reads `event:`
    and `data:`. It emits `TextDelta`, `CitationDelta`, `MessageDelta(usage, stop_reason)` and
    `Error`. It is tested against the recorded `.sse` fixtures from S4.
  - Citation mapping is ported from `domain/chats/map-citation/v1.ts`: page plus the part offset,
    title capped at 200, quote capped at 4000, `kind:"doc"`, `sourceUrl:null`.
- **`AnthropicFollowUps`**: non-streaming Haiku call, `max_tokens 400`, 15 s timeout, JSON-schema
  output, the same prompt as `model-config.ts:28-31`. Trust the output only on `end_turn`, then
  validate: ≤ 5 items, each 1–120 chars. Any failure → `[]`.
- Model ids and labels come from one config constant: Sonnet 5.5 (default), Opus 5.5, Haiku 5.5.
  `MAX_TOKENS` is 4096 and the daily cap is 100. They are read only in the composition root.

## 3. Ask use case (core)

Port the order in `usecases/ask-question/v1.ts:303-336` and drop the kill switch:

1. Check the model is in the allowlist.
2. Load the chat.
3. Run the daily meter.
4. Load history.
5. Resolve attachments to `fileId`s.
6. Store the user message. Auto-title the chat at 60 chars.
7. Stream.
8. Get follow-ups.
9. Always store the assistant message: use `FAILED_NOTE` / `CUT_OFF_NOTE` and `stopReason`, and
   append ` [n]` per citation.
10. Add usage to the day and total counters.

The use case talks to the outside through an `AnswerSink` port (`text`, `citation`, `followups`,
`done`, `error`).

## 4. Command and web

- `ask {chatId, text, model, onEvent: Channel<AskEvent>}`. `AskEvent` is a tagged enum whose
  variants match the SSE events: `text`, `citation`, `followups`, `done {message, usage}` and
  `error {code, message}`.
- `cancel_ask {chatId}` aborts the reqwest stream through a per-chat `CancellationToken` (tokio
  `sync`). The partial answer is stored with `stopReason:"error"`. Window close cancels every open ask.
- `api-desktop.js` `askStream` keeps the signature from `api.js:220`: it opens a `Channel`, calls
  the callbacks, maps `signal` abort to `cancel_ask`, and rejects with `ApiError` on `error`.
- PDF viewer, as decided by S3:
  - Register the `kristoff://doc/<id>[/part-<n>]` scheme, served from `DiskFileStore`. It checks
    the gate and answers `DOC_NOT_FOUND` for a missing doc.
  - New-window handling opens a `WebviewWindow` titled after the doc.

## Exit

- Upload a 150-page, 60 MB decision. It splits into 2 parts and becomes `ready`. Ask "Digest this
  case". The answer streams, Sources cards show page numbers, and clicking one opens the viewer at
  that page. Follow-ups appear. Usage goes up.
- Close the window mid-answer and reopen. The thread shows the partial answer with the failed note.
- Wire tests assert the exact request JSON against `contracts/fixtures/anthropic-request.json`
  (updated for file sources) and the follow-ups fixture.
- Guard → test pairs, each mutation-checked:
  - Daily cap
  - Model allowlist
  - Attachment not ready
  - `end_turn`-only follow-ups
  - Cut-off note
