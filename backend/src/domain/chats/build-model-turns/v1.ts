// Builds the model turns (docs/contract.md "Model call"): the last MAX_HISTORY_TURNS messages as
// text, the attached docs as cited document blocks in the first user turn (cache breakpoint on the
// last doc), then the new question. Pure. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { Message, Role } from '../../model.js';
import type { AttachedDoc, ModelTurn, TurnBlock } from '../model-request.js';

/** Text sent for a stored message whose text is empty (as the original server did). */
const EMPTY_TEXT = '(no answer)';

interface TextTurn {
  readonly role: Role;
  readonly text: string;
}

/** Joins consecutive turns of the same role, so roles always alternate. */
const alternate = (turns: readonly TextTurn[]): TextTurn[] =>
  turns.reduce<TextTurn[]>((out, turn) => {
    const last = out[out.length - 1];
    if (last?.role === turn.role)
      out[out.length - 1] = { ...last, text: `${last.text}\n\n${turn.text}` };
    else out.push(turn);
    return out;
  }, []);

const docBlocks = (docs: readonly AttachedDoc[]): TurnBlock[] =>
  docs.map((doc, i) => ({
    kind: 'doc',
    title: doc.title,
    url: doc.url,
    isCacheBreakpoint: i === docs.length - 1,
  }));

/**
 * Builds the turns for one question.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param history - Stored messages of the chat, oldest first (before the new question).
 * @param question - The new question text.
 * @param docs - Ready attached docs, in the order the model will index them.
 * @param maxHistoryTurns - Most past messages sent (at least 0).
 * @returns Alternating turns that start and end with a user turn.
 */
export const buildModelTurns = (
  history: readonly Message[],
  question: string,
  docs: readonly AttachedDoc[],
  maxHistoryTurns: number,
): ModelTurn[] => {
  const window = maxHistoryTurns > 0 ? history.slice(-maxHistoryTurns) : [];
  const firstUser = window.findIndex((m) => m.role === 'user');
  const past = firstUser < 0 ? [] : window.slice(firstUser);
  const turns = alternate([
    ...past.map((m) => ({ role: m.role, text: m.text === '' ? EMPTY_TEXT : m.text })),
    { role: 'user', text: question },
  ]);
  return turns.map((turn, i) => ({
    role: turn.role,
    blocks: [...(i === 0 ? docBlocks(docs) : []), { kind: 'text', text: turn.text }],
  }));
};
