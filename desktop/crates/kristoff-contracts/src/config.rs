//! The app configuration: the one constant module the composition root reads (the web backend
//! had these as per-stage env values). Labels are the same as `backend/src/contracts/model-config.ts`
//! (a contracts test pins them).

/// Allowed model ids, first = default.
pub const MODEL_IDS: [&str; 3] = ["claude-sonnet-5-5", "claude-opus-5-5", "claude-haiku-5-5"];

/// The label the web shows per model id.
pub const MODEL_LABELS: [(&str, &str); 3] = [
    ("claude-sonnet-5-5", "Sonnet 5.5"),
    ("claude-opus-5-5", "Opus 5.5"),
    ("claude-haiku-5-5", "Haiku 5.5"),
];

/// Most attachments per chat.
pub const MAX_ATTACHED_DOCS: usize = 5;

/// Questions per UTC day.
pub const DAILY_QUESTION_CAP: u32 = 100;

/// How long a TOTP unlock lasts, in seconds (12 h, the web session lifetime).
pub const UNLOCK_TTL_SECONDS: i64 = 43_200;

/// The cosmetic token `verify_otp` returns (the unlock lives in Rust memory, not in a token).
pub const LOCAL_TOKEN: &str = "local";

/// Hosts `open_external` may open in the default browser (https only, exact host match).
pub const EXTERNAL_HOSTS: [&str; 6] = [
    "lawphil.net",
    "www.officialgazette.gov.ph",
    "officialgazette.gov.ph",
    "elibrary.judiciary.gov.ph",
    "legacy.senate.gov.ph",
    "issuances-library.senate.gov.ph",
];
