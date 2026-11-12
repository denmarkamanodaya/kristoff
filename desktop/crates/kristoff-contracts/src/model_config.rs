//! The model prompts, the statute tools and the follow-up schema: the single source on the desktop
//! (phase-2 §2, phase-4 §1 and §3). The shared prompt texts are copied verbatim from
//! `backend/src/contracts/model-config.ts` and a contracts test pins them to that file; the
//! desktop-only statute addition is a separate constant. A prompt change is a behaviour change: it goes
//! through review with a benchmark run (stack/ai-features.md "Quality").

use kristoff_core::limits::{
    TOOL_ARTICLE_MAX, TOOL_CASE_MAX, TOOL_FOCUS_MAX, TOOL_LAW_NUMBER_MAX, TOOL_QUERY_MAX,
    TOOL_RESULTS_MAX, TOOL_YEAR_MAX, TOOL_YEAR_MIN,
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

/// The desktop addition to [`SYSTEM_PROMPT`] when the statute tools are offered (phase-4 §3,
/// D-14, phase-7 "Rules for answers"). Desktop only, so the backend parity test does not cover it.
pub const STATUTE_PROMPT: &str = "Statute search (the search_ph_laws and get_law_article tools):
- The indexed laws are Republic Acts, the 1987 Constitution, and these codes as originally enacted: the Revised Penal Code (Act No. 3815), the Family Code (Executive Order No. 209), the Labor Code (Presidential Decree No. 442) and the Rules of Court. Some laws are missing, and the Official Gazette text prevails.
- A result marked [As enacted], [As promulgated] or [As issued] is the original text, not the current one. Say so, and search for the Republic Acts that amended it (for example RA 8353 for rape, RA 10951 for Revised Penal Code fines, and the renumbered Labor Code). Never state a current article number, penalty or fine from memory.
- Labor Code article numbers in the index are the 1974 original numbering (P.D. No. 442 as enacted). When a user cites a Labor Code article number, do not assume it matches: find the provision by subject with search_ph_laws, say which numbering the result uses, and note that current references use the renumbered edition. Revised Penal Code articles moved too: rape is Art. 335 in the 1930 text and Art. 266-A after RA 8353.
- Never state what a provision says unless it comes from a tool result or an attached document in this conversation. For a law that is not indexed you may name it and say: \"not in the indexed laws — upload it as a PDF to cite it\".
- Every sentence that states what a provision says carries a citation. When no indexed law supports a claim, say \"not found in the indexed laws\".
- Scope notes, caveats and follow-up offers must not state statutory content.
- When a result is marked [Repealed], [Amended], [Partially repealed] or [Declared unconstitutional], say so.";

/// The desktop addition when the case tools are offered (phase-7 "Rules for answers").
pub const CASE_PROMPT: &str = "Supreme Court decisions (the search_ph_cases and get_case tools):
- The indexed decisions run from 1996 to April 2024. Cite a decision by its G.R. number and short title, and quote only from tool results.
- Name the parties as the decision does, including the Court's own pseudonyms such as AAA or XXX.
- Never state what a court ruled from memory. When no indexed decision supports a claim, say \"not found in the indexed decisions\".";

/// The last line of the tool rules, whatever tools are offered.
pub const TOOL_CAP_PROMPT: &str = "Use the search tools at most 3 times per question in total.";

/// The system prompt of an answer: the shared base, plus the statute and case rules for the tools
/// that are offered.
#[must_use]
pub fn answer_system_prompt(statutes_on: bool, cases_on: bool) -> String {
    let mut prompt = SYSTEM_PROMPT.to_owned();
    for (on, part) in [(statutes_on, STATUTE_PROMPT), (cases_on, CASE_PROMPT)] {
        if on {
            prompt.push_str("\n\n");
            prompt.push_str(part);
        }
    }
    if statutes_on || cases_on {
        prompt.push_str("\n\n");
        prompt.push_str(TOOL_CAP_PROMPT);
    }
    prompt
}

/// Name of the statute search tool.
pub const SEARCH_TOOL_NAME: &str = "search_ph_laws";

/// Name of the article lookup tool.
pub const ARTICLE_TOOL_NAME: &str = "get_law_article";

/// One tool definition: a strict object schema whose `required` fields are listed. Integer ranges
/// are stated in the descriptions, not as schema keywords, because strict tool schemas reject
/// numeric constraints; core's limits enforce them.
fn tool(
    name: &str,
    description: &str,
    properties: &serde_json::Value,
    required: &[&str],
) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "description": description,
        "input_schema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        },
        "strict": true
    })
}

/// The fields every search tool takes: `query`, a year range (`dated` names the date, e.g.
/// "approval") and `limit`.
fn search_properties(dated: &str) -> serde_json::Map<String, serde_json::Value> {
    let year = format!("{dated} year, {TOOL_YEAR_MIN}-{TOOL_YEAR_MAX}.");
    let limit = format!("Most results, 1-{TOOL_RESULTS_MAX} (default {TOOL_RESULTS_MAX}).");
    let props = serde_json::json!({
        "query": { "type": "string", "maxLength": TOOL_QUERY_MAX },
        "year_from": { "type": "integer", "description": format!("Earliest {year}") },
        "year_to": { "type": "integer", "description": format!("Latest {year}") },
        "limit": { "type": "integer", "description": limit }
    });
    props.as_object().cloned().unwrap_or_default()
}

