// Desktop twin of services/api.js: the same exports, but every call goes to the Rust shell over
// Tauri IPC (`window.__TAURI__.core`, enabled by `app.withGlobalTauri`). The desktop build copies
// this file over dist-desktop/services/api.js, so ui/ modules import it unchanged.
// Plan: docs/desktop/phases/phase-1-core-storage.md §6 (D-8, D-9, D-10); phase-2 §4 (askStream).
// Contract: docs/contract.md, "Desktop IPC". scripts/check.mjs keeps the export names equal to api.js
// and allows Tauri IPC only in this file. The desktop app has no sign-in (decision D-17): no token
// check, `requiresUnlock` is false, and `verifyCode` is a stub kept only for export parity.
const UPLOAD_CHUNK_BYTES = 8 * 1024 * 1024; // D-8: 8 MiB raw-body chunks
const UPLOAD_SCHEME = 'kristoff-upload://';
// Tauri does not order Channel messages against the command's own reply, so a `done` sent just
// before `ask` returns can arrive after it. Wait this long before calling the stream cut off.
const STREAM_END_GRACE_MS = 2_000;

/**
 * A failed API call. `code` is a contract error code (`OTP_INVALID`, `DOC_NOT_FOUND`, ...) or one
 * of the client-only codes `NETWORK_ERROR` and `UPLOAD_FAILED`.
 */
export class ApiError extends Error {
  /**
   * @param code - UPPER_SNAKE error code.
   * @param message - Human text from the server, or a client fallback.
   * @param extra - `{ status, requestId, retryAfter }`; `retryAfter` is seconds or null.
   */
  constructor(code, message, extra = {}) {
    super(message);
    this.name = 'ApiError';
    this.code = code;
    this.status = extra.status ?? 0;
    this.requestId = extra.requestId ?? null;
    this.retryAfter = extra.retryAfter ?? null;
  }
}

/** Returns the Tauri core API, read at call time so the module loads before the shell injects it. */
function tauri() {
  const core = globalThis.window?.__TAURI__?.core;
  if (!core) throw new ApiError('INTERNAL', 'The desktop bridge is not available.');
  return core;
}

/**
 * Maps a rejected invoke (`{ code, message, retryAfter, requestId? }`, or a plain string for a
 * Tauri-level failure such as an unknown command) to an ApiError.
 */
function toApiError(err) {
  if (err instanceof ApiError) return err;
  const isDto = err && typeof err === 'object' && typeof err.code === 'string';
  const e = isDto
    ? new ApiError(err.code, typeof err.message === 'string' ? err.message : err.code, {
        requestId: err.requestId ?? null,
        retryAfter: Number.isFinite(err.retryAfter) ? err.retryAfter : null,
      })
    : new ApiError('INTERNAL', typeof err === 'string' && err ? err : 'The desktop app failed.');
  return e;
}

/**
 * Calls one Rust command.
 * @param cmd - Command name from the Desktop IPC table.
 * @param args - camelCase arguments; absent optionals are null.
 * @returns The command's resolved value.
 * @throws ApiError mapped from the command's error.
 */
async function call(cmd, args) {
  try {
    return await tauri().invoke(cmd, args);
  } catch (err) {
    throw toApiError(err);
  }
}

/**
 * Subscribes to the shell's `law-ready` event (the law index has warmed). Without the event API
 * (an older shell) it subscribes to nothing; the launch screen then relies on polling.
 * @param fn - Called with no arguments.
 * @returns An unsubscribe function.
 */
export const onLawReady = async (fn) => {
  const events = globalThis.window?.__TAURI__?.event;
  if (!events) return () => {};
  return events.listen('law-ready', () => fn());
};

/** False: the desktop app opens straight to Home, with no sign-in (D-17). */
export const requiresUnlock = false;

/**
 * Export-parity stub (D-17): there is no `verify_otp` command, and app.js never calls this on
 * desktop because `requiresUnlock` is false.
 * @returns Never resolves.
 * @throws ApiError `INTERNAL` always, without calling the shell.
 */
export const verifyCode = async () => {
  throw new ApiError('INTERNAL', 'The desktop app has no sign-in.');
};

/** `get_config` → `{ model, models: [{ id, label }], maxAttachedDocs, dailyQuestionCap }`. */
export const getConfig = () => call('get_config', {});

/** `get_key_status {}` → `{ configured, hint }`; `hint` is the key's last 4 characters or null (Phase 8). */
export const getKeyStatus = () => call('get_key_status', {});

