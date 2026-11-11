// Redacting structured-JSON logger (stack/observability.md). Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md (copied from dexfs).
// Redaction is an allowlist: only keys in LOG_FIELDS are emitted, everything else is dropped.
// Prompts, model output, transcripts, captions, query text and object keys are never on the list.

/** The only field names a log line may carry; a test pins this list. */
export const LOG_FIELDS = [
  'requestId',
  'operation',
  'outcome',
  'durationMs',
  'code',
  'principalHash',
  'stage',
  'model',
  'inputTokens',
  'outputTokens',
  'latencyMs',
  'count',
  'version',
  'http.route',
  'http.request.method',
  'http.response.status_code',
  'error.type',
  'trace_id',
  'step',
] as const;

/** One allowlisted log field name. */
type LogField = (typeof LOG_FIELDS)[number];

/** Field values a log line may carry: scalars only, never objects or bodies. */
type LogFields = Readonly<Partial<Record<LogField, string | number | boolean>>>;

/** Log severity, lowest first. `debug` is off in prod (the composition root sets the level). */
type LogLevel = 'debug' | 'info' | 'warn' | 'error';

/** Structured logger handed to adapters, handlers and workers. */
export interface Logger {
  /** Writes a line at `debug`. */
  debug(event: string, fields?: LogFields): void;
  /** Writes a line at `info` (outcome lines, lifecycle events). */
  info(event: string, fields?: LogFields): void;
  /** Writes a line at `warn` (degraded but handled). */
  warn(event: string, fields?: LogFields): void;
  /** Writes a line at `error` (a human must act; an alarm watches it). */
  error(event: string, fields?: LogFields): void;
}

const RANK: Readonly<Record<LogLevel, number>> = { debug: 10, info: 20, warn: 30, error: 40 };
const ALLOWED: ReadonlySet<string> = new Set(LOG_FIELDS);
const MAX_EVENT = 64;
const MAX_VALUE = 200;

/**
 * Keeps only allowlisted scalar fields, truncating long strings.
 * @param fields - Candidate fields (untrusted: callers may pass extra keys at runtime).
 * @returns A new object holding only allowlisted keys.
 */
export const redact = (fields: Readonly<Record<string, unknown>>): Record<string, unknown> => {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(fields)) {
    if (!ALLOWED.has(key)) continue;
    if (typeof value === 'string') out[key] = value.slice(0, MAX_VALUE);
    else if (typeof value === 'number' || typeof value === 'boolean') out[key] = value;
  }
  return out;
};

/** Options for {@link createLogger}. */
export interface LoggerOptions {
  /** Minimum level written. */
  readonly level: LogLevel;
  /** Sink for one serialised line (defaults to stdout). */
  readonly write?: (line: string) => void;
}

const stdoutWrite = (line: string): void => {
  process.stdout.write(`${line}\n`);
};

/**
 * Builds a JSON-lines logger that applies the allowlist to every line.
 * @param options - Minimum level and optional sink.
 * @returns The logger.
 */
export const createLogger = (options: LoggerOptions): Logger => {
  const write = options.write ?? stdoutWrite;
  const emit =
    (level: LogLevel) =>
    (event: string, fields: LogFields = {}): void => {
      if (RANK[level] < RANK[options.level]) return;
      write(JSON.stringify({ level, event: event.slice(0, MAX_EVENT), ...redact(fields) }));
    };
  return { debug: emit('debug'), info: emit('info'), warn: emit('warn'), error: emit('error') };
};
