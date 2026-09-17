//! `CredentialStore` trait, OS-keyring impl, and an in-memory test impl (#16/#36).

use std::sync::Mutex;

const SERVICE: &str = "family-chat-cli";

pub trait CredentialStore: Send + Sync {
    fn load(&self) -> Option<String>;
    fn save(&self, token: &str);
    fn clear(&self);
}

/// Namespaces a keyring entry by profile *and* API origin (`scheme://host[:port]`,
/// e.g. `https://chat.thevachonfamily.ca`) rather than profile alone — so
/// switching `--server`/`FAMILY_CHAT_URL`/the config file's `server` under
/// the same profile (say, to point at a staging server) can never silently
/// reuse, or overwrite, a session token that belongs to a different server
/// (#36). `keyring::Entry`'s "username" is really just an opaque namespacing
/// string as far as this app is concerned, so both go in there together.
fn credential_namespace(profile: &str, origin: &str) -> String {
    format!("{profile}@{origin}")
}

pub struct KeyringStore {
    entry: keyring::Entry,
}

impl KeyringStore {
    pub fn new(profile: &str, origin: &str) -> anyhow::Result<Self> {
        Ok(Self {
            entry: keyring::Entry::new(SERVICE, &credential_namespace(profile, origin))?,
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

    #[test]
    fn the_credential_namespace_combines_profile_and_origin() {
        assert_eq!(
            credential_namespace("default", "https://chat.thevachonfamily.ca"),
            "default@https://chat.thevachonfamily.ca"
        );
    }

    #[test]
    fn different_origins_under_the_same_profile_get_different_namespaces() {
        let prod = credential_namespace("default", "https://chat.thevachonfamily.ca");
        let staging = credential_namespace("default", "https://staging.example.com");
        assert_ne!(
            prod, staging,
            "a stored session for one server must never be reused against another"
        );
    }

    /// Hits the real OS keychain — `#[ignore]`d so normal `cargo test`/CI runs
    /// never touch it. Run explicitly with `cargo test -- --ignored` on a
    /// machine with a keychain/Secret Service available to manually verify #16.
    #[test]
    #[ignore]
    fn keyring_store_round_trips_on_this_machine() {
        let store = KeyringStore::new("test-profile-do-not-use", "https://example.com")
            .expect("keyring backend available");
        store.clear(); // in case a previous failed run left something behind
        assert_eq!(store.load(), None);
        store.save("token-456");
        assert_eq!(store.load(), Some("token-456".to_string()));
        store.clear();
        assert_eq!(store.load(), None);
    }
}
