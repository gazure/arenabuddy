use std::error::Error as StdError;

use thiserror::Error;
use tracing::{error, info};

use crate::backend::{BackgroundRuntime, Service, SharedAuthState};

#[derive(Debug)]
pub struct LoginOutcome {
    pub username: String,
}

#[derive(Debug, Error)]
pub enum AuthControllerError {
    #[error("background task dropped before completing")]
    TaskDropped,
    #[error("login failed: {0}")]
    LoginFailed(String),
    #[error("logout failed: {0}")]
    LogoutFailed(String),
}

async fn run_on_background<T, F>(background: &BackgroundRuntime, future: F) -> Result<T, AuthControllerError>
where
    T: Send + 'static,
    F: std::future::Future<Output = T> + Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    background.spawn(async move {
        let _ = tx.send(future.await);
    });
    rx.await.map_err(|_| AuthControllerError::TaskDropped)
}

fn to_error_string(err: &(dyn StdError + Send + Sync)) -> String {
    err.to_string()
}

pub async fn login(
    auth_state: SharedAuthState,
    service: Service,
    background: BackgroundRuntime,
) -> Result<LoginOutcome, AuthControllerError> {
    let grpc_url = crate::backend::paths::grpc_url();
    let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_else(|_| "1469498901886271663".to_string());

    let state = run_on_background(&background, async move {
        crate::backend::auth::login(&grpc_url, &client_id).await
    })
    .await?
    .map_err(|err| to_error_string(err.as_ref()))
    .map_err(AuthControllerError::LoginFailed)?;

    let username = state.user.username.clone();
    *auth_state.lock().await = Some(state);

    // Keep sync in background so UI can update immediately after login.
    let sync_db = service.db.clone();
    let sync_auth = auth_state.clone();
    background.spawn(async move {
        match crate::backend::sync::sync_matches(&sync_db, &sync_auth).await {
            Ok(n) => info!("Post-login sync complete: {n} new matches"),
            Err(e) => error!("Post-login sync failed: {e}"),
        }
    });

    Ok(LoginOutcome { username })
}

pub async fn logout(auth_state: SharedAuthState, background: BackgroundRuntime) -> Result<(), AuthControllerError> {
    let grpc_url = crate::backend::paths::grpc_url();
    let refresh_token = auth_state.lock().await.as_ref().map(|s| s.refresh_token.clone());

    if let Some(refresh_token) = refresh_token {
        run_on_background(&background, async move {
            crate::backend::auth::logout(&grpc_url, &refresh_token).await
        })
        .await?
        .map_err(|err| to_error_string(err.as_ref()))
        .map_err(AuthControllerError::LogoutFailed)?;
    } else {
        crate::backend::auth::delete_saved_auth();
    }

    *auth_state.lock().await = None;
    Ok(())
}
