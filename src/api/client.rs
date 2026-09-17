//! `ApiClient`: base URL + bearer token + JSON request helpers (#20).

use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::de::DeserializeOwned;

use super::error::ApiError;
use super::types::{
    ChannelMembersResponse, ChannelsResponse, MeResponse, MessagesResponse, SignInResponse,
};

#[derive(Clone)]
pub struct ApiClient {
    http: Client,
    base_url: Arc<str>,
    token: Arc<RwLock<Option<String>>>,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: Client::builder()
                .user_agent(concat!("family-chat-cli/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("reqwest client builds with no custom TLS config"),
            base_url: Arc::from(base_url.into()),
            token: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_token(&self, token: Option<String>) {
        *self.token.write().expect("token lock poisoned") = token;
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    fn authed(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let token = self.token.read().expect("token lock poisoned").clone();
        match token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    async fn json_or_error<T: DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<T, ApiError> {
        if response.status().is_success() {
            Ok(response.json::<T>().await?)
        } else {
            Err(ApiError::from_response(response).await)
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let response = self.authed(self.http.get(self.url(path))).send().await?;
        Self::json_or_error(response).await
    }

    /// `POST /api/auth/sign-in/email` — not under `/api/v1`, and not bearer-authed
    /// (there's no session yet). On success the response carries the session token
    /// directly in its JSON body (confirmed against the Better Auth source; the
    /// `set-auth-token` header carries the same value but the body is simpler to read).
    pub async fn sign_in_email(
        &self,
        email: &str,
        password: &str,
    ) -> Result<SignInResponse, ApiError> {
        let response = self
            .http
            .post(self.url("/api/auth/sign-in/email"))
            .json(&serde_json::json!({ "email": email, "password": password }))
            .send()
            .await?;
        if response.status().is_success() {
            Ok(response.json::<SignInResponse>().await?)
        } else {
            Err(ApiError::from_auth_response(response).await)
        }
    }

    /// `POST /api/auth/sign-out` — best-effort; callers should clear local
    /// credentials regardless of whether this succeeds (see #17).
    pub async fn sign_out(&self) -> Result<(), ApiError> {
        let response = self
            .authed(self.http.post(self.url("/api/auth/sign-out")))
            .send()
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(ApiError::from_auth_response(response).await)
        }
    }

    pub async fn me(&self) -> Result<MeResponse, ApiError> {
        self.get("/api/v1/me").await
    }

    pub async fn list_channels(&self) -> Result<ChannelsResponse, ApiError> {
        self.get("/api/v1/channels").await
    }

    /// `GET /channels/:id/members` — the nearest thing to "list users" the
    /// server exposes (see `ChannelMember`'s doc comment for why this is
    /// per-channel rather than instance-wide). Not yet consumed by the TUI
    /// (see #50), hence the `#[allow(dead_code)]`.
    #[allow(dead_code)]
    pub async fn channel_members(
        &self,
        channel_id: &str,
    ) -> Result<ChannelMembersResponse, ApiError> {
        self.get(&format!("/api/v1/channels/{channel_id}/members"))
            .await
    }

    /// `before`, when given, is the oldest currently-loaded message's
    /// `(id, created_at)` — the keyset cursor the server pages backward
    /// from (see docs/api-contract.md). `None` fetches the latest page.
    pub async fn channel_messages(
        &self,
        channel_id: &str,
        before: Option<(&str, DateTime<Utc>)>,
    ) -> Result<MessagesResponse, ApiError> {
        let mut request = self.authed(
            self.http
                .get(self.url(&format!("/api/v1/channels/{channel_id}/messages"))),
        );
        if let Some((before_id, before_created_at)) = before {
            request = request.query(&[
                ("beforeId", before_id),
                ("beforeCreatedAt", &before_created_at.to_rfc3339()),
            ]);
        }
        let response = request.send().await?;
        Self::json_or_error(response).await
    }

    /// `body_html` is a full message body (sanitized HTML — see
    /// `text::html::plain_text_to_html`), not plain text. The response's
    /// `message` is the raw insert row (no author/reactions/mentions), so
    /// this deliberately doesn't try to parse and return it — callers
    /// reconcile by reloading the channel's messages.
    pub async fn send_message(&self, channel_id: &str, body_html: &str) -> Result<(), ApiError> {
        let response = self
            .authed(
                self.http
                    .post(self.url(&format!("/api/v1/channels/{channel_id}/messages"))),
            )
            .json(&serde_json::json!({ "body": body_html }))
            .send()
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(ApiError::from_response(response).await)
        }
    }

    /// An authenticated, unsent request for `GET /api/v1/stream` — handed to
    /// `reqwest_eventsource::EventSource`, which owns actually sending it
    /// (and re-sending it on reconnect).
    pub(crate) fn stream_request(&self) -> reqwest::RequestBuilder {
        self.authed(self.http.get(self.url("/api/v1/stream")))
    }
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    /// Exercises the real (de)serialization against payloads shaped like the
    /// actual server (see docs/api-contract.md), not just hand-picked field
    /// names — this is what caught #48's `set-auth-token` vs body-`token`
    /// question and the sign-in error message bug during manual testing.
    #[tokio::test]
    async fn sign_in_parses_the_token_and_user_from_the_response_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/auth/sign-in/email"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "redirect": false,
                "token": "tok_abc123",
                "url": null,
                "user": {
                    "id": "u1",
                    "name": "Chris",
                    "email": "chris@example.com",
                    "approvalStatus": "approved"
                }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri());
        let response = client
            .sign_in_email("chris@example.com", "hunter2")
            .await
            .unwrap();

        assert_eq!(response.token, "tok_abc123");
        assert_eq!(response.user.name, "Chris");
        assert_eq!(response.user.approval_status, "approved");
    }

    #[tokio::test]
    async fn sign_in_surfaces_the_servers_own_error_message() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/auth/sign-in/email"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "message": "Invalid email or password",
                "code": "INVALID_EMAIL_OR_PASSWORD"
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri());
        let error = client
            .sign_in_email("chris@example.com", "wrong")
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), "Invalid email or password");
    }

    #[tokio::test]
    async fn authenticated_calls_send_the_bearer_token() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/channels"))
            .and(header("Authorization", "Bearer tok_abc123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "channels": [
                    {
                        "id": "c1",
                        "name": "General",
                        "description": null,
                        "isPrivate": false,
                        "isArchived": false,
                        "isFavorite": true,
                        "unreadCount": 3
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri());
        client.set_token(Some("tok_abc123".to_string()));
        let response = client.list_channels().await.unwrap();

        assert_eq!(response.channels.len(), 1);
        assert_eq!(response.channels[0].name, "General");
        assert_eq!(response.channels[0].unread_count, 3);
    }

    /// Payload shape confirmed live against the real server for a channel
    /// with a mix of a null-avatar owner and members with avatars.
    #[tokio::test]
    async fn channel_members_parses_the_real_server_shape() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/channels/c1/members"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "members": [
                    {
                        "userId": "u1",
                        "role": "owner",
                        "name": "Louise",
                        "colorHue": 220,
                        "avatarUrl": null
                    },
                    {
                        "userId": "u2",
                        "role": "user",
                        "name": "Christopher",
                        "colorHue": 210,
                        "avatarUrl": "https://res.cloudinary.com/example.jpg"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri());
        let response = client.channel_members("c1").await.unwrap();

        assert_eq!(response.members.len(), 2);
        assert_eq!(response.members[0].role, "owner");
        assert_eq!(response.members[0].avatar_url, None);
        assert_eq!(
            response.members[1].avatar_url.as_deref(),
            Some("https://res.cloudinary.com/example.jpg")
        );
    }

    #[tokio::test]
    async fn messages_parse_nested_author_and_timestamps() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/channels/c1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "messages": [
                    {
                        "id": "m1",
                        "type": "user",
                        "body": "<p>Hello <strong>family</strong></p>",
                        "createdAt": "2026-09-16T12:34:56.000Z",
                        "deletedAt": null,
                        "author": {
                            "id": "u1",
                            "name": "Chris",
                            "preferences": { "displayName": "Dad" }
                        }
                    }
                ],
                "hasMore": false
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri());
        let response = client.channel_messages("c1", None).await.unwrap();

        assert_eq!(response.messages.len(), 1);
        assert!(!response.has_more);
        assert_eq!(response.messages[0].author.display_name(), "Dad");
        assert_eq!(
            response.messages[0].created_at.to_rfc3339(),
            "2026-09-16T12:34:56+00:00"
        );
    }

    #[tokio::test]
    async fn channel_messages_sends_the_before_cursor_when_paginating() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/channels/c1/messages"))
            .and(wiremock::matchers::query_param("beforeId", "m1"))
            .and(wiremock::matchers::query_param(
                "beforeCreatedAt",
                "2026-09-16T12:34:56+00:00",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "messages": [],
                "hasMore": true
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri());
        let cursor_time = chrono::DateTime::parse_from_rfc3339("2026-09-16T12:34:56Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let response = client
            .channel_messages("c1", Some(("m1", cursor_time)))
            .await
            .unwrap();

        assert!(response.has_more);
    }

    #[tokio::test]
    async fn send_message_posts_the_html_body_with_the_bearer_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/channels/c1/messages"))
            .and(header("Authorization", "Bearer tok_abc123"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "message": { "id": "m1", "channelId": "c1", "body": "<p>hi</p>" }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri());
        client.set_token(Some("tok_abc123".to_string()));

        client.send_message("c1", "<p>hi</p>").await.unwrap();
    }

    #[tokio::test]
    async fn send_message_surfaces_a_validation_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/channels/c1/messages"))
            .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({
                "error": { "message": "Message cannot be empty" }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri());
        let error = client.send_message("c1", "<p></p>").await.unwrap_err();

        assert_eq!(error.to_string(), "Message cannot be empty");
    }
}
