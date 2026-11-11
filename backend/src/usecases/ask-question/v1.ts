// POST {STREAM_URL}/v1/chats/{id}/ask use case (docs/contract.md "Stream", "Model call"). Order:
// kill switch, chat lookup, daily meter (one conditional write) BEFORE the model call; then the
// user message is stored, the answer streams, and the assistant message is stored even when the
// model fails midway. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { buildModelTurns } from '../../domain/chats/build-model-turns/index.js';
import { mapCitation } from '../../domain/chats/map-citation/index.js';
import type { AttachedDoc, AnswerEnd } from '../../domain/chats/model-request.js';
import {
  AUTO_TITLE_CHARS,
  MAX_CITATIONS,
  MAX_MESSAGE_TEXT,
  NEW_CHAT_TITLE,
} from '../../domain/limits.js';
import type {
  Chat,
  Citation,
  FollowUps,
  Message,
  Principal,
  Usage,
  Versioned,
} from '../../domain/model.js';
import { secondsUntilNextUsageDay, usageDay } from '../../domain/usage/usage-day/index.js';
import { RETRY_AFTER_DETAIL } from '../../contracts/errors.js';
import type { AnswerSink } from '../../ports/answer-sink.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../shared/result/result.js';
import { docObjectKey, docPartKey } from '../../domain/keys/index.js';
import type { AttachmentTarget } from '../../domain/docs/attachment-target/index.js';
import { loadAttachments } from '../load-attachments/index.js';
import type { AnswerModel } from '../../ports/answer-model.js';
import type { FollowUpSuggester } from '../../ports/follow-up-suggester.js';
import { sumUsage } from '../../domain/usage/sum-usage/index.js';
import type { ChatStore } from '../../ports/chat-store.js';
import type { Clock } from '../../ports/clock.js';
import type { DocStore } from '../../ports/doc-store.js';
import type { FileStore } from '../../ports/file-store.js';
import type { FlagReader } from '../../ports/flag-reader.js';
import type { IdGenerator } from '../../ports/id-generator.js';
import type { UsageMeter } from '../../ports/usage-meter.js';
import { FOLLOW_UP_ANSWER_CHARS, MODEL_DOC_URL_TTL_SECONDS } from '../../domain/limits.js';
import { loadOwnedChat } from '../load-owned-chat/index.js';

/** Model settings from the env (`MODEL_IDS`, `MAX_TOKENS`, `MAX_HISTORY_TURNS`, `DAILY_QUESTION_CAP`). */
interface AskConfig {
  /** Allowed model ids, first = default (docs/contract.md "Model choice"). */
  readonly modelIds: readonly [string, ...string[]];
  readonly maxTokens: number;
  readonly maxHistoryTurns: number;
  readonly dailyQuestionCap: number;
}

/** What ask-question needs. */
export interface AskQuestionDeps {
  readonly chats: ChatStore;
  readonly docs: DocStore;
  readonly files: FileStore;
  readonly model: AnswerModel;
  readonly followUps: FollowUpSuggester;
  readonly meter: UsageMeter;
  readonly flags: FlagReader;
  readonly ids: IdGenerator;
  readonly clock: Clock;
  readonly config: AskConfig;
}

/** The model document for one attachment: whole doc, or one part titled with its page range. */
const toAttached = async (
  deps: AskQuestionDeps,
  principal: Principal,
  target: AttachmentTarget,
): Promise<Result<AttachedDoc, AppError>> => {
  const { doc } = target;
  const key =
    target.kind === 'whole'
      ? docObjectKey(principal.uid, doc.id)
      : docPartKey(principal.uid, doc.id, target.part.n);
  const url = await deps.files.presignGet(key, MODEL_DOC_URL_TTL_SECONDS);
  if (!url.ok) return url;
  const base = { id: doc.id, libraryTitle: doc.title, url: url.value.url };
  if (target.kind === 'whole') return ok({ ...base, title: doc.title, pageOffset: 0 });
  const { startPage, endPage } = target.part;
  const title = `${doc.title} (pp. ${String(startPage)}\u2013${String(endPage)})`;
  return ok({ ...base, title, pageOffset: startPage - 1 });
};

/**
 * The chat's usable attachments (whole docs and parts), each with a 900 s presigned GET, in the
 * chat's order. Attachments that are no longer usable (deleted, failed) are skipped.
 */
