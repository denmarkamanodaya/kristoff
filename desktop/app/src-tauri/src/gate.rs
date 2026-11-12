//! The TOTP gate (decision D-6): the unlock lives only in this process's memory, ends after 12 h
//! and on quit. Every command except `verify_otp` checks it first and fails closed.

use kristoff_core::error::{AppError, ErrorCode};
use std::sync::atomic::{AtomicI64, Ordering};

/// Unlocked until an instant (epoch ms); `0` = locked.
#[derive(Debug, Default)]
pub struct Gate {
    unlocked_until_ms: AtomicI64,
}

impl Gate {
    /// A locked gate.
    #[must_use]
    pub fn locked() -> Self {
        Self::default()
    }

    /// Unlocks until `until_ms`.
    pub fn unlock(&self, until_ms: i64) {
        self.unlocked_until_ms.store(until_ms, Ordering::SeqCst);
    }

    /// Ok while unlocked at `now_ms`.
    ///
    /// # Errors
    /// `UNAUTHENTICATED` when never unlocked or the unlock has ended.
    pub fn check(&self, now_ms: i64) -> Result<(), AppError> {
        if now_ms < self.unlocked_until_ms.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::Unauthenticated,
                "the app is locked",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Gate;
    use kristoff_core::error::ErrorCode;

    #[test]
    fn starts_locked_opens_until_the_expiry_and_locks_again_at_it() {
        let gate = Gate::locked();
        assert_eq!(gate.check(10).unwrap_err().code, ErrorCode::Unauthenticated);

        gate.unlock(100);

        assert!(gate.check(99).is_ok());
        assert_eq!(
            gate.check(100).unwrap_err().code,
            ErrorCode::Unauthenticated
        );
    }
}
