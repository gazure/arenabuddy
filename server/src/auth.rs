use std::sync::Arc;

use arenabuddy_core::services::auth_service::{
    ExchangeTokenRequest, ExchangeTokenResponse, GetCurrentUserRequest, GetCurrentUserResponse, LogoutRequest,
    LogoutResponse, RefreshTokenRequest, RefreshTokenResponse, User, auth_service_server::AuthService,
};
use arenabuddy_data::MatchDB;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tonic::{Request, Response, Status};
use tracingx::{error, info};
use uuid::Uuid;

const ACCESS_TOKEN_LIFETIME_MINUTES: i64 = 15;
const REFRESH_TOKEN_LIFETIME_DAYS: i64 = 30;

#[derive(Debug, Serialize, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    avatar: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user UUID
    pub discord_id: String,
    pub exp: usize,
}

pub struct AuthConfig {
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub jwt_secret: String,
}

pub struct AuthServiceImpl {
    pool: PgPool,
    config: Arc<AuthConfig>,
    http: reqwest::Client,
}

impl AuthServiceImpl {
    pub fn new(db: &MatchDB, config: Arc<AuthConfig>) -> Self {
        Self {
            pool: db.pool().clone(),
            config,
            http: reqwest::Client::new(),
        }
    }

    async fn exchange_discord_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<DiscordTokenResponse, Status> {
        let params = [
            ("client_id", self.config.discord_client_id.as_str()),
            ("client_secret", self.config.discord_client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
        ];

        let resp = self
            .http
            .post("https://discord.com/api/v10/oauth2/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                error!("Discord token exchange HTTP error: {e}");
                Status::internal("failed to contact Discord")
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!("Discord token exchange failed: {status} {body}");
            return Err(Status::unauthenticated("Discord authentication failed"));
        }

        resp.json::<DiscordTokenResponse>().await.map_err(|e| {
            error!("Failed to parse Discord token response: {e}");
            Status::internal("failed to parse Discord response")
        })
    }

    async fn get_discord_user(&self, access_token: &str) -> Result<DiscordUser, Status> {
        let resp = self
            .http
            .get("https://discord.com/api/v10/users/@me")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| {
                error!("Discord user fetch HTTP error: {e}");
                Status::internal("failed to contact Discord")
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!("Discord user fetch failed: {status} {body}");
            return Err(Status::internal("failed to fetch Discord user"));
        }

        resp.json::<DiscordUser>().await.map_err(|e| {
            error!("Failed to parse Discord user response: {e}");
            Status::internal("failed to parse Discord user")
        })
    }

    async fn upsert_user(&self, discord_user: &DiscordUser) -> Result<Uuid, Status> {
        let avatar_url = discord_user
            .avatar
            .as_ref()
            .map(|hash| format!("https://cdn.discordapp.com/avatars/{}/{hash}.png", discord_user.id));

        let row: (Uuid,) = sqlx::query_as(
            r"
            INSERT INTO app_user (discord_id, username, avatar_url)
            VALUES ($1, $2, $3)
            ON CONFLICT (discord_id)
            DO UPDATE SET username = excluded.username, avatar_url = excluded.avatar_url, updated_at = now()
            RETURNING id
            ",
        )
        .bind(&discord_user.id)
        .bind(&discord_user.username)
        .bind(&avatar_url)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to upsert user: {e}");
            Status::internal("failed to create user")
        })?;