const attachDocs = async (
  deps: AskQuestionDeps,
  principal: Principal,
  attachmentIds: readonly string[],
): Promise<Result<AttachedDoc[], AppError>> => {
  const targets = await loadAttachments(deps.docs, principal.uid, attachmentIds);
  if (!targets.ok) return targets;
  const attached: AttachedDoc[] = [];
  for (const target of targets.value) {
    if (target === undefined) continue;
    const doc = await toAttached(deps, principal, target);
    if (!doc.ok) return doc;
    attached.push(doc.value);
  }
  return ok(attached);
};

/** The stored answer and the call's token usage (the `done` event). */
export interface AskOutcome {
  readonly message: Message;
  readonly usage: Usage;
}

const CUT_OFF_NOTE = '\n\n_(Answer cut off at the length limit. Ask "continue" for the rest.)_';
const FAILED_NOTE = '\n\n_(The answer was interrupted. Please ask again.)_';

/** One question: its text and the model the client chose (omitted = the default). */
export interface Question {
  readonly text: string;
  readonly model?: string | undefined;
}

/** The requested model if it is allowed, the default when none was asked for; else VALIDATION_FAILED. */
const chooseModel = (
  allowed: readonly [string, ...string[]],
  requested: string | undefined,
): Result<string, AppError> => {
  if (requested === undefined) return ok(allowed[0]);
  return allowed.includes(requested)
    ? ok(requested)
    : err(appError('VALIDATION_FAILED', 'this model is not available'));
};

const admit = async (
  deps: AskQuestionDeps,
  principal: Principal,
  chatId: string,
): Promise<Result<Versioned<Chat>, AppError>> => {
  const enabled = await deps.flags.isAiEnabled();
  if (!enabled.ok) return enabled;
  if (!enabled.value) return err(appError('AI_DISABLED', 'answers are switched off'));
  const chat = await loadOwnedChat(deps.chats, principal, chatId);
  if (!chat.ok) return chat;
  const now = deps.clock.now();
  const admitted = await deps.meter.admitQuestion(
    principal.uid,
    usageDay(now),
    deps.config.dailyQuestionCap,
  );
  if (admitted.ok) return chat;
  return admitted.error.code === 'DAILY_CAP_REACHED'
    ? err(
        appError('DAILY_CAP_REACHED', 'daily question limit reached', {
          [RETRY_AFTER_DETAIL]: secondsUntilNextUsageDay(now),
        }),
      )
    : admitted;
};

/** The chat title from a first question: its first 60 characters (contract Clarifications). */
const autoTitle = (text: string): string =>
  text.trim().slice(0, AUTO_TITLE_CHARS) || NEW_CHAT_TITLE;

const storeQuestion = async (
  deps: AskQuestionDeps,
  principal: Principal,
  chat: Versioned<Chat>,
  question: { readonly text: string; readonly newTitle: string | undefined },
): Promise<Result<void, AppError>> => {
  const { text, newTitle } = question;
  const at = new Date(deps.clock.now()).toISOString();
  const message: Message = {
    id: deps.ids.newUlid(),
    role: 'user',
    text,
    citations: [],
    usage: null,
    model: null,
    stopReason: null,
    at,
    durationMs: null,
    followUps: [],
  };
  const stored = await deps.chats.putMessage(chat.value.id, message);
  if (!stored.ok) return stored;
  const title = newTitle === undefined ? {} : { title: newTitle };
  const updated = await deps.chats.updateChat(
    principal.uid,
    chat.value.id,
    { ...title, updatedAt: at },
    chat.version,
  );
  return updated.ok ? ok(undefined) : updated;
};

interface Draft {
  text: string;
  readonly citations: Citation[];
}

/** A finished (or failed) model call: the accumulated answer, how it ended, the model used. */
interface Answer {
  readonly draft: Draft;
  readonly end: Result<AnswerEnd, AppError>;
  readonly model: string;
}

const streamAnswer = async (
  deps: AskQuestionDeps,
  call: { readonly model: string; readonly turns: Parameters<typeof buildModelTurns> },
  docs: readonly AttachedDoc[],
  sink: AnswerSink,
): Promise<Answer> => {
  const draft: Draft = { text: '', citations: [] };
  const request = {
    model: call.model,
    maxTokens: deps.config.maxTokens,
    turns: buildModelTurns(...call.turns),
  };
  sink.open();
  const end = await deps.model.stream(request, (event) => {
    if (event.kind === 'text') {
      draft.text += event.text;
      sink.text(event.text);
    } else if (draft.citations.length < MAX_CITATIONS) {
      const citation = mapCitation(event.source, docs, draft.citations.length + 1);
      draft.citations.push(citation);
      draft.text += ` [${String(citation.n)}]`;
      sink.citation(citation);
    }
  });
  return { draft, end, model: call.model };
};

