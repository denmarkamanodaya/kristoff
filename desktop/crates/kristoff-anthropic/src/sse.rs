//! Hand-written parser for the Messages API event stream. Pure.
//!
//! Fed byte chunks that may split anywhere (inside a line, a UTF-8 character or a CRLF). Frames
//! end at a blank line; only `data:` lines matter, because each JSON payload names its own `type`.
//! Besides the events it emits, the parser rebuilds the assistant content blocks exactly as the API
//! produced them ([`SseParser::assistant_content`]), so a tool round can send them back unchanged
//! (D-11), thinking blocks and signatures included.

use kristoff_core::domain::model_request::CitationSource;
use kristoff_core::model::Usage;
use serde_json::{Map, Value};

/// One event the parser emits.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// Answer text to append.
    TextDelta(String),
    /// A citation that belongs to the text that follows it.
    CitationDelta(CitationSource),
    /// A complete tool call (emitted when its block closes).
    ToolUse {
        /// The `tool_use` id.
        id: String,
        /// The tool name.
        name: String,
        /// The input, parsed from the accumulated `input_json_delta` fragments.
        input: Value,
    },
    /// Usage so far and the stop reason.
    MessageDelta {
        /// Token usage (cumulative).
        usage: Usage,
        /// `end_turn`, `max_tokens`, `tool_use`, `refusal`, ...
        stop_reason: Option<String>,
    },
    /// An `error` event, or a payload the parser could not read.
    Error {
        /// The API error type, or `invalid_event` / `invalid_tool_input`.
        error_type: String,
        /// A human message.
        message: String,
    },
}

/// Incremental event-stream parser.
#[derive(Debug, Default)]
pub struct SseParser {
    buf: Vec<u8>,
    blocks: Vec<Value>,
    tool_json: Vec<(usize, String)>,
    start_usage: Usage,
}

impl SseParser {
    /// A parser at the start of a stream.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one chunk and returns the events completed by it.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<StreamEvent> {
        // CR never appears raw inside JSON, so dropping it turns CRLF framing into LF framing.
        self.buf
            .extend(chunk.iter().copied().filter(|b| *b != b'\r'));
        let mut events = Vec::new();
        while let Some(end) = self.buf.windows(2).position(|w| w == b"\n\n") {
            let frame: Vec<u8> = self.buf.drain(..end + 2).collect();
            self.frame(&frame, &mut events);
        }
        events
    }

    /// Ends the stream: a last frame without its blank line is still parsed.
    pub fn finish(&mut self) -> Vec<StreamEvent> {
        let frame = std::mem::take(&mut self.buf);
        let mut events = Vec::new();
        self.frame(&frame, &mut events);
        events
    }

    /// The assistant content blocks so far, as the API produced them.
    #[must_use]
    pub fn assistant_content(&self) -> Vec<Value> {
        self.blocks
            .iter()
            .filter(|b| !b.is_null())
            .cloned()
            .collect()
    }

    fn frame(&mut self, frame: &[u8], events: &mut Vec<StreamEvent>) {
        let text = String::from_utf8_lossy(frame);
        let data: Vec<&str> = text
            .lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .map(|d| d.strip_prefix(' ').unwrap_or(d))
            .collect();
        if data.is_empty() {
            return;
        }
        match serde_json::from_str::<Value>(&data.join("\n")) {
            Ok(payload) => self.payload(&payload, events),
            Err(e) => events.push(invalid("invalid_event", &e.to_string())),
        }
    }

    fn payload(&mut self, p: &Value, events: &mut Vec<StreamEvent>) {
        match p["type"].as_str() {
            Some("message_start") => {
                self.start_usage = usage_of(&p["message"]["usage"], Usage::default());
            }
            Some("content_block_start") => self.block_start(p),
            Some("content_block_delta") => self.block_delta(p, events),
            Some("content_block_stop") => self.block_stop(p, events),
            Some("message_delta") => events.push(StreamEvent::MessageDelta {
                usage: usage_of(&p["usage"], self.start_usage),
                stop_reason: p["delta"]["stop_reason"].as_str().map(str::to_owned),
            }),
            Some("error") => events.push(StreamEvent::Error {
                error_type: p["error"]["type"].as_str().unwrap_or("error").to_owned(),
                message: p["error"]["message"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            }),
            _ => {}
        }
    }

    fn block_start(&mut self, p: &Value) {
        let Some(i) = index_of(p) else { return };
        if self.blocks.len() <= i {
            self.blocks.resize(i + 1, Value::Null);
        }
        self.blocks[i] = p["content_block"].clone();
        if p["content_block"]["type"] == "tool_use" {
            self.tool_json.push((i, String::new()));
        }
    }

    fn block_delta(&mut self, p: &Value, events: &mut Vec<StreamEvent>) {
        let (Some(i), delta) = (index_of(p), &p["delta"]) else {
            return;
        };
        let Some(block) = self.blocks.get_mut(i).and_then(Value::as_object_mut) else {
            return;
        };
        match delta["type"].as_str() {
            Some("text_delta") => {
                let text = delta["text"].as_str().unwrap_or_default();
                append(block, "text", text);
                events.push(StreamEvent::TextDelta(text.to_owned()));
            }
            Some("citations_delta") => {
                push_citation(block, &delta["citation"]);
                events.push(match raw_citation(&delta["citation"]) {
                    Some(c) => StreamEvent::CitationDelta(c),
                    None => invalid("invalid_event", "unreadable citation"),
                });
            }
            Some("thinking_delta") => append(
                block,
                "thinking",
                delta["thinking"].as_str().unwrap_or_default(),
            ),
            Some("signature_delta") => {
                block.insert("signature".into(), delta["signature"].clone());
            }
            Some("input_json_delta") => {
                if let Some((_, json)) = self.tool_json.iter_mut().find(|(at, _)| *at == i) {
                    json.push_str(delta["partial_json"].as_str().unwrap_or_default());
                }
            }
            _ => {}
        }
    }

    fn block_stop(&mut self, p: &Value, events: &mut Vec<StreamEvent>) {
        let Some(i) = index_of(p) else { return };
        let Some(pos) = self.tool_json.iter().position(|(at, _)| *at == i) else {
            return;
        };
        let (_, json) = self.tool_json.remove(pos);
        let input = if json.trim().is_empty() {
            Ok(Value::Object(Map::new()))
        } else {
            serde_json::from_str(&json)
        };
        let Some(block) = self.blocks.get_mut(i).and_then(Value::as_object_mut) else {
            return;
        };
        match input {
            Ok(input) => {
                block.insert("input".into(), input.clone());
                events.push(StreamEvent::ToolUse {
                    id: block
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    name: block
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    input,
                });
            }
            Err(e) => events.push(invalid("invalid_tool_input", &e.to_string())),
        }
    }
}

