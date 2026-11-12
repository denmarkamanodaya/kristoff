//! The model prompts, the statute tool and the follow-up schema: the single source on the desktop
//! (phase-2 §2). The prompt texts are copied verbatim from `backend/src/contracts/model-config.ts`
//! and a contracts test pins them to that file. A prompt change is a behaviour change: it goes
//! through review with a benchmark run (stack/ai-features.md "Quality").

/// `max_tokens` of an answer.
pub const MAX_TOKENS: u32 = 4096;

/// The system prompt every answer is generated under.
pub const SYSTEM_PROMPT: &str = "You are Kristoff, a legal research assistant for a Philippine law student.

How to work:
- When documents are attached, ground every legal claim in them and cite the specific passage. If the answer is not in the documents, say so plainly before offering general knowledge, and label that part as general knowledge.
- Never invent case names, G.R. numbers, dates, ponentes, statutes or quotations. If unsure, say you are unsure.
- For case analysis, default to: Facts, Issue(s), Ruling, Ratio decidendi, and Notable doctrines. Use IRAC when the student asks for problem-solving.
- Be clear about which jurisdiction a rule comes from. Assume Philippine law unless told otherwise.
- Remind the student to verify important citations against the official source (SC E-Library, Lawphil, Official Gazette) when the stakes are high.
- Write clearly and concisely. Use headings and bullet points for digests.";

/// The follow-up model.
pub const FOLLOW_UP_MODEL: &str = "claude-haiku-5-5";

/// System prompt of the follow-up call.
pub const FOLLOW_UP_PROMPT: &str = "You suggest follow-up questions for a Philippine law student who is using a legal research assistant.
Given the student's question and the start of the assistant's answer, suggest up to 5 short, specific next questions the student is likely to ask next.
Each question must be under 120 characters, self-contained, and in the student's language. Do not repeat the original question.
Return only the JSON object the schema describes.";

/// `max_tokens` of the follow-up call.
pub const FOLLOW_UP_MAX_TOKENS: u32 = 400;

/// Timeout of the follow-up call in milliseconds.
pub const FOLLOW_UP_TIMEOUT_MS: u64 = 15_000;

/// JSON schema the follow-up call's output must follow (structured output).
pub const FOLLOW_UP_SCHEMA: &str = r#"{"type":"object","properties":{"followUps":{"type":"array","items":{"type":"string"}}},"required":["followUps"],"additionalProperties":false}"#;

/// Name of the statute search tool (Phase 4 offers it).
pub const STATUTE_TOOL_NAME: &str = "search_ph_laws";

/// The statute search tool definition (spike S5), as sent in `tools`.
pub const STATUTE_TOOL: &str = r#"{"name":"search_ph_laws","description":"Search Philippine statutes (Republic Acts, 1987 Constitution). Returns matching articles as citable search results.","input_schema":{"type":"object","properties":{"query":{"type":"string","maxLength":300},"law_number":{"type":"string","maxLength":40}},"required":["query"],"additionalProperties":false},"strict":true}"#;

#[cfg(test)]
mod tests {
    use super::{FOLLOW_UP_SCHEMA, STATUTE_TOOL, STATUTE_TOOL_NAME};

    #[test]
    fn the_schema_and_the_tool_are_valid_json_naming_the_tool() {
        let schema: serde_json::Value = serde_json::from_str(FOLLOW_UP_SCHEMA).unwrap();
        let tool: serde_json::Value = serde_json::from_str(STATUTE_TOOL).unwrap();
        assert_eq!(schema["required"][0], "followUps");
        assert_eq!(tool["name"], STATUTE_TOOL_NAME);
    }
}
