//! A cancellation token on `tokio::sync::watch` (no `tokio-util` needed).

use std::sync::Arc;
use tokio::sync::watch;

/// Cancels one ask. Clones share the same state.
#[derive(Debug, Clone)]
pub struct CancelToken {
    tx: Arc<watch::Sender<bool>>,
}

impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancelToken {
    /// A token that is not cancelled.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tx: Arc::new(watch::Sender::new(false)),
        }
    }

    /// Cancels every clone of this token. Idempotent.
    pub fn cancel(&self) {
        self.tx.send_replace(true);
    }

    /// Whether the token was cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        *self.tx.borrow()
    }

    /// Completes when the token is cancelled (at once if it already is).
    pub async fn cancelled(&self) {
        let mut rx = self.tx.subscribe();
        // The sender lives in `self`, so it cannot close while this future borrows `self`.
        if rx.wait_for(|cancelled| *cancelled).await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}