/** The request a finished answer belongs to. */
interface AnswerContext {
  readonly principal: Principal;
  readonly chatId: string;
  readonly question: string;
  /** Clock reading when the request started (epoch ms). */
  readonly startedMs: number;
  readonly sink: AnswerSink;
}

/** Suggested next questions for a successful answer (none after a failed one), sent to the sink. */
const suggestFollowUps = async (
  deps: AskQuestionDeps,
  ctx: AnswerContext,
  answer: Answer,
): Promise<FollowUps> => {
  if (!answer.end.ok) return { questions: [], usage: null };
  const answerExcerpt = answer.draft.text.slice(0, FOLLOW_UP_ANSWER_CHARS);
  const suggested = await deps.followUps.suggest({ question: ctx.question, answerExcerpt });
  ctx.sink.followUps(suggested.questions);
  return suggested;
};

const storeAnswer = async (
  deps: AskQuestionDeps,
  ctx: AnswerContext,
  answer: Answer,
): Promise<Result<AskOutcome, AppError>> => {
  const { draft, end } = answer;
  const followUps = await suggestFollowUps(deps, ctx, answer);
  const { principal, chatId } = ctx;
  const note = !end.ok ? FAILED_NOTE : end.value.stopReason === 'max_tokens' ? CUT_OFF_NOTE : '';
  const message: Message = {
    id: deps.ids.newUlid(),
    role: 'assistant',
    text: `${draft.text}${note}`.slice(0, MAX_MESSAGE_TEXT),
    citations: draft.citations,
    usage: end.ok ? end.value.usage : null,
    model: answer.model,
    stopReason: end.ok ? end.value.stopReason : 'error',
    at: new Date(deps.clock.now()).toISOString(),
    durationMs: Math.max(0, deps.clock.now() - ctx.startedMs),
    followUps: followUps.questions,
  };
  const stored = await deps.chats.putMessage(chatId, message);
  if (!stored.ok) return stored;
  if (!end.ok) return end;
  const counted = await deps.meter.addTokens(
    principal.uid,
    usageDay(deps.clock.now()),
    sumUsage(end.value.usage, followUps.usage),
  );
  return counted.ok ? ok({ message, usage: end.value.usage }) : counted;
};

/**
 * Builds the ask-question use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Stores, file store, model, meter, kill switch, ids, clock and model config.
 * @returns A function from the principal, chat id, question and sink to the stored answer. Before
 *   `sink.open()`, in this order: `VALIDATION_FAILED` (model not in `MODEL_IDS`), `AI_DISABLED`, `CHAT_NOT_FOUND`, `DAILY_CAP_REACHED` (with the seconds to the
 *   next UTC day) or a store failure. After it: `UPSTREAM_UNAVAILABLE` or a store failure, with
 *   both messages already stored when the model failed.
 */
export const askQuestion =
  (deps: AskQuestionDeps) =>
  async (
    principal: Principal,
    chatId: string,
    question: Question,
    sink: AnswerSink,
  ): Promise<Result<AskOutcome, AppError>> => {
    const startedMs = deps.clock.now();
    const { text } = question;
    const model = chooseModel(deps.config.modelIds, question.model);
    if (!model.ok) return model;
    const chat = await admit(deps, principal, chatId);
    if (!chat.ok) return chat;
    // At least one message is read, so "no messages yet" is known even when no history is sent.
    const history = await deps.chats.recentMessages(
      chatId,
      Math.max(1, deps.config.maxHistoryTurns),
    );
    if (!history.ok) return history;
    const docs = await attachDocs(deps, principal, chat.value.value.docIds);
    if (!docs.ok) return docs;
    const newTitle = history.value.length === 0 ? autoTitle(text) : undefined;
    const asked = await storeQuestion(deps, principal, chat.value, { text, newTitle });
    if (!asked.ok) return asked;
    const turns: Parameters<typeof buildModelTurns> = [
      history.value,
      text,
      docs.value,
      deps.config.maxHistoryTurns,
    ];
    const answer = await streamAnswer(deps, { model: model.value, turns }, docs.value, sink);
    return storeAnswer(deps, { principal, chatId, question: text, startedMs, sink }, answer);
  };
