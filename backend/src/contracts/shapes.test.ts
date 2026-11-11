import { describe, expect, expectTypeOf, it } from 'vitest';
import type { Chat, ChatSummary, Citation, Doc, Message, Project, Usage } from '../domain/model.js';
import { ERROR_CODES, WIRE_ERRORS } from './errors.js';
import { UpdateChatRequest, VerifyRequest } from './requests.js';
import type {
  ChatSummaryWire,
  ChatWire,
  CitationWire,
  DocWire,
  MessageWire,
  ProjectWire,
  UsageWire,
} from './shapes.js';
import type { z } from 'zod';

/** The wire type with readonly arrays and properties, as the domain declares them. */
type DeepReadonly<T> = T extends (infer U)[]
  ? readonly DeepReadonly<U>[]
  : T extends object
    ? { readonly [K in keyof T]: DeepReadonly<T[K]> }
    : T;

/** The readonly view of a wire schema's type. */
type WireOf<S extends z.ZodType> = DeepReadonly<z.infer<S>>;

describe('contract shapes', () => {
  it('keeps every domain type assignable to its wire schema (no hand-written twin drift)', () => {
    expectTypeOf<Doc>().toExtend<WireOf<typeof DocWire>>();
    expectTypeOf<Usage>().toExtend<WireOf<typeof UsageWire>>();
    expectTypeOf<Citation>().toExtend<WireOf<typeof CitationWire>>();
    expectTypeOf<ChatSummary>().toExtend<WireOf<typeof ChatSummaryWire>>();
    expectTypeOf<Chat>().toExtend<WireOf<typeof ChatWire>>();
    expectTypeOf<Message>().toExtend<WireOf<typeof MessageWire>>();
    expectTypeOf<Project>().toExtend<WireOf<typeof ProjectWire>>();
  });

  it('maps every wire code to the contract status table', () => {
    const table = Object.fromEntries(
      Object.entries(WIRE_ERRORS).map(([code, w]) => [code, w.status]),
    );

    expect(table).toEqual({
      VALIDATION_FAILED: 400,
      OTP_INVALID: 401,
      OTP_REPLAYED: 401,
      OTP_LOCKED: 429,
      UNAUTHENTICATED: 401,
      DOC_NOT_FOUND: 404,
      CHAT_NOT_FOUND: 404,
      PROJECT_NOT_FOUND: 404,
      PROJECT_LIMIT_REACHED: 409,
      UPLOAD_INVALID: 422,
      DAILY_CAP_REACHED: 429,
      AI_DISABLED: 503,
      UPSTREAM_UNAVAILABLE: 503,
      INTERNAL: 500,
    });
    expect(ERROR_CODES.filter((c) => !(c in WIRE_ERRORS))).toEqual(['VERSION_CONFLICT']);
  });

  it('requires a six-digit code and at least one field on a chat update', () => {
    expect(VerifyRequest.safeParse({ code: '123456' }).success).toBe(true);
    expect(UpdateChatRequest.safeParse({}).success).toBe(false);
    expect(UpdateChatRequest.safeParse({ docIds: [] }).success).toBe(true);
  });
});
