// The only module that calls the Kristoff API (HTTP API + ask stream) and the S3 upload URL.
// Every route, shape, SSE event and error code comes from docs/contract.md; nothing here adds one.
import { loadConfig } from './config.js';
import { expire, getToken } from './session.js';

const JSON_TIMEOUT_MS = 20_000; // HTTP API Lambdas time out at 10 s; leave room for cold starts
const STREAM_TIMEOUT_MS = 310_000; // ask-stream Lambda timeout is 300 s
const UPLOAD_TIMEOUT_MS = 15 * 60_000; // a 32 MB PUT on a slow link

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

/** Parses a `Retry-After` header (delta seconds or HTTP date) into whole seconds, or null. */
function parseRetryAfter(value) {
  if (!value) return null;
  if (/^\d+$/.test(value.trim())) return Number(value.trim());
  const at = Date.parse(value);
  return Number.isNaN(at) ? null : Math.max(0, Math.ceil((at - Date.now()) / 1000));
}

/** Builds an ApiError from a non-2xx response carrying the contract error envelope. */
async function errorFrom(res) {
  const body = await res.json().catch(() => null);
  const e = body?.error;
  // The HTTP API authorizer answers a missing/invalid/expired token itself (401 or 403 with
  // `{ message }`, no envelope) and HTTP APIs cannot reshape it: treat that as UNAUTHENTICATED.
  const isGatewayAuthDenial = !e && (res.status === 401 || res.status === 403);
  const fallbackCode = isGatewayAuthDenial ? 'UNAUTHENTICATED' : 'INTERNAL';
  const err = new ApiError(
    typeof e?.code === 'string' ? e.code : fallbackCode,
    typeof e?.message === 'string' ? e.message : `HTTP ${res.status}`,
    {
      status: res.status,
      requestId: e?.requestId ?? null,
      retryAfter: parseRetryAfter(res.headers.get('Retry-After')),
    },
  );
  if (err.code === 'UNAUTHENTICATED') expire();
  return err;
}

/** Maps a thrown fetch failure (offline, CORS, timeout) to an ApiError. */
function networkError(err) {
  if (err instanceof ApiError) return err;
  if (err?.name === 'AbortError') return err; // caller cancelled; let it through untouched
  if (err instanceof SyntaxError) return new ApiError('INTERNAL', 'The server sent a malformed answer.');
  const timedOut = err?.name === 'TimeoutError';
  return new ApiError('NETWORK_ERROR', timedOut ? 'The request timed out.' : 'Could not reach the server.');
}

/** Returns the headers for a JSON request, with the bearer token when `auth` is true. */
function headersFor(auth, hasBody) {
  const h = {};
  if (hasBody) h['Content-Type'] = 'application/json';
  if (auth) {
    const token = getToken();
    if (!token) throw new ApiError('UNAUTHENTICATED', 'Locked. Enter your code.', { status: 401 });
    h.Authorization = `Bearer ${token}`;
  }
  return h;
}

/**
 * Sends one JSON request to the HTTP API.
 * @param method - HTTP method.
 * @param path - Path starting with `/v1/`.
 * @param opts - `{ body, auth = true }`.
 * @returns The parsed 200 body.
 * @throws ApiError on any non-2xx status or network failure.
 */
async function request(method, path, { body, auth = true } = {}) {
  const { apiBaseUrl } = await loadConfig();
  let res;
  try {
    res = await fetch(`${apiBaseUrl}${path}`, {
      method,
      headers: headersFor(auth, body !== undefined),
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(JSON_TIMEOUT_MS),
    });
  } catch (err) {
    throw networkError(err);
  }
  if (!res.ok) throw await errorFrom(res);
  return res.json();
}

const id = (s) => encodeURIComponent(s);
const withCursor = (path, cursor) => (cursor ? `${path}?cursor=${encodeURIComponent(cursor)}` : path);

/** `POST /v1/auth/verify` → `{ token, expiresAt }`. */
/** True: the web app opens behind the TOTP gate (the desktop twin says false, D-17). */
export const requiresUnlock = true;

export const verifyCode = (code) => request('POST', '/v1/auth/verify', { body: { code }, auth: false });

/** `GET /v1/config` → `{ model, models: [{ id, label }], maxAttachedDocs, dailyQuestionCap }`. */
export const getConfig = () => request('GET', '/v1/config');

/** `GET /v1/docs` → `{ items: Doc[], nextCursor }`. */
export const listDocs = (cursor) => request('GET', withCursor('/v1/docs', cursor));

