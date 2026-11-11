// Test builders for domain values (shared test support; never assertions).
import type { Chat, Doc, Message } from '../../src/domain/model.js';

/** A ready doc; override any field. */
export const aDoc = (over: Partial<Doc> = {}): Doc => ({
  id: 'aaaaaaaaaaaaaaaa',
  title: 'People v. Cruz',
  size: 1234,
  status: 'ready',
  createdAt: '2026-11-12T00:00:00.000Z',
  ...over,
});

/** A chat with no docs; override any field. */
export const aChat = (over: Partial<Chat> = {}): Chat => ({
  id: 'cccccccccccccccc',
  title: 'New chat',
  docIds: [],
  createdAt: '2026-11-12T00:00:00.000Z',
  updatedAt: '2026-11-12T00:00:00.000Z',
  ...over,
});

/** A user message; override any field. */
export const aMessage = (over: Partial<Message> = {}): Message => ({
  id: '01JAAAAAAAAAAAAAAAAAAAAAAA',
  role: 'user',
  text: 'What is estoppel?',
  citations: [],
  usage: null,
  model: null,
  stopReason: null,
  at: '2026-11-12T00:00:00.000Z',
  ...over,
});
