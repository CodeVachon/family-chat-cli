//! Config file format, locations, load/save, and validation (#34/#35).
//!
//! Non-secret settings only — credentials live in the OS keyring (see
//! `auth::store`), never here.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

/// Non-secret CLI settings, stored as TOML. Every field is optional (and
/// `#[serde(default)]`-backed) so an empty file, or one written by an older
/// version that predates a newer field, still parses — additive by
/// construction, with no version/migration story needed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// The Family Chat server's base URL — the config-file equivalent of
    /// `cli::Cli::server`. `--server`/`FAMILY_CHAT_URL` take precedence over
    /// this when both are set (see `main`'s merge of the two).
    pub server: Option<String>,

    /// Which keyring/session profile to use (see `auth::KeyringStore`),
    /// letting more than one account or environment keep separate stored
    /// sessions rather than one clobbering the other's (#36 — combined with
    /// the server's origin, not used alone, so a profile reused against a
    /// different server never collides with the wrong session).
    pub profile: Option<String>,

    /// The channel to select on startup, matched by name. Readable/writable
    /// via `config get/set default-channel` (#37), but still not consumed
    /// by the app itself — there's no "jump to a named channel on startup"
    /// behavior yet for this to feed.
    pub default_channel: Option<String>,
    // Non-secret UI preferences (the plan's fourth category) are
    // deliberately not modeled yet — there isn't a concrete one to define a
    // shape for. Add a field (or a nested table) here when one exists;
    // `#[serde(default)]` above means that's additive too.
}

/// A single `Config` field, addressable by name for the `config
/// get`/`set`/`unset` subcommands (#37) — keeps their key names (kebab-case,
/// the usual CLI convention) decoupled from Rust's own field names.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigKey {
    Server,
    Profile,
    #[value(name = "default-channel")]
    DefaultChannel,
}

impl ConfigKey {
    /// Every key, in the order `config get` (with no key given) prints
    /// them.
    pub const ALL: [ConfigKey; 3] = [
        ConfigKey::Server,
        ConfigKey::Profile,
        ConfigKey::DefaultChannel,
    ];

    /// The kebab-case name this key is known by on the command line — kept
    /// in sync by hand with the `#[value(name = ...)]` attributes above
    /// (clap has no API to read those back) and pinned by a test.
    pub fn name(self) -> &'static str {
        match self {
            ConfigKey::Server => "server",
            ConfigKey::Profile => "profile",
            ConfigKey::DefaultChannel => "default-channel",
        }
    }
}

impl Config {
    /// The current value of `key`, if set.
    pub fn get(&self, key: ConfigKey) -> Option<&str> {
        match key {
            ConfigKey::Server => self.server.as_deref(),
            ConfigKey::Profile => self.profile.as_deref(),
            ConfigKey::DefaultChannel => self.default_channel.as_deref(),
        }
    }

    /// Sets `key` to `value`. Deliberately doesn't validate or save —
    /// callers do both (see `main::run_config_command`), so a rejected
    /// value is reported clearly and never gets written to disk.
    pub fn set(&mut self, key: ConfigKey, value: String) {
        match key {
            ConfigKey::Server => self.server = Some(value),
            ConfigKey::Profile => self.profile = Some(value),
            ConfigKey::DefaultChannel => self.default_channel = Some(value),
        }
    }

    /// Clears `key` back to unset.
    pub fn unset(&mut self, key: ConfigKey) {
        match key {
            ConfigKey::Server => self.server = None,
            ConfigKey::Profile => self.profile = None,
            ConfigKey::DefaultChannel => self.default_channel = None,
        }
    }
}