/** `POST /v1/docs` → `{ doc, uploadUrl, uploadHeaders }` (doc status `pending`). */
export const createDoc = (fileName, size) => request('POST', '/v1/docs', { body: { fileName, size } });

/** `POST /v1/docs/{id}/confirm` → `{ doc }` (status `ready`). */
export const confirmDoc = (docId) => request('POST', `/v1/docs/${id(docId)}/confirm`);

/** `PATCH /v1/docs/{id}` → `{ doc }`. */
export const renameDoc = (docId, title) => request('PATCH', `/v1/docs/${id(docId)}`, { body: { title } });

/** `GET /v1/docs/{id}/link[?part=n]` → `{ url, expiresAt }` (presigned GET, 300 s); `part` links one part. */
export const getDocLink = (docId, part = null) =>
  request('GET', `/v1/docs/${id(docId)}/link${part ? `?part=${encodeURIComponent(part)}` : ''}`);

/** `DELETE /v1/docs/{id}` → `{ deleted: true }` (also removed from every chat's docIds). */
export const deleteDoc = (docId) => request('DELETE', `/v1/docs/${id(docId)}`);

/** `GET /v1/projects` → `{ items: Project[] }` (name order, ≤ 50). */
export const listProjects = () => request('GET', '/v1/projects');

/** `POST /v1/projects` `{ name }` → `{ project }`. */
export const createProject = (name) => request('POST', '/v1/projects', { body: { name } });

/** `PATCH /v1/projects/{id}` `{ name }` → `{ project }`. */
export const renameProject = (projectId, name) => request('PATCH', `/v1/projects/${id(projectId)}`, { body: { name } });

/** `DELETE /v1/projects/{id}` → `{ deleted: true }` (its chats move to no project). */
export const deleteProject = (projectId) => request('DELETE', `/v1/projects/${id(projectId)}`);

/** `GET /v1/chats` → `{ items: ChatSummary[], nextCursor }`, newest first. */
export const listChats = (cursor) => request('GET', withCursor('/v1/chats', cursor));

/** `POST /v1/chats` → `{ chat }`. */
export const createChat = (docIds) => request('POST', '/v1/chats', { body: { docIds } });

/** `GET /v1/chats/{id}` → `{ chat, messages }`, messages oldest first. */
export const getChat = (chatId) => request('GET', `/v1/chats/${id(chatId)}`);

/** `PATCH /v1/chats/{id}` with `{ title?, docIds?, pinned?, projectId? }` → `{ chat }`. */
export const updateChat = (chatId, patch) => request('PATCH', `/v1/chats/${id(chatId)}`, { body: patch });

/** `DELETE /v1/chats/{id}` → `{ deleted: true }`; unknown id → `CHAT_NOT_FOUND`. */
export const deleteChat = (chatId) => request('DELETE', `/v1/chats/${id(chatId)}`);

/** `GET /v1/usage` → `{ today: Usage, total: Usage }`. */
export const getUsage = () => request('GET', '/v1/usage');

/**
 * Opens an external source link (a law citation's `sourceUrl`) in a new tab. No network call;
 * it lives here because api.js is the one module the desktop build swaps (api-desktop.js).
 * @param url - An `https:` URL.
 * @returns Promise that resolves once the tab is requested.
 */
export const openExternal = async (url) => {
  window.open(url, '_blank', 'noopener');
};

/**
 * Opens a document page in a new tab (D-12). The tab opens synchronously, before the first await,
 * so call this directly from the click handler and popup blockers allow it; the tab then gets the
 * short-lived presigned URL once `getDocLink` returns.
 * @param docId - Doc id.
 * @param part - Part number for split docs, or null for the whole file.
 * @param page - Page inside that file (1-based), or null for the first page.
 * @returns Promise that resolves once the tab is pointed at the document.
 * @throws ApiError `POPUP_BLOCKED` (client-only) when the browser refused the tab, or the
 *   `getDocLink` error (e.g. `DOC_NOT_FOUND`) after closing the tab.
 */
export async function openDoc(docId, part = null, page = null) {
  const w = window.open('', '_blank');
  if (w) { w.opener = null; w.document.title = 'Opening…'; w.document.body.textContent = 'Opening document…'; }
  try {
    const { url } = await getDocLink(docId, part);
    if (!w) throw new ApiError('POPUP_BLOCKED', 'The browser blocked the new tab.');
    w.location.href = page ? `${url}#page=${page}` : url;
  } catch (err) {
    w?.close();
    throw err;
  }
}

