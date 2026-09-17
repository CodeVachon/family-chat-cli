//! Config file format and locations (#34).
//!
//! Non-secret settings only — credentials live in the OS keyring (see
//! `auth::store`), never here. Loading, saving, and validating this file
//! against the CLI/env overrides is #35's job; this module just defines the
//! shape and where it lives on disk, so nothing here is wired into `main`
//! yet.
#![allow(dead_code)]

use std::ffi::OsString;
use std::path::PathBuf;

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
    /// this when both are set (#35 wires up that precedence).
    pub server: Option<String>,

    /// Which keyring/session profile to use (see `auth::KeyringStore`),
    /// letting more than one account or environment keep separate stored
    /// sessions rather than one clobbering the other's (#36).
    pub profile: Option<String>,

    /// The channel to select on startup, matched by name.
    pub default_channel: Option<String>,
    // Non-secret UI preferences (the plan's fourth category) are
    // deliberately not modeled yet — there isn't a concrete one to define a
    // shape for. Add a field (or a nested table) here when one exists;
    // `#[serde(default)]` above means that's additive too.
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
}
