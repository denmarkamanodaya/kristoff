//! The running asks: one cancel flag per chat, set by `cancel_ask` and by closing the main window
//! (phase-2 §4).

use kristoff_core::ports::CancelSignal;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// The cancel flag of one running ask.
#[derive(Debug, Default)]
pub struct AskCancel(AtomicBool);

impl AskCancel {
    /// Stops the ask.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl CancelSignal for AskCancel {
    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Running asks by chat id.
#[derive(Debug, Default)]
pub struct Asks(Mutex<HashMap<String, Arc<AskCancel>>>);

impl Asks {
    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<AskCancel>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Registers a new ask for `chat_id` and returns its flag.
    pub fn begin(&self, chat_id: &str) -> Arc<AskCancel> {
        let flag = Arc::new(AskCancel::default());
        self.map().insert(chat_id.to_owned(), Arc::clone(&flag));
        flag
    }

    /// Forgets the ask, unless a newer ask for the same chat replaced it.
    pub fn end(&self, chat_id: &str, flag: &Arc<AskCancel>) {
        let mut map = self.map();
        if map.get(chat_id).is_some_and(|f| Arc::ptr_eq(f, flag)) {
            map.remove(chat_id);
        }
    }

    /// Cancels the running ask of `chat_id`, if any.
    pub fn cancel(&self, chat_id: &str) {
        if let Some(flag) = self.map().get(chat_id) {
            flag.cancel();
        }
    }

    /// Cancels every running ask (the main window is closing).
    pub fn cancel_all(&self) {
        self.map().values().for_each(|f| f.cancel());
    }
}

#[cfg(test)]
mod tests {
    use super::Asks;
    use kristoff_core::ports::CancelSignal;

    #[test]
    fn cancels_one_chat_or_all_and_ends_only_its_own_ask() {
        let asks = Asks::default();
        let first = asks.begin("a");
        let replaced = asks.begin("b");
        let newer = asks.begin("b");

        asks.cancel("a");
        asks.end("b", &replaced);
        asks.cancel_all();

        assert!(first.is_cancelled());
        assert!(newer.is_cancelled());
        assert!(
            !replaced.is_cancelled(),
            "a replaced ask is no longer reachable"
        );
    }
}
