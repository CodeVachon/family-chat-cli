//! REST client, domain types, and the SSE stream. See docs/api-contract.md.

pub mod client;
pub mod error;
mod stream;
// Wire-format mirrors of server JSON — not every field this prototype
// deserializes is read yet (e.g. `Channel::is_private`, kept for when
// channel-visibility UI lands).
#[allow(dead_code)]
pub mod types;

pub use client::ApiClient;
pub use error::ApiError;
