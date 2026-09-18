use clap::{Parser, Subcommand};

use crate::config::ConfigKey;

#[derive(Parser, Debug)]
#[command(name = "family-chat-cli", about = "Terminal client for Family Chat")]
pub struct Cli {
    /// Base URL of the Family Chat server. Falls back to the config file's
    /// `server`, then to a built-in default, when neither this flag nor
    /// `FAMILY_CHAT_URL` is set (see `main::resolve_server`). Ignored by the
    /// `config` subcommand, which only ever reads/writes the config file.
    #[arg(long, env = "FAMILY_CHAT_URL")]
    pub server: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// With no subcommand given, `Cli::command` is `None` and `main` starts the
/// TUI as before — `config` is the only subcommand so far (#37).
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Read or edit the non-secret config file (server URL, profile,
    /// default channel) without hand-editing it.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Print the config file's path (it may not exist yet).
    Path,
    /// Print a config value, or every value when no key is given.
    Get { key: Option<ConfigKey> },
    /// Set a config value and save the file.
    Set { key: ConfigKey, value: String },
    /// Clear a config value back to unset and save the file.
    Unset { key: ConfigKey },
}
