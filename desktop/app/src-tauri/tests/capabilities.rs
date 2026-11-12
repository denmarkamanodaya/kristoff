//! The capability file grants exactly `core:default` plus one `allow-<command>` per name in
//! `kristoff_contracts::ipc::COMMANDS` (the one command list).

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_contracts::ipc::COMMANDS;

#[test]
fn the_default_capability_grants_exactly_the_command_list() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/capabilities/default.json");
    let file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let granted: Vec<String> = file["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap().to_owned())
        .collect();

    let mut expected = vec!["core:default".to_owned()];
    expected.extend(
        COMMANDS
            .iter()
            .map(|c| format!("allow-{}", c.replace('_', "-"))),
    );

    assert_eq!(granted, expected);
}
