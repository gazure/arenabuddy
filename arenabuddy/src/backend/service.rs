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
    models::{Card, CardFace, CardType, Cost, Draft},
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
    pub oracle_text: String,
    pub stats: Option<(&'static str, String)>,
    pub flavor_text: String,
}

impl From<&CardFace> for CardFaceSummary {
    fn from(face: &CardFace) -> Self {
        Self {
            name: face.name.clone(),
            type_line: face.type_line.clone(),
            mana_cost: face.mana_cost.clone(),
            image_uri: face.image_uri.clone().unwrap_or_default(),
            colors: face.colors.clone(),
            oracle_text: face.oracle_text.clone(),
            stats: stats_summary(
                face.power.as_deref(),
                face.toughness.as_deref(),
                face.loyalty.as_deref(),
                face.defense.as_deref(),
            ),
            flavor_text: face.flavor_text.clone(),
        }
    }
}

/// Condense power/toughness, loyalty, or defense into a single labeled stat
fn stats_summary(
    power: Option<&str>,
    toughness: Option<&str>,
    loyalty: Option<&str>,
    defense: Option<&str>,
) -> Option<(&'static str, String)> {
    if let (Some(power), Some(toughness)) = (power, toughness) {
        Some(("Power / Toughness", format!("{power}/{toughness}")))
    } else if let Some(loyalty) = loyalty {
        Some(("Loyalty", loyalty.to_string()))
    } else {
        defense.map(|defense| ("Defense", defense.to_string()))
    }
}

