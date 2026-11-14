//! `SecretStore` on the macOS Keychain (Phase 8, docs/desktop/phases/phase-8-lexph-byok.md §1):
//! one generic-password item. Status codes are mapped once; the key never appears in an error.
//! Pattern: `dexfs/crates/footage-adapters/src/keychain.rs`.

use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::SecretStore;
use security_framework::passwords::{
    PasswordOptions, delete_generic_password, generic_password, set_generic_password,
};

/// `errSecItemNotFound` (Security framework `SecBase.h`).
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25_300;

/// The Keychain service of the production key item.
pub const KEY_SERVICE: &str = "ph.lexph.desktop.claude-key";

/// The account of the key item (one key per Mac user).
pub const KEY_ACCOUNT: &str = "claude-api-key";

/// `INTERNAL` naming the failed step and its `OSStatus` (never the item's value).
fn keychain(what: &str, e: security_framework::base::Error) -> AppError {
    AppError::new(
        ErrorCode::Internal,
        format!("keychain {what}: OSStatus {}", e.code()),
    )
}

/// The Claude key as one Keychain item (`service`, `account`).
#[derive(Debug, Clone)]
pub struct KeychainSecretStore {
    service: String,
    account: String,
}

impl KeychainSecretStore {
    /// The production item ([`KEY_SERVICE`], [`KEY_ACCOUNT`]).
    #[must_use]
    pub fn production() -> Self {
        Self::at(KEY_SERVICE, KEY_ACCOUNT)
    }

    /// Another item (tests use a throwaway service name).
    #[must_use]
    pub fn at(service: &str, account: &str) -> Self {
        Self {
            service: service.to_owned(),
            account: account.to_owned(),
        }
    }
}

impl SecretStore for KeychainSecretStore {
    fn get_key(&self) -> AppResult<Option<String>> {
        let options = PasswordOptions::new_generic_password(&self.service, &self.account);
        let bytes = match generic_password(options) {
            Ok(bytes) => bytes,
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => return Ok(None),
            Err(e) => return Err(keychain("read", e)),
        };
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| AppError::new(ErrorCode::Internal, "keychain read: the item is not text"))
    }

    fn set_key(&self, key: &str) -> AppResult<()> {
        set_generic_password(&self.service, &self.account, key.as_bytes())
            .map_err(|e| keychain("write", e))
    }

    fn delete_key(&self) -> AppResult<()> {
        match delete_generic_password(&self.service, &self.account) {
            Ok(()) => Ok(()),
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(()),
            Err(e) => Err(keychain("delete", e)),
        }
    }
}