/**
 * PUTs a file to the presigned `uploadUrl` from `createDoc`. Uses XMLHttpRequest because fetch
 * has no upload progress events.
 * @param uploadUrl - Presigned S3 PUT URL.
 * @param uploadHeaders - Headers the presign pinned (`Content-Type: application/pdf`).
 * @param file - The File to send.
 * @param onProgress - Called with a fraction 0..1.
 * @returns Promise that resolves on a 2xx response.
 * @throws ApiError `UPLOAD_FAILED` or `NETWORK_ERROR`.
 */
export function uploadFile(uploadUrl, uploadHeaders, file, onProgress) {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open('PUT', uploadUrl);
    xhr.timeout = UPLOAD_TIMEOUT_MS;
    for (const [k, v] of Object.entries(uploadHeaders ?? {})) xhr.setRequestHeader(k, v);
    xhr.upload.onprogress = (e) => e.lengthComputable && onProgress?.(e.loaded / e.total);
    xhr.onload = () =>
      xhr.status >= 200 && xhr.status < 300
        ? resolve()
        : reject(new ApiError('UPLOAD_FAILED', `Storage rejected the file (HTTP ${xhr.status}).`, { status: xhr.status }));
    xhr.onerror = () => reject(new ApiError('NETWORK_ERROR', 'Upload connection failed.'));
    xhr.ontimeout = () => reject(new ApiError('NETWORK_ERROR', 'Upload timed out.'));
    xhr.send(file);
  });
}

/**
 * Splits buffered SSE text into complete events.
 * @param buf - Text received so far.
 * @returns `{ events: [{ event, data }], rest }`; `rest` is the incomplete tail.
 */
export function parseSse(buf) {
  const events = [];
  const blocks = buf.replace(/\r\n?/g, '\n').split('\n\n');
  const rest = blocks.pop();
  for (const block of blocks) {
    let event = 'message';
    const data = [];
    for (const line of block.split('\n')) {
      if (line.startsWith('event:')) event = line.slice(6).trim();
      else if (line.startsWith('data:')) data.push(line.slice(5).replace(/^ /, ''));
    }
    if (data.length) events.push({ event, data: data.join('\n') });
  }
  return { events, rest };
}

/**
 * Asks a question on the stream URL and dispatches SSE events as they arrive.
 * @param chatId - Chat id.
 * @param text - Question, 1..8000 chars.
 * @param handlers - `{ model, onText(text), onCitation(citation), onFollowUps(list), onSearch({ query, count }), signal }`;
 *   `model` is an id from `config.models` (≤ 64 chars), omitted from the body when empty so the server uses its
 *   default. `onSearch` fires for the additive `search` event (statute search; servers without it never send one).
 * @returns `{ message, usage }` from the `done` event.
 * @throws ApiError for a pre-stream envelope, an `error` event, or a stream that ends without `done`.
 */
export async function askStream(chatId, text, { model, onText, onCitation, onFollowUps, onSearch, signal } = {}) {
  const { streamUrl } = await loadConfig();
  let res;
  try {
    const timeout = AbortSignal.timeout(STREAM_TIMEOUT_MS);
    res = await fetch(`${streamUrl}/v1/chats/${id(chatId)}/ask`, {
      method: 'POST',
      headers: headersFor(true, true),
      body: JSON.stringify(model ? { text, model } : { text }),
      signal: signal ? AbortSignal.any([signal, timeout]) : timeout,
    });
  } catch (err) {
    throw networkError(err);
  }
  if (!res.ok) throw await errorFrom(res);

  const reader = res.body.getReader();
  const dec = new TextDecoder();
  let buf = '';
  try {
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      buf += dec.decode(value, { stream: true });
      const parsed = parseSse(buf);
      buf = parsed.rest;
      for (const { event, data } of parsed.events) {
        const payload = JSON.parse(data);
        if (event === 'text') onText?.(payload.text);
        else if (event === 'citation') onCitation?.(payload);
        else if (event === 'followups') onFollowUps?.(payload.followUps ?? []);
        else if (event === 'search') onSearch?.(payload);
        else if (event === 'done') return payload;
        else if (event === 'error') {
          const e = payload.error ?? {};
          throw new ApiError(e.code ?? 'INTERNAL', e.message ?? 'The answer failed.', { requestId: e.requestId });
        }
      }
    }
  } catch (err) {
    throw networkError(err);
  } finally {
    reader.cancel().catch(() => {}); // stream already finished or failed; nothing left to release
  }
  throw new ApiError('UPSTREAM_UNAVAILABLE', 'The answer stopped before it finished.');
}