/// Formats in the order they should appear in the legality grid
const LEGALITY_FORMATS: &[&str] = &[
    "Standard",
    "Alchemy",
    "Historic",
    "Timeless",
    "Brawl",
    "Standard Brawl",
    "Gladiator",
    "Pioneer",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CardSearchResult {
    pub id: i64,
    pub name: String,
    pub set: String,
    pub set_name: String,
    pub type_line: String,
    pub mana_cost: String,
    pub mana_value: i32,
    pub image_uri: String,
    pub colors: Vec<String>,
    pub color_identity: Vec<String>,
    pub layout: String,
    pub faces: Vec<CardFaceSummary>,
    pub oracle_text: String,
    pub stats: Option<(&'static str, String)>,
    pub keywords: Vec<String>,
    pub rarity: String,
    pub collector_number: String,
    pub artist: String,
    pub flavor_text: String,
    /// (format label, legality status) pairs in `LEGALITY_FORMATS` order;
    /// empty when the card predates legality scraping
    pub legalities: Vec<(&'static str, String)>,
    pub edhrec_rank: Option<i32>,
    pub scryfall_uri: String,
}

impl CardSearchResult {
    pub fn cost(&self) -> Cost {
        self.mana_cost.parse().unwrap_or_default()
    }
}

impl From<&Card> for CardSearchResult {
    fn from(card: &Card) -> Self {
        let legalities = card.legalities.as_ref().map_or_else(Vec::new, |l| {
            LEGALITY_FORMATS
                .iter()
                .zip([
                    &l.standard,
                    &l.alchemy,
                    &l.historic,
                    &l.timeless,
                    &l.brawl,
                    &l.standard_brawl,
                    &l.gladiator,
                    &l.pioneer,
                ])
                .map(|(format, status)| (*format, status.clone()))
                .collect()
        });

        Self {
            id: card.id,
            name: card.name.clone(),
            set: card.set.clone(),
            set_name: card.set_name.clone(),
            type_line: card.type_line.clone(),
            mana_cost: card.mana_cost.clone(),
            mana_value: card.cmc,
            image_uri: card.primary_image_uri().unwrap_or_default().to_string(),
            colors: card.colors.clone(),
            color_identity: card.color_identity.clone(),
            layout: card.layout.clone(),
            faces: card.card_faces.iter().map(CardFaceSummary::from).collect(),
            oracle_text: card.oracle_text.clone(),
            stats: stats_summary(
                card.power.as_deref(),
                card.toughness.as_deref(),
                card.loyalty.as_deref(),
                card.defense.as_deref(),
            ),
            keywords: card.keywords.clone(),
            rarity: card.rarity.clone(),
            collector_number: card.collector_number.clone(),
            artist: card.artist.clone(),
            flavor_text: card.flavor_text.clone(),
            legalities,
            edhrec_rank: card.edhrec_rank,
            scryfall_uri: card.scryfall_uri.clone(),
        }
    }
}

/// Filters for the card database search; empty fields are ignored
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CardSearchFilters {
    /// Case-insensitive name prefix
    pub name: String,
    /// Case-insensitive substring matched against oracle text (any face)
    pub text: String,
    /// Exact set code
    pub set: String,
    /// Exact rarity (e.g. "common", "mythic")
    pub rarity: String,
    /// Dominant card type (e.g. "Creature"), matched via `CardType`
    pub card_type: String,
}

impl CardSearchFilters {
    pub fn is_active(&self) -> bool {
        !self.name.trim().is_empty()
            || !self.text.trim().is_empty()
            || !self.set.trim().is_empty()
            || !self.rarity.trim().is_empty()
            || !self.card_type.trim().is_empty()
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

        let (mtga_match, result) = self.db.get_match(&id, None).await?;

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

    pub fn get_card_database_summary(&self) -> CardDatabaseSummary {
        card_database_summary(&self.cards)
    }

    pub fn search_cards(&self, filters: &CardSearchFilters) -> Vec<CardSearchResult> {
        search_cards(&self.cards, filters)
    }

    pub fn get_card_by_arena_id(&self, arena_id: i64) -> Option<CardSearchResult> {
        self.cards.get(&arena_id.to_string()).map(CardSearchResult::from)
    }

    pub fn get_card_json(&self, arena_id: i64) -> Result<Option<String>> {
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

fn normalize_filter(filter: &str) -> Option<String> {
    let trimmed = filter.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_lowercase())
    }
}

fn matches_oracle_text(card: &Card, text: &str) -> bool {
    card.oracle_text.to_lowercase().contains(text)
        || card
            .card_faces
            .iter()
            .any(|face| face.oracle_text.to_lowercase().contains(text))
}

fn search_cards(cards: &CardsDatabase, filters: &CardSearchFilters) -> Vec<CardSearchResult> {
    let name_query = filters.name.trim().to_lowercase();
    let text_query = normalize_filter(&filters.text);
    let set_query = normalize_filter(&filters.set);
    let rarity_query = normalize_filter(&filters.rarity);
    let type_query = filters.card_type.trim().parse::<CardType>().ok();

    let mut matches: Vec<_> = cards
        .values()
        .filter(|card| {
            let matches_name = name_query.is_empty() || card.name.to_lowercase().starts_with(&name_query);
            let matches_text = text_query.as_deref().is_none_or(|text| matches_oracle_text(card, text));
            let matches_set = set_query.as_deref().is_none_or(|set| card.set.to_lowercase() == set);
            let matches_rarity = rarity_query
                .as_deref()
                .is_none_or(|rarity| card.rarity.to_lowercase() == rarity);
            let matches_type = type_query.is_none_or(|card_type| card.dominant_type() == card_type);

            matches_name && matches_text && matches_set && matches_rarity && matches_type
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

    fn name_filter(name: &str) -> CardSearchFilters {
        CardSearchFilters {
            name: name.to_string(),
            ..CardSearchFilters::default()
        }
    }

    #[test]
    fn searches_name_prefix_case_insensitively() {
        let cards = test_database(vec![
            test_card(1, "BRO", "Opt"),
            test_card(2, "BRO", "Omenpath Journey"),
            test_card(3, "TDM", "Lightning Strike"),
        ]);

        let matches = search_cards(&cards, &name_filter("om"));

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

        let filters = CardSearchFilters {
            set: "tdm".to_string(),
            ..CardSearchFilters::default()
        };
        let matches = search_cards(&cards, &filters);

        assert_eq!(matches.len(), 2);
        assert!(matches.iter().all(|card| card.set == "TDM"));
    }

    #[test]
    fn filters_by_oracle_text_substring() {
        let mut bolt = test_card(1, "BRO", "Lightning Strike");
        bolt.oracle_text = "Lightning Strike deals 3 damage to any target.".to_string();
        let mut counter = test_card(2, "BRO", "Cancel");
        counter.oracle_text = "Counter target spell.".to_string();
        let mut mdfc = test_card(3, "BRO", "Rise // Fall");
        mdfc.card_faces = vec![
            arenabuddy_core::models::CardFace {
                name: "Rise".to_string(),
                oracle_text: "Draw a card, then deal 3 damage to any target.".to_string(),
                ..Default::default()
            },
            arenabuddy_core::models::CardFace {
                name: "Fall".to_string(),
                oracle_text: "Destroy target land.".to_string(),
                ..Default::default()
            },
        ];
        let cards = test_database(vec![bolt, counter, mdfc]);

        let filters = CardSearchFilters {
            text: "3 DAMAGE".to_string(),
            ..CardSearchFilters::default()
        };
        let matches = search_cards(&cards, &filters);

        assert_eq!(matches.len(), 2);
        assert!(matches.iter().any(|card| card.name == "Lightning Strike"));
        assert!(matches.iter().any(|card| card.name == "Rise // Fall"));
    }

    #[test]
    fn filters_by_rarity_and_type() {
        let mut creature = test_card(1, "BRO", "Grizzly Bears");
        creature.type_line = "Creature — Bear".to_string();
        creature.rarity = "common".to_string();
        let mut mythic_creature = test_card(2, "BRO", "Sheoldred");
        mythic_creature.type_line = "Legendary Creature — Phyrexian Praetor".to_string();
        mythic_creature.rarity = "mythic".to_string();
        let mut instant = test_card(3, "BRO", "Opt");
        instant.rarity = "common".to_string();
        let cards = test_database(vec![creature, mythic_creature, instant]);

        let filters = CardSearchFilters {
            card_type: "Creature".to_string(),
            ..CardSearchFilters::default()
        };
        let matches = search_cards(&cards, &filters);
        assert_eq!(matches.len(), 2);

        let filters = CardSearchFilters {
            card_type: "Creature".to_string(),
            rarity: "mythic".to_string(),
            ..CardSearchFilters::default()
        };
        let matches = search_cards(&cards, &filters);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Sheoldred");
    }

    #[test]
    fn search_result_carries_enriched_fields() {
        let mut card = test_card(1, "BRO", "Grizzly Bears");
        card.oracle_text = "A bear.".to_string();
        card.power = Some("2".to_string());
        card.toughness = Some("2".to_string());
        card.rarity = "common".to_string();
        card.keywords = vec!["Trample".to_string()];
        card.legalities = Some(arenabuddy_core::models::Legalities {
            standard: "legal".to_string(),
            ..Default::default()
        });
        let cards = test_database(vec![card]);

        let matches = search_cards(&cards, &name_filter("grizzly"));

        assert_eq!(matches.len(), 1);
        let result = &matches[0];
        assert_eq!(result.oracle_text, "A bear.");
        assert_eq!(result.stats, Some(("Power / Toughness", "2/2".to_string())));
        assert_eq!(result.rarity, "common");
        assert_eq!(result.keywords, ["Trample"]);
        assert_eq!(result.legalities[0], ("Standard", "legal".to_string()));
        assert_eq!(result.legalities.len(), 8);
    }
}