        Ok(row.0)
    }

    fn mint_jwt(&self, user_id: &Uuid, discord_id: &str) -> Result<(String, i64), Status> {
        let exp = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::minutes(ACCESS_TOKEN_LIFETIME_MINUTES))
            .ok_or_else(|| Status::internal("failed to compute expiry"))?;

        let claims = Claims {
            sub: user_id.to_string(),
            discord_id: discord_id.to_string(),
            #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            exp: exp.timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(|e| {
            error!("Failed to mint JWT: {e}");
            Status::internal("failed to create token")
        })?;

        Ok((token, exp.timestamp()))
    }

    async fn create_refresh_token(&self, user_id: &Uuid) -> Result<(String, i64), Status> {
        let raw_token = generate_refresh_token();
        let token_hash = hash_token(&raw_token);
        let expires_at = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::days(REFRESH_TOKEN_LIFETIME_DAYS))
            .ok_or_else(|| Status::internal("failed to compute refresh token expiry"))?;

        sqlx::query("INSERT INTO refresh_token (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(&token_hash)
            .bind(expires_at)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to store refresh token: {e}");
                Status::internal("failed to create refresh token")
            })?;

        Ok((raw_token, expires_at.timestamp()))
    }

    /// Validate a refresh token, revoke it, and issue a new one (rotation).
    /// Returns (`user_id`, `new_raw_token`, `new_expires_at`) on success.
    async fn validate_and_rotate_refresh_token(&self, raw_token: &str) -> Result<(Uuid, String, i64), Status> {
        let token_hash = hash_token(raw_token);

        // Look up the token by hash
        let row: Option<RefreshTokenRow> = sqlx::query_as(
            "SELECT id, user_id, revoked FROM refresh_token WHERE token_hash = $1 AND expires_at > now()",
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to look up refresh token: {e}");
            Status::internal("failed to validate refresh token")
        })?;

        let row = row.ok_or_else(|| {
            info!("Refresh token not found or expired");
            Status::unauthenticated("invalid or expired refresh token")
        })?;

        // Reuse detection: if the token was already revoked, someone may have stolen it.
        // Revoke ALL tokens for this user as a precaution.
        if row.revoked {
            error!("Revoked refresh token reuse detected for user {}", row.user_id);
            self.revoke_all_user_tokens(row.user_id).await?;
            return Err(Status::unauthenticated("token reuse detected"));
        }

        // Revoke the old token
        sqlx::query("UPDATE refresh_token SET revoked = true WHERE id = $1")
            .bind(row.id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to revoke old refresh token: {e}");
                Status::internal("failed to rotate refresh token")
            })?;

        // Issue a new refresh token
        let (new_token, new_expires_at) = self.create_refresh_token(&row.user_id).await?;

        // Opportunistic cleanup: delete expired tokens for this user
        let _ = sqlx::query("DELETE FROM refresh_token WHERE user_id = $1 AND expires_at < now()")
            .bind(row.user_id)
            .execute(&self.pool)
            .await;

        Ok((row.user_id, new_token, new_expires_at))
    }

    async fn revoke_all_user_tokens(&self, user_id: Uuid) -> Result<(), Status> {
        sqlx::query("UPDATE refresh_token SET revoked = true WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to revoke all tokens for user {user_id}: {e}");
                Status::internal("failed to revoke tokens")
            })?;
        info!("Revoked all refresh tokens for user {user_id}");
        Ok(())
    }
}

