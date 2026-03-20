use arenabuddy_core::{
    cards::CardsDatabase,
    models::{ArenaId, MatchData, OpponentDeck},
    services::match_service::{
        ArchetypeClassification, ClassifyMatchRequest, ClassifyMatchResponse, DeleteMatchRequest, DeleteMatchResponse,
        GetMatchDataRequest, GetMatchDataResponse, ListMatchesRequest, ListMatchesResponse, UpsertMatchDataRequest,
        UpsertMatchDataResponse, match_service_server::MatchService,
    },
};
use arenabuddy_data::{ArenabuddyRepository, MatchDB};
use tonic::{Request, Response, Status};
use tracingx::{error, info, instrument};

use crate::auth::UserId;

pub(crate) struct MatchServiceImpl {
    pub(crate) db: MatchDB,
    pub(crate) cards: CardsDatabase,
    pub(crate) spreadsheet_id: Option<String>,
}

#[tonic::async_trait]
impl MatchService for MatchServiceImpl {
    #[instrument(skip(self, request))]
    async fn upsert_match_data(
        &self,
        request: Request<UpsertMatchDataRequest>,
    ) -> Result<Response<UpsertMatchDataResponse>, Status> {
        let user_id = request.extensions().get::<UserId>().map(|u| u.0);
        let match_data_proto = request
            .into_inner()
            .match_data
            .ok_or_else(|| Status::invalid_argument("match_data is required"))?;

        let match_data: MatchData = (&match_data_proto)
            .try_into()
            .map_err(|_| Status::invalid_argument("match_data is missing required fields"))?;
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
                Status::internal("failed to upsert match data")
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

    #[instrument(skip(self, request))]
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
            Status::internal("failed to get match")
        })?;

        if mtga_match.id().is_empty() {
            return Err(Status::not_found(format!("match not found: {match_id}")));
        }

        let decks = self.db.list_decklists(&match_id).await.map_err(|e| {
            error!("Failed to list decklists: {e}");
            Status::internal("failed to list decklists")
        })?;

        let mulligans = self.db.list_mulligans(&match_id).await.map_err(|e| {
            error!("Failed to list mulligans: {e}");
            Status::internal("failed to list mulligans")
        })?;

        let results = self.db.list_match_results(&match_id).await.map_err(|e| {
            error!("Failed to list match results: {e}");
            Status::internal("failed to list match results")
        })?;

        let opponent_deck = self
            .db
            .get_opponent_deck(&match_id)
            .await
            .ok()
            .map_or_else(OpponentDeck::empty, |d| {
                OpponentDeck::new(d.mainboard().iter().map(|&id| ArenaId::from(id)).collect())
            });

        let event_logs = self.db.list_event_logs(&match_id).await.map_err(|e| {
            error!("Failed to list event logs: {e}");
            Status::internal("failed to list event logs")
        })?;

        let match_data_model = MatchData {
            mtga_match,
            decks,
            mulligans,
            results,
            opponent_deck,
            event_logs,
        };

        Ok(Response::new(GetMatchDataResponse {
            match_data: Some((&match_data_model).into()),
        }))
    }

    #[instrument(skip(self, request))]
    async fn list_matches(
        &self,
        request: Request<ListMatchesRequest>,
    ) -> Result<Response<ListMatchesResponse>, Status> {
        let user_id = request.extensions().get::<UserId>().map(|u| u.0);
        let matches = self.db.list_matches(user_id).await.map_err(|e| {
            error!("Failed to list matches: {e}");
            Status::internal("failed to list matches")
        })?;

        Ok(Response::new(ListMatchesResponse {
            matches: matches.iter().map(Into::into).collect(),
        }))
    }

    #[instrument(skip(self, request))]
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
            Status::internal("failed to delete match")
        })?;

        info!("Deleted match: {match_id}");
        Ok(Response::new(DeleteMatchResponse {}))
    }

    #[instrument(skip(self, request))]
    async fn classify_match(
        &self,
        request: Request<ClassifyMatchRequest>,
    ) -> Result<Response<ClassifyMatchResponse>, Status> {
        let user_id = request.extensions().get::<UserId>().map(|u| u.0);
        let match_id = request.into_inner().match_id;
        if match_id.is_empty() {
            return Err(Status::invalid_argument("match_id is required"));
        }

        // Get the match to determine its format
        let (mtga_match, _) = self.db.get_match(&match_id, user_id).await.map_err(|e| {
            error!("Failed to get match for classification: {e}");
            Status::internal("failed to get match")
        })?;

        if mtga_match.id().is_empty() {
            return Err(Status::not_found(format!("match not found: {match_id}")));
        }

        let Some(format) = mtga_match.format() else {
            return Ok(Response::new(ClassifyMatchResponse {
                classifications: vec![],
            }));
        };

        let archetypes = arenabuddy_metagame::classification::classify_single_match(&self.db, &match_id, format)
            .await
            .map_err(|e| {
                error!("Classification failed for match {match_id}: {e}");
                Status::internal("classification failed")
            })?;

        let classifications = archetypes
            .into_iter()
            .map(|a| ArchetypeClassification {
                side: a.side,
                archetype_name: a.archetype_name,
                confidence: a.confidence,
            })
            .collect();

        Ok(Response::new(ClassifyMatchResponse { classifications }))
    }
}
