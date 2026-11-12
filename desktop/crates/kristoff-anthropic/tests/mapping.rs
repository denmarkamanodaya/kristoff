//! Follow-up validation (pure): trusted only on `end_turn`, bounded like the Zod schema.

#![cfg(test)]

use kristoff_anthropic::followups::{from_message, questions_of};
use serde_json::json;

fn reply(stop: &str, text: &str) -> serde_json::Value {
    json!({
        "stop_reason": stop,
        "content": [{ "type": "text", "text": text }],
        "usage": { "input_tokens": 10, "output_tokens": 20 }
    })
}

#[test]
fn follow_ups_are_trusted_only_on_end_turn() {
    let ok = r#"{"followUps":["What is BWS?"]}"#;
    assert_eq!(
        questions_of(&reply("end_turn", ok)),
        Some(vec!["What is BWS?".to_owned()])
    );
    assert_eq!(questions_of(&reply("max_tokens", ok)), None);
    assert_eq!(questions_of(&reply("refusal", ok)), None);
}

#[test]
fn follow_ups_are_trimmed_and_bounded_like_the_zod_schema() {
    let five = json!({ "followUps": ["a", "b", "c", "d", " e "] }).to_string();
    assert_eq!(questions_of(&reply("end_turn", &five)).unwrap()[4], "e");
    let cases = [
        json!({ "followUps": ["a", "b", "c", "d", "e", "f"] }),
        json!({ "followUps": ["x".repeat(121)] }),
        json!({ "followUps": ["   "] }),
        json!({ "followUps": [1] }),
        json!({ "followUps": [], "extra": true }),
        json!({ "other": [] }),
    ];
    for case in cases {
        assert_eq!(
            questions_of(&reply("end_turn", &case.to_string())),
            None,
            "{case}"
        );
    }
    assert_eq!(questions_of(&reply("end_turn", "not json")), None);
    let max = json!({ "followUps": ["\u{f1}".repeat(120)] }).to_string();
    assert!(questions_of(&reply("end_turn", &max)).is_some());
}

#[test]
fn an_untrusted_reply_gives_no_questions_but_keeps_usage() {
    let s = from_message(&reply("max_tokens", "{}"));
    assert_eq!(s.follow_ups.questions, [] as [std::string::String; 0]);
    assert_eq!(s.failure, Some("invalid_output"));
    assert_eq!(
        s.follow_ups.usage.map(|u| (u.input, u.output)),
        Some((10, 20))
    );
}
