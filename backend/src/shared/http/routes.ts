// Route shapes every HTTP handler is one of (driving-adapter plumbing, handlers only): list with
// `?cursor`, `{id}` path, JSON body, or path + body. Each parses with the contract schema before
// the use case runs, so a handler is only its use case call. Contract: docs/contract.md "Routes".
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { z } from 'zod';
import { IdPath, ListQuery } from '../../contracts/requests.js';
import type { Principal } from '../../domain/model.js';
import type { AppError } from '../errors/app-error.js';
import { ok, type Result } from '../result/result.js';
import {
  parseJson,
  parseValue,
  runRoute,
  type HttpDeps,
  type HttpEvent,
  type HttpResult,
  type RouteSuccess,
} from './http.js';

/** A wired use case as a handler sees it: arguments in, a typed result out. */
export type UseCase<A extends readonly unknown[], T> = (...args: A) => Promise<Result<T, AppError>>;

/** What a route body returns. */
type Reply = Promise<Result<RouteSuccess, AppError>>;

/** A handler for one route. */
type Handler = (event: HttpEvent) => Promise<HttpResult>;

/** Parses the JSON body (an absent body counts as `{}`) with the route's schema. */
const bodyOf = <S extends z.ZodType>(event: HttpEvent, schema: S) =>
  parseJson(event.body ?? '{}', event.isBase64Encoded, schema);

/**
 * Shapes a use-case result as the 200 body.
 * @param result - The use case's result.
 * @param shape - Wraps the value in the contract's body (identity when the value is the body).
 * @returns The route success, or the error unchanged.
 */
export const toBody = <T>(
  result: Result<T, AppError>,
  shape: (value: T) => unknown = (value) => value,
): Result<RouteSuccess, AppError> => (result.ok ? ok({ body: shape(result.value) }) : result);

/**
 * A `GET` list route: parses `?cursor`.
 * @param deps - Plumbing.
 * @param operation - Route label.
 * @param run - Calls the use case with the principal and cursor.
 * @returns The handler.
 */
export const listRoute =
  (
    deps: HttpDeps,
    operation: string,
    run: (p: Principal, cursor: string | undefined) => Reply,
  ): Handler =>
  (event) =>
    runRoute(deps, operation, event, async (principal) => {
      const query = parseValue(event.queryStringParameters, ListQuery, 'query');
      return query.ok ? run(principal, query.value.cursor) : query;
    });

/**
 * A route on `{id}`: parses the 16-hex id.
 * @param deps - Plumbing.
 * @param operation - Route label.
 * @param run - Calls the use case with the principal and id.
 * @returns The handler.
 */
export const pathRoute =
  (deps: HttpDeps, operation: string, run: (p: Principal, id: string) => Reply): Handler =>
  (event) =>
    runRoute(deps, operation, event, async (principal) => {
      const path = parseValue(event.pathParameters, IdPath, 'path');
      return path.ok ? run(principal, path.value.id) : path;
    });

/**
 * A route with a JSON body.
 * @param deps - Plumbing.
 * @param operation - Route label.
 * @param schema - The strict request schema.
 * @param run - Calls the use case with the principal and parsed body.
 * @returns The handler.
 */
export const bodyRoute =
  <S extends z.ZodType>(
    deps: HttpDeps,
    operation: string,
    schema: S,
    run: (p: Principal, body: z.infer<S>) => Reply,
  ): Handler =>
  (event) =>
    runRoute(deps, operation, event, async (principal) => {
      const body = bodyOf(event, schema);
      return body.ok ? run(principal, body.value) : body;
    });

/**
 * A route on `{id}` with a JSON body.
 * @param deps - Plumbing.
 * @param operation - Route label.
 * @param schema - The strict request schema.
 * @param run - Calls the use case with the principal, id and parsed body.
 * @returns The handler.
 */
export const pathBodyRoute =
  <S extends z.ZodType>(
    deps: HttpDeps,
    operation: string,
    schema: S,
    run: (p: Principal, id: string, body: z.infer<S>) => Reply,
  ): Handler =>
  (event) =>
    pathRoute(deps, operation, async (principal, id) => {
      const body = bodyOf(event, schema);
      return body.ok ? run(principal, id, body.value) : body;
    })(event);
