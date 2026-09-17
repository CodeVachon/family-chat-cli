mod api;
mod app;
mod auth;
mod cli;
mod config;
mod text;
mod tui;

use anyhow::Context;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use api::ApiClient;
use auth::KeyringStore;

/// The server to connect to when neither `--server`, `FAMILY_CHAT_URL`, nor
/// the config file's `server` is set.
const DEFAULT_SERVER: &str = "https://chat.thevachonfamily.ca";

/// The keyring profile to use when the config file doesn't set one.
const DEFAULT_PROFILE: &str = "default";

fn main() -> anyhow::Result<()> {
    let _log_guard = init_logging()?;
    install_panic_hook();

    let args = cli::Cli::parse();
    let config = config::load()?;
    let server = resolve_server(args.server, config.server);
    let profile = config
        .profile
        .unwrap_or_else(|| DEFAULT_PROFILE.to_string());
    // The credential store is namespaced by profile *and* server origin
    // (#36) — otherwise switching --server under the same profile could
    // silently reuse (or clobber) a session token that belongs to a
    // different server.
    let origin = url::Url::parse(&server)
        .with_context(|| format!("server URL '{server}' is not valid"))?
        .origin()
        .ascii_serialization();
    let store = KeyringStore::new(&profile, &origin)?;
    let client = ApiClient::new(server);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(tui::run(client, Box::new(store)))
}

/// `--server`/`FAMILY_CHAT_URL` (clap already merges those two into one
/// `Option`) wins over the config file's `server`, which wins over the
/// built-in default — the precedence promised in config::Config's doc
/// comment.
fn resolve_server(cli_server: Option<String>, config_server: Option<String>) -> String {
    cli_server
        .or(config_server)
        .unwrap_or_else(|| DEFAULT_SERVER.to_string())
}

/// Logs go to a file: once the TUI enables raw mode and the alternate screen,
/// anything written to stdout *or* stderr lands on the same terminal and
/// corrupts the display.
fn init_logging() -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    let dir = log_dir()?;
    std::fs::create_dir_all(&dir)?;
    let file_appender = tracing_appender::rolling::daily(dir, "family-chat-cli.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(writer)
        .with_ansi(false)
        .init();

    Ok(guard)
}

/// The state directory (logs) is XDG-fallback like `config::path`, but a
/// separate location by convention — `XDG_STATE_HOME`, not
/// `XDG_CONFIG_HOME`, since logs aren't user-editable settings.
fn log_dir() -> anyhow::Result<std::path::PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".local/state"))
        })
        .ok_or_else(|| {
            anyhow::anyhow!("could not determine a log directory (set HOME or XDG_STATE_HOME)")
        })?;
    Ok(base.join("family-chat-cli"))
}

/// Restore the terminal before the default panic handler prints — otherwise a
/// panic mid-session leaves the terminal in raw/alternate-screen mode (#13/#28).
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = tui::restore_terminal();
        default_hook(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cli_flag_wins_over_the_config_file() {
        let server = resolve_server(
            Some("https://cli.example.com".to_string()),
            Some("https://config.example.com".to_string()),
        );
        assert_eq!(server, "https://cli.example.com");
    }

    #[test]
    fn the_config_file_wins_over_the_default_when_the_cli_flag_is_unset() {
        let server = resolve_server(None, Some("https://config.example.com".to_string()));
        assert_eq!(server, "https://config.example.com");
    }

    #[test]
    fn the_built_in_default_is_used_when_neither_is_set() {
        let server = resolve_server(None, None);
        assert_eq!(server, DEFAULT_SERVER);
    }
}
