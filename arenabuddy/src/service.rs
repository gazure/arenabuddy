use std::sync::Arc;

use anyhow::Result;
use arenabuddy_core::{
    display::{
        deck::{DeckDisplayRecord, Difference},
        game::GameResultDisplay,
        match_details::MatchDetails,
        mulligan::Mulligan,
    },
    models::MTGAMatch,
};
use arenabuddy_data::{DirectoryStorage, MatchDB};
use dioxus::prelude::*;
#[cfg(feature = "server")]
use tokio::sync::Mutex;
use tracing::{error, info};

#[cfg(feature = "server")]
#[derive(Clone)]
pub struct AppService {
    pub db: Arc<Mutex<MatchDB>>,
    pub log_collector: Arc<Mutex<Vec<String>>>,
    pub debug_storage: Arc<Mutex<Option<DirectoryStorage>>>,
}

#[cfg(feature = "server")]
impl std::fmt::Debug for AppService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppService")
            .field("db", &"Arc<Mutex<MatchDB>>")
            .field("log_collector", &"Arc<Mutex<Vec<String>>>")
            .field("debug_backend", &"Arc<Mutex<Option<DirectoryStorage>>>")
            .finish()
    }
}

#[cfg(feature = "server")]
impl AppService {
    pub fn new(
        db: Arc<Mutex<MatchDB>>,
        log_collector: Arc<Mutex<Vec<String>>>,
        debug_backend: Arc<Mutex<Option<DirectoryStorage>>>,
    ) -> Self {
        Self {
            db,
            log_collector,
            debug_storage: debug_backend,
        }
    }

    pub async fn get_matches(&self) -> Result<Vec<MTGAMatch>> {
        let db = self.db.lock().await;
        db.get_matches().await.map_err(Into::into)
    }

    pub async fn get_match_details(&self, id: String) -> Result<MatchDetails> {
        let db = self.db.lock().await;
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

        match_details.decklists = db.get_decklists(&id).await.unwrap_or_default();

        match_details.primary_decklist = match_details
            .decklists
            .first()
            .map(|primary_decklist| DeckDisplayRecord::from_decklist(primary_decklist, &db.cards));

        match_details.decklists.windows(2).for_each(|pair| {
            if let [prev, next] = pair {
                let diff = Difference::diff(prev, next, &db.cards);
                match_details.differences.get_or_insert_with(Vec::new).push(diff);
            }
        });

        let raw_mulligans = db.get_mulligans(&id).await.unwrap_or_else(|e| {
            error!("Error retrieving Mulligans: {}", e);
            Vec::default()
        });

        match_details.mulligans = raw_mulligans
            .iter()
            .map(|mulligan| Mulligan::from_model(mulligan, &db.cards))
            .collect();

        match_details.mulligans.sort();

        match_details.game_results = db
            .get_match_results(&id)
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

// Convenience functions that mirror the original Tauri commands
#[server]
pub async fn command_matches() -> ServerFnResult<Vec<MTGAMatch>> {
    let FromContext(service): FromContext<AppService> = extract().await?;
    service
        .get_matches()
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[server]
pub async fn command_match_details(id: String) -> ServerFnResult<MatchDetails> {
    let FromContext(service): FromContext<AppService> = extract().await?;
    service
        .get_match_details(id)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[server]
pub async fn command_error_logs() -> ServerFnResult<Vec<String>> {
    let FromContext(service): FromContext<AppService> = extract().await?;
    service
        .get_error_logs()
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[server]
pub async fn command_set_debug_logs(path: String) -> ServerFnResult<()> {
    let FromContext(service): FromContext<AppService> = extract().await?;
    service
        .set_debug_logs(path)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[server]
pub async fn command_get_debug_logs() -> ServerFnResult<Option<Vec<String>>> {
    let FromContext(service): FromContext<AppService> = extract().await?;
    service
        .get_debug_logs()
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}
