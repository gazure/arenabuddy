use std::collections::HashSet;

use arenabuddy_core::{
    models::{ArenaId, MatchData, OpponentDeck},
    services::match_service::{
        GetMatchDataRequest, ListMatchesRequest, UpsertMatchDataRequest, match_service_client::MatchServiceClient,
    },
};
use arenabuddy_data::{ArenabuddyRepository, MatchDB};
use tracing::{error, info};

use super::auth::{SharedAuthState, needs_refresh, refresh};

fn attach_token<T>(request: &mut tonic::Request<T>, token: &str) {
    let bearer = format!("Bearer {token}");
    if let Ok(value) = bearer.parse() {
        request.metadata_mut().insert("authorization", value);
    }
}

async fn current_token(auth_state: &SharedAuthState, grpc_url: &str) -> Option<String> {
    let mut guard = auth_state.lock().await;
    let state = guard.as_ref()?;

    if needs_refresh(state) {
        info!("Access token expiring soon, refreshing for sync");
        match refresh(grpc_url, state).await {
            Ok(new_state) => {
                let token = new_state.token.clone();
                *guard = Some(new_state);
                return Some(token);
            }
            Err(e) => {
                error!("Failed to refresh token for sync: {e}");
            }
        }
    }

    Some(state.token.clone())
}

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

    let token = current_token(auth_state, &grpc_url).await.ok_or("not authenticated")?;

    let mut client = MatchServiceClient::connect(grpc_url).await?;

    // Get server match list
    let mut request = tonic::Request::new(ListMatchesRequest {});
    attach_token(&mut request, &token);

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
        attach_token(&mut request, &token);

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

/// Push a specific local match to the server via gRPC upsert.
///
/// Returns `Ok(true)` when the local match exists and was uploaded.
/// Returns `Ok(false)` when the local match is missing.
pub async fn push_match(
    db: &MatchDB,
    auth_state: &SharedAuthState,
    match_id: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let grpc_url = super::paths::grpc_url();
    let token = current_token(auth_state, &grpc_url).await.ok_or("not authenticated")?;

    let Ok((mtga_match, _result)) = db.get_match(match_id, None).await else {
        return Ok(false);
    };
    let decks = db.list_decklists(match_id).await?;
    let mulligans = db.list_mulligans(match_id).await?;
    let results = db.list_match_results(match_id).await?;
    let event_logs = db.list_event_logs(match_id).await?;
    let opponent_deck = db
        .get_opponent_deck(match_id)
        .await
        .ok()
        .map_or_else(OpponentDeck::empty, |d| {
            OpponentDeck::new(d.mainboard().iter().map(|&id| ArenaId::from(id)).collect())
        });

    let match_data = MatchData {
        mtga_match,
        decks,
        mulligans,
        results,
        opponent_deck,
        event_logs,
    };

    let mut request = tonic::Request::new(UpsertMatchDataRequest {
        match_data: Some((&match_data).into()),
    });
    attach_token(&mut request, &token);

    let mut client = MatchServiceClient::connect(grpc_url).await?;
    client.upsert_match_data(request).await?;
    info!("Pushed local match {match_id} to server");
    Ok(true)
}
