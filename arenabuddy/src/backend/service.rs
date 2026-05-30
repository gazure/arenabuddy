use std::{collections::BTreeMap, sync::Arc};

use arenabuddy_core::{
    cards::CardsDatabase,
    display::{
        deck::{DeckDisplayRecord, Difference},
        draft::DraftDetailsDisplay,
        game::GameResultDisplay,
        match_details::MatchDetails,
        match_summary::MatchSummary,
        mulligan::Mulligan,
        stats::{MatchStats, TimeWindow},
    },
    models::{Card, CardFace, Cost, Draft},
};
use arenabuddy_data::{DirectoryStorage, MetagameRepository};
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CardFaceSummary {
    pub name: String,
    pub type_line: String,
    pub mana_cost: String,
    pub image_uri: String,
    pub colors: Vec<String>,
}

impl From<&CardFace> for CardFaceSummary {
    fn from(face: &CardFace) -> Self {
        Self {
            name: face.name.clone(),
            type_line: face.type_line.clone(),
            mana_cost: face.mana_cost.clone(),
            image_uri: face.image_uri.clone().unwrap_or_default(),
            colors: face.colors.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CardSearchResult {
    pub id: i64,
    pub name: String,
    pub set: String,
    pub type_line: String,
    pub mana_cost: String,
    pub mana_value: i32,
    pub image_uri: String,
    pub colors: Vec<String>,
    pub color_identity: Vec<String>,
    pub layout: String,
    pub faces: Vec<CardFaceSummary>,
}

impl CardSearchResult {
    pub fn cost(&self) -> Cost {
        self.mana_cost.parse().unwrap_or_default()
    }
}

impl From<&Card> for CardSearchResult {
    fn from(card: &Card) -> Self {
        Self {
            id: card.id,
            name: card.name.clone(),
            set: card.set.clone(),
            type_line: card.type_line.clone(),
            mana_cost: card.mana_cost.clone(),
            mana_value: card.cmc,
            image_uri: card.primary_image_uri().unwrap_or_default().to_string(),
            colors: card.colors.clone(),
            color_identity: card.color_identity.clone(),
            layout: card.layout.clone(),
            faces: card.card_faces.iter().map(CardFaceSummary::from).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CardSetSummary {
    pub set: String,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CardDatabaseSummary {
    pub total_cards: usize,
    pub total_sets: usize,
    pub sets: Vec<CardSetSummary>,
}

#[derive(Clone)]
pub struct AppService<D: arenabuddy_data::ArenabuddyRepository> {
    pub db: D,
    pub cards: CardsDatabase,
    pub log_collector: Arc<Mutex<Vec<String>>>,
    /// Shared mutable debug storage. `Arc<Mutex<Option<..>>>` is intentional:
    /// both `AppService` (UI) and the ingestion service need shared mutable
    /// access, and the `Option` represents "not yet configured".
    pub debug_storage: Arc<Mutex<Option<DirectoryStorage>>>,
}

impl<D> std::fmt::Debug for AppService<D>
where
    D: arenabuddy_data::ArenabuddyRepository + MetagameRepository,
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
    D: arenabuddy_data::ArenabuddyRepository + MetagameRepository,
{
    pub fn new(
        db: D,
        cards: CardsDatabase,
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

    pub async fn get_match_summaries(&self) -> Result<Vec<MatchSummary>> {
        Ok(self.db.list_match_summaries(None).await?)
    }

    pub async fn get_match_details(&self, id: String) -> Result<MatchDetails> {
        info!("looking for match {id}");

        let (mtga_match, result) = self.db.get_match(&id, None).await.unwrap_or_default();

        let mut match_details = MatchDetails {
            id: id.clone(),
            controller_seat_id: mtga_match.controller_seat_id(),
            controller_player_name: mtga_match.controller_player_name().to_string(),
            opponent_player_name: mtga_match.opponent_player_name().to_string(),
            created_at: mtga_match.created_at(),
            format: mtga_match.format().map(ToString::to_string),
            did_controller_win: result.is_some_and(|r| r.is_winner(mtga_match.controller_seat_id())),
            ..Default::default()
        };

        match_details.decklists = self.db.list_decklists(&id).await.unwrap_or_default();

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

        let raw_mulligans = self.db.list_mulligans(&id).await.unwrap_or_else(|e| {
            error!("Error retrieving Mulligans: {}", e);
            Vec::default()
        });

        match_details.mulligans = raw_mulligans
            .iter()
            .map(|mulligan| Mulligan::from_model(mulligan, &self.cards))
            .collect();

        match_details.mulligans.sort();

        match_details.game_results = self
            .db
            .list_match_results(&id)
            .await
            .unwrap_or_else(|e| {
                error!("Error retrieving game results: {}", e);
                Vec::default()
            })
            .iter()
            .filter(|mr| mr.game_number() > 0)
            .map(|mr| {
                GameResultDisplay::from_match_result(
                    mr,
                    match_details.controller_seat_id,
                    &match_details.controller_player_name,
                    &match_details.opponent_player_name,
                )
            })
            .collect();

        match_details.opponent_deck = self
            .db
            .get_opponent_deck(&id)
            .await
            .map(|deck| DeckDisplayRecord::from_decklist(&deck, &self.cards))
            .ok();

        match_details.event_logs = self.db.list_event_logs(&id).await.unwrap_or_else(|e| {
            error!("Error retrieving event logs: {}", e);
            Vec::default()
        });

        let (controller_archetype, opponent_archetype) = self.db.get_match_archetypes(&id).await.unwrap_or_default();
        match_details.controller_archetype = controller_archetype;
        match_details.opponent_archetype = opponent_archetype;

        Ok(match_details)
    }

    pub async fn get_drafts(&self) -> Result<Vec<Draft>> {
        Ok(self.db.list_drafts().await?)
    }

    pub async fn get_draft_details(&self, draft_id: String) -> Result<DraftDetailsDisplay> {
        info!("looking for draft {draft_id}");

        let draft = self.db.get_draft(&draft_id).await?;
        Ok(DraftDetailsDisplay::new(draft, &self.cards))
    }

    pub async fn get_stats(&self, time_window: TimeWindow) -> Result<MatchStats> {
        Ok(self.db.get_match_stats(None, time_window).await?)
    }

    pub async fn get_card_database_summary(&self) -> Result<CardDatabaseSummary> {
        Ok(card_database_summary(&self.cards))
    }

    pub async fn search_cards(&self, query: String, set_filter: Option<String>) -> Result<Vec<CardSearchResult>> {
        Ok(search_cards(&self.cards, &query, set_filter.as_deref()))
    }

    pub async fn get_card_by_arena_id(&self, arena_id: i64) -> Result<Option<CardSearchResult>> {
        Ok(self.cards.get(&arena_id.to_string()).map(CardSearchResult::from))
    }

    pub async fn get_card_json(&self, arena_id: i64) -> Result<Option<String>> {
        self.cards
            .get(&arena_id.to_string())
            .map(serde_json::to_string_pretty)
            .transpose()
            .map_err(Into::into)
    }

    pub async fn get_error_logs(&self) -> Result<Vec<String>> {
        let logs = self.log_collector.lock().await;
        Ok(logs.clone())
    }

    pub async fn set_debug_logs(&self, path: String) {
        let storage = DirectoryStorage::new(path.into());
        let mut debug_backend = self.debug_storage.lock().await;
        *debug_backend = Some(storage);
    }

    pub async fn get_debug_logs(&self) -> Result<Option<Vec<String>>> {
        let debug_backend = self.debug_storage.lock().await;
        if let Some(storage) = &*debug_backend {
            let replays = storage.list_replays().await?;
            Ok(Some(replays))
        } else {
            Ok(None)
        }
    }
}

fn card_database_summary(cards: &CardsDatabase) -> CardDatabaseSummary {
    let mut set_counts = BTreeMap::<String, usize>::new();
    for card in cards.values() {
        *set_counts.entry(card.set.clone()).or_default() += 1;
    }

    let sets: Vec<_> = set_counts
        .into_iter()
        .map(|(set, count)| CardSetSummary { set, count })
        .collect();

    CardDatabaseSummary {
        total_cards: cards.len(),
        total_sets: sets.len(),
        sets,
    }
}

fn search_cards(cards: &CardsDatabase, query: &str, set_filter: Option<&str>) -> Vec<CardSearchResult> {
    let normalized_query = query.trim().to_lowercase();
    let normalized_set = set_filter
        .map(str::trim)
        .filter(|set| !set.is_empty())
        .map(str::to_lowercase);

    let mut matches: Vec<_> = cards
        .values()
        .filter(|card| {
            let matches_query = normalized_query.is_empty() || card.name.to_lowercase().starts_with(&normalized_query);
            let matches_set = normalized_set
                .as_deref()
                .is_none_or(|set| card.set.to_lowercase() == set);

            matches_query && matches_set
        })
        .map(CardSearchResult::from)
        .collect();

    matches.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.set.cmp(&b.set))
            .then_with(|| a.id.cmp(&b.id))
    });
    matches
}

#[cfg(test)]
mod tests {
    use arenabuddy_core::models::CardCollection;

    use super::*;

    fn test_database(cards: Vec<Card>) -> CardsDatabase {
        CardsDatabase::from_bytes(&CardCollection::with_cards(cards).encode_to_vec()).expect("cards encode")
    }

    fn test_card(id: i64, set: &str, name: &str) -> Card {
        let mut card = Card::new(id, set, name);
        card.type_line = "Instant".to_string();
        card.mana_cost = "{U}".to_string();
        card.cmc = 1;
        card
    }

    #[test]
    fn summarizes_sets_in_code_order() {
        let cards = test_database(vec![
            test_card(3, "TDM", "Opt"),
            test_card(1, "BRO", "Island"),
            test_card(2, "BRO", "Forest"),
        ]);

        let summary = card_database_summary(&cards);

        assert_eq!(summary.total_cards, 3);
        assert_eq!(summary.total_sets, 2);
        assert_eq!(summary.sets[0].set, "BRO");
        assert_eq!(summary.sets[0].count, 2);
        assert_eq!(summary.sets[1].set, "TDM");
        assert_eq!(summary.sets[1].count, 1);
    }

    #[test]
    fn searches_name_prefix_case_insensitively() {
        let cards = test_database(vec![
            test_card(1, "BRO", "Opt"),
            test_card(2, "BRO", "Omenpath Journey"),
            test_card(3, "TDM", "Lightning Strike"),
        ]);

        let matches = search_cards(&cards, "om", None);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Omenpath Journey");
    }

    #[test]
    fn empty_query_can_filter_to_a_set() {
        let cards = test_database(vec![
            test_card(1, "BRO", "Opt"),
            test_card(2, "TDM", "Omenpath Journey"),
            test_card(3, "TDM", "Lightning Strike"),
        ]);

        let matches = search_cards(&cards, "", Some("tdm"));

        assert_eq!(matches.len(), 2);
        assert!(matches.iter().all(|card| card.set == "TDM"));
    }
}
