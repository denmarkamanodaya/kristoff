// HTTP plumbing for the API Gateway handlers (driving adapters): the authorizer's principal, body
// cap, Zod parse, the error envelope and the one outcome log line. Adapted from dexfs. Contract:
// docs/contract.md "Error envelope and codes"; stack/api-design.md; stack/observability.md.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md. Only handlers import this file.
import type {
  APIGatewayProxyEventV2WithLambdaAuthorizer,
  APIGatewayProxyStructuredResultV2,
} from 'aws-lambda';
import type { z } from 'zod';
import {
  RETRY_AFTER_DETAIL,
  WIRE_ERRORS,
  type ErrorEnvelope,
  type WireError,
} from '../../contracts/errors.js';
import { MAX_BODY_BYTES } from '../../contracts/requests.js';
import type { Principal } from '../../domain/model.js';
import { appError, type AppError } from '../errors/app-error.js';
import { sha256Hex } from '../hash/sha256.js';
import type { Logger } from '../log/logger.js';
import { err, ok, type Result } from '../result/result.js';

/** The context the Lambda authorizer passes (simple response `context`). */
interface AuthorizerContext {
  readonly sub?: unknown;
}

/** An HTTP API v2 event behind the Lambda authorizer (or none, for the public route). */
export type HttpEvent = APIGatewayProxyEventV2WithLambdaAuthorizer<AuthorizerContext>;

/** A structured Lambda proxy result. */
export type HttpResult = APIGatewayProxyStructuredResultV2;

/** What a route returns on success: always 200 with a contract-shaped body. */
export interface RouteSuccess {
  readonly body: unknown;
}

/** Logger and time source every handler gets from its composition root. */
export interface HttpDeps {
  readonly log: Logger;
  readonly now: () => number;
}

/** A shaped error: status, headers and the envelope body. */
export interface ErrorReply {
  readonly status: number;
  readonly headers: Record<string, string>;
  readonly envelope: ErrorEnvelope;
}

/** What a handler reads from an event, each part optional because the event is untrusted. */
interface LooseEvent {
  readonly routeKey?: unknown;
  readonly requestContext?: {
    readonly requestId?: unknown;
    readonly authorizer?: { readonly lambda?: AuthorizerContext | null } | null;
  } | null;
}

const loose = (event: unknown): LooseEvent =>
  typeof event === 'object' && event !== null ? event : {};

const text = (value: unknown, fallback: string): string =>
  typeof value === 'string' && value.length > 0 ? value.slice(0, 128) : fallback;

/**
 * The gateway request id, or `unknown` for a direct invoke.
 * @param event - The event as received.
 * @returns At most 128 characters.
 */
export const requestIdOf = (event: unknown): string =>
  text(loose(event).requestContext?.requestId, 'unknown');

/**
 * The verified caller from the authorizer context (never the body, query or a header). A missing
 * or malformed `sub` is `UNAUTHENTICATED`: fail closed.
 * @param event - The event as received.
 * @returns The principal or `UNAUTHENTICATED`.
 */
const principalOf = (event: HttpEvent): Result<Principal, AppError> => {
  const sub = loose(event).requestContext?.authorizer?.lambda?.sub;
  return typeof sub === 'string' && /^[A-Za-z0-9_-]{1,64}$/.test(sub)
    ? ok({ uid: sub })
    : err(appError('UNAUTHENTICATED', 'sign in again'));
};

/**
 * Parses a JSON body with its contract schema after checking its size.
 * @param body - Raw body text (`undefined` when none was sent).
 * @param isBase64 - Whether the gateway base64-encoded the body.
 * @param schema - The route's strict request schema.
 * @returns The parsed body, or `VALIDATION_FAILED`.
 */
export const parseJson = <S extends z.ZodType>(
  body: string | undefined,
  isBase64: boolean,
  schema: S,
): Result<z.infer<S>, AppError> => {
  const raw = isBase64 ? Buffer.from(body ?? '', 'base64').toString('utf8') : (body ?? '');
  if (Buffer.byteLength(raw, 'utf8') > MAX_BODY_BYTES) {
    return err(appError('VALIDATION_FAILED', 'request body too large'));
  }
  let json: unknown;
  try {
    json = JSON.parse(raw) as unknown;
  } catch {
    return err(appError('VALIDATION_FAILED', 'body is not JSON'));
  }
  return parseValue(json, schema, 'body');
};

