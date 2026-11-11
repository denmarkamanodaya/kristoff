// The one error-code list and the code-to-status table (stack/api-design.md, core/dry.md).
// Contract: docs/contract.md "Error envelope and codes". Every other file imports ErrorCode from
// here. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { z } from 'zod';

/** Every error code the backend can produce, wire-visible or internal. */
export const ERROR_CODES = [
  'VALIDATION_FAILED',
  'OTP_INVALID',
  'OTP_REPLAYED',
  'OTP_LOCKED',
  'UNAUTHENTICATED',
  'DOC_NOT_FOUND',
  'CHAT_NOT_FOUND',
  'UPLOAD_INVALID',
  'DAILY_CAP_REACHED',
  'AI_DISABLED',
  'UPSTREAM_UNAVAILABLE',
  // Internal only: an optimistic-lock write lost a race. Never on the wire (sent as INTERNAL).
  'VERSION_CONFLICT',
  'INTERNAL',
] as const;

/** One error code (`UPPER_SNAKE`). */
export type ErrorCode = (typeof ERROR_CODES)[number];

/** What a client does when it receives a code (contract "Client action"). */
type ClientAction =
  | 'fix-request'
  | 'retry-new-code'
  | 'wait-next-code'
  | 'wait'
  | 'sign-in'
  | 'resync'
  | 'tell-user'
  | 'retry';

/** A wire-visible code: its HTTP status and the client's one action. */
export interface WireError {
  readonly status: number;
  readonly action: ClientAction;
  /**
   * Default `Retry-After` seconds. A use case may override it per error with the
   * `retryAfterSeconds` detail (lock end, next UTC day).
   */
  readonly retryAfterSeconds?: number;
}

/** Codes that reach the wire as themselves. Any code missing here is sent as 500 `INTERNAL`. */
export const WIRE_ERRORS: Readonly<Partial<Record<ErrorCode, WireError>>> = {
  VALIDATION_FAILED: { status: 400, action: 'fix-request' },
  OTP_INVALID: { status: 401, action: 'retry-new-code' },
  OTP_REPLAYED: { status: 401, action: 'wait-next-code' },
  OTP_LOCKED: { status: 429, action: 'wait', retryAfterSeconds: 900 },
  UNAUTHENTICATED: { status: 401, action: 'sign-in' },
  DOC_NOT_FOUND: { status: 404, action: 'resync' },
  CHAT_NOT_FOUND: { status: 404, action: 'resync' },
  UPLOAD_INVALID: { status: 422, action: 'tell-user' },
  DAILY_CAP_REACHED: { status: 429, action: 'tell-user', retryAfterSeconds: 3600 },
  AI_DISABLED: { status: 503, action: 'tell-user', retryAfterSeconds: 300 },
  UPSTREAM_UNAVAILABLE: { status: 503, action: 'retry' },
  INTERNAL: { status: 500, action: 'retry' },
};

/** The detail key a use case sets to override the default `Retry-After` seconds. */
export const RETRY_AFTER_DETAIL = 'retryAfterSeconds';

/** Error envelope for every non-2xx response and the stream's `error` event. */
export const ErrorEnvelope = z
  .object({
    error: z
      .object({
        code: z.enum(ERROR_CODES),
        message: z.string().max(300),
        requestId: z.string().max(128),
      })
      .strict(),
  })
  .strict();

/** The error envelope type. */
export type ErrorEnvelope = z.infer<typeof ErrorEnvelope>;