fn generate_refresh_token() -> String {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub fn validate_jwt(token: &str, jwt_secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

pub fn auth_interceptor(jwt_secret: String) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    move |mut req: Request<()>| {
        info!("Incoming authenticated request");

        let token = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| {
                info!("Rejected: missing authorization token");
                Status::unauthenticated("missing authorization token")
            })?;

        let claims = validate_jwt(token, &jwt_secret).map_err(|e| {
            error!("Rejected: JWT validation failed: {e}");
            Status::unauthenticated("invalid token")
        })?;

        let user_id = claims
            .sub
            .parse::<Uuid>()
            .map_err(|_| Status::unauthenticated("invalid token claims"))?;

        info!("Authorized user {user_id}");
        req.extensions_mut().insert(UserId(user_id));
        Ok(req)
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn exchange_token(
        &self,
        request: Request<ExchangeTokenRequest>,
    ) -> Result<Response<ExchangeTokenResponse>, Status> {
        info!("ExchangeToken request received");
        let req = request.into_inner();

        if req.authorization_code.is_empty() {
            return Err(Status::invalid_argument("authorization_code is required"));
        }
        if req.code_verifier.is_empty() {
            return Err(Status::invalid_argument("code_verifier is required"));
        }
        if req.redirect_uri.is_empty() {
            return Err(Status::invalid_argument("redirect_uri is required"));
        }

        let discord_token = self
            .exchange_discord_code(&req.authorization_code, &req.code_verifier, &req.redirect_uri)
            .await?;

        let discord_user = self.get_discord_user(&discord_token.access_token).await?;
        info!(
            "Discord user authenticated: {} ({})",
            discord_user.username, discord_user.id
        );

        let user_id = self.upsert_user(&discord_user).await?;
        let (jwt, expires_at) = self.mint_jwt(&user_id, &discord_user.id)?;
        let (refresh_token, refresh_expires_at) = self.create_refresh_token(&user_id).await?;

        let avatar_url = discord_user.avatar.as_ref().map_or_else(String::new, |hash| {
            format!("https://cdn.discordapp.com/avatars/{}/{hash}.png", discord_user.id)
        });

        Ok(Response::new(ExchangeTokenResponse {
            access_token: jwt,
            expires_at,
            user: Some(User {
                id: user_id.to_string(),
                discord_id: discord_user.id,
                username: discord_user.username,
                avatar_url,
            }),
            refresh_token,
            refresh_expires_at,
        }))
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        info!("RefreshToken request received");
        let req = request.into_inner();

        if req.refresh_token.is_empty() {
            return Err(Status::invalid_argument("refresh_token is required"));
        }

        let (user_id, new_refresh_token, new_refresh_expires_at) =
            self.validate_and_rotate_refresh_token(&req.refresh_token).await?;

        // Fetch user to get discord_id for JWT claims
        let user_row: AppUserRow =
            sqlx::query_as("SELECT id, discord_id, username, avatar_url FROM app_user WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    error!("Failed to fetch user for refresh: {e}");
                    Status::internal("failed to fetch user")
                })?
                .ok_or_else(|| Status::not_found("user not found"))?;

        let (jwt, expires_at) = self.mint_jwt(&user_id, &user_row.discord_id)?;

        info!("Refreshed tokens for user {user_id}");
        Ok(Response::new(RefreshTokenResponse {
            access_token: jwt,
            expires_at,
            refresh_token: new_refresh_token,
            refresh_expires_at: new_refresh_expires_at,
        }))
    }

    async fn logout(&self, request: Request<LogoutRequest>) -> Result<Response<LogoutResponse>, Status> {
        info!("Logout request received");
        let req = request.into_inner();

        if req.refresh_token.is_empty() {
            return Err(Status::invalid_argument("refresh_token is required"));
        }

        let token_hash = hash_token(&req.refresh_token);

        // Find the user who owns this token and revoke all their tokens
        let row: Option<(Uuid,)> = sqlx::query_as("SELECT user_id FROM refresh_token WHERE token_hash = $1")
            .bind(&token_hash)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to look up refresh token for logout: {e}");
                Status::internal("failed to process logout")
            })?;

        if let Some((user_id,)) = row {
            self.revoke_all_user_tokens(user_id).await?;
        }

        Ok(Response::new(LogoutResponse {}))
    }

    async fn get_current_user(
        &self,
        request: Request<GetCurrentUserRequest>,
    ) -> Result<Response<GetCurrentUserResponse>, Status> {
        let user_id = request
            .extensions()
            .get::<UserId>()
            .ok_or_else(|| Status::unauthenticated("not authenticated"))?;

        let row: AppUserRow = sqlx::query_as("SELECT id, discord_id, username, avatar_url FROM app_user WHERE id = $1")
            .bind(user_id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch user: {e}");
                Status::internal("failed to fetch user")
            })?
            .ok_or_else(|| Status::not_found("user not found"))?;

        Ok(Response::new(GetCurrentUserResponse {
            user: Some(User {
                id: row.id.to_string(),
                discord_id: row.discord_id,
                username: row.username,
                avatar_url: row.avatar_url.unwrap_or_default(),
            }),
        }))
    }
}

#[derive(Debug, Clone)]
pub struct UserId(pub Uuid);

#[derive(Debug, sqlx::FromRow)]
struct AppUserRow {
    id: Uuid,
    discord_id: String,
    username: String,
    avatar_url: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct RefreshTokenRow {
    id: Uuid,
    user_id: Uuid,
    revoked: bool,
}