fn invalid(error_type: &str, message: &str) -> StreamEvent {
    StreamEvent::Error {
        error_type: error_type.to_owned(),
        message: message.to_owned(),
    }
}

fn index_of(p: &Value) -> Option<usize> {
    p["index"].as_u64().and_then(|i| usize::try_from(i).ok())
}

fn append(block: &mut Map<String, Value>, key: &str, text: &str) {
    let mut joined = block
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    joined.push_str(text);
    block.insert(key.to_owned(), Value::String(joined));
}

fn push_citation(block: &mut Map<String, Value>, citation: &Value) {
    let list = block
        .entry("citations")
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Some(list) = list.as_array_mut() {
        list.push(citation.clone());
    }
}

/// Reads usage; a field missing from `u` keeps its value from `base`.
pub(crate) fn usage_of(u: &Value, base: Usage) -> Usage {
    let field = |key: &str, old: u64| u[key].as_u64().unwrap_or(old);
    Usage {
        input: field("input_tokens", base.input),
        output: field("output_tokens", base.output),
        cache_read: field("cache_read_input_tokens", base.cache_read),
        cache_write: field("cache_creation_input_tokens", base.cache_write),
    }
}

fn opt_str(v: &Value) -> Option<String> {
    v.as_str().map(str::to_owned)
}

fn raw_citation(c: &Value) -> Option<CitationSource> {
    let cited_text = c["cited_text"].as_str()?.to_owned();
    let document_index = c["document_index"]
        .as_u64()
        .and_then(|i| usize::try_from(i).ok());
    match c["type"].as_str()? {
        "page_location" => Some(CitationSource::Page {
            document_index: document_index?,
            document_title: opt_str(&c["document_title"]),
            start_page: u32::try_from(c["start_page_number"].as_u64()?).ok()?,
            end_page_exclusive: u32::try_from(c["end_page_number"].as_u64()?).ok()?,
            cited_text,
        }),
        "search_result_location" => Some(CitationSource::SearchResult {
            source: c["source"].as_str()?.to_owned(),
            title: opt_str(&c["title"]).unwrap_or_default(),
            cited_text,
            index: c["search_result_index"]
                .as_u64()
                .and_then(|i| usize::try_from(i).ok()),
        }),
        _ => Some(CitationSource::Other {
            document_index,
            document_title: opt_str(&c["document_title"]),
            cited_text,
        }),
    }
}
