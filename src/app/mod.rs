//! Application state and event model (#12). Pure, no IO.

pub mod event;
pub mod state;

pub use event::{Command, Event};
pub use state::AppState;