impl Config {
    /// Clear, specific errors for the failure modes #35 calls out: an
    /// explicitly-blank server URL, a syntactically invalid one, a
    /// non-http(s) scheme, and a profile name that couldn't safely be used
    /// as a keyring namespace component (see `auth::KeyringStore`, which
    /// takes the profile as a plain string service-name suffix). Also
    /// enforces the credential security policy's HTTPS requirement (#52,
    /// see `docs/security-policy.md`) for this field specifically — the
    /// same check also runs on the fully resolved server (config, CLI flag,
    /// or env var) in `main`, since this validation only ever sees the
    /// config file's own value.
    fn validate(&self) -> anyhow::Result<()> {
        if let Some(server) = &self.server {
            if server.trim().is_empty() {
                bail!(
                    "config's server URL is set but empty — remove the `server` line or provide a real URL"
                );
            }
            let url = url::Url::parse(server)
                .with_context(|| format!("config's server URL '{server}' is not a valid URL"))?;
            if url.scheme() != "http" && url.scheme() != "https" {
                bail!(
                    "config's server URL '{server}' must be http or https, not '{}'",
                    url.scheme()
                );
            }
            require_https_outside_local_dev(&url)?;
        }
        if let Some(profile) = &self.profile {
            let valid = !profile.is_empty()
                && profile
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
            if !valid {
                bail!(
                    "config's profile name '{profile}' is invalid — must be non-empty and contain only letters, digits, '-', or '_'"
                );
            }
        }
        Ok(())
    }
}

/// The credential security policy (#52, see `docs/security-policy.md`)
/// requires https for every server this app talks to, except a host that's
/// unambiguously local development — otherwise a bearer token (or the
/// sign-in password) would go out in plaintext over the network to
/// anything else. Called on every candidate server URL: the config file's
/// own value (via `Config::validate`) and the fully resolved one `main`
/// actually connects to (config, `--server`, or `FAMILY_CHAT_URL`), since a
/// CLI flag or env var never goes through `Config::validate` at all.
pub fn require_https_outside_local_dev(url: &url::Url) -> anyhow::Result<()> {
    if url.scheme() == "https" || is_local_dev_host(url) {
        return Ok(());
    }
    bail!(
        "server URL '{url}' uses '{}' — only https is allowed, except for local development \
         (localhost/127.0.0.1/::1)",
        url.scheme()
    );
}

fn is_local_dev_host(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(domain)) => domain == "localhost",
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}

/// Reads and validates the config file, if one exists. A missing file is
/// not an error — it's `Config::default()`, matching a fresh install with
/// no config written yet. A malformed file or an invalid value in an
/// existing file *is* an error (rather than silently falling back to
/// defaults), so a typo gets surfaced immediately instead of silently doing
/// something the user didn't intend.
pub fn load() -> anyhow::Result<Config> {
    load_from(&path()?)
}

fn load_from(path: &Path) -> anyhow::Result<Config> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Config::default());
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("couldn't read config file at {}", path.display()));
        }
    };
    let config: Config = toml::from_str(&contents)
        .with_context(|| format!("config file at {} is not valid TOML", path.display()))?;
    config.validate()?;
    Ok(config)
}

/// Validates and writes `config` to disk, creating the parent directory if
/// needed. Used by the `config set`/`unset` subcommands (#37) — the only
/// way to edit the config file short of hand-editing it.
pub fn save(config: &Config) -> anyhow::Result<()> {
    save_to(config, &path()?)
}

fn save_to(config: &Config, path: &Path) -> anyhow::Result<()> {
    config.validate()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("couldn't create config directory {}", parent.display()))?;
    }
    let serialized = toml::to_string_pretty(config).context("couldn't serialize config")?;
    std::fs::write(path, serialized)
        .with_context(|| format!("couldn't write config file at {}", path.display()))?;
    Ok(())
}

/// Where the config file lives: `$XDG_CONFIG_HOME/family-chat-cli/config.toml`,
/// falling back to `$HOME/.config/family-chat-cli/config.toml` — the same
/// XDG-with-fallback convention `main::log_dir` already uses for state.
pub fn path() -> anyhow::Result<PathBuf> {
    resolve_path(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )
}

