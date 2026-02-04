use std::sync::Arc;

use arenabuddy_core::{
    cards::CardsDatabase,
    models::{MTGAMatch, MatchData, MatchResult, OpponentDeck},
    player_log::replay::MatchReplay,
    proto::{UpsertMatchDataRequest, match_service_client::MatchServiceClient},
};
use chrono::Utc;
use tonic::transport::Channel;
use tracingx::{error, info};

pub struct GrpcReplayWriter {
    client: MatchServiceClient<Channel>,
    cards: Arc<CardsDatabase>,
}

impl GrpcReplayWriter {
    pub async fn connect(url: &str, cards: Arc<CardsDatabase>) -> Result<Self, tonic::transport::Error> {
        let client = MatchServiceClient::connect(url.to_string()).await?;
        Ok(Self { client, cards })
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

        let match_data = MatchData {
            mtga_match,
            decks,
            mulligans,
            results,
            opponent_deck: OpponentDeck::new(opponent_cards),
        };

        let request = UpsertMatchDataRequest {
            match_data: Some((&match_data).into()),
        };

        self.client.upsert_match_data(request).await.map_err(|e| {
            error!("gRPC upsert failed for match {match_id}: {e}");
            arenabuddy_core::Error::Io(format!("gRPC upsert failed: {e}"))
        })?;

        info!("Sent match {match_id} to gRPC backend");
        Ok(())
    }
}
