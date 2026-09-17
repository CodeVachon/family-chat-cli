//! `ApiError` mirroring the server's `{error:{message}}` / 422 `issues` envelope (#23).
//!
//! Better Auth's own routes (`/api/auth/*`) use a different, flatter shape —
//! `{message, code}` — than the versioned `/api/v1/*` routes (`{error:{message}}`).
//! `from_response` tries both.

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Deserialize)]
struct V1ErrorBody {
    error: V1ErrorInner,
}

#[derive(Debug, Deserialize)]
struct V1ErrorInner {
    message: String,
    /// Only present on a 422 (Zod validation failure) — payload confirmed
    /// live against the real server (#23): each issue carries at least
    /// `path` (the field, as JSON-path segments) and `message`.
    #[serde(default)]
    issues: Vec<ValidationIssue>,
}

#[derive(Debug, Deserialize)]
struct ValidationIssue {
    #[serde(default)]
    path: Vec<serde_json::Value>,
    message: String,
}

impl ValidationIssue {
    /// `"body: Message cannot be empty"` — falls back to just the message
    /// if `path` is empty (seen on some Zod issue kinds).
    fn display(&self) -> String {
        if self.path.is_empty() {
            return self.message.clone();
        }
        let path = self
            .path
            .iter()
            .map(|segment| match segment {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join(".");
        format!("{path}: {}", self.message)
    }
}

#[derive(Debug, Deserialize)]
struct AuthErrorBody {
    message: String,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    Server(String),
    #[error("authentication required")]
    Unauthorized,
    /// A session exists but the account isn't `approved` yet (still pending
    /// admin review, or rejected — the server doesn't distinguish in the
    /// message, see docs/api-contract.md).
    #[error("account is not approved")]
    NotApproved,
    #[error("not authorized")]
    Forbidden,
    /// 422 — a Zod validation failure (see docs/api-contract.md). The
    /// message already combines every issue into one readable string
    /// (`"path: message"` per issue, joined with `"; "`), since nothing in
    /// this prototype needs the structured issues themselves, only
    /// something to show the user.
    #[error("{0}")]
    Validation(String),
    /// 429 — the server already formats a human-readable retry message
    /// from its own rate-limit window (see docs/api-contract.md), so this
    /// is a distinctly-named passthrough rather than a generic `Server`.
    #[error("{0}")]
    RateLimited(String),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
}

impl ApiError {
    async fn message_from(
        response: reqwest::Response,
    ) -> (reqwest::StatusCode, String, Vec<String>) {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let parsed = serde_json::from_str::<V1ErrorBody>(&body)
            .map(|b| (b.error.message, b.error.issues))
            .or_else(|_| {
                serde_json::from_str::<AuthErrorBody>(&body).map(|b| (b.message, Vec::new()))
            });
        match parsed {
            Ok((message, issues)) => {
                let issues = issues.iter().map(ValidationIssue::display).collect();
                (status, message, issues)
            }
            Err(_) => {
                let message = status
                    .canonical_reason()
                    .unwrap_or("request failed")
                    .to_string();
                (status, message, Vec::new())
            }
        }
    }

    /// For `/api/v1/*` calls, which assume a session already exists: a 401
    /// means that session died (expired/revoked), so the specific server
    /// message ("Authentication required") isn't worth surfacing over the
    /// generic `Unauthorized` — callers branch on the variant to drop the
    /// dead session and re-prompt (see `auth::login::resume`).
    pub(crate) async fn from_response(response: reqwest::Response) -> ApiError {
        let (status, message, issues) = Self::message_from(response).await;
        match status.as_u16() {
            401 => ApiError::Unauthorized,
            403 if message.to_lowercase().contains("approved") => ApiError::NotApproved,
            403 => ApiError::Forbidden,
            422 => {
                let combined = if issues.is_empty() {
                    message
                } else {
                    issues.join("; ")
                };
                ApiError::Validation(combined)
            }
            429 => ApiError::RateLimited(message),
            _ => ApiError::Server(message),
        }
    }

    /// For `/api/auth/*` calls (sign-in, sign-out): there's no existing
    /// session to have expired, so a 401 here always means "wrong
    /// credentials" — the specific message is exactly what the login form
    /// should show, not the generic `Unauthorized`.
    pub(crate) async fn from_auth_response(response: reqwest::Response) -> ApiError {
        let (_, message, _) = Self::message_from(response).await;
        ApiError::Server(message)
    }
}
