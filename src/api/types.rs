//! Domain types mirroring the `/api/v1` contract (#19). See docs/api-contract.md.
//!
//! Only the fields this prototype actually reads are modeled — serde ignores
//! whatever else the server sends.

use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    #[serde(rename = "approvalStatus")]
    pub approval_status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SignInResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MeResponse {
    pub user: User,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "isPrivate")]
    pub is_private: bool,
    #[serde(rename = "isArchived")]
    pub is_archived: bool,
    #[serde(rename = "isFavorite")]
    pub is_favorite: bool,
    #[serde(rename = "unreadCount")]
    pub unread_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelsResponse {
    pub channels: Vec<Channel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageAuthorPreferences {
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageAuthor {
    pub id: String,
    pub name: String,
    pub preferences: Option<MessageAuthorPreferences>,
}

impl MessageAuthor {
    /// The name to render — the user's chosen display name if they've set
    /// one, otherwise their account name.
    pub fn display_name(&self) -> &str {
        self.preferences
            .as_ref()
            .and_then(|p| p.display_name.as_deref())
            .unwrap_or(&self.name)
    }
}

/// One uploaded image/video/pdf/file (Cloudinary-hosted — see
/// docs/api-contract.md). `width`/`height` are only meaningful for `image`
/// (and sometimes `video`); the server sends `null` for the rest, hence
/// `Option`.
#[derive(Debug, Clone, Deserialize)]
pub struct Attachment {
    pub kind: String,
    #[serde(rename = "secureUrl")]
    pub secure_url: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
    #[serde(rename = "originalFilename")]
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub body: String,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
    pub author: MessageAuthor,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagesResponse {
    pub messages: Vec<Message>,
}

/// The `GET /api/v1/stream` payload (see docs/api-contract.md). Every event
/// arrives as a default-`message` SSE frame with its `type` inside the JSON
/// body, so this is what a `MessageEvent.data` deserializes into — not tied
/// to the SSE `event:` field at all.
///
/// Only the kinds this prototype acts on get their own variant; everything
/// else (typing, presence, reactions, mentions, read receipts, users/settings
/// changes — nothing the TUI renders yet) falls into `Other` and is ignored.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum RealtimeEvent {
    #[serde(rename = "ready")]
    Ready,
    /// The broker reconnected to Postgres and may have missed notifications
    /// — refetch anything being tracked rather than trust local state.
    #[serde(rename = "resync")]
    Resync,
    #[serde(rename = "channels.changed")]
    ChannelsChanged,
    #[serde(rename = "message.created")]
    MessageCreated {
        #[serde(rename = "channelId")]
        channel_id: String,
    },
    #[serde(rename = "message.updated")]
    MessageUpdated {
        #[serde(rename = "channelId")]
        channel_id: String,
    },
    #[serde(rename = "message.deleted")]
    MessageDeleted {
        #[serde(rename = "channelId")]
        channel_id: String,
    },
    #[serde(other)]
    Other,
}
