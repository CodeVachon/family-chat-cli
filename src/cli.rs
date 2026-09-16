use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "family-chat-cli", about = "Terminal client for Family Chat")]
pub struct Cli {
    /// Base URL of the Family Chat server.
    #[arg(
        long,
        env = "FAMILY_CHAT_URL",
        default_value = "https://chat.thevachonfamily.ca"
    )]
    pub server: String,
}
