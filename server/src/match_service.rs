use arenabuddy_core::{
    cards::CardsDatabase,
    models::{ArenaId, MatchData, OpponentDeck},
    services::match_service::{
        DeleteMatchRequest, DeleteMatchResponse, GetMatchDataRequest, GetMatchDataResponse, ListMatchesRequest,
        ListMatchesResponse, UpsertMatchDataRequest, UpsertMatchDataResponse, match_service_server::MatchService,
    },
};
use arenabuddy_data::{ArenabuddyRepository, MatchDB};
use tonic::{Request, Response, Status};
use tracingx::{error, info};

use crate::auth::UserId;

pub(crate) struct MatchServiceImpl {
    pub(crate) db: MatchDB,
    pub(crate) cards: CardsDatabase,
    pub(crate) spreadsheet_id: Option<String>,
}

#[tonic::async_trait]
impl MatchService for MatchServiceImpl {
    async fn upsert_match_data(
        &self,
        request: Request<UpsertMatchDataRequest>,
    ) -> Result<Response<UpsertMatchDataResponse>, Status> {
        let user_id = request.extensions().get::<UserId>().map(|u| u.0);
        let match_data_proto = request
            .into_inner()
            .match_data
            .ok_or_else(|| Status::invalid_argument("match_data is required"))?;

        let match_data: MatchData = (&match_data_proto).into();
        let match_id = match_data.mtga_match.id().to_string();

        let opponent_cards: Vec<ArenaId> = match_data.opponent_deck.cards.clone();

        self.db
            .upsert_match_data(
                &match_data.mtga_match,
                &match_data.decks,
                &match_data.mulligans,
                &match_data.results,
                &opponent_cards,
                &match_data.event_logs,
                user_id,
            )
            .await
            .map_err(|e| {
                error!("Failed to upsert match data: {e}");
                Status::internal(format!("failed to upsert match data: {e}"))
            })?;

        info!("Upserted match data for match_id: {match_id}");

        if let Some(ref spreadsheet_id) = self.spreadsheet_id {
            crate::sheets_sync::spawn_sheets_sync(
                self.db.clone(),
                self.cards.clone(),
                match_id,
                user_id,
                spreadsheet_id.clone(),
            );
        }

        Ok(Response::new(UpsertMatchDataResponse {}))
    }

    async fn get_match_data(
        &self,
        request: Request<GetMatchDataRequest>,
    ) -> Result<Response<GetMatchDataResponse>, Status> {
        let user_id = request.extensions().get::<UserId>().map(|u| u.0);
        let match_id = request.into_inner().match_id;
        if match_id.is_empty() {
            return Err(Status::invalid_argument("match_id is required"));
        }

        let (mtga_match, _match_result) = self.db.get_match(&match_id, user_id).await.map_err(|e| {
            error!("Failed to get match: {e}");
            Status::internal(format!("failed to get match: {e}"))
        })?;

        if mtga_match.id().is_empty() {
            return Err(Status::not_found(format!("match not found: {match_id}")));
        }

        let decks = self.db.list_decklists(&match_id).await.map_err(|e| {
            error!("Failed to list decklists: {e}");
            Status::internal(format!("failed to list decklists: {e}"))
        })?;

        let mulligans = self.db.list_mulligans(&match_id).await.map_err(|e| {
            error!("Failed to list mulligans: {e}");
            Status::internal(format!("failed to list mulligans: {e}"))
        })?;

        let results = self.db.list_match_results(&match_id).await.map_err(|e| {
            error!("Failed to list match results: {e}");
            Status::internal(format!("failed to list match results: {e}"))
        })?;

        let opponent_deck = self
            .db
            .get_opponent_deck(&match_id)
            .await
            .ok()
            .map_or_else(OpponentDeck::empty, |d| {
                OpponentDeck::new(d.mainboard().iter().map(|&id| ArenaId::from(id)).collect())
            });

        let match_data_model = MatchData {
            mtga_match,
            decks,
            mulligans,
            results,
            opponent_deck,
            event_logs: Vec::new(), // TODO: retrieve from database
        };

        Ok(Response::new(GetMatchDataResponse {
            match_data: Some((&match_data_model).into()),
        }))
    }

    async fn list_matches(
        &self,
        request: Request<ListMatchesRequest>,
    ) -> Result<Response<ListMatchesResponse>, Status> {
        let user_id = request.extensions().get::<UserId>().map(|u| u.0);
        let matches = self.db.list_matches(user_id).await.map_err(|e| {
            error!("Failed to list matches: {e}");
            Status::internal(format!("failed to list matches: {e}"))
        })?;

        Ok(Response::new(ListMatchesResponse {
            matches: matches.iter().map(Into::into).collect(),
        }))
    }

    async fn delete_match(
        &self,
        request: Request<DeleteMatchRequest>,
    ) -> Result<Response<DeleteMatchResponse>, Status> {
        let user_id = request.extensions().get::<UserId>().map(|u| u.0);
        let match_id = request.into_inner().match_id;
        if match_id.is_empty() {
            return Err(Status::invalid_argument("match_id is required"));
        }

        self.db.delete_match(&match_id, user_id).await.map_err(|e| {
            error!("Failed to delete match: {e}");
            Status::internal(format!("failed to delete match: {e}"))
        })?;

        info!("Deleted match: {match_id}");
        Ok(Response::new(DeleteMatchResponse {}))
    }
}