/**
 * `set_api_key {key}` → `{ configured, hint }`. The shell checks the format and the key live, stores
 * it in the Keychain and swaps the AI ports.
 * @param key - The pasted Claude API key (`sk-ant-…`).
 * @returns The new key status.
 * @throws ApiError `KEY_INVALID` (bad format or rejected) or `UPSTREAM_UNAVAILABLE`.
 */
export const setApiKey = (key) => call('set_api_key', { key });

/** `remove_api_key {}` → `{ configured: false, hint: null }`: deletes the key from the Keychain. */
export const removeApiKey = () => call('remove_api_key', {});

/** `get_settings {}` → `{ dailyQuestionCap }` (user settings kept on this Mac). */
export const getSettings = () => call('get_settings', {});

/**
 * `update_settings {dailyQuestionCap}` (flat) → `{ dailyQuestionCap }`.
 * @param settings - `{ dailyQuestionCap }`, an integer 1..10000.
 * @returns `{ dailyQuestionCap }` after the change.
 * @throws ApiError `VALIDATION_FAILED` for an out-of-range value.
 */
export const updateSettings = ({ dailyQuestionCap }) => call('update_settings', { dailyQuestionCap });

/** `list_docs {cursor}` → `{ items: Doc[], nextCursor }`. */
export const listDocs = (cursor) => call('list_docs', { cursor: cursor || null });

/** `create_doc {fileName, size}` → `{ doc, uploadUrl: "kristoff-upload://<docId>", uploadHeaders }`. */
export const createDoc = (fileName, size) => call('create_doc', { fileName, size });

/** `confirm_doc {docId}` → `{ doc }`. */
export const confirmDoc = (docId) => call('confirm_doc', { docId });

/** `rename_doc {docId, title}` → `{ doc }`. */
export const renameDoc = (docId, title) => call('rename_doc', { docId, title });

/** `get_doc_link {docId, part}` → `{ url: "kristoff://doc/<id>[/part-<n>]", expiresAt }` (D-9). */
export const getDocLink = (docId, part = null) => call('get_doc_link', { docId, part: part || null });

/** `delete_doc {docId}` → `{ deleted: true }`. */
export const deleteDoc = (docId) => call('delete_doc', { docId });

/** `list_projects` → `{ items: Project[] }`. */
export const listProjects = () => call('list_projects', {});

/** `create_project {name}` → `{ project }`. */
export const createProject = (name) => call('create_project', { name });

/** `rename_project {projectId, name}` → `{ project }`. */
export const renameProject = (projectId, name) => call('rename_project', { projectId, name });

/** `delete_project {projectId}` → `{ deleted: true }`. */
export const deleteProject = (projectId) => call('delete_project', { projectId });

/** `list_chats {cursor}` → `{ items: ChatSummary[], nextCursor }`. */
export const listChats = (cursor) => call('list_chats', { cursor: cursor || null });

/** `create_chat {docIds}` → `{ chat }`. */
export const createChat = (docIds) => call('create_chat', { docIds });

/** `get_chat {chatId}` → `{ chat, messages }`. */
export const getChat = (chatId) => call('get_chat', { chatId });

/** `update_chat {chatId, patch}` → `{ chat }`. */
export const updateChat = (chatId, patch) => call('update_chat', { chatId, patch });

/** `delete_chat {chatId}` → `{ deleted: true }`. */
export const deleteChat = (chatId) => call('delete_chat', { chatId });

/** `get_usage` → `{ today: Usage, total: Usage }`. */
export const getUsage = () => call('get_usage', {});

/**
 * `open_doc {docId, part, page}` → null (D-12): the shell opens its PDF viewer window, because
 * WKWebView refuses `window.open` from the page.
 * @param docId - Doc id.
 * @param part - Part number for split docs, or null for the whole file.
 * @param page - Page inside that file (1-based), or null for the first page.
 * @returns Promise that resolves once the viewer is opening.
 * @throws ApiError mapped from the shell (e.g. `DOC_NOT_FOUND`).
 */
export const openDoc = (docId, part = null, page = null) =>
  call('open_doc', { docId, part: part ?? null, page: page ?? null });

/**
 * Opens an external source link (a law citation's `sourceUrl`) in the system browser through the
 * shell's `open_external` command, which enforces the https host allowlist.
 * @param url - An `https:` URL.
 * @returns Promise that resolves once the shell accepted the URL.
 * @throws ApiError mapped from the shell's refusal.
 */
