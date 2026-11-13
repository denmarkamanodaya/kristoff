# Phase 8: LexPH, shareable with bring-your-own Claude key

## Context

The Phase 7 DMG works and is going to the owner's brother. The owner now wants anyone to be able to
download and use the app. Today that is blocked by one design choice: **the owner's Claude API key
is baked into the binary** (`desktop/app/src-tauri/src/lib.rs:20`
`pub const ANTHROPIC_KEY: &str = env!("KRISTOFF_ANTHROPIC_KEY")`, passed to
`Composition::open(data_dir, anthropic_key)` at `composition.rs:51`, which builds `AiPorts::anthropic`
once at start-up in `ai.rs:34`). A public DMG would hand everyone the owner's key and bill.

Goals, in priority order:
1. **Each user enters their own Claude API key.** The app must work with no baked key, store the
   key securely, and let the user change or remove it.
2. **A generic name** in place of "Kristoff": the window title, the UI brand, the bundle name and
   identifier, and the docs.
3. **Shareable distribution:** a DMG that strangers can install, plus a page or release to get it from.

## 1. Bring-your-own key (main work)

**Behaviour**
- **First launch with no key:** after the loading screen, a "Connect your Claude account" screen
  explains what the key is for, links to `console.anthropic.com` → API keys, and has a password
  field and **Save**. The app checks the key with one free call (`GET /v1/models`) before saving it.
- **Settings** (a gear icon in the sidebar footer) shows "Claude key: sk-ant-…ab12 ✓" with
  **Change** and **Remove**.
- **No key, or a revoked key:** browsing the Library and old threads still works. Asking shows
  "Add your Claude API key in Settings". A 401 from the API maps to a clear "Your key was rejected,
  update it in Settings".
- **Model list:** Sonnet, Opus and Haiku stay as now. A key without access to a model gets the
  normal API error, shown plainly.

**Where the user enters it:** a **Settings** section inside the app, which you asked for. It holds
the Claude API key field and the daily question limit. On first launch with no key, the app opens
Settings automatically.

**Where it's saved:** the **macOS Keychain** (a generic-password item, service
`ph.lexph.desktop.claude-key`), so it's encrypted by macOS and never written to a plain file. This
uses the `security-framework` crate, the same version (3.7.0) already approved in DexFS. **Approving
this plan approves that one dependency.** The key never touches disk, logs or the
web layer. The web gets only a masked hint (last 4 characters) and a configured or not-configured
flag.

**Code changes** (keeping the existing ports and adapters):
- `lib.rs`: remove `env!("KRISTOFF_ANTHROPIC_KEY")`. `build.rs`: stop requiring the key. The
  release fake-key check goes, because no key is baked any more.
- `composition.rs` / `ai.rs`: `AiPorts` becomes **swappable at runtime**: an
  `RwLock<Option<AiPorts>>` filled at start-up from the Keychain, and replaced on save or remove.
  The ask use case already takes its ports per call, so only the composition changes.
- **New core port** `SecretStore { get, set, delete }`, with a Keychain adapter, an in-memory
  fake, and a contract suite (the house pattern).
- **New commands:**
  - `get_key_status` returns `{configured, hint}`.
  - `set_api_key {key}` validates the format (`sk-ant-`, bounded length), runs the live
    `/v1/models` check, stores the key and swaps the ports. It returns `{configured, hint}` or
    `KEY_INVALID` / `UPSTREAM_UNAVAILABLE`.
  - `remove_api_key` deletes the key.
- **Config and errors:** `get_config` gains `features.aiReady`. New error codes `KEY_MISSING` and
  `KEY_INVALID`. The backend is retired, so the desktop error list stops being pinned to the old
  TS list (`ts_parity`); that pin is dropped.
- **Web:**
  - a key screen and a settings panel (new `web/ui/settings.js`);
  - `api-desktop.js` gains `getKeyStatus`, `setApiKey` and `removeApiKey`, plus web stubs so the
    export-parity check holds;
  - the composer disables Send, with a link to Settings, when `aiReady` is false.
