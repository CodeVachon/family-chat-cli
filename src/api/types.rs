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

/// One row of `GET /channels/:id/members` — the server has no flat,
/// cross-channel "list all users" endpoint (confirmed against the live
/// server and docs/api-contract.md's resource list), only per-channel
/// membership and a single-user profile lookup. This is the closest match
/// to a "list users" capability, scoped to one channel at a time.
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelMember {
    #[serde(rename = "userId")]
    pub user_id: String,
    pub role: String,
    pub name: String,
    #[serde(rename = "colorHue")]
    pub color_hue: Option<i64>,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelMembersResponse {
    pub members: Vec<ChannelMember>,
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

/// Present (with `kind == "system"`) on channel events — joins, leaves,
/// additions/removals by another member, renames, and so on. `body` is
/// empty for these; the renderer builds its text from this instead (#60).
/// `subject_user_id` is only present on membership events (`join`/`leave`)
/// — a `channel_updated` event, for example, has no subject, only an actor.
#[derive(Debug, Clone, Deserialize)]
pub struct SystemEvent {
    pub event: String,
    #[serde(rename = "actorUserId")]
    pub actor_user_id: String,
    #[serde(rename = "subjectUserId", default)]
    pub subject_user_id: Option<String>,
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
    #[serde(rename = "systemEvent", default)]
    pub system_event: Option<SystemEvent>,
    pub author: MessageAuthor,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagesResponse {
    pub messages: Vec<Message>,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
}

/// The `GET /api/v1/stream` payload (see docs/api-contract.md). Every event
/// arrives as a default-`message` SSE frame with its `type` inside the JSON
/// body, so this is what a `MessageEvent.data` deserializes into — not tied
/// to the SSE `event:` field at all.
///
/// Only the kinds this prototype acts on get their own variant; everything
/// else (typing, presence, reactions, mentions, users/settings changes —
/// nothing the TUI renders yet) falls into `Other` and is ignored.
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
    /// Someone (this client or another one, e.g. the web app) marked a
    /// channel read — refetch channels to pick up the new unread counts
    /// (#33). The exact payload shape isn't confirmed against a live event
    /// yet, so this ignores it and just triggers the same full reload
    /// `ChannelsChanged` does, which is always correct even if a more
    /// targeted update would also be possible.
    #[serde(rename = "read.updated")]
    ReadUpdated,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A real `channel_updated` system event has no `subjectUserId` at all
    /// (only `join`/`leave` do) — this used to be a required field, which
    /// broke decoding every message in a channel the moment one rename
    /// event showed up in its history (#60).
    #[test]
    fn a_system_event_without_a_subject_still_deserializes() {
        let json = r#"{
            "id": "m1",
            "type": "system",
            "body": "",
            "createdAt": "2026-07-28T16:00:12.788Z",
            "deletedAt": null,
            "systemEvent": {
                "event": "channel_updated",
                "renamedTo": "New Name",
                "actorUserId": "u1"
            },
            "author": {"id": "u1", "name": "Christopher"}
        }"#;

        let message: Message = serde_json::from_str(json).expect("should deserialize");
        let event = message
            .system_event
            .expect("system_event should be present");
        assert_eq!(event.event, "channel_updated");
        assert_eq!(event.subject_user_id, None);
    }

    #[test]
    fn a_read_updated_event_deserializes() {
        let event: RealtimeEvent =
            serde_json::from_str(r#"{"type": "read.updated"}"#).expect("should deserialize");
        assert!(matches!(event, RealtimeEvent::ReadUpdated));
    }
}