/// The pure half of `path()` — env lookups passed in rather than read
/// directly, so the resolution logic is testable without mutating the real
/// process environment (which isn't thread-safe to do from parallel tests).
fn resolve_path(
    xdg_config_home: Option<OsString>,
    home: Option<OsString>,
) -> anyhow::Result<PathBuf> {
    let base = xdg_config_home
        .map(PathBuf::from)
        .or_else(|| home.map(|home| PathBuf::from(home).join(".config")))
        .ok_or_else(|| {
            anyhow::anyhow!("could not determine a config directory (set HOME or XDG_CONFIG_HOME)")
        })?;
    Ok(base.join("family-chat-cli").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_config_home_wins_when_set() {
        let path = resolve_path(Some("/xdg".into()), Some("/home/chris".into())).unwrap();
        assert_eq!(path, PathBuf::from("/xdg/family-chat-cli/config.toml"));
    }

    #[test]
    fn home_is_the_fallback_when_xdg_config_home_is_unset() {
        let path = resolve_path(None, Some("/home/chris".into())).unwrap();
        assert_eq!(
            path,
            PathBuf::from("/home/chris/.config/family-chat-cli/config.toml")
        );
    }

    #[test]
    fn missing_both_env_vars_is_an_error_not_a_panic() {
        assert!(resolve_path(None, None).is_err());
    }

    #[test]
    fn an_empty_file_parses_to_all_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn a_partial_file_leaves_unset_fields_at_their_default() {
        let config: Config = toml::from_str(r#"server = "https://chat.example.com""#).unwrap();
        assert_eq!(config.server.as_deref(), Some("https://chat.example.com"));
        assert_eq!(config.profile, None);
        assert_eq!(config.default_channel, None);
    }

    #[test]
    fn a_full_config_round_trips_through_toml() {
        let config = Config {
            server: Some("https://chat.example.com".to_string()),
            profile: Some("work".to_string()),
            default_channel: Some("General".to_string()),
        };
        let serialized = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(parsed, config);
    }

    #[test]
    fn a_config_with_no_optional_fields_set_validates_fine() {
        Config::default().validate().unwrap();
    }

    #[test]
    fn get_returns_none_for_an_unset_field_and_some_once_set() {
        let mut config = Config::default();
        assert_eq!(config.get(ConfigKey::Server), None);

        config.set(ConfigKey::Server, "https://chat.example.com".to_string());
        assert_eq!(
            config.get(ConfigKey::Server),
            Some("https://chat.example.com")
        );
    }

    #[test]
    fn set_and_unset_round_trip_every_key() {
        for (key, value) in [
            (ConfigKey::Server, "https://chat.example.com"),
            (ConfigKey::Profile, "work"),
            (ConfigKey::DefaultChannel, "General"),
        ] {
            let mut config = Config::default();
            config.set(key, value.to_string());
            assert_eq!(config.get(key), Some(value));

            config.unset(key);
            assert_eq!(
                config.get(key),
                None,
                "unset should clear {key:?} back to None"
            );
        }
    }

    #[test]
    fn config_key_names_are_kebab_case_and_match_the_clap_value_names() {
        // Pins the hand-kept sync between `ConfigKey::name` and each
        // variant's `#[value(name = ...)]` attribute — clap has no API to
        // read the attribute back, so this is the only thing that would
        // catch the two drifting apart.
        assert_eq!(ConfigKey::Server.name(), "server");
        assert_eq!(ConfigKey::Profile.name(), "profile");
        assert_eq!(ConfigKey::DefaultChannel.name(), "default-channel");
        for key in ConfigKey::ALL {
            assert_eq!(
                clap::ValueEnum::to_possible_value(&key).unwrap().get_name(),
                key.name()
            );
        }
    }

    #[test]
    fn an_explicitly_blank_server_url_fails_validation() {
        let config = Config {
            server: Some("   ".to_string()),
            ..Config::default()
        };
        let error = config.validate().unwrap_err().to_string();
        assert!(error.contains("empty"), "unexpected message: {error}");
    }

    #[test]
    fn a_malformed_server_url_fails_validation() {
        let config = Config {
            server: Some("not a url".to_string()),
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn a_non_http_server_url_scheme_fails_validation() {
        let config = Config {
            server: Some("ftp://chat.example.com".to_string()),
            ..Config::default()
        };
        let error = config.validate().unwrap_err().to_string();
        assert!(error.contains("http"), "unexpected message: {error}");
    }

    #[test]
    fn a_valid_https_server_url_passes_validation() {
        let config = Config {
            server: Some("https://chat.example.com".to_string()),
            ..Config::default()
        };
        config.validate().unwrap();
    }

    #[test]
    fn a_plain_http_server_url_against_a_real_host_fails_validation() {
        // The credential security policy (#52): a bearer token (or the
        // sign-in password) must never go out over plaintext http to
        // anything but local development.
        let config = Config {
            server: Some("http://chat.example.com".to_string()),
            ..Config::default()
        };
        let error = config.validate().unwrap_err().to_string();
        assert!(error.contains("https"), "unexpected message: {error}");
    }

    #[test]
    fn require_https_outside_local_dev_allows_http_for_localhost_and_loopback_ips() {
        for url in [
            "http://localhost:3000",
            "http://127.0.0.1:3000",
            "http://[::1]:3000",
        ] {
            require_https_outside_local_dev(&url::Url::parse(url).unwrap())
                .unwrap_or_else(|error| panic!("{url} should be allowed over http: {error}"));
        }
    }

    #[test]
    fn require_https_outside_local_dev_rejects_http_for_a_real_host() {
        let url = url::Url::parse("http://chat.thevachonfamily.ca").unwrap();
        assert!(require_https_outside_local_dev(&url).is_err());
    }

    #[test]
    fn require_https_outside_local_dev_always_allows_https() {
        // https is fine regardless of host, local or not.
        let url = url::Url::parse("https://localhost:3000").unwrap();
        require_https_outside_local_dev(&url).unwrap();
    }

    #[test]
    fn an_empty_profile_name_fails_validation() {
        let config = Config {
            profile: Some(String::new()),
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn a_profile_name_with_unsafe_characters_fails_validation() {
        let config = Config {
            profile: Some("../etc/passwd".to_string()),
            ..Config::default()
        };
        let error = config.validate().unwrap_err().to_string();
        assert!(error.contains("invalid"), "unexpected message: {error}");
    }

    #[test]
    fn a_missing_file_loads_as_defaults_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let config = load_from(&dir.path().join("does-not-exist.toml")).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn a_malformed_file_fails_to_load_with_a_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "this is not [ valid toml").unwrap();

        let error = load_from(&path).unwrap_err().to_string();
        assert!(
            error.contains("not valid TOML"),
            "unexpected message: {error}"
        );
    }

    #[test]
    fn an_invalid_value_in_an_existing_file_fails_to_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, r#"server = "not a url""#).unwrap();

        assert!(load_from(&path).is_err());
    }

    #[test]
    fn save_then_load_round_trips_through_the_real_filesystem() {
        let dir = tempfile::tempdir().unwrap();
        // A nested, not-yet-created directory — save_to must create it.
        let path = dir.path().join("nested").join("config.toml");
        let config = Config {
            server: Some("https://chat.example.com".to_string()),
            profile: Some("work".to_string()),
            default_channel: Some("General".to_string()),
        };

        save_to(&config, &path).unwrap();
        let loaded = load_from(&path).unwrap();

        assert_eq!(loaded, config);
    }

    #[test]
    fn save_refuses_to_write_an_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let config = Config {
            server: Some("not a url".to_string()),
            ..Config::default()
        };

        assert!(save_to(&config, &path).is_err());
        assert!(
            !path.exists(),
            "an invalid config should never be written to disk"
        );
    }
}
