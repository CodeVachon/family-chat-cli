//! Email/password sign-in against `/api/auth/sign-in/email` (#15), plus
//! resuming/dropping a stored session (#17).

use crate::api::types::User;
use crate::api::{ApiClient, ApiError};

use super::store::CredentialStore;

pub struct LoggedInSession {
    pub user: User,
}

pub async fn sign_in(
    client: &ApiClient,
    store: &dyn CredentialStore,
    email: &str,
    password: &str,
) -> Result<LoggedInSession, ApiError> {
    let response = client.sign_in_email(email, password).await?;
    client.set_token(Some(response.token.clone()));
    store.save(&response.token);
    Ok(LoggedInSession {
        user: response.user,
    })
}

pub async fn sign_out(client: &ApiClient, store: &dyn CredentialStore) {
    // Best-effort: proceed with local cleanup even if the server call fails
    // (e.g. offline) — see #17.
    let _ = client.sign_out().await;
    client.set_token(None);
    store.clear();
}

/// Resume a session from a stored token, if one exists and is still valid.
/// A dead or rejected token is cleared rather than left to fail the same way
/// on every future launch.
pub async fn resume(client: &ApiClient, store: &dyn CredentialStore) -> Option<User> {
    let token = store.load()?;
    client.set_token(Some(token));
    match client.me().await {
        Ok(response) => Some(response.user),
        Err(_) => {
            client.set_token(None);
            store.clear();
            None
        }
    }
}
