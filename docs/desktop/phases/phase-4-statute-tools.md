# Phase 4: statute tools in the answer loop

Goal: Claude searches the law index when a question needs it, and one answer cites uploaded PDF
pages and statute articles together.

## 1. Tools (definitions in `kristoff-contracts`, one source)

| Tool | Input schema | Result |
|---|---|---|
| `search_ph_laws` | `query` (≤ 300), `law_number?` (≤ 40), `year_from?`, `year_to?` (1900–2100), `limit?` (1–8, default 8) | Up to 8 `search_result` blocks |
| `get_law_article` | `law_number` (≤ 40), `article` (≤ 40) | That article plus its neighbours as `search_result` blocks |

Each `search_result` block is shaped like this:

```
{type:"search_result", source:<source_url>, title:"<identifier> · <label>",
 content:[{type:"text", text}], citations:{enabled:true}}
```

The text starts with a status line when `law_status` is not in force, or when the row is truncated.

## 2. Loop (core use case, extends the ask flow)

- Send `tools` only when the chat's Statutes toggle is on (the default is on).
- Stream. On `stop_reason:"tool_use"`:
  1. Validate the tool input with the schema; a bad input becomes an error `tool_result`.
  2. Run the `LawIndex` call.
  3. Append the assistant turn and the `tool_result`, then stream again.
- Cap: 3 tool calls per question. Once the cap is reached, send `tool_choice:{type:"none"}`.
- Each turn's usage is summed into the stored message.
- Text deltas from every turn go to the same `AnswerSink`, so the UI sees one answer.
- Citation mapping, by location type:
  - `search_result_location` → `kind:"law"`, `docId:null`, `page:null`,
    `docTitle` = the result title, `sourceUrl` = the result source, `quote` capped at 4000.
  - `page_location` → as in Phase 2.
- If S5 was "no": the system prompt asks for `[[law: <identifier> · <label>]]` markers. A core
  parser turns them into law citations, but only for identifiers that a tool actually returned in
  this question. Others are dropped and logged.

## 3. System prompt additions (one constant)

Keep the existing prompt and add:
- Cite a law number and article for every statutory claim. With no citation, say "not found in the
  indexed laws".
- Say when a law is marked amended or repealed.
- Note that coverage is an interim snapshot (about 1,400 laws missing) and that the Official
  Gazette text prevails.
- Search at most 3 times.

## 4. UI (edit in place; UI is not versioned)

- `web/ui/citations.js`:
  - A `kind:"law"` card shows "RA 9262 · Sec. 5", the quote, and a status badge from the title's
    status line.
  - Clicking it opens `sourceUrl` through a Rust `open_external` command, which accepts only
    `https` URLs on hosts from the dataset's `source_url` allowlist (one config list).
- `web/ui/composer.js`: a "Statutes" toggle, saved per chat as a new `statutes: boolean` field on
  Chat (an additive contract change; default `true`).
- Desktop only: `getConfig` returns `features.statutes: true`, and the web build without it hides
  the toggle.

## Exit

- Attach a decision PDF and ask "Did the Court apply Sec. 5 of RA 9262 correctly?". The answer
  cites a page of the decision and RA 9262 Sec. 5, and both cards open correctly.
- With Statutes off, no tools are sent (asserted in a wire test).
- Wire tests use the S5 fixtures. They assert the tool definitions, the `tool_result` shape and
  the `tool_choice:none` after the cap.
- Guard → test pairs, each mutation-checked:
  - Max 3 tool calls
  - Tool input validation
  - The `open_external` host allowlist
  - Statutes off → no tools
