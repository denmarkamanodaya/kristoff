// The one keys unit (core/dry.md "Storage keys"): every DynamoDB key and S3 object key is built
// here. Contract: docs/contract.md "Storage". Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.

/** A DynamoDB primary key. */
export interface ItemKey {
  readonly PK: string;
  readonly SK: string;
}

const OWNER = 'OWNER#';
const DOC = 'DOC#';
const CHAT = 'CHAT#';
const MSG = 'MSG#';
const USAGE = 'USAGE#';

/** Sort-key prefix of every doc item under an owner (for `begins_with` queries). */
export const DOC_SK_PREFIX = DOC;
/** Sort-key prefix of every chat item under an owner. */
export const CHAT_SK_PREFIX = CHAT;
/** Sort-key prefix of every message item under a chat (for `begins_with` queries). */
export const MESSAGE_SK_PREFIX = MSG;

/**
 * Builds the key of a library document.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param uid - Owner id from the principal.
 * @param docId - 16-hex document id.
 * @returns `OWNER#<uid>` / `DOC#<id>`.
 */
export const docKey = (uid: string, docId: string): ItemKey => ({
  PK: `${OWNER}${uid}`,
  SK: `${DOC}${docId}`,
});

/**
 * Builds the partition key every owner-scoped item shares.
 * @param uid - Owner id from the principal.
 * @returns `OWNER#<uid>`.
 */
export const ownerPk = (uid: string): string => `${OWNER}${uid}`;

/**
 * Builds the key of a chat.
 * @param uid - Owner id from the principal.
 * @param chatId - 16-hex chat id.
 * @returns `OWNER#<uid>` / `CHAT#<id>`.
 */
export const chatKey = (uid: string, chatId: string): ItemKey => ({
  PK: `${OWNER}${uid}`,
  SK: `${CHAT}${chatId}`,
});

/**
 * Builds the GSI1 key that orders an owner's chats by last update.
 * @param uid - Owner id from the principal.
 * @param updatedAt - ISO instant of the last update.
 * @param chatId - 16-hex chat id (tie-break, so the order is total).
 * @returns `GSI1PK=OWNER#<uid>#CHATS`, `GSI1SK=<updatedAt>#<id>`.
 */
export const chatGsiKey = (
  uid: string,
  updatedAt: string,
  chatId: string,
): { readonly GSI1PK: string; readonly GSI1SK: string } => ({
  GSI1PK: chatsGsiPk(uid),
  GSI1SK: `${updatedAt}#${chatId}`,
});

/**
 * Builds the GSI1 partition key of an owner's chat list.
 * @param uid - Owner id from the principal.
 * @returns `OWNER#<uid>#CHATS`.
 */
export const chatsGsiPk = (uid: string): string => `${OWNER}${uid}#CHATS`;

/**
 * Builds the partition key every message of a chat shares.
 * @param chatId - 16-hex chat id.
 * @returns `CHAT#<id>`.
 */
export const chatPk = (chatId: string): string => `${CHAT}${chatId}`;

/**
 * Builds the key of one chat message.
 * @param chatId - 16-hex chat id.
 * @param messageId - ULID of the message.
 * @returns `CHAT#<id>` / `MSG#<ulid>`.
 */
export const messageKey = (chatId: string, messageId: string): ItemKey => ({
  PK: chatPk(chatId),
  SK: `${MSG}${messageId}`,
});

/**
 * Builds the key of one UTC day's usage counters.
 * @param uid - Owner id from the principal.
 * @param day - UTC date `yyyy-mm-dd`.
 * @returns `OWNER#<uid>` / `USAGE#<day>`.
 */
export const usageDayKey = (uid: string, day: string): ItemKey => ({
  PK: `${OWNER}${uid}`,
  SK: `${USAGE}${day}`,
});

/**
 * Builds the key of the all-time usage counters.
 * @param uid - Owner id from the principal.
 * @returns `OWNER#<uid>` / `USAGE#TOTAL`.
 */
export const usageTotalKey = (uid: string): ItemKey => ({
  PK: `${OWNER}${uid}`,
  SK: `${USAGE}TOTAL`,
});

/**
 * Builds the key of the owner's TOTP state (last step, failures, lock).
 * @param uid - Owner id.
 * @returns `AUTH#<uid>` / `TOTP`.
 */
export const totpKey = (uid: string): ItemKey => ({ PK: `AUTH#${uid}`, SK: 'TOTP' });

/**
 * Builds the S3 object key of a document's PDF. Server-built only; never from a request.
 * @param uid - Owner id from the principal.
 * @param docId - 16-hex document id.
 * @returns `docs/<uid>/<docId>.pdf`.
 */
export const docObjectKey = (uid: string, docId: string): string => `docs/${uid}/${docId}.pdf`;
