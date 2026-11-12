// Shared UI state for the Kristoff web client (memory only; the theme lives in services/prefs.js).

/** Upload size cap: contract v2 `POST /v1/docs` `size` 1..209715200. */
export const MAX_UPLOAD_BYTES = 209715200;
/** File name cap: contract `POST /v1/docs` `fileName` ≤ 255. */
export const MAX_FILE_NAME = 255;
/** Name caps: thread title 1..120, project name 1..80, doc title 1..200 (contract). */
export const NAME_MAX = { chat: 120, project: 80, doc: 200 };

export const state = {
  model: '', models: [], maxAttachedDocs: 0,
  features: {}, // config.features (desktop: { statutes: true }); absent on the web
  pendingStatutes: true, // Statutes toggle for the next new thread (contract: Chat.statutes, default true)
  chats: [], chatsCursor: null, projects: [],
  docs: [], docsCursor: null, docsLoaded: false, deletedDocs: new Set(),
  chat: null, pendingDocIds: [], busy: false, view: 'home', booted: false, uploads: [],
  confirm: null, // { kind, id, where, busy } while an inline delete confirm shows
  renaming: null, // { kind, id, where } while an inline rename input shows
  newProject: null, // { forChatId } while the "new project" input shows
  collapsed: new Set(), // collapsed project ids
  threadTab: 'answer', sourcesCollapsed: false, drawerOpen: false,
};
