//! `CredentialStore` trait, OS-keyring impl, and an in-memory test impl (#16).

use std::sync::Mutex;

const SERVICE: &str = "family-chat-cli";

pub trait CredentialStore: Send + Sync {
    fn load(&self) -> Option<String>;
    fn save(&self, token: &str);
    fn clear(&self);
}

/// Namespaced by profile (see #34) — for now there's just the one, "default".
pub struct KeyringStore {
    entry: keyring::Entry,
}

impl KeyringStore {
    pub fn new(profile: &str) -> anyhow::Result<Self> {
        Ok(Self {
            entry: keyring::Entry::new(SERVICE, profile)?,
        })
    }
}

impl CredentialStore for KeyringStore {
    fn load(&self) -> Option<String> {
        self.entry.get_password().ok()
    }

    fn save(&self, token: &str) {
        if let Err(error) = self.entry.set_password(token) {
            tracing::error!(%error, "failed to save credential to the OS keychain");
        }
    }

    fn clear(&self) {
        if let Err(error) = self.entry.delete_credential() {
            tracing::warn!(%error, "failed to clear credential from the OS keychain");
        }
    }
}

/// Test double — never touches the real OS keychain. Only exercised under
/// `#[cfg(test)]` today; kept public (not `#[cfg(test)]`-gated itself) so
/// future integration tests outside this module can use it too.
#[derive(Default)]
#[allow(dead_code)]
pub struct InMemoryStore(Mutex<Option<String>>);

impl CredentialStore for InMemoryStore {
    fn load(&self) -> Option<String> {
        self.0.lock().expect("lock poisoned").clone()
    }

    fn save(&self, token: &str) {
        *self.0.lock().expect("lock poisoned") = Some(token.to_string());
    }

    fn clear(&self) {
        *self.0.lock().expect("lock poisoned") = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_store_round_trips() {
        let store = InMemoryStore::default();
        assert_eq!(store.load(), None);
        store.save("token-123");
        assert_eq!(store.load(), Some("token-123".to_string()));
        store.clear();
        assert_eq!(store.load(), None);
    }

    /// Hits the real OS keychain — `#[ignore]`d so normal `cargo test`/CI runs
    /// never touch it. Run explicitly with `cargo test -- --ignored` on a
    /// machine with a keychain/Secret Service available to manually verify #16.
    #[test]
    #[ignore]
    fn keyring_store_round_trips_on_this_machine() {
        let store =
            KeyringStore::new("test-profile-do-not-use").expect("keyring backend available");
        store.clear(); // in case a previous failed run left something behind
        assert_eq!(store.load(), None);
        store.save("token-456");
        assert_eq!(store.load(), Some("token-456".to_string()));
        store.clear();
        assert_eq!(store.load(), None);
    }
}
