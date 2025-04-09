use std::sync::{Arc, Mutex};

use arenabuddy_core::{
    display::{
        deck::{DeckDisplayRecord, Difference},
        game::GameResultDisplay,
        match_details::MatchDetails,
        mulligan::Mulligan,
    },
    match_insights::MatchDB,
};
use tauri::State;
use tracing::{error, info};

#[tauri::command]
pub(crate) fn command_match_details(
    match_id: String,
    db: State<'_, Arc<Mutex<MatchDB>>>,
) -> MatchDetails {
    let db_lock_result = db.inner().lock();
    info!("looking for match {match_id}");
    if let Err(e) = db_lock_result {
        error!("Failed to obtain db lock: {}", e);
        return MatchDetails::default();
    }
    let mut db = db_lock_result.expect("handled error case");

    let (mtga_match, did_controller_win) = db.get_match(&match_id).unwrap_or_default();

    let mut match_details = MatchDetails {
        id: match_id.clone(),
        controller_seat_id: mtga_match.controller_seat_id,
        controller_player_name: mtga_match.controller_player_name,
        opponent_player_name: mtga_match.opponent_player_name,
        created_at: mtga_match.created_at,
        did_controller_win,
        ..Default::default()
    };

    match_details.decklists = db.get_decklists(&match_id).unwrap_or_default();

    match_details.primary_decklist = match_details.decklists.first().map(|primary_decklist| {
        DeckDisplayRecord::from_decklist(primary_decklist, &db.cards_database)
    });

    match_details.decklists.windows(2).for_each(|pair| {
        if let [prev, next] = pair {
            let diff = Difference::diff(prev, next, &db.cards_database);
            match_details
                .differences
                .get_or_insert_with(Vec::new)
                .push(diff);
        }
    });

    let raw_mulligans = db.get_mulligans(&match_id).unwrap_or_else(|e| {
        error!("Error retrieving Mulligans: {}", e);
        Vec::default()
    });

    match_details.mulligans = raw_mulligans
        .iter()
        .map(|mulligan| Mulligan::from_mulligan_info(mulligan, &db.cards_database))
        .collect();

    match_details.game_results = db
        .get_match_results(&match_id)
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

    match_details
}
