mod api;
mod app;
mod auth;
mod cli;
mod config;
mod text;
mod tui;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use api::ApiClient;
use auth::KeyringStore;

fn main() -> anyhow::Result<()> {
    let _log_guard = init_logging()?;
    install_panic_hook();

    let args = cli::Cli::parse();
    let store = KeyringStore::new("default")?;
    let client = ApiClient::new(args.server);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(tui::run(client, Box::new(store)))
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

/// Placeholder location until #34/#35 define the real config/state directory layout.
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