export const openExternal = (url) => call('open_external', { url }, { auth: false });

/**
 * Sends a file to the shell in 8 MiB raw-body chunks (D-8); same contract as api.js `uploadFile`.
 * @param uploadUrl - `kristoff-upload://<docId>` from `createDoc`.
 * @param uploadHeaders - Ignored on desktop (the shell pins the type itself).
 * @param file - The File to send.
 * @param onProgress - Called with a fraction 0..1 after each chunk.
 * @returns Promise that resolves once every chunk is written.
 * @throws ApiError `UPLOAD_FAILED` for a bad upload URL, or the code the shell returns.
 */
export async function uploadFile(uploadUrl, uploadHeaders, file, onProgress) {
  if (typeof uploadUrl !== 'string' || !uploadUrl.startsWith(UPLOAD_SCHEME)) {
    throw new ApiError('UPLOAD_FAILED', 'The upload target is not valid.');
  }
  const docId = uploadUrl.slice(UPLOAD_SCHEME.length);
  for (let offset = 0; offset < file.size; offset += UPLOAD_CHUNK_BYTES) {
    const chunk = await file.slice(offset, offset + UPLOAD_CHUNK_BYTES).arrayBuffer();
    const headers = { 'x-doc-id': docId, 'x-offset': String(offset) };
    try {
      await tauri().invoke('upload_chunk', chunk, { headers });
    } catch (err) {
      throw toApiError(err);
    }
    onProgress?.((offset + chunk.byteLength) / file.size);
  }
}

/**
 * D-10: desktop streams answers through a Tauri Channel, not SSE. Kept for export parity only.
 * @param buf - Ignored.
 * @returns `{ events: [], rest: '' }`.
 */
export function parseSse(buf) {
  void buf;
  return { events: [], rest: '' };
}

/** The rejection value a fetch aborted by `signal` produces (its reason, default `AbortError`). */
const abortReason = (signal) => signal.reason ?? new DOMException('The operation was aborted.', 'AbortError');

/**
 * Asks a question through the shell's `ask` command and dispatches Channel events as they arrive.
 * Same signature and callback semantics as api.js `askStream`.
 * @param chatId - Chat id.
 * @param text - Question, 1..8000 chars.
 * @param handlers - `{ model, onText(text), onCitation(citation), onFollowUps(list), onSearch({ query, count }),
 *   signal }`; an empty `model` is sent as null so the shell uses its default.
 * @returns `{ message, usage }` from the `done` event.
 * @throws ApiError for a rejected command, an `error` event, or a stream that ends without `done`;
 *   the abort reason (an `AbortError`) when `signal` aborts, after `cancel_ask` is sent.
 */
export async function askStream(chatId, text, { model, onText, onCitation, onFollowUps, onSearch, signal } = {}) {
  const { invoke, Channel } = tauri();
  if (signal?.aborted) throw abortReason(signal);
  return new Promise((resolve, reject) => {
    let settled = false;
    let graceTimer = null;
    const onAbort = () => {
      invoke('cancel_ask', { chatId }).catch(() => {}); // best effort; the answer is already abandoned
      finish(reject, abortReason(signal));
    };
    function finish(fn, value) {
      if (settled) return;
      settled = true;
      clearTimeout(graceTimer);
      signal?.removeEventListener('abort', onAbort);
      fn(value);
    }
    const channel = new Channel();
    channel.onmessage = ({ event, data } = {}) => {
      if (settled) return;
      if (event === 'text') onText?.(data?.text);
      else if (event === 'citation') onCitation?.(data);
      else if (event === 'followups') onFollowUps?.(data?.followUps ?? []);
      else if (event === 'search') onSearch?.(data);
      else if (event === 'done') finish(resolve, data);
      else if (event === 'error') {
        const e = data?.error ?? {};
        finish(reject, new ApiError(e.code ?? 'INTERNAL', e.message ?? 'The answer failed.', { requestId: e.requestId }));
      }
    };
    signal?.addEventListener('abort', onAbort, { once: true });
    invoke('ask', { chatId, text, model: model || null, onEvent: channel }).then(
      () => {
        graceTimer = setTimeout(
          () => finish(reject, new ApiError('UPSTREAM_UNAVAILABLE', 'The answer stopped before it finished.')),
          STREAM_END_GRACE_MS,
        );
      },
      (err) => finish(reject, toApiError(err)),
    );
  });
}
