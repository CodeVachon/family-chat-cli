//! The unified event enum (things the loop reacts to) and the command enum
//! (async side effects the pure state layer asks the loop to perform) (#12/#27).

use crate::api::ApiError;
use crate::api::types::{Channel, Message, User};

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
        result: Result<Vec<Message>, ApiError>,
    },
}

/// An async side effect the state layer needs performed. Kept separate from
/// `Event` so `app::state` stays pure (no IO) and unit-testable — the main
/// loop is what actually executes these against the `ApiClient`.
#[derive(Debug, Clone)]
pub enum Command {
    SubmitLogin { email: String, password: String },
    LoadChannels,
    LoadMessages { channel_id: String },
    Logout,
    Quit,
}
