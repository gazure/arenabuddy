use std::collections::HashSet;

use arenabuddy_core::{
    models::MatchData,
    services::match_service::{GetMatchDataRequest, ListMatchesRequest, match_service_client::MatchServiceClient},
};
use arenabuddy_data::{ArenabuddyRepository, MatchDB};
use tracing::{error, info};

use super::auth::{SharedAuthState, attach_bearer, current_session};

/// Sync matches from the server into the local database.
///
/// Fetches the server's match list for the authenticated user, compares
/// against local matches, and downloads any that are missing locally.
///
/// Returns the number of newly synced matches.
///
/// # Errors
///
/// Returns an error if the user is not authenticated, the gRPC connection
/// fails, or the server returns an error from `ListMatches`.
pub async fn sync_matches(
    db: &MatchDB,
    auth_state: &SharedAuthState,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let grpc_url = super::paths::grpc_url();

    let token = current_session(auth_state, &grpc_url, false).await?.token;

    let mut client = MatchServiceClient::connect(grpc_url).await?;

    // Get server match list
    let mut request = tonic::Request::new(ListMatchesRequest {});
    attach_bearer(&mut request, Some(&token));

    let server_matches = client.list_matches(request).await?.into_inner().matches;
    info!("Server has {} matches for this user", server_matches.len());

    // Get local match IDs
    let local_ids: HashSet<_> = db
        .list_matches(None)
        .await
        .map_err(|e| e.to_string())?
        .iter()
        .map(|m| m.id().to_owned())
        .collect();

    // Find matches we're missing locally
    let missing: Vec<_> = server_matches
        .iter()
        .filter(|m| !local_ids.contains(m.id.as_str()))
        .collect();

    if missing.is_empty() {
        info!("Local database is up to date");
        return Ok(0);
    }

    info!("Syncing {} new matches from server", missing.len());

    let mut synced = 0;
    for server_match in &missing {
        let mut request = tonic::Request::new(GetMatchDataRequest {
            match_id: server_match.id.clone(),
        });
        attach_bearer(&mut request, Some(&token));

        let response = match client.get_match_data(request).await {
            Ok(r) => r.into_inner(),
            Err(e) => {
                error!("Failed to fetch match {}: {e}", server_match.id);
                continue;
            }
        };

        let Some(match_data_proto) = response.match_data else {
            error!("Server returned empty match_data for {}", server_match.id);
            continue;
        };

        let match_data: MatchData = match (&match_data_proto).try_into() {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to convert match {}: {e}", server_match.id);
                continue;
            }
        };

        if let Err(e) = db
            .upsert_match_data(
                &match_data.mtga_match,
                &match_data.decks,
                &match_data.mulligans,
                &match_data.results,
                &match_data.opponent_deck.cards,
                &match_data.event_logs,
                None,
            )
            .await
        {
            error!("Failed to write match {} locally: {e}", server_match.id);
            continue;
        }

        synced += 1;
    }

    info!("Synced {synced}/{} matches from server", missing.len());
    Ok(synced)
}

/// Queues a local match for durable upload by the signed-in account.
/// Returns `false` if the local match is missing and an error if queueing fails.
pub async fn push_match(
    db: &MatchDB,
    auth_state: &SharedAuthState,
    match_id: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let account = auth_state
        .lock()
        .await
        .as_ref()
        .map(|state| state.user.id.clone())
        .ok_or("sign in before queueing a match")?;
    Ok(db.queue_match_upload(match_id, &account).await?)
}
