//! The one list of IPC command names (the shell registers exactly these and `build.rs` generates
//! one `allow-<command>` permission per name) and the raw-body upload headers.

/// Every command the web may invoke, in contract order.
pub const COMMANDS: &[&str] = &[
    "get_config",
    "list_docs",
    "create_doc",
    "upload_chunk",
    "confirm_doc",
    "rename_doc",
    "get_doc_link",
    "delete_doc",
    "list_projects",
    "create_project",
    "rename_project",
    "delete_project",
    "list_chats",
    "create_chat",
    "get_chat",
    "update_chat",
    "delete_chat",
    "get_usage",
    "ask",
    "cancel_ask",
    "open_doc",
    "open_external",
];

/// `upload_chunk` header naming the doc.
pub const HEADER_DOC_ID: &str = "x-doc-id";
/// `upload_chunk` header carrying the byte offset (decimal).
pub const HEADER_OFFSET: &str = "x-offset";
