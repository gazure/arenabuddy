use std::sync::Arc;

use arenabuddy_core::services::auth_service::{
    ExchangeTokenRequest, ExchangeTokenResponse, GetCurrentUserRequest, GetCurrentUserResponse, User,
    auth_service_server::AuthService,
};
use arenabuddy_data::MatchDB;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tonic::{Request, Response, Status};
use tracingx::{error, info};
use uuid::Uuid;

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
            .checked_add_signed(chrono::Duration::days(30))
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
}

pub fn validate_jwt(token: &str, jwt_secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

pub fn auth_interceptor(jwt_secret: Arc<String>) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
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
        }))
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
