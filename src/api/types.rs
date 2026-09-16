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
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagesResponse {
    pub messages: Vec<Message>,
}
