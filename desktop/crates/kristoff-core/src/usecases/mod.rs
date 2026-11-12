//! Use cases: each takes the driven ports it needs (`*Deps`) plus its input, and returns a domain
//! value or a typed error. Ported as behaviour from `backend/src/usecases/*`.

pub mod ask;
pub mod attachments;
pub mod chats;
pub mod docs;
pub mod processing;
pub mod projects;
pub mod settings;
pub mod upload;
pub mod viewer;
