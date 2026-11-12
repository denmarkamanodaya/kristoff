//! Conversions from domain values to the wire shapes (instants become ISO strings here,
//! and only here).

use crate::shapes::{
    ChatDto, ChatSummaryDto, CitationDto, CitationKindDto, DocDto, DocPartDto, DocStatusDto,
    MessageDto, ProjectDto, RoleDto, UsageDto,
};
use kristoff_core::model::{
    Chat, Citation, CitationKind, Doc, DocPart, DocStatus, Message, Project, Role, Usage,
};
use kristoff_core::util::time::iso_instant;

impl From<&Usage> for UsageDto {
    fn from(u: &Usage) -> Self {
        Self {
            input: u.input,
            output: u.output,
            cache_read: u.cache_read,
            cache_write: u.cache_write,
        }
    }
}

impl From<&DocPart> for DocPartDto {
    fn from(p: &DocPart) -> Self {
        Self {
            n: p.n,
            start_page: p.start_page,
            end_page: p.end_page,
            size: p.size,
        }
    }
}

impl From<DocStatus> for DocStatusDto {
    fn from(s: DocStatus) -> Self {
        match s {
            DocStatus::Pending => Self::Pending,
            DocStatus::Processing => Self::Processing,
            DocStatus::Ready => Self::Ready,
            DocStatus::Failed => Self::Failed,
        }
    }
}

impl From<&Doc> for DocDto {
    fn from(d: &Doc) -> Self {
        Self {
            id: d.id.clone(),
            title: d.title.clone(),
            size: d.size,
            status: d.status.into(),
            created_at: iso_instant(d.created_at_ms),
            page_count: d.page_count,
            parts: d.parts.iter().map(DocPartDto::from).collect(),
        }
    }
}

impl From<&Citation> for CitationDto {
    fn from(c: &Citation) -> Self {
        let kind = match c.kind {
            CitationKind::Doc => CitationKindDto::Doc,
            CitationKind::Law => CitationKindDto::Law,
            CitationKind::Case => CitationKindDto::Case,
        };
        Self {
            n: c.n,
            doc_id: c.doc_id.clone(),
            doc_title: c.doc_title.clone(),
            page: c.page,
            end_page: c.end_page,
            quote: c.quote.clone(),
            kind,
            source_url: c.source_url.clone(),
        }
    }
}

impl From<&Chat> for ChatSummaryDto {
    fn from(c: &Chat) -> Self {
        Self {
            id: c.id.clone(),
            title: c.title.clone(),
            doc_ids: c.doc_ids.clone(),
            updated_at: iso_instant(c.updated_at_ms),
            pinned: c.pinned,
            project_id: c.project_id.clone(),
            statutes: c.statutes,
        }
    }
}

impl From<&Chat> for ChatDto {
    fn from(c: &Chat) -> Self {
        Self {
            summary: ChatSummaryDto::from(c),
            created_at: iso_instant(c.created_at_ms),
        }
    }
}

impl From<&Message> for MessageDto {
    fn from(m: &Message) -> Self {
        let role = match m.role {
            Role::User => RoleDto::User,
            Role::Assistant => RoleDto::Assistant,
        };
        Self {
            id: m.id.clone(),
            role,
            text: m.text.clone(),
            citations: m.citations.iter().map(CitationDto::from).collect(),
            usage: m.usage.as_ref().map(UsageDto::from),
            model: m.model.clone(),
            stop_reason: m.stop_reason.clone(),
            at: iso_instant(m.at_ms),
            duration_ms: m.duration_ms,
            follow_ups: m.follow_ups.clone(),
        }
    }
}

impl From<&Project> for ProjectDto {
    fn from(p: &Project) -> Self {
        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            created_at: iso_instant(p.created_at_ms),
            updated_at: iso_instant(p.updated_at_ms),
        }
    }
}
