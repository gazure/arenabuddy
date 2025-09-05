use std::sync::Arc;

use arenabuddy_core::{
    cards::CardsDatabase,
    display::{
        deck::{DeckDisplayRecord, Difference},
        game::GameResultDisplay,
        match_details::MatchDetails,
        mulligan::Mulligan,
    },
    models::MTGAMatch,
};
use arenabuddy_data::DirectoryStorage;
use tokio::sync::Mutex;
use tracingx::{error, info};

use crate::Result;

#[derive(Clone)]
pub struct AppService<D: arenabuddy_data::ArenabuddyRepository> {
    pub db: Arc<Mutex<D>>,
    pub cards: Arc<CardsDatabase>,
    pub log_collector: Arc<Mutex<Vec<String>>>,
    pub debug_storage: Arc<Mutex<Option<DirectoryStorage>>>,
}

impl<D> std::fmt::Debug for AppService<D>
where
    D: arenabuddy_data::ArenabuddyRepository,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppService")
            .field("db", &"Arc<Mutex<MatchDB>>")
            .field("cards", &"CardsDatabase")
            .field("log_collector", &"Arc<Mutex<Vec<String>>>")
            .field("debug_backend", &"Arc<Mutex<Option<DirectoryStorage>>>")
            .finish()
    }
}

impl<D> AppService<D>
where
    D: arenabuddy_data::ArenabuddyRepository,
{
    pub fn new(
        db: Arc<Mutex<D>>,
        cards: Arc<CardsDatabase>,
        log_collector: Arc<Mutex<Vec<String>>>,
        debug_backend: Arc<Mutex<Option<DirectoryStorage>>>,
    ) -> Self {
        Self {
            db,
            cards,
            log_collector,
            debug_storage: debug_backend,
        }
    }

    pub async fn get_matches(&self) -> Result<Vec<MTGAMatch>> {
        let mut db = self.db.lock().await;
        Ok(db.list_matches().await?)
    }

    pub async fn get_match_details(&self, id: String) -> Result<MatchDetails> {
        let mut db = self.db.lock().await;
        info!("looking for match {id}");

        let (mtga_match, result) = db.get_match(&id).await.unwrap_or_default();

        let mut match_details = MatchDetails {
            id: id.clone(),
            controller_seat_id: mtga_match.controller_seat_id(),
            controller_player_name: mtga_match.controller_player_name().to_string(),
            opponent_player_name: mtga_match.opponent_player_name().to_string(),
            created_at: mtga_match.created_at(),
            did_controller_win: result.is_some_and(|r| r.is_winner(mtga_match.controller_seat_id())),
            ..Default::default()
        };

        match_details.decklists = db.list_decklists(&id).await.unwrap_or_default();

        match_details.primary_decklist = match_details
            .decklists
            .first()
            .map(|primary_decklist| DeckDisplayRecord::from_decklist(primary_decklist, &self.cards));

        match_details.decklists.windows(2).for_each(|pair| {
            if let [prev, next] = pair {
                let diff = Difference::diff(prev, next, &self.cards);
                match_details.differences.get_or_insert_with(Vec::new).push(diff);
            }
        });

        let raw_mulligans = db.list_mulligans(&id).await.unwrap_or_else(|e| {
            error!("Error retrieving Mulligans: {}", e);
            Vec::default()
        });

        match_details.mulligans = raw_mulligans
            .iter()
            .map(|mulligan| Mulligan::from_model(mulligan, &self.cards))
            .collect();

        match_details.mulligans.sort();

        match_details.game_results = db
            .list_match_results(&id)
            .await
            .unwrap_or_else(|e| {
                error!("Error retrieving game results: {}", e);
                Vec::default()
            })
            .iter()
            .map(|mr| {
                GameResultDisplay::from_match_result(
                    mr,
                    match_details.controller_seat_id,
                    &match_details.controller_player_name,
                    &match_details.opponent_player_name,
                )
            })
            .collect();

        match_details.opponent_deck = db
            .get_opponent_deck(&id)
            .await
            .map(|deck| DeckDisplayRecord::from_decklist(&deck, &self.cards))
            .ok();

        Ok(match_details)
    }

    pub async fn get_error_logs(&self) -> Result<Vec<String>> {
        let logs = self.log_collector.lock().await;
        Ok(logs.clone())
    }

    pub async fn set_debug_logs(&self, path: String) -> Result<()> {
        let storage = DirectoryStorage::new(path.into());
        let mut debug_backend = self.debug_storage.lock().await;
        *debug_backend = Some(storage);
        Ok(())
    }

    pub async fn get_debug_logs(&self) -> Result<Option<Vec<String>>> {
        let debug_backend = self.debug_storage.lock().await;
        if let Some(_storage) = &*debug_backend {
            // Implementation depends on DirectoryStorage interface
            // This is a placeholder - adjust based on actual interface
            Ok(Some(vec!["Debug logs not yet implemented".to_string()]))
        } else {
            Ok(None)
        }
    }
}
