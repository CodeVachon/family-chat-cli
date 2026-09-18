//! The unified event enum (things the loop reacts to) and the command enum
//! (async side effects the pure state layer asks the loop to perform) (#12/#27).

use chrono::{DateTime, Utc};

use crate::api::ApiError;
use crate::api::types::{Channel, ChannelMember, Message, RealtimeEvent, User};

/// Results of async work started by a `Command`, delivered back to the main
/// loop over an mpsc channel. Terminal key events don't go through here —
/// they're handled directly as `crossterm::event::KeyEvent`, since they need
/// no round trip through a channel.
#[derive(Debug)]
pub enum Event {
    ResumeFinished(Option<User>),
    LoginFinished(Result<User, ApiError>),
    ChannelsLoaded(Result<Vec<Channel>, ApiError>),
    MessagesLoaded {
        channel_id: String,
        seq: u64,
        /// `(messages, has_more)` — `has_more` says whether the server has
        /// even older messages beyond this page (#30/#51).
        result: Result<(Vec<Message>, bool), ApiError>,
    },
    OlderMessagesLoaded {
        channel_id: String,
        result: Result<(Vec<Message>, bool), ApiError>,
    },
    MessageSent {
        channel_id: String,
        result: Result<(), ApiError>,
    },
    /// The response to a members fetch that rides along with every
    /// `Command::LoadMessages` (#50) — no dedicated `Command` variant exists
    /// for it since `tui::run` spawns the fetch itself whenever it handles
    /// `LoadMessages`, the same way it fires `mark_channel_read`.
    MembersLoaded {
        channel_id: String,
        result: Result<Vec<ChannelMember>, ApiError>,
    },
    /// The response to a thread fetch (#60) — spawned directly by
    /// `tui::run` whenever `AppState::threads_needing_fetch` says one's
    /// needed (right after a successful messages load), the same
    /// no-dedicated-`Command` pattern as `MembersLoaded`.
    ThreadLoaded {
        root_id: String,
        result: Result<Vec<Message>, ApiError>,
    },
    Realtime(RealtimeEvent),
}

/// An async side effect the state layer needs performed. Kept separate from
/// `Event` so `app::state` stays pure (no IO) and unit-testable — the main
/// loop is what actually executes these against the `ApiClient`.
#[derive(Debug, Clone)]
pub enum Command {
    SubmitLogin {
        email: String,
        password: String,
    },
    LoadChannels,
    /// Fetches the *latest* page for `channel_id`, replacing whatever's
    /// cached (a channel switch, a manual refresh, or an SSE-triggered
    /// reload — always the newest page, never a stale one, since going back
    /// to an already-cached channel used to silently skip refreshing it).
    LoadMessages {
        channel_id: String,
        seq: u64,
    },
    /// Fetches the page immediately before `(before_id, before_created_at)`
    /// — the oldest currently-loaded message — and prepends it (#30).
    LoadOlderMessages {
        channel_id: String,
        before_id: String,
        before_created_at: DateTime<Utc>,
    },
    SendMessage {
        channel_id: String,
        body: String,
        /// `Some(root_id)` when composed while replying to a thread (#61) —
        /// confirmed live that the server accepts this field on
        /// `POST .../messages` and creates a real reply (round-tripped
        /// through `GET .../thread` to verify).
        thread_root_id: Option<String>,
    },
    Logout,
    Quit,
}
