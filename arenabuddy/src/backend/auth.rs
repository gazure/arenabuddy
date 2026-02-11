use std::{path::PathBuf, sync::Arc};

use arenabuddy_core::services::auth_service::{
    ExchangeTokenRequest, LogoutRequest, RefreshTokenRequest, User, auth_service_client::AuthServiceClient,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use tracingx::{error, info};

/// Stored authentication state for the current session.
#[derive(Debug, Clone)]
pub struct AuthState {
    pub token: String,
    pub token_expires_at: i64,
    pub refresh_token: String,
    pub refresh_expires_at: i64,
    pub user: User,
}

/// Shared auth state accessible across the app.
pub type SharedAuthState = Arc<Mutex<Option<AuthState>>>;

pub fn new_shared_auth_state() -> SharedAuthState {
    Arc::new(Mutex::new(None))
}

/// Returns true if the access token expires within 60 seconds.
pub fn needs_refresh(state: &AuthState) -> bool {
    let now = chrono::Utc::now().timestamp();
    state.token_expires_at - now < 60
}

/// Serializable form of auth state for file persistence.
#[derive(Serialize, Deserialize)]
struct SavedAuth {
    token: String,
    token_expires_at: i64,
    refresh_token: String,
    refresh_expires_at: i64,
    user_id: String,
    discord_id: String,
    username: String,
    avatar_url: String,
}

fn auth_file_path() -> Option<PathBuf> {
    let home = std::env::home_dir()?;
    let dir = match std::env::consts::OS {
        "macos" => home.join("Library/Application Support/com.gazure.dev.arenabuddy.app"),
        "windows" => home.join("AppData/Roaming/com.gazure.dev.arenabuddy.app"),
        "linux" => home.join(".local/share/com.gazure.dev.arenabuddy.app"),
        _ => return None,
    };
    Some(dir.join("auth.json"))
}

/// Save auth state to disk.
pub fn save_auth(state: &AuthState) {
    let Some(path) = auth_file_path() else {
        return;
    };
    let saved = SavedAuth {
        token: state.token.clone(),
        token_expires_at: state.token_expires_at,
        refresh_token: state.refresh_token.clone(),
        refresh_expires_at: state.refresh_expires_at,
        user_id: state.user.id.clone(),
        discord_id: state.user.discord_id.clone(),
        username: state.user.username.clone(),
        avatar_url: state.user.avatar_url.clone(),
    };
    match serde_json::to_string(&saved) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                error!("Failed to save auth token: {e}");
            } else {
                info!("Auth token saved to {}", path.display());
            }
        }
        Err(e) => error!("Failed to serialize auth state: {e}"),
    }
}

/// Load auth state from disk, if a saved token exists.
/// Returns None if the saved file is missing, malformed, or lacks refresh token fields
/// (backwards compatibility: forces re-login for old format).
pub fn load_auth() -> Option<AuthState> {
    let path = auth_file_path()?;
    let json = std::fs::read_to_string(&path).ok()?;
    let saved: SavedAuth = serde_json::from_str(&json).ok()?;
    info!("Loaded saved auth for user: {}", saved.username);
    Some(AuthState {
        token: saved.token,
        token_expires_at: saved.token_expires_at,
        refresh_token: saved.refresh_token,
        refresh_expires_at: saved.refresh_expires_at,
        user: User {
            id: saved.user_id,
            discord_id: saved.discord_id,
            username: saved.username,
            avatar_url: saved.avatar_url,
        },
    })
}

/// Delete the saved auth file from disk.
pub fn delete_saved_auth() {
    if let Some(path) = auth_file_path() {
        if let Err(e) = std::fs::remove_file(&path) {
            error!("Failed to delete auth file: {e}");
        } else {
            info!("Deleted saved auth file");
        }
    }
}

/// Generate a PKCE code verifier and code challenge.
fn generate_pkce() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    let verifier = URL_SAFE_NO_PAD.encode(bytes);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

/// Refresh the access token using a refresh token.
/// Returns a new `AuthState` with updated tokens.
pub async fn refresh(
    grpc_url: &str,
    current_state: &AuthState,
) -> Result<AuthState, Box<dyn std::error::Error + Send + Sync>> {
    info!("Refreshing access token");
    let mut client = AuthServiceClient::connect(grpc_url.to_string()).await?;

    let response = client
        .refresh_token(RefreshTokenRequest {
            refresh_token: current_state.refresh_token.clone(),
        })
        .await?
        .into_inner();

    let state = AuthState {
        token: response.access_token,
        token_expires_at: response.expires_at,
        refresh_token: response.refresh_token,
        refresh_expires_at: response.refresh_expires_at,
        user: current_state.user.clone(),
    };

    save_auth(&state);
    info!("Access token refreshed successfully");

    Ok(state)
}

