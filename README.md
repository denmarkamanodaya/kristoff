# Kristoff

A local legal research assistant. Upload decisions, statutes or law review articles as PDFs, attach them to a chat, and ask Claude about them. Answers cite the exact passage and page.

Everything runs on your machine. Documents and chats are stored in `./data`. The only outside call is to the Claude API.

## Setup

Requires Node.js 18+.

```bash
cd kristoff
npm install
cp .env.example .env      # then paste your API key into .env
npm start
```

Open http://localhost:3333

## Using it

1. Click **Attach** in the search box to upload PDFs (up to 32 MB each) or pick from your Library. Scanned PDFs work too.
2. Ask a question, or pick a suggestion (case digest, summary, bar-style practice).
3. Each answer shows **Sources** cards with the quoted passage. Click a card or a numbered pill to open the PDF at that page.
4. Ask follow-ups in the box at the bottom. Past threads are in the sidebar; all documents are under **Library**.

## Cost control

- Set a monthly spend limit in the Claude Console (Settings → Limits).
- Attached PDFs are sent with every question but are **prompt-cached**, so follow-up questions within a few minutes are much cheaper.
- Attach only the documents a chat needs. Each one adds tokens to every question.
- Token totals appear at the bottom of the sidebar.
- Default model is `claude-sonnet-5-5`. Change `KRISTOFF_MODEL` in `.env` to switch.

## Settings (.env)

| Variable | Default | Notes |
|---|---|---|
| `ANTHROPIC_API_KEY` | — | Required |
| `KRISTOFF_MODEL` | `claude-sonnet-5-5` | Any Claude model ID |
| `KRISTOFF_MAX_TOKENS` | `4096` | Max answer length |
| `PORT` | `3333` | |
| `HOST` | `127.0.0.1` | `0.0.0.0` to allow other devices on your network |

## Project layout

```
server.js         Express server, Claude API calls, JSON storage
public/           Frontend (index.html, app.js, style.css)
data/             Created on first run: docs/*.pdf, chats/*.json, db.json
```

## Limits

- Very long PDFs (hundreds of pages) or many attached at once may exceed the model's context or page limit. Split them or attach fewer.
- The AI can still be wrong. Verify important citations against the SC E-Library, Lawphil or the Official Gazette.
