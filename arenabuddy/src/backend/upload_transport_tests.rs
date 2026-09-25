use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use arenabuddy_core::{
    models::{MTGAMatch, OpponentDeck},
    services::match_service::{
        ClassifyMatchResponse, DeleteMatchRequest, DeleteMatchResponse, GetMatchDataRequest, GetMatchDataResponse,
        ListMatchesRequest, ListMatchesResponse, UpsertMatchDataResponse,
        match_service_server::{MatchService, MatchServiceServer},
    },
};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, transport::Server};

use super::*;

struct RecoveringServer(Arc<AtomicUsize>);

#[tonic::async_trait]
impl MatchService for RecoveringServer {
    async fn upsert_match_data(
        &self,
        request: Request<UpsertMatchDataRequest>,
    ) -> Result<Response<UpsertMatchDataResponse>, Status> {
        assert_eq!(
            request.metadata().get("authorization").expect("bearer"),
            "Bearer original-token"
        );
        let data = request.into_inner().match_data.expect("data");
        assert_eq!(data.mtga_match.expect("match").id, "test-match");
        if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(Status::unavailable("temporary failure"));
        }
        Ok(Response::new(UpsertMatchDataResponse {}))
    }

    async fn get_match_data(&self, _: Request<GetMatchDataRequest>) -> Result<Response<GetMatchDataResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }

    async fn list_matches(&self, _: Request<ListMatchesRequest>) -> Result<Response<ListMatchesResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }

    async fn delete_match(&self, _: Request<DeleteMatchRequest>) -> Result<Response<DeleteMatchResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }

    async fn classify_match(
        &self,
        _: Request<ClassifyMatchRequest>,
    ) -> Result<Response<ClassifyMatchResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
}

#[tokio::test]
async fn a_failed_attempt_can_be_delivered_again_with_the_original_account_and_payload() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = format!("http://{}", listener.local_addr().expect("address"));
    let calls = Arc::new(AtomicUsize::new(0));
    let service = RecoveringServer(calls.clone());
    let server = tokio::spawn(
        Server::builder()
            .add_service(MatchServiceServer::new(service))
            .serve_with_incoming(TcpListenerStream::new(listener)),
    );
    let auth = super::super::auth::new_shared_auth_state();
    *auth.lock().await = Some(super::super::auth::AuthState {
        token: "original-token".into(),
        token_expires_at: i64::MAX,
        refresh_token: String::new(),
        refresh_expires_at: i64::MAX,
        user: arenabuddy_core::services::auth_service::User {
            id: "original".into(),
            ..Default::default()
        },
    });
    let data = MatchData {
        mtga_match: MTGAMatch::new("test-match", 1, "player", "opponent"),
        decks: vec![],
        mulligans: vec![],
        results: vec![],
        event_logs: vec![],
        opponent_deck: OpponentDeck::empty(),
    };
    let job = MatchUploadJob {
        match_id: "test-match".into(),
        user_id: "original".into(),
        revision: 1,
        lease_token: "lease".into(),
        payload: serde_json::to_string(&data).expect("payload"),
        attempts: 1,
    };
    let failed = deliver(&auth, &url, &job).await.expect_err("temporary failure");
    assert_eq!(failed.code(), Code::Unavailable);
    assert!(!is_blocked(&failed));
    assert_eq!(deliver(&auth, &url, &job).await.expect("recovered"), "original-token");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    server.abort();
}

use arenabuddy_core::services::auth_service as auth_api;

struct SlowRefresh {
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}

#[tonic::async_trait]
impl auth_api::auth_service_server::AuthService for SlowRefresh {
    async fn refresh_token(
        &self,
        _: Request<auth_api::RefreshTokenRequest>,
    ) -> Result<Response<auth_api::RefreshTokenResponse>, Status> {
        self.entered.notify_one();
        self.release.notified().await;
        Ok(Response::new(auth_api::RefreshTokenResponse {
            access_token: "refreshed".into(),
            expires_at: i64::MAX,
            refresh_token: "rotated".into(),
            refresh_expires_at: i64::MAX,
        }))
    }

    async fn exchange_token(
        &self,
        _: Request<auth_api::ExchangeTokenRequest>,
    ) -> Result<Response<auth_api::ExchangeTokenResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }

    async fn logout(&self, _: Request<auth_api::LogoutRequest>) -> Result<Response<auth_api::LogoutResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }

    async fn get_current_user(
        &self,
        _: Request<auth_api::GetCurrentUserRequest>,
    ) -> Result<Response<auth_api::GetCurrentUserResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
}

#[tokio::test]
async fn refresh_does_not_block_capture_or_restore_a_logged_out_session() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = format!("http://{}", listener.local_addr().expect("address"));
    let server = tokio::spawn(
        Server::builder()
            .add_service(auth_api::auth_service_server::AuthServiceServer::new(SlowRefresh {
                entered: entered.clone(),
                release: release.clone(),
            }))
            .serve_with_incoming(TcpListenerStream::new(listener)),
    );
    let auth = super::super::auth::new_shared_auth_state();
    *auth.lock().await = Some(super::super::auth::AuthState {
        token: "expired".into(),
        token_expires_at: 0,
        refresh_token: "original".into(),
        refresh_expires_at: i64::MAX,
        user: auth_api::User {
            id: "original".into(),
            ..Default::default()
        },
    });
    let refreshed_auth = auth.clone();
    let refresh = tokio::spawn(async move { current_session(&refreshed_auth, &url, false).await });
    timeout(Duration::from_secs(5), entered.notified())
        .await
        .expect("refresh started");
    {
        let mut state = timeout(Duration::from_secs(1), auth.lock())
            .await
            .expect("capture can read auth during network I/O");
        *state = None;
    }
    release.notify_one();
    assert!(refresh.await.expect("refresh task").is_err());
    assert!(auth.lock().await.is_none());
    server.abort();
}
