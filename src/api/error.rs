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
    #[error(transparent)]
    Request(#[from] reqwest::Error),
}

impl ApiError {
    async fn message_from(response: reqwest::Response) -> (reqwest::StatusCode, String) {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let message = serde_json::from_str::<V1ErrorBody>(&body)
            .map(|b| b.error.message)
            .or_else(|_| serde_json::from_str::<AuthErrorBody>(&body).map(|b| b.message))
            .unwrap_or_else(|_| {
                status
                    .canonical_reason()
                    .unwrap_or("request failed")
                    .to_string()
            });
        (status, message)
    }

    /// For `/api/v1/*` calls, which assume a session already exists: a 401
    /// means that session died (expired/revoked), so the specific server
    /// message ("Authentication required") isn't worth surfacing over the
    /// generic `Unauthorized` — callers branch on the variant to drop the
    /// dead session and re-prompt (see `auth::login::resume`).
    pub(crate) async fn from_response(response: reqwest::Response) -> ApiError {
        let (status, message) = Self::message_from(response).await;
        match status.as_u16() {
            401 => ApiError::Unauthorized,
            403 if message.to_lowercase().contains("approved") => ApiError::NotApproved,
            403 => ApiError::Forbidden,
            _ => ApiError::Server(message),
        }
    }

    /// For `/api/auth/*` calls (sign-in, sign-out): there's no existing
    /// session to have expired, so a 401 here always means "wrong
    /// credentials" — the specific message is exactly what the login form
    /// should show, not the generic `Unauthorized`.
    pub(crate) async fn from_auth_response(response: reqwest::Response) -> ApiError {
        let (_, message) = Self::message_from(response).await;
        ApiError::Server(message)
    }
}
