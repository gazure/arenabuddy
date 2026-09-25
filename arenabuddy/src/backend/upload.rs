use std::time::Duration;

use arenabuddy_core::{
    models::MatchData,
    services::match_service::{ClassifyMatchRequest, UpsertMatchDataRequest, match_service_client::MatchServiceClient},
};
use arenabuddy_data::{MatchDB, MatchUploadJob, MetagameRepository, metagame_models::MatchArchetype};
use tokio::time::{MissedTickBehavior, timeout};
use tonic::{Code, Status, transport::Endpoint};
use tracing::{error, info};

use super::auth::{SharedAuthState, attach_bearer, current_session};

const NETWORK_TIMEOUT: Duration = Duration::from_secs(20);

fn retry_delay(attempts: i32) -> i32 {
    5 * 2_i32.pow(u32::try_from(attempts.saturating_sub(1).clamp(0, 6)).unwrap_or(6))
}

fn is_blocked(status: &Status) -> bool {
    matches!(
        status.code(),
        Code::PermissionDenied | Code::InvalidArgument | Code::FailedPrecondition | Code::Unimplemented
    )
}

async fn token_for_account(
    auth: &SharedAuthState,
    url: &str,
    account: &str,
    force_refresh: bool,
) -> Result<String, Status> {
    let state = current_session(auth, url, force_refresh)
        .await
        .map_err(|_| Status::unauthenticated("could not refresh session; sign in again if retries keep failing"))?;
    if state.user.id != account {
        return Err(Status::unauthenticated("sign in with the upload's original account"));
    }
    Ok(state.token)
}

async fn connect(url: &str) -> Result<MatchServiceClient<tonic::transport::Channel>, Status> {
    let endpoint = Endpoint::new(url.to_owned())
        .map_err(|_| Status::invalid_argument("invalid server URL"))?
        .connect_timeout(Duration::from_secs(5))
        .timeout(NETWORK_TIMEOUT);
    let channel = timeout(NETWORK_TIMEOUT, endpoint.connect())
        .await
        .map_err(|_| Status::deadline_exceeded("server connection timed out"))?
        .map_err(|_| Status::unavailable("cannot connect to server"))?;
    Ok(MatchServiceClient::new(channel))
}

async fn send(
    client: &mut MatchServiceClient<tonic::transport::Channel>,
    data: &MatchData,
    token: &str,
) -> Result<(), Status> {
    let mut request = tonic::Request::new(UpsertMatchDataRequest {
        match_data: Some(data.into()),
    });
    attach_bearer(&mut request, Some(token));
    request.set_timeout(NETWORK_TIMEOUT);
    timeout(NETWORK_TIMEOUT, client.upsert_match_data(request))
        .await
        .map_err(|_| Status::deadline_exceeded("match upload timed out"))??;
    Ok(())
}

async fn deliver(auth: &SharedAuthState, url: &str, job: &MatchUploadJob) -> Result<String, Status> {
    let data: MatchData = serde_json::from_str(&job.payload)
        .map_err(|_| Status::invalid_argument("saved upload cannot be decoded; queue the match again"))?;
    let mut token = token_for_account(auth, url, &job.user_id, false).await?;
    let mut client = connect(url).await?;
    match send(&mut client, &data, &token).await {
        Err(status) if status.code() == Code::Unauthenticated => {
            token = token_for_account(auth, url, &job.user_id, true).await?;
            send(&mut client, &data, &token).await?;
        }
        result => result?,
    }
    Ok(token)
}

async fn classify(
    db: &MatchDB,
    url: &str,
    match_id: &str,
    token: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = connect(url).await?;
    let mut request = tonic::Request::new(ClassifyMatchRequest {
        match_id: match_id.to_string(),
    });
    attach_bearer(&mut request, Some(token));
    request.set_timeout(NETWORK_TIMEOUT);
    let response = timeout(NETWORK_TIMEOUT, client.classify_match(request)).await??;
    for classification in response.into_inner().classifications {
        db.upsert_match_archetype(&MatchArchetype {
            match_id: match_id.to_string(),
            side: classification.side,
            archetype_id: None,
            archetype_name: classification.archetype_name,
            confidence: classification.confidence,
        })
        .await?;
    }
    Ok(())
}

/// Runs durable delivery independently of log ingestion and the UI runtime.
pub async fn run(db: MatchDB, auth: SharedAuthState) {
    let url = super::paths::grpc_url();
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tick.tick().await;
        let Some(account) = auth.lock().await.as_ref().map(|state| state.user.id.clone()) else {
            continue;
        };
        let job = match db.claim_match_upload(&account).await {
            Ok(Some(job)) => job,
            Ok(None) => continue,
            Err(e) => {
                error!("Could not claim upload: {e}");
                continue;
            }
        };
        let delivered = timeout(Duration::from_secs(90), deliver(&auth, &url, &job))
            .await
            .unwrap_or_else(|_| Err(Status::deadline_exceeded("upload attempt timed out")));
        match delivered {
            Ok(token) => {
                if let Err(e) = db.finish_match_upload(&job, None, 0, false).await {
                    error!("Could not acknowledge upload {}: {e}", job.match_id);
                    continue;
                }
                info!("Uploaded match {}", job.match_id);
                // Classification is best effort and never determines upload success.
                if let Err(e) = classify(&db, &url, &job.match_id, &token).await {
                    error!("Classification failed for {}: {e}", job.match_id);
                }
            }
            Err(status) => {
                let message = format!("{}: {}", status.code(), status.message());
                if let Err(e) = db
                    .finish_match_upload(&job, Some(&message), retry_delay(job.attempts), is_blocked(&status))
                    .await
                {
                    error!("Could not persist upload failure {}: {e}", job.match_id);
                }
                error!("Upload failed for {}: {message}", job.match_id);
            }
        }
    }
}

#[cfg(test)]
#[path = "upload_transport_tests.rs"]
mod transport_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_backoff_is_bounded() {
        assert_eq!(retry_delay(1), 5);
        assert_eq!(retry_delay(2), 10);
        assert_eq!(retry_delay(i32::MAX), 320);
    }

    #[test]
    fn ownership_and_bad_payloads_require_explicit_retry() {
        assert!(is_blocked(&Status::permission_denied("owner")));
        assert!(is_blocked(&Status::invalid_argument("payload")));
        for code in [
            Code::Unavailable,
            Code::DeadlineExceeded,
            Code::ResourceExhausted,
            Code::Unauthenticated,
        ] {
            assert!(!is_blocked(&Status::new(code, "retry")));
        }
    }

    #[tokio::test]
    async fn account_switch_cannot_supply_another_accounts_token() {
        let auth = super::super::auth::new_shared_auth_state();
        *auth.lock().await = Some(super::super::auth::AuthState {
            token: "other-token".into(),
            token_expires_at: i64::MAX,
            refresh_token: String::new(),
            refresh_expires_at: i64::MAX,
            user: arenabuddy_core::services::auth_service::User {
                id: "other".into(),
                ..Default::default()
            },
        });
        let error = token_for_account(&auth, "http://127.0.0.1:1", "original", false)
            .await
            .expect_err("different account");
        assert_eq!(error.code(), Code::Unauthenticated);
    }
}
