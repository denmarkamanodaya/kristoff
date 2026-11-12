//! Reads one `tool_use` block into core's `ToolCall`. Only the value types are checked here (the
//! JSON is the vendor shape); the bounds are core's rule (`domain::law::tools::validate`).

use kristoff_contracts::model_config::{
    ARTICLE_TOOL_NAME, CASE_SEARCH_TOOL_NAME, CASE_TOOL_NAME, SEARCH_TOOL_NAME,
};
use kristoff_core::domain::model_request::{
    RawArticleInput, RawCaseSearchInput, RawGetCaseInput, RawSearchInput, ToolCall, ToolInput,
};
use serde_json::{Map, Value};

/// A string field: `Ok(None)` when absent, `Err` when present but not a string.
fn text(obj: &Map<String, Value>, key: &str) -> Result<Option<String>, ()> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(()),
    }
}

/// An integer field: `Ok(None)` when absent, `Err` when present but not an integer.
fn int(obj: &Map<String, Value>, key: &str) -> Result<Option<i64>, ()> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_i64().map(Some).ok_or(()),
    }
}

fn only_keys(obj: &Map<String, Value>, allowed: &[&str]) -> Result<(), ()> {
    if obj.keys().all(|k| allowed.contains(&k.as_str())) {
        Ok(())
    } else {
        Err(())
    }
}

/// The fields every search tool takes: `query`, `year_from`, `year_to`, `limit`.
type SearchFields = (Option<String>, Option<i64>, Option<i64>, Option<i64>);

fn search_fields(obj: &Map<String, Value>) -> Result<SearchFields, ()> {
    Ok((
        text(obj, "query")?,
        int(obj, "year_from")?,
        int(obj, "year_to")?,
        int(obj, "limit")?,
    ))
}

fn search(obj: &Map<String, Value>) -> Result<RawSearchInput, ()> {
    only_keys(
        obj,
        &["query", "law_number", "year_from", "year_to", "limit"],
    )?;
    let (query, year_from, year_to, limit) = search_fields(obj)?;
    let law_number = text(obj, "law_number")?;
    Ok(RawSearchInput {
        query,
        law_number,
        year_from,
        year_to,
        limit,
    })
}

fn article(obj: &Map<String, Value>) -> Result<RawArticleInput, ()> {
    only_keys(obj, &["law_number", "article"])?;
    Ok(RawArticleInput {
        law_number: text(obj, "law_number")?,
        article: text(obj, "article")?,
    })
}

fn case_search(obj: &Map<String, Value>) -> Result<RawCaseSearchInput, ()> {
    only_keys(obj, &["query", "year_from", "year_to", "limit"])?;
    let (query, year_from, year_to, limit) = search_fields(obj)?;
    Ok(RawCaseSearchInput {
        query,
        year_from,
        year_to,
        limit,
    })
}

fn get_case(obj: &Map<String, Value>) -> Result<RawGetCaseInput, ()> {
    only_keys(obj, &["case", "focus"])?;
    Ok(RawGetCaseInput {
        case: text(obj, "case")?,
        focus: text(obj, "focus")?,
    })
}

/// The call for a `tool_use` block named `name` with `input`.
#[must_use]
pub fn tool_call(id: &str, name: &str, input: &Value) -> ToolCall {
    let malformed = || ToolInput::Malformed(name.to_owned());
    let input = match (name, input.as_object()) {
        (SEARCH_TOOL_NAME, Some(obj)) => {
            search(obj).map_or_else(|()| malformed(), ToolInput::SearchLaws)
        }
        (ARTICLE_TOOL_NAME, Some(obj)) => {
            article(obj).map_or_else(|()| malformed(), ToolInput::GetLawArticle)
        }
        (CASE_SEARCH_TOOL_NAME, Some(obj)) => {
            case_search(obj).map_or_else(|()| malformed(), ToolInput::SearchCases)
        }
        (CASE_TOOL_NAME, Some(obj)) => {
            get_case(obj).map_or_else(|()| malformed(), ToolInput::GetCase)
        }
        (SEARCH_TOOL_NAME | ARTICLE_TOOL_NAME | CASE_SEARCH_TOOL_NAME | CASE_TOOL_NAME, None) => {
            malformed()
        }
        _ => ToolInput::Unknown(name.to_owned()),
    };
    ToolCall {
        id: id.to_owned(),
        input,
    }
}

#[cfg(test)]
mod tests {
    use super::tool_call;
    use kristoff_core::domain::model_request::{RawSearchInput, ToolInput};
    use serde_json::json;

    #[test]
    fn reads_a_search_and_flags_wrong_types_extra_keys_and_unknown_tools() {
        let ok = tool_call(
            "t1",
            "search_ph_laws",
            &json!({ "query": "bws", "law_number": "RA 9262" }),
        );
        let typed = tool_call("t2", "search_ph_laws", &json!({ "query": 5 }));
        let extra = tool_call(
            "t3",
            "get_law_article",
            &json!({ "law_number": "RA 1", "article": "1", "x": 1 }),
        );
        let unknown = tool_call("t4", "web_search", &json!({}));

        let want = RawSearchInput {
            query: Some("bws".into()),
            law_number: Some("RA 9262".into()),
            ..RawSearchInput::default()
        };
        assert_eq!(ok.input, ToolInput::SearchLaws(want));
        assert_eq!(typed.input, ToolInput::Malformed("search_ph_laws".into()));
        assert_eq!(extra.input, ToolInput::Malformed("get_law_article".into()));
        assert_eq!(unknown.input, ToolInput::Unknown("web_search".into()));
    }
}