- **Usage counters** stay local per Mac. The daily question cap (100) stays as a safety brake on
  the user's own bill, and moves into Settings so the user can change it.

## 2. Generic name (small)

- The name is **LexPH**, your choice. Apply it to `tauri.conf.json` (`productName`, window
  `title`), `web/index.html` and the brand mark (the "K" mark becomes "L"), the loading screen,
  docs and the README. The DMG becomes `LexPH-<version>-arm64.dmg`, at version 0.2.0.
- **Bundle identifier:** change `app.kristoff.desktop` → `ph.lexph.desktop`. The data
  folder follows the identifier, so on first launch the app **moves the old
  `~/Library/Application Support/app.kristoff.desktop` data** (threads, PDFs, usage) into the new
  folder. Your brother loses nothing.
- **Kept on purpose:** internal crate names (`kristoff-core` and the rest) and the repo name. They
  are invisible to users, and renaming them is churn.

## 3. Sharing with friends (your answers: no notarisation, Mac + Claude only)

- **Signing:** none. The build stays ad-hoc signed, as now. Each friend opens it once through
  System Settings → Privacy & Security → **Open Anyway**. The install guide already explains this,
  and it gets a generic version.
- **Sharing:** you send the DMG (1.4 GB) by AirDrop or a Google Drive / iCloud link, along with a
  one-page `docs/desktop/install-guide.md`. It's generic, with no family names, and covers: get a
  Claude API key, Open Anyway, paste the key in Settings.
- **Scope:** Mac (Apple Silicon) and Claude only. No other AI providers and no Windows, so the
  model code stays as it is.
- **In-app and guide notices:**
  - "Research aid, not legal advice; verify against official sources".
  - Coverage and edition caveats.
  - The ODC-By credit to the OmniCorpus author.
  - "Your questions and attached PDFs are sent to Anthropic under your own API account".
  - Decisions contain party names exactly as published by the Supreme Court.
- **Data licences:** the government texts carry no copyright (RA 8293 Sec. 176), and OmniCorpus is
  ODC-By, which allows redistribution with credit. The ipfs dataset is "other"; free, non-commercial
  distribution fits Sec. 176. This is a reading of the law, not legal advice.

## Phases (one PR, `feat/desktop-public`, after your yes)

| # | Work | Est. |
|---|---|---|
| A | SecretStore port + Keychain adapter, swappable AiPorts, the 3 key commands, no baked key, errors and config | ~0.5 day |
| B | Web key screen + settings + aiReady gating | ~0.5 day |
| C | Rename + bundle-id data migration | ~2 h |
| D | Notices, the generic install guide, README, then the LexPH DMG build | ~2 h |

Your brother moves to this build too: he pastes a key once. If you want to keep paying for him,
paste your key on his Mac. It stays in his Keychain only, never in a public file.

## Verification

- `ci-local` green. Tests (MVP, kept light):
  - Keychain contract suite (fake + real Keychain in a temp service name);
  - `set_api_key` rejecting a bad format;
  - a 401 mapping to `KEY_INVALID`;
  - ask with no key returning `KEY_MISSING`;
  - the data-folder migration test;
  - a scan that the release binary contains **no** `sk-ant-` string.
- On this Mac, a fresh user data dir:
  - the key screen appears, a wrong key is rejected, and your key is accepted;
  - one question gets answered;
  - Remove shows "Add your key".
- Old data migrates: launch over a copy of the current data folder, and the threads appear.
- The ad-hoc signed LexPH DMG installs through Open Anyway and opens to Settings when no key is
  saved.

## Your decisions (2026-11-13)

- **Name:** LexPH.
- **Key entry:** in a Settings section in the app. Mac + Claude only.
- **Signing:** none. Shared privately with friends and friends of friends.
