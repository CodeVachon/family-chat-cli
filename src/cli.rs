use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "family-chat-cli", about = "Terminal client for Family Chat")]
pub struct Cli {
    /// Base URL of the Family Chat server. Falls back to the config file's
    /// `server`, then to a built-in default, when neither this flag nor
    /// `FAMILY_CHAT_URL` is set (see `main::resolve_server`).
    #[arg(long, env = "FAMILY_CHAT_URL")]
    pub server: Option<String>,
}
