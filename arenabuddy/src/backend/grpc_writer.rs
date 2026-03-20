use arenabuddy_core::{
    cards::CardsDatabase,
    models::{MTGAMatch, MatchData, MatchResult, OpponentDeck},
    player_log::replay::MatchReplay,
    services::match_service::{ClassifyMatchRequest, UpsertMatchDataRequest, match_service_client::MatchServiceClient},
};
use arenabuddy_data::{MatchDB, MetagameRepository, metagame_models::MatchArchetype};
use chrono::Utc;
use tonic::transport::Channel;
use tracingx::{error, info};

use super::auth::{SharedAuthState, needs_refresh, refresh};

pub struct GrpcReplayWriter {
    client: MatchServiceClient<Channel>,
    cards: CardsDatabase,
    auth_state: SharedAuthState,
    grpc_url: String,
    local_db: MatchDB,
}

impl GrpcReplayWriter {
    pub async fn connect(
        url: &str,
        cards: CardsDatabase,
        auth_state: SharedAuthState,
        local_db: MatchDB,
    ) -> Result<Self, tonic::transport::Error> {
        let client = MatchServiceClient::connect(url.to_string()).await?;
        Ok(Self {
            client,
            cards,
            auth_state,
            grpc_url: url.to_string(),
            local_db,
        })
    }

    /// Ensure the access token is fresh, refreshing if needed.
    /// Returns the current Bearer token string, or None if not authenticated.
    async fn current_token(&self) -> Option<String> {
        let mut guard = self.auth_state.lock().await;
        let state = guard.as_ref()?;

        if needs_refresh(state) {
            info!("Access token expiring soon, refreshing");
            match refresh(&self.grpc_url, state).await {
                Ok(new_state) => {
                    *guard = Some(new_state.clone());
                    return Some(new_state.token);
                }
                Err(e) => {
                    error!("Failed to refresh token: {e}");
                    // Fall through and use the existing (possibly expired) token
                }
            }
        }

        Some(state.token.clone())
    }

    /// Attempt to refresh and retry the request once after an UNAUTHENTICATED error.
    async fn refresh_and_retry(&mut self, match_data: &MatchData, match_id: &str) -> arenabuddy_core::Result<()> {
        let new_token = {
            let mut guard = self.auth_state.lock().await;
            let state = guard
                .as_ref()
                .ok_or_else(|| arenabuddy_core::Error::Io("not authenticated".to_string()))?;

            match refresh(&self.grpc_url, state).await {
                Ok(new_state) => {
                    let token = new_state.token.clone();
                    *guard = Some(new_state);
                    token
                }
                Err(e) => {
                    error!("Retry refresh failed: {e}");
                    return Err(arenabuddy_core::Error::Io(format!("refresh failed: {e}")));
                }
            }
        };

        let request = build_request(match_data, Some(new_token.clone()));
        self.client.upsert_match_data(request).await.map_err(|e| {
            error!("gRPC retry failed for match {match_id}: {e}");
            arenabuddy_core::Error::Io(format!("gRPC retry failed: {e}"))
        })?;

        info!("Sent match {match_id} to gRPC backend (after refresh)");
        self.classify_and_cache(match_id, Some(new_token)).await;
        Ok(())
    }

    /// Request classification from the server and cache results locally.
    /// Best-effort: errors are logged but not propagated.
    async fn classify_and_cache(&mut self, match_id: &str, token: Option<String>) {
        let mut request = tonic::Request::new(ClassifyMatchRequest {
            match_id: match_id.to_string(),
        });

        if let Some(token) = token {
            let bearer = format!("Bearer {token}");
            if let Ok(value) = bearer.parse() {
                request.metadata_mut().insert("authorization", value);
            }
        }

        match self.client.classify_match(request).await {
            Ok(response) => {
                let classifications = response.into_inner().classifications;
                for c in &classifications {
                    let ma = MatchArchetype {
                        match_id: match_id.to_string(),
                        side: c.side.clone(),
                        archetype_id: None,
                        archetype_name: c.archetype_name.clone(),
                        confidence: c.confidence,
                    };
                    if let Err(e) = self.local_db.upsert_match_archetype(&ma).await {
                        error!("Failed to cache archetype for match {match_id}: {e}");
                    }
                }
                if !classifications.is_empty() {
                    info!(
                        "Cached {} archetype classification(s) for match {match_id}",
                        classifications.len()
                    );
                }
            }
            Err(e) => {
                error!("Classification request failed for match {match_id}: {e}");
            }
        }
    }
}

#[async_trait::async_trait]
impl arenabuddy_core::player_log::ingest::ReplayWriter for GrpcReplayWriter {
    async fn write(&mut self, replay: &MatchReplay) -> arenabuddy_core::Result<()> {
        let controller_seat_id = replay.get_controller_seat_id();
        let (controller_name, opponent_name) = replay.get_player_names(controller_seat_id)?;
        let event_start = replay.match_start_time().unwrap_or(Utc::now());

        let mtga_match = MTGAMatch::new_with_timestamp(
            &replay.match_id,
            controller_seat_id,
            controller_name,
            opponent_name,
            event_start,
        );
        let match_id = mtga_match.id().to_string();

        let decks = replay.get_decklists()?;
        let mulligans = replay.get_mulligan_infos(&self.cards)?;
        let match_results = replay.get_match_results()?;
        let opponent_cards = replay.get_opponent_cards();

        let results: Vec<MatchResult> = match_results
            .result_list
            .iter()
            .enumerate()
            .map(|(i, result)| {
                let game_number = if result.scope == "MatchScope_Game" {
                    i32::try_from(i + 1).unwrap_or(0)
                } else {
                    0
                };
                MatchResult::new(&match_id, game_number, result.winning_team_id, &result.scope)
            })
            .collect();

        let event_logs = replay.get_event_logs(&self.cards);

        let match_data = MatchData {
            mtga_match,
            decks,
            mulligans,
            results,
            opponent_deck: OpponentDeck::new(opponent_cards),
            event_logs,
        };

        let token = self.current_token().await;
        let request = build_request(&match_data, token.clone());

        match self.client.upsert_match_data(request).await {
            Ok(_) => {
                info!("Sent match {match_id} to gRPC backend");
                self.classify_and_cache(&match_id, token).await;
                Ok(())
            }
            Err(e) if e.code() == tonic::Code::Unauthenticated => {
                info!("Got UNAUTHENTICATED, attempting refresh and retry");
                self.refresh_and_retry(&match_data, &match_id).await
            }
            Err(e) => {
                error!("gRPC upsert failed for match {match_id}: {e}");
                Err(arenabuddy_core::Error::Io(format!("gRPC upsert failed: {e}")))
            }
        }
    }
}

fn build_request(match_data: &MatchData, token: Option<String>) -> tonic::Request<UpsertMatchDataRequest> {
    let mut request = tonic::Request::new(UpsertMatchDataRequest {
        match_data: Some(match_data.into()),
    });

    if let Some(token) = token {
        let bearer = format!("Bearer {token}");
        if let Ok(value) = bearer.parse() {
            request.metadata_mut().insert("authorization", value);
        }
    }

    request
}