/// Log out: revoke the refresh token on the server and delete local auth state.
pub async fn logout(grpc_url: &str, refresh_token: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Logging out");
    let mut client = AuthServiceClient::connect(grpc_url.to_string()).await?;

    client
        .logout(LogoutRequest {
            refresh_token: refresh_token.to_string(),
        })
        .await?;

    delete_saved_auth();
    info!("Logged out successfully");

    Ok(())
}

/// Run the full Discord `OAuth2` login flow:
/// 1. Generate PKCE pair
/// 2. Start local HTTP server for callback
/// 3. Open browser to Discord authorize URL
/// 4. Receive auth code via callback
/// 5. Exchange code for JWT via gRPC `AuthService`
/// 6. Save token to disk
pub async fn login(
    grpc_url: &str,
    discord_client_id: &str,
) -> Result<AuthState, Box<dyn std::error::Error + Send + Sync>> {
    let (code_verifier, code_challenge) = generate_pkce();

    // Use a fixed port so the redirect URI matches Discord's allowlist
    let listener = tokio::net::TcpListener::bind("127.0.0.1:9274").await?;
    let redirect_uri = "http://localhost:9274/callback".to_string();

    // Channel to send the auth code from the callback handler
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    // Build the axum callback server
    let tx_clone = tx.clone();
    let app = axum::Router::new().route(
        "/callback",
        axum::routing::get(
            move |axum::extract::Query(params): axum::extract::Query<
                std::collections::HashMap<String, String>,
            >| {
                let tx = tx_clone.clone();
                async move {
                    if let Some(code) = params.get("code") {
                        if let Some(sender) = tx.lock().await.take() {
                            let _ = sender.send(code.clone());
                        }
                        axum::response::Html(
                            "<html><body><h1>Login successful!</h1><p>You can close this tab and return to ArenaBuddy.</p></body></html>"
                        )
                    } else {
                        axum::response::Html(
                            "<html><body><h1>Login failed</h1><p>No authorization code received.</p></body></html>"
                        )
                    }
                }
            },
        ),
    );

    // Spawn the callback server
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    // Open browser to Discord authorization URL
    let authorize_url = format!(
        "https://discord.com/oauth2/authorize?client_id={discord_client_id}&response_type=code&redirect_uri={redirect_uri}&scope=identify&code_challenge={code_challenge}&code_challenge_method=S256",
        redirect_uri = urlencoding::encode(&redirect_uri),
    );

    info!("Opening browser for Discord login");
    if let Err(e) = open::that(&authorize_url) {
        error!("Failed to open browser: {e}");
        return Err(format!("Failed to open browser: {e}").into());
    }

    // Wait for the callback with the auth code (with timeout)
    let code = tokio::time::timeout(std::time::Duration::from_secs(300), rx)
        .await
        .map_err(|_| "Login timed out after 5 minutes")?
        .map_err(|_| "Login callback channel closed")?;

    // Shut down the callback server
    server_handle.abort();

    info!("Received Discord auth code, exchanging for token");

    // Exchange the code for a JWT via the gRPC AuthService
    info!("Connecting to gRPC auth service at {grpc_url}");
    let mut client = AuthServiceClient::connect(grpc_url.to_string()).await.map_err(|e| {
        error!("Failed to connect auth channel: {e}");
        e
    })?;
    info!("Auth channel connected");

    let response = client
        .exchange_token(ExchangeTokenRequest {
            authorization_code: code,
            code_verifier,
            redirect_uri,
        })
        .await
        .map_err(|e| {
            error!("ExchangeToken RPC failed: {e}");
            e
        })?
        .into_inner();

    let user = response.user.ok_or("Server did not return user info")?;
    info!("Logged in as: {}", user.username);

    let state = AuthState {
        token: response.access_token,
        token_expires_at: response.expires_at,
        refresh_token: response.refresh_token,
        refresh_expires_at: response.refresh_expires_at,
        user,
    };

    save_auth(&state);

    Ok(state)
}
