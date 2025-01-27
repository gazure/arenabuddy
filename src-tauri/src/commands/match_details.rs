use std::sync::{Arc, Mutex};

use arenabuddy_core::cards::CardsDatabase;
use arenabuddy_core::match_insights::MatchInsightDB;
use arenabuddy_core::models::deck::Deck;
use arenabuddy_core::models::match_result::MatchResult;
use arenabuddy_core::models::mulligan::MulliganInfo;
use chrono::{DateTime, Utc};
use indoc::indoc;
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{error, info};

use crate::card::CardDisplayRecord;
use crate::deck::{DeckDifference, DeckDisplayRecord};

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
struct Mulligan {
    hand: Vec<CardDisplayRecord>,
    opponent_identity: String,
    game_number: i32,
    number_to_keep: i32,
    play_draw: String,
    decision: String,
}

impl Mulligan {
    pub fn new(
        hand: &str,
        opponent_identity: String,
        game_number: i32,
        number_to_keep: i32,
        play_draw: String,
        decision: String,
        cards_database: &CardsDatabase,
    ) -> Self {
        let hand = hand
            .split(',')
            .filter_map(|card_id_str| card_id_str.parse::<i32>().ok())
            .map(|card_id| -> CardDisplayRecord {
                cards_database.get(&card_id).map_or_else(
                    || CardDisplayRecord::new(card_id.to_string()),
                    std::convert::Into::into,
                )
            })
            .collect();

        Self {
            hand,
            opponent_identity,
            game_number,
            number_to_keep,
            play_draw,
            decision,
        }
    }

    pub fn from_mulligan_info(
        mulligan_info: &MulliganInfo,
        cards_database: &CardsDatabase,
    ) -> Self {
        Self::new(
            &mulligan_info.hand,
            mulligan_info.opponent_identity.clone(),
            mulligan_info.game_number,
            mulligan_info.number_to_keep,
            mulligan_info.play_draw.clone(),
            mulligan_info.decision.clone(),
            cards_database,
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GameResultDisplay {
    pub game_number: i32,
    pub winning_player: String,
}

impl GameResultDisplay {
    pub fn from_match_result(
        mr: &MatchResult,
        controller_seat_id: i32,
        controller_player_name: &str,
        opponent_player_name: &str,
    ) -> Self {
        Self {
            game_number: mr.game_number,
            winning_player: if mr.winning_team_id == controller_seat_id {
                controller_player_name.into()
            } else {
                opponent_player_name.into()
            },
        }
    }
}

// TODO: Builder pattern, lol
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub(crate) struct MatchDetails {
    id: String,
    did_controller_win: bool,
    controller_seat_id: i32,
    controller_player_name: String,
    opponent_player_name: String,
    created_at: DateTime<Utc>,
    primary_decklist: Option<DeckDisplayRecord>,
    differences: Option<Vec<DeckDifference>>,
    game_results: Vec<GameResultDisplay>,
    decklists: Vec<Deck>,
    mulligans: Vec<Mulligan>,
}

#[tauri::command]
pub(crate) fn command_match_details(
    match_id: String,
    db: State<'_, Arc<Mutex<MatchInsightDB>>>,
) -> MatchDetails {
    let db_lock_result = db.inner().lock();
    if let Err(e) = db_lock_result {
        error!("Failed to obtain db lock: {}", e);
        return MatchDetails::default();
    }
    let mut db = db_lock_result.expect("handled error case");

    let mut match_details = {
        let mut statement = db.conn.prepare(indoc! {r#"
            SELECT
                m.id, m.controller_player_name, m.opponent_player_name, m.controller_seat_id = mr.winning_team_id, m.controller_seat_id, m.created_at
            FROM matches m JOIN match_results mr ON m.id = mr.match_id
            WHERE m.id = ?1 AND mr.result_scope = "MatchScope_Match" LIMIT 1
            "#}
        ).expect("valid sql");

        info!("Getting match details for match_id: {}", match_id);
        statement
            .query_row([&match_id], |row| {
                let id: String = row.get(0)?;
                let controller_player_name: String = row.get(1)?;
                let opponent_player_name: String = row.get(2)?;
                let did_controller_win: bool = row.get(3)?;
                let controller_seat_id: i32 = row.get(4)?;
                let created_at: DateTime<Utc> = row.get(5)?;
                Ok(MatchDetails {
                    id,
                    did_controller_win,
                    controller_seat_id,
                    controller_player_name,
                    opponent_player_name,
                    created_at,
                    primary_decklist: None,
                    differences: None,
                    game_results: Vec::new(),
                    decklists: Vec::new(),
                    mulligans: Vec::new(),
                })
            })
            .unwrap_or_else(|e| {
                error!("Error getting match details: {:?}", e);
                MatchDetails::default()
            })
    };

    match_details.decklists = db.get_decklists(&match_id).unwrap_or_default();

    match_details.primary_decklist = match_details.decklists.first().map(|primary_decklist| {
        DeckDisplayRecord::from_decklist(primary_decklist, &db.cards_database)
    });

    match_details.decklists.windows(2).for_each(|pair| {
        if let [prev, next] = pair {
            let diff = DeckDifference::difference(prev, next, &db.cards_database);
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