/**
 * Parses an already-decoded value (path or query parameters) with a contract schema.
 * @param value - The untrusted value (`undefined` becomes `{}`).
 * @param schema - A strict schema.
 * @param label - Names the part in the error message.
 * @returns The parsed value, or `VALIDATION_FAILED`.
 */
export const parseValue = <S extends z.ZodType>(
  value: unknown,
  schema: S,
  label: string,
): Result<z.infer<S>, AppError> => {
  const parsed = schema.safeParse(value ?? {});
  return parsed.success
    ? ok(parsed.data)
    : err(appError('VALIDATION_FAILED', `${label} does not match the contract`));
};

/**
 * Shapes a typed error. Codes without a wire entry go out as 500 `INTERNAL`.
 * @param error - The typed error.
 * @param requestId - Echoed in the body and `x-request-id`.
 * @returns Status, headers (`Retry-After` when the contract documents one) and envelope.
 */
export const errorReply = (error: AppError, requestId: string): ErrorReply => {
  const wire = WIRE_ERRORS[error.code];
  const code = wire ? error.code : 'INTERNAL';
  const entry: WireError = wire ?? { status: 500, action: 'retry' };
  const message = wire ? error.message.slice(0, 300) : 'internal error';
  const override = wire ? error.details?.[RETRY_AFTER_DETAIL] : undefined;
  const retry = typeof override === 'number' ? override : entry.retryAfterSeconds;
  return {
    status: entry.status,
    headers: {
      'x-request-id': requestId,
      ...(retry === undefined ? {} : { 'retry-after': String(retry) }),
    },
    envelope: { error: { code, message, requestId } },
  };
};

const json = (status: number, body: unknown, headers: Record<string, string>): HttpResult => ({
  statusCode: status,
  headers: { 'content-type': 'application/json', ...headers },
  body: JSON.stringify(body),
});

/** The route label and the verified caller (absent on the public route or a refused call). */
interface RouteMeta {
  readonly operation: string;
  readonly principal?: Principal;
}

const respond = async (
  deps: HttpDeps,
  meta: RouteMeta,
  event: HttpEvent,
  route: () => Promise<Result<RouteSuccess, AppError>>,
): Promise<HttpResult> => {
  const started = deps.now();
  const requestId = requestIdOf(event);
  const outcome = await route();
  const reply = outcome.ok ? undefined : errorReply(outcome.error, requestId);
  const response = reply
    ? json(reply.status, reply.envelope, reply.headers)
    : json(200, outcome.ok ? outcome.value.body : null, { 'x-request-id': requestId });
  deps.log.info('http.request', {
    operation: meta.operation,
    requestId,
    outcome: outcome.ok ? 'ok' : outcome.error.code,
    'http.route': text(loose(event).routeKey, ''),
    'http.response.status_code': response.statusCode ?? 0,
    durationMs: deps.now() - started,
    ...(meta.principal ? { principalHash: sha256Hex(meta.principal.uid).slice(0, 16) } : {}),
  });
  return response;
};

/**
 * Runs a route behind the authorizer: resolves the principal, calls `route`, shapes the result
 * and writes exactly one outcome line.
 * @param deps - Logger and time source.
 * @param operation - Route label for logs.
 * @param event - The API Gateway event.
 * @param route - Parses, calls one use case and shapes the success body.
 * @returns The HTTP response.
 */
export const runRoute = (
  deps: HttpDeps,
  operation: string,
  event: HttpEvent,
  route: (principal: Principal) => Promise<Result<RouteSuccess, AppError>>,
): Promise<HttpResult> => {
  const principal = principalOf(event);
  return principal.ok
    ? respond(deps, { operation, principal: principal.value }, event, () => route(principal.value))
    : respond(deps, { operation }, event, () => Promise.resolve(principal));
};

/**
 * Runs the one public route (`POST /v1/auth/verify`): no principal, same envelope and log line.
 * @param deps - Logger and time source.
 * @param operation - Route label for logs.
 * @param event - The API Gateway event.
 * @param route - Parses, calls one use case and shapes the success body.
 * @returns The HTTP response.
 */
export const runPublicRoute = (
  deps: HttpDeps,
  operation: string,
  event: HttpEvent,
  route: () => Promise<Result<RouteSuccess, AppError>>,
): Promise<HttpResult> => respond(deps, { operation }, event, route);
