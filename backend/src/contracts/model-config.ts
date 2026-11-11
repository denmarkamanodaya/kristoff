// Model prompt and inference parameters (docs/contract.md "Model call"; stack/ai-features.md).
// The allowed model ids and max tokens are per-stage config (env `MODEL_IDS`, `MAX_TOKENS`); the prompt text
// is copied verbatim from the original server.js SYSTEM_PROMPT. A prompt change is a behaviour
// change: it goes through review with a benchmark run (stack/ai-features.md "Quality").

/** The system prompt every answer is generated under. */
export const SYSTEM_PROMPT = `You are Kristoff, a legal research assistant for a Philippine law student.

How to work:
- When documents are attached, ground every legal claim in them and cite the specific passage. If the answer is not in the documents, say so plainly before offering general knowledge, and label that part as general knowledge.
- Never invent case names, G.R. numbers, dates, ponentes, statutes or quotations. If unsure, say you are unsure.
- For case analysis, default to: Facts, Issue(s), Ruling, Ratio decidendi, and Notable doctrines. Use IRAC when the student asks for problem-solving.
- Be clear about which jurisdiction a rule comes from. Assume Philippine law unless told otherwise.
- Remind the student to verify important citations against the official source (SC E-Library, Lawphil, Official Gazette) when the stakes are high.
- Write clearly and concisely. Use headings and bullet points for digests.`;

/** SDK retries per call (one layer owns retries: the adapter). Kept low: the stream Lambda has 300 s. */
export const MODEL_MAX_RETRIES = 1;

/** Labels the web shows per model id (docs/contract.md "Model choice"); an id without one shows the id. */
export const MODEL_LABELS: Readonly<Record<string, string>> = {
  'claude-sonnet-5-5': 'Sonnet 5.5',
  'claude-opus-5-5': 'Opus 5.5',
  'claude-haiku-5-5': 'Haiku 5.5',
};

/** System prompt of the follow-up call (docs/contract.md "v2 Answer extras"). */
export const FOLLOW_UP_PROMPT = `You suggest follow-up questions for a Philippine law student who is using a legal research assistant.
Given the student's question and the start of the assistant's answer, suggest up to 5 short, specific next questions the student is likely to ask next.
Each question must be under 120 characters, self-contained, and in the student's language. Do not repeat the original question.
Return only the JSON object the schema describes.`;

/** max_tokens of the follow-up call. */
export const FOLLOW_UP_MAX_TOKENS = 400;

/** Timeout of the follow-up call in milliseconds. */
export const FOLLOW_UP_TIMEOUT_MS = 15_000;

/** JSON schema the follow-up call's output must follow (structured output). */
export const FOLLOW_UP_SCHEMA = {
  type: 'object',
  properties: { followUps: { type: 'array', items: { type: 'string' } } },
  required: ['followUps'],
  additionalProperties: false,
} as const;
