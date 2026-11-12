//! SSE parser against the recorded S4/S5 streams, split at every byte offset.
#![cfg(test)]

mod support;

use kristoff_anthropic::sse::{SseParser, StreamEvent};
use kristoff_core::domain::model_request::CitationSource;
use kristoff_core::model::Usage;
use proptest::prelude::*;
use serde_json::json;
use support::fixture_bytes;

fn parse_in(chunks: &[&[u8]]) -> (Vec<StreamEvent>, Vec<serde_json::Value>) {
    let mut p = SseParser::new();
    let mut events = Vec::new();
    for c in chunks {
        events.extend(p.feed(c));
    }
    events.extend(p.finish());
    (events, p.assistant_content())
}

fn text_of(events: &[StreamEvent]) -> String {
    events
        .iter()
        .filter_map(|e| {
            if let StreamEvent::TextDelta(t) = e {
                Some(t.as_str())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn the_file_doc_stream_gives_text_and_final_usage() {
    let bytes = fixture_bytes("anthropic-file-doc-stream.sse");

    let (events, _) = parse_in(&[&bytes]);

    assert!(text_of(&events).starts_with("The Court ruled that the accused acted in self-defense"));
    assert_eq!(
        events.last(),
        Some(&StreamEvent::MessageDelta {
            usage: Usage {
                input: 26,
                output: 160,
                cache_read: 0,
                cache_write: 10214
            },
            stop_reason: Some("end_turn".into()),
        })
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, StreamEvent::Error { .. }))
    );
}

#[test]
fn the_file_doc_stream_gives_a_page_citation() {
    let bytes = fixture_bytes("anthropic-file-doc-stream.sse");

    let (events, _) = parse_in(&[&bytes]);

    let first = events.iter().find_map(|e| match e {
        StreamEvent::CitationDelta(c) => Some(c.clone()),
        _ => None,
    });
    let Some(CitationSource::Page {
        document_index,
        document_title,
        start_page,
        end_page_exclusive,
        cited_text,
    }) = first
    else {
        panic!("expected a page citation")
    };
    assert_eq!((document_index, start_page, end_page_exclusive), (0, 3, 4));
    assert_eq!(document_title.as_deref(), Some("Sample decision"));
    assert!(
        cited_text.contains("\r\n"),
        "raw text keeps CRLF; mapping normalises it"
    );
}

#[test]
fn the_tool_stream_gives_two_search_result_citations() {
    let bytes = fixture_bytes("anthropic-tool-stream.sse");

    let (events, _) = parse_in(&[&bytes]);

    let sources: Vec<(&str, &str)> = events
        .iter()
        .filter_map(|e| match e {
            StreamEvent::CitationDelta(CitationSource::SearchResult { source, title, .. }) => {
                Some((source.as_str(), title.as_str()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        sources,
        vec![
            (
                "https://lawphil.net/statutes/repacts/ra2004/ra_9262_2004.html#sec26",
                "RA 9262 \u{b7} Sec. 26"
            ),
            (
                "https://lawphil.net/statutes/repacts/ra2004/ra_9262_2004.html#sec3",
                "RA 9262 \u{b7} Sec. 3(c)"
            ),
        ]
    );
}

#[test]
fn every_single_split_point_gives_the_same_events_and_content() {
    for name in ["anthropic-file-doc-stream.sse", "anthropic-tool-stream.sse"] {
        let bytes = fixture_bytes(name);
        let whole = parse_in(&[&bytes]);
        for k in 0..=bytes.len() {
            let split = parse_in(&[&bytes[..k], &bytes[k..]]);
            assert_eq!(split, whole, "{name} split at byte {k}");
        }
    }
}

#[test]
fn one_byte_chunks_and_crlf_framing_give_the_same_events() {
    let bytes = fixture_bytes("anthropic-file-doc-stream.sse");
    let whole = parse_in(&[&bytes]);
    let singles: Vec<&[u8]> = bytes.chunks(1).collect();
    assert_eq!(parse_in(&singles), whole);
    let crlf = String::from_utf8(bytes)
        .unwrap()
        .replace('\n', "\r\n")
        .into_bytes();
    for k in 0..=crlf.len() {
        assert_eq!(
            parse_in(&[&crlf[..k], &crlf[k..]]),
            whole,
            "crlf split at {k}"
        );
    }
}

proptest! {
    #[test]
    fn any_chunking_gives_the_same_events(cuts in proptest::collection::vec(0usize..7639, 0..20)) {
        let bytes = fixture_bytes("anthropic-tool-stream.sse");
        let whole = parse_in(&[&bytes]);
        let mut cuts: Vec<usize> = cuts.into_iter().map(|c| c % (bytes.len() + 1)).collect();
        cuts.sort_unstable();
        let mut pieces: Vec<&[u8]> = Vec::new();
        let mut at = 0;
        for c in cuts {
            pieces.push(&bytes[at..c]);
            at = c;
        }
        pieces.push(&bytes[at..]);
        prop_assert_eq!(parse_in(&pieces), whole);
    }
}

const TOOL_TURN: &str = concat!(
    "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":50,\"output_tokens\":1}}}\n\n",
    "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\",\"signature\":\"\"}}\n\n",
    "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"Need RA \"}}\n\n",
    "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"9262.\"}}\n\n",
    "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"SIG==\"}}\n\n",
    "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
    "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"search_ph_laws\",\"input\":{},\"caller\":{\"type\":\"direct\"}}}\n\n",
    "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"query\\\": \\\"battered\"}}\n\n",
    "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\" woman\\\", \\\"law_number\\\": \\\"RA 9262\\\"}\"}}\n\n",
    "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
    "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"output_tokens\":40}}\n\n",
    "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
);

#[test]
fn a_tool_call_is_emitted_with_its_accumulated_input_and_content_is_rebuilt_verbatim() {
    // Hand-written: no recorded turn-1 stream exists (the S5 stream is turn 2).
    let (events, content) = parse_in(&[TOOL_TURN.as_bytes()]);

    let input = json!({ "query": "battered woman", "law_number": "RA 9262" });
    assert!(events.contains(&StreamEvent::ToolUse {
        id: "toolu_1".into(),
        name: "search_ph_laws".into(),
        input: input.clone(),
    }));
    assert_eq!(
        content,
        vec![
            json!({ "type": "thinking", "thinking": "Need RA 9262.", "signature": "SIG==" }),
            json!({ "type": "tool_use", "id": "toolu_1", "name": "search_ph_laws", "input": input, "caller": { "type": "direct" } }),
        ]
    );
    assert_eq!(
        events.last(),
        Some(&StreamEvent::MessageDelta {
            usage: Usage {
                input: 50,
                output: 40,
                cache_read: 0,
                cache_write: 0
            },
            stop_reason: Some("tool_use".into()),
        })
    );
}

#[test]
fn rebuilt_text_content_keeps_its_citations_in_order() {
    let (_, content) = parse_in(&[&fixture_bytes("anthropic-file-doc-stream.sse")]);
    let with_citations: Vec<_> = content
        .iter()
        .filter(|b| b.get("citations").is_some())
        .collect();
    assert_ne!(with_citations, [] as [&serde_json::Value; 0]);
    assert_eq!(with_citations[0]["citations"][0]["start_page_number"], 3);
    assert!(
        with_citations[0]["text"]
            .as_str()
            .unwrap()
            .starts_with("the accused acted")
    );
}

#[test]
fn an_error_event_and_a_garbled_payload_become_error_events() {
    let raw = "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\ndata: {not json\n\n";
    let (events, _) = parse_in(&[raw.as_bytes()]);
    assert_eq!(
        events[0],
        StreamEvent::Error {
            error_type: "overloaded_error".into(),
            message: "Overloaded".into()
        }
    );
    assert!(
        matches!(&events[1], StreamEvent::Error { error_type, .. } if error_type == "invalid_event")
    );
}

#[test]
fn a_last_frame_without_its_blank_line_is_parsed_on_finish() {
    let raw = "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":3}}";
    let mut p = SseParser::new();
    assert_eq!(
        p.feed(raw.as_bytes()),
        [] as [kristoff_anthropic::sse::StreamEvent; 0]
    );
    assert_eq!(p.finish().len(), 1);
}
