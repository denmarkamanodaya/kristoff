//! The model prompts, the statute tools and the follow-up schema: the single source on the desktop
//! (phase-2 §2, phase-4 §1 and §3). The shared prompt texts are copied verbatim from
//! `backend/src/contracts/model-config.ts` and a contracts test pins them to that file; the
//! desktop-only statute addition is a separate constant. A prompt change is a behaviour change: it goes
//! through review with a benchmark run (stack/ai-features.md "Quality").

use kristoff_core::limits::{
    TOOL_ARTICLE_MAX, TOOL_LAW_NUMBER_MAX, TOOL_QUERY_MAX, TOOL_RESULTS_MAX, TOOL_YEAR_MAX,
    TOOL_YEAR_MIN,
};

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

/// The desktop addition to [`SYSTEM_PROMPT`] for chats with statute search on (phase-4 §3, D-14).
/// Desktop only, so the backend parity test does not cover it.
pub const STATUTE_PROMPT: &str = "Statute search (the search_ph_laws and get_law_article tools):
- The indexed laws are Republic Acts and the 1987 Constitution only. They do not include the Revised Penal Code, the Family Code, the Labor Code or the Rules of Court.
- Never state what a provision or a court ruling says unless it comes from a tool result or an attached document in this conversation. This applies to the Revised Penal Code, the Family Code, the Labor Code, the Rules of Court and to court rulings you remember. For such a law you may name it and say: \"not in the indexed laws — upload the code as a PDF to cite it\".
- Every sentence that states what a provision says carries a citation. When no indexed law supports a claim, say \"not found in the indexed laws\".
- Scope notes, caveats and follow-up offers must not state statutory content.
- When a result is marked [Repealed], [Amended], [Partially repealed] or [Declared unconstitutional], say so.
- The index is an interim snapshot (about 1,400 laws are missing), and the Official Gazette text prevails.
- Search at most 3 times per question.";

/// The system prompt of an answer: the shared base, plus [`STATUTE_PROMPT`] when the law tools
/// are offered.
#[must_use]
pub fn answer_system_prompt(statutes_on: bool) -> String {
    if statutes_on {
        format!("{SYSTEM_PROMPT}\n\n{STATUTE_PROMPT}")
    } else {
        SYSTEM_PROMPT.to_owned()
    }
}

/// Name of the statute search tool.
pub const SEARCH_TOOL_NAME: &str = "search_ph_laws";

/// Name of the article lookup tool.
pub const ARTICLE_TOOL_NAME: &str = "get_law_article";

/// The two statute tools as sent in `tools` (the one definition; the bounds are core's limits,
/// which the ask use case also enforces). Integer ranges are stated in the descriptions, not as
/// schema keywords, because strict tool schemas reject numeric constraints.
#[must_use]
pub fn law_tools() -> serde_json::Value {
    let year = format!("year, {TOOL_YEAR_MIN}-{TOOL_YEAR_MAX}.");
    serde_json::json!([
        {
            "name": SEARCH_TOOL_NAME,
            "description": "Search Philippine statutes (Republic Acts, 1987 Constitution). Returns matching articles as citable search results.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "maxLength": TOOL_QUERY_MAX },
                    "law_number": { "type": "string", "maxLength": TOOL_LAW_NUMBER_MAX },
                    "year_from": { "type": "integer", "description": format!("Earliest approval {year}") },
                    "year_to": { "type": "integer", "description": format!("Latest approval {year}") },
                    "limit": { "type": "integer", "description": format!("Most results, 1-{TOOL_RESULTS_MAX} (default {TOOL_RESULTS_MAX}).") }
                },
                "required": ["query"],
                "additionalProperties": false
            },
            "strict": true
        },
        {
            "name": ARTICLE_TOOL_NAME,
            "description": "Get one provision of a Philippine statute, e.g. law_number \"RA 9262\" and article \"Sec. 26\", with the provisions around it, as citable search results.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "law_number": { "type": "string", "maxLength": TOOL_LAW_NUMBER_MAX },
                    "article": { "type": "string", "maxLength": TOOL_ARTICLE_MAX }
                },
                "required": ["law_number", "article"],
                "additionalProperties": false
            },
            "strict": true
        }
    ])
}

#[cfg(test)]
mod tests {
    use super::{FOLLOW_UP_SCHEMA, SEARCH_TOOL_NAME, answer_system_prompt, law_tools};

    #[test]
    fn the_schema_is_valid_json_and_the_tools_are_named_once() {
        let schema: serde_json::Value = serde_json::from_str(FOLLOW_UP_SCHEMA).unwrap();
        assert_eq!(schema["required"][0], "followUps");
        let tools = law_tools();
        assert_eq!(tools[0]["name"], SEARCH_TOOL_NAME);
        assert_eq!(tools.as_array().unwrap().len(), 2);
    }

    #[test]
    fn the_statute_addition_is_sent_only_with_the_tools() {
        assert!(
            answer_system_prompt(true).contains("Republic Acts and the 1987 Constitution only")
        );
        assert!(!answer_system_prompt(false).contains("search_ph_laws"));
    }
}
