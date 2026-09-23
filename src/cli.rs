use clap::{Args, Parser, Subcommand};

use crate::config::ConfigKey;

#[derive(Parser, Debug)]
#[command(
    name = "family-chat-cli",
    about = "Terminal client for Family Chat",
    version = crate::VERSION
)]
pub struct Cli {
    /// Base URL of the Family Chat server. Falls back to the config file's
    /// `server`, then to a built-in default, when neither this flag nor
    /// `FAMILY_CHAT_URL` is set (see `main::resolve_server`). Ignored by the
    /// `config`/`upgrade`/`uninstall` subcommands, which don't talk to the
    /// chat server at all.
    #[arg(long, env = "FAMILY_CHAT_URL")]
    pub server: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// With no subcommand given, `Cli::command` is `None` and `main` starts the
/// TUI as before.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Read or edit the non-secret config file (server URL, profile,
    /// default channel) without hand-editing it.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Update family-chat-cli in place, or check whether an update is available
    Upgrade(UpgradeArgs),
    /// Remove the managed installation from this machine
    Uninstall(UninstallArgs),
}

#[derive(Debug, Clone, Default, Args)]
pub struct UpgradeArgs {
    /// Install this version instead of the newest
    pub target: Option<String>,

    /// Report whether an update is available, changing nothing
    #[arg(long)]
    pub check: bool,

    /// How many versions to keep on disk
    #[arg(long, default_value_t = 2, value_name = "N")]
    pub keep: usize,

    /// Reinstall even if already on the target version
    #[arg(long)]
    pub force: bool,

    /// Machine-readable output
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone, Default, Args)]
pub struct UninstallArgs {
    /// Skip the confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Machine-readable output
    #[arg(long)]
    pub json: bool,
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
