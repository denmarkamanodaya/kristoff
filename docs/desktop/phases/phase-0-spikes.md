# Phase 0: spikes

Goal: prove the six risky assumptions before writing production code. Each spike gets a few hours
at most, lives on a throwaway `spike/desktop-<n>` branch, and is deleted afterwards. Only the
results and any golden files stay; they go into `README.md` → Spike results.

Prerequisites:
- Rust 1.99.1 from rustup (installed). `cargo-tauri` is in `~/.cargo/bin`.
- A dev Claude API key in the env as `KRISTOFF_ANTHROPIC_KEY`, used only for S4 and S5.
- Use a scratch app made with `cargo tauri init` in `/private/tmp/...` or on the spike branch.
  Never in `desktop/`.

## S1. Unsigned app runs on another account

1. Build a hello-world Tauri app with `bundle.macOS.signingIdentity: "-"` and `hardenedRuntime: false`.
2. Run `cargo tauri build --bundles app,dmg --target aarch64-apple-darwin`.
3. Copy the DMG to a second macOS user account. Add quarantine the way a download would:
   `xattr -w com.apple.quarantine "0081;$(printf %x $(date +%s));Safari;" Kristoff.dmg`.
4. Install it. Record which path opens it on macOS 26: Privacy & Security → Open Anyway, or the
   `xattr -dr` command.

Yes when: the app window opens. Record the exact steps; they go into `install-for-brother.md`.

## S2. Dictation inside WKWebView

1. Add `NSMicrophoneUsageDescription` and `NSSpeechRecognitionUsageDescription` to the app's
   `Info.plist` (Tauri `bundle.macOS.infoPlist`).
2. Load a page that runs `new (window.SpeechRecognition || window.webkitSpeechRecognition)()`,
   starts it, and logs the results.

Yes when: speech produces text. If not, `web/ui/composer.js:128` already hides the mic button, so
the answer only decides whether Phase 2 adds native dictation. Native dictation would be a new
dependency, so it would need its own owner yes. The default is to accept the hidden button.

## S3. Open a PDF at a page

1. Register a custom scheme `kristoff` with `register_asynchronous_uri_scheme_protocol` that serves
   a local PDF with `Content-Type: application/pdf`.
2. From the page, run the same sequence as `web/ui/citations.js:73-101`:
   `const w = window.open('', '_blank'); w.location = 'kristoff://doc/x#page=7'`.
3. Handle the new-window request in Rust by opening a `WebviewWindow` at that URL. Check that
   Tauri 2.12.3 has `on_new_window`; read the docs.rs source before relying on it.

Yes when: page 7 shows. Fallbacks, in this order:
1. A Rust command opens the file in the default browser at `file://…#page=N`.
2. `open -a Preview` (no page jump).

## S4. Files API citations

1. Upload a 30-page PDF to `POST /v1/files` with the current Files API beta header. Check the
   header name in the Claude API docs on the day; don't use memory.
2. Ask a question with
   `{type:"document", source:{type:"file", file_id}, citations:{enabled:true}}` and stream it.

Yes when: `citations_delta` events carry `page_location` with `start_page_number`. Save the
redacted request and SSE bytes as `contracts/fixtures/anthropic-file-doc-request.json` and
`anthropic-file-doc-stream.sse`. If no, Phase 2 falls back to base64 sources, as the MVP did.

## S5. Search-result citations through a tool

1. Define a client tool `search_ph_laws`. Answer its `tool_use` with a `tool_result` whose content
   is two `search_result` blocks (`source`, `title`, `content:[{type:"text"}]`,
   `citations:{enabled:true}`).
2. Stream the second turn.

Yes when: `citations_delta` events carry `search_result_location` with `source` and `title`. Save
the fixtures as `anthropic-tool-request.json` and `anthropic-tool-stream.sse`. If no, law
citations become plain `[RA 9262 §5]` text that the server parses, and Phase 4 grows by half a day.

## S6. Query embeddings in candle

1. Load `intfloat/multilingual-e5-small` (safetensors + `tokenizer.json`) with candle-transformers
   BERT on Metal. Use mean pooling and L2 normalisation, and prefix queries with `"query: "`.
2. Embed 10 fixed sentences (English, Tagalog, Taglish).
3. Compute reference vectors once with the model card's reference code in a throwaway Python venv
   (`uv`). Save them as `desktop/crates/kristoff-adapters/tests/golden/e5-small.json`. Python is
   not a project dependency.

Yes when: every cosine is ≥ 0.999 and one query takes under 50 ms warm. Also record the f16 size.
If no, try fp32 weights. If that also fails, use a Swift Core ML sidecar, the DexFS pattern, and
re-plan.

## Exit

- The spike results table in `README.md` is filled and committed on `feat/desktop-plan`.
- Fixtures from S4, S5 and S6 are committed on that branch (redacted: no key, no file ids that
  matter).
- Spike branches are deleted.