/// The two statute tools as sent in `tools` (the one definition; the bounds are core's limits).
#[must_use]
pub fn law_tools() -> serde_json::Value {
    let mut search = search_properties("approval");
    search.insert(
        "law_number".to_owned(),
        serde_json::json!({ "type": "string", "maxLength": TOOL_LAW_NUMBER_MAX }),
    );
    let article = serde_json::json!({
        "law_number": { "type": "string", "maxLength": TOOL_LAW_NUMBER_MAX },
        "article": { "type": "string", "maxLength": TOOL_ARTICLE_MAX }
    });
    serde_json::json!([
        tool(
            SEARCH_TOOL_NAME,
            LAW_SEARCH_DESCRIPTION,
            &search.into(),
            &["query"]
        ),
        tool(
            ARTICLE_TOOL_NAME,
            ARTICLE_DESCRIPTION,
            &article,
            &["law_number", "article"]
        ),
    ])
}

const LAW_SEARCH_DESCRIPTION: &str = "Search Philippine laws: Republic Acts and the 1987 Constitution as currently published, plus the codes (Revised Penal Code, Family Code, Labor Code, Rules of Court) and older Acts, decrees and orders as originally enacted. Results marked [As enacted] are the original text. Returns matching provisions as citable search results.";

const ARTICLE_DESCRIPTION: &str = "Get one provision of a Philippine law, e.g. law_number \"RA 9262\" and article \"Sec. 26\", or \"Act No. 3815\" (Revised Penal Code) and \"Art. 335\", or \"Rules of Court\" and \"Rule 113 Sec. 5\", with the provisions around it, as citable search results. Codes and older laws are the text as enacted.";

const CASE_SEARCH_DESCRIPTION: &str = "Search Philippine Supreme Court decisions (1996 to April 2024). Returns matching decisions as citable search results.";

const CASE_DESCRIPTION: &str = "Get one Supreme Court decision by G.R. number (e.g. \"G.R. No. 135981\") or case id, with up to three passages about focus, as a citable search result.";

/// Name of the case search tool (Phase 7).
pub const CASE_SEARCH_TOOL_NAME: &str = "search_ph_cases";

/// Name of the decision lookup tool (Phase 7).
pub const CASE_TOOL_NAME: &str = "get_case";

/// The two case tools as sent in `tools` (the one definition; the bounds are core's limits).
#[must_use]
pub fn case_tools() -> serde_json::Value {
    let get = serde_json::json!({
        "case": { "type": "string", "maxLength": TOOL_CASE_MAX },
        "focus": { "type": "string", "maxLength": TOOL_FOCUS_MAX }
    });
    serde_json::json!([
        tool(
            CASE_SEARCH_TOOL_NAME,
            CASE_SEARCH_DESCRIPTION,
            &search_properties("promulgation").into(),
            &["query"]
        ),
        tool(CASE_TOOL_NAME, CASE_DESCRIPTION, &get, &["case"]),
    ])
}

/// The tools for an answer: the statute tools, then the case tools, each when offered.
#[must_use]
pub fn tools_for(statutes_on: bool, cases_on: bool) -> serde_json::Value {
    let families = [(statutes_on, law_tools()), (cases_on, case_tools())];
    let tools = families
        .into_iter()
        .filter(|(on, _)| *on)
        .flat_map(|(_, t)| t.as_array().cloned().unwrap_or_default());
    serde_json::Value::Array(tools.collect())
}

#[cfg(test)]
mod tests {
    use super::{
        CASE_SEARCH_TOOL_NAME, CASE_TOOL_NAME, FOLLOW_UP_SCHEMA, SEARCH_TOOL_NAME,
        answer_system_prompt, law_tools, tools_for,
    };

    #[test]
    fn the_schema_is_valid_json_and_the_tools_are_named_once() {
        let schema: serde_json::Value = serde_json::from_str(FOLLOW_UP_SCHEMA).unwrap();
        assert_eq!(schema["required"][0], "followUps");
        assert_eq!(law_tools()[0]["name"], SEARCH_TOOL_NAME);
        let names: Vec<_> = tools_for(true, true)
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].clone())
            .collect();
        assert_eq!(
            names,
            [
                "search_ph_laws",
                "get_law_article",
                CASE_SEARCH_TOOL_NAME,
                CASE_TOOL_NAME
            ]
        );
        assert_eq!(tools_for(false, true).as_array().unwrap().len(), 2);
        assert_eq!(tools_for(false, false), serde_json::json!([]));
    }

    #[test]
    fn each_addition_is_sent_only_with_its_tools() {
        let both = answer_system_prompt(true, true);
        assert!(
            both.contains("[As enacted]")
                && both.contains("G.R. number")
                && both.contains("at most 3 times")
        );
        assert!(!answer_system_prompt(true, false).contains("search_ph_cases"));
        assert!(!answer_system_prompt(false, true).contains("search_ph_laws"));
        assert!(!answer_system_prompt(false, false).contains("at most 3 times"));
    }

    #[test]
    fn the_numbering_warning_is_sent_with_the_statute_tools() {
        let on = answer_system_prompt(true, false);
        assert!(on.contains("1974 original numbering") && on.contains("Art. 266-A"));
        assert!(!answer_system_prompt(false, true).contains("1974 original numbering"));
    }
}
