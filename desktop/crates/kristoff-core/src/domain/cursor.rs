//! The opaque list cursor: base64url of `(sort key, id)`. Clients echo it and never build it.
//! Every store (SQLite and the fakes) uses this one encoding.

use crate::error::{AppError, AppResult, ErrorCode};
use crate::limits::MAX_CURSOR;
use crate::model::{Chat, Doc, Page};
use crate::util::{base64url, text::is_hex_id};

/// Where the next page starts: after the item with this sort key and id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorPosition {
    /// The sort key of the last item returned (epoch ms or the id itself).
    pub key: i64,
    /// The id of the last item returned (the tie-break).
    pub id: String,
}

/// Encodes a position as an opaque cursor.
#[must_use]
pub fn encode_cursor(position: &CursorPosition) -> String {
    base64url::encode(format!("{}|{}", position.key, position.id).as_bytes())
}

/// Decodes a cursor. `VALIDATION_FAILED` for anything this app did not produce.
///
/// # Errors
/// `VALIDATION_FAILED` when the cursor is too long, not base64url, or not `key|id`.
pub fn decode_cursor(cursor: &str) -> AppResult<CursorPosition> {
    let foreign = || AppError::new(ErrorCode::ValidationFailed, "unknown cursor");
    if cursor.is_empty() || cursor.len() > MAX_CURSOR {
        return Err(foreign());
    }
    let bytes = base64url::decode(cursor).ok_or_else(foreign)?;
    let text = String::from_utf8(bytes).map_err(|_| foreign())?;
    let (key, id) = text.split_once('|').ok_or_else(foreign)?;
    let key = key.parse::<i64>().map_err(|_| foreign())?;
    if !is_hex_id(id) {
        return Err(foreign());
    }
    Ok(CursorPosition {
        key,
        id: id.to_owned(),
    })
}

/// Where a doc sits in the library order (newest first by id; ids are time-ordered).
#[must_use]
pub fn doc_position(doc: &Doc) -> CursorPosition {
    CursorPosition {
        key: 0,
        id: doc.id.clone(),
    }
}

/// Where a chat sits in the chat order (newest `updated_at` first, tie-break id).
#[must_use]
pub fn chat_position(chat: &Chat) -> CursorPosition {
    CursorPosition {
        key: chat.updated_at_ms,
        id: chat.id.clone(),
    }
}

/// Cuts an over-fetched list (in list order, up to `limit + 1` items) to one page. The cursor
/// points after the last item kept, and only when more items exist.
#[must_use]
pub fn page_of<T>(
    mut items: Vec<T>,
    limit: usize,
    position: impl Fn(&T) -> CursorPosition,
) -> Page<T> {
    let has_more = items.len() > limit;
    items.truncate(limit);
    let next_cursor = if has_more {
        items.last().map(|last| encode_cursor(&position(last)))
    } else {
        None
    };
    Page { items, next_cursor }
}

#[cfg(test)]
mod tests {
    use super::{CursorPosition, decode_cursor, encode_cursor, page_of};
    use crate::error::ErrorCode;

    #[test]
    fn round_trips_a_position() {
        let position = CursorPosition {
            key: 1_794_445_200_000,
            id: "0123456789abcdef".into(),
        };

        assert_eq!(decode_cursor(&encode_cursor(&position)), Ok(position));
    }

    #[test]
    fn refuses_foreign_cursors() {
        for bad in ["", "!!!", "Zm9v", &"A".repeat(600)] {
            let err = decode_cursor(bad).unwrap_err();
            assert_eq!(err.code, ErrorCode::ValidationFailed, "{bad}");
        }
    }

    #[test]
    fn cuts_an_over_fetched_list_and_points_after_the_last_kept_item() {
        let at = |n: &i64| CursorPosition {
            key: *n,
            id: "0123456789abcdef".into(),
        };

        let more = page_of(vec![3, 2, 1], 2, at);
        let last = page_of(vec![3, 2], 2, at);

        assert_eq!(more.items, [3, 2]);
        assert_eq!(decode_cursor(&more.next_cursor.unwrap()).unwrap().key, 2);
        assert_eq!((last.items, last.next_cursor), (vec![3, 2], None));
    }
}
