//! Draft display module providing enriched draft data with full card details
//!
//! This module provides a `DraftDetailsDisplay` that enriches raw `MTGADraft` data
//! with complete card information from the `CardsDatabase`, allowing frontends to
//! simply iterate through packs without needing to perform card lookups.
//!
//! # Example Usage
//!
//! ```ignore
//! // In the backend service
//! let draft = db.get_draft(&draft_id).await?;
//! let display = DraftDetailsDisplay::new(draft, &cards_database);
//!
//! // In the frontend - no card lookups needed!
//! for pack in display.packs() {
//!     println!("Pack {}, Pick {}", pack.pack_number, pack.pick_number);
//!
//!     if let Some(picked) = &pack.picked_card {
//!         println!("  Picked: {}", picked.name());
//!     }
//!
//!     println!("  Available cards:");
//!     for card in &pack.available_cards {
//!         println!("    - {}", card.name());
//!     }
//! }
//! ```

use crate::{
    cards::CardsDatabase,
    display::card::CardDisplayRecord,
    models::{Draft, DraftPack, MTGADraft},
};

/// Represents a draft pack with enriched card details
#[derive(Debug, Default, Clone)]
pub struct EnrichedDraftPack {
    pack_number: u8,
    pick_number: u8,
    selection_nubmer: u8,
    picked_card: Option<CardDisplayRecord>,
    available_cards: Vec<CardDisplayRecord>,
}

impl PartialEq for EnrichedDraftPack {
    fn eq(&self, other: &Self) -> bool {
        // Compare pack/pick numbers and card counts since Card doesn't implement PartialEq
        self.pack_number == other.pack_number
            && self.pick_number == other.pick_number
            && self.selection_nubmer == other.selection_nubmer
            && self.picked_card.is_some() == other.picked_card.is_some()
            && self.available_cards.len() == other.available_cards.len()
    }
}

impl EnrichedDraftPack {
    /// Creates a new enriched draft pack from a regular draft pack and cards database
    pub fn from_draft_pack(pack: &DraftPack, cards: &CardsDatabase) -> Self {
        let picked_card = cards.get(&pack.picked_card()).map(CardDisplayRecord::from);
        let available_cards = pack
            .cards()
            .iter()
            .filter_map(|id| cards.get(id).map(CardDisplayRecord::from))
            .collect();

        Self {
            pack_number: pack.pack_number(),
            pick_number: pack.pick_number(),
            selection_nubmer: pack.selection_number(),
            picked_card,
            available_cards,
        }
    }

    /// Returns the pack number
    pub fn pack_number(&self) -> u8 {
        self.pack_number
    }

    /// Returns the pick number
    pub fn pick_number(&self) -> u8 {
        self.pick_number
    }

    /// Returns the name of the picked card, or None if no card was picked
    pub fn picked_card_name(&self) -> Option<&str> {
        self.picked_card.as_ref().map(|c| c.name.as_str())
    }

    /// Returns true if a card was picked
    pub fn has_picked_card(&self) -> bool {
        self.picked_card.is_some()
    }

    /// Returns the number of available cards in this pack
    pub fn available_count(&self) -> usize {
        self.available_cards.len()
    }

    /// Returns the names of all available cards
    pub fn available_card_names(&self) -> Vec<&str> {
        self.available_cards.iter().map(|c| c.name.as_str()).collect()
    }

    /// Returns the picked card if it exists
    pub fn picked(&self) -> Option<&CardDisplayRecord> {
        self.picked_card.as_ref()
    }

    /// Returns all available cards
    pub fn available(&self) -> &[CardDisplayRecord] {
        &self.available_cards
    }

    /// Finds a card by name in the available cards
    pub fn find_available_by_name(&self, name: &str) -> Option<&CardDisplayRecord> {
        self.available_cards.iter().find(|c| c.name == name)
    }
}

/// Display representation of a draft with enriched card details
///
/// This struct provides a convenient interface for displaying draft information
/// without requiring the frontend to perform card lookups. All card data is
/// pre-fetched and included in the enriched packs.
#[derive(Debug, Default, Clone)]
pub struct DraftDetailsDisplay {
    draft: MTGADraft,
    enriched_packs: Vec<EnrichedDraftPack>,
}

impl PartialEq for DraftDetailsDisplay {
    fn eq(&self, other: &Self) -> bool {
        // Compare only the draft ID and enriched pack count
        self.draft.draft().id() == other.draft.draft().id() && self.enriched_packs.len() == other.enriched_packs.len()
    }
}

impl DraftDetailsDisplay {
    pub fn new(draft: MTGADraft, cards_db: &CardsDatabase) -> Self {
        // Create enriched packs with full card details
        let enriched_packs = draft
            .packs()
            .iter()
            .map(|pack| EnrichedDraftPack::from_draft_pack(pack, cards_db))
            .collect();

        DraftDetailsDisplay { draft, enriched_packs }
    }

    /// Returns the underlying `MTGADraft`
    pub fn draft(&self) -> &MTGADraft {
        &self.draft
    }

    /// Returns all enriched packs with full card details
    pub fn packs(&self) -> &[EnrichedDraftPack] {
        &self.enriched_packs
    }

    /// Returns enriched packs grouped by pack number
    pub fn by_packs(&self) -> Vec<Vec<&EnrichedDraftPack>> {
        vec![self.first(), self.second(), self.third()]
    }

    /// Returns enriched packs from the first pack
    pub fn first(&self) -> Vec<&EnrichedDraftPack> {
        self.enriched_packs.iter().filter(|p| p.pack_number == 1).collect()
    }

    /// Returns enriched packs from the second pack
    pub fn second(&self) -> Vec<&EnrichedDraftPack> {
        self.enriched_packs.iter().filter(|p| p.pack_number == 2).collect()
    }

    /// Returns enriched packs from the third pack
    pub fn third(&self) -> Vec<&EnrichedDraftPack> {
        self.enriched_packs.iter().filter(|p| p.pack_number == 3).collect()
    }

    /// Returns the draft metadata
    pub fn metadata(&self) -> &Draft {
        self.draft.draft()
    }

    /// Returns the total number of picks made
    pub fn total_picks(&self) -> usize {
        self.enriched_packs.len()
    }

    /// Returns all picked cards in order
    pub fn picked_cards(&self) -> Vec<&CardDisplayRecord> {
        self.enriched_packs
            .iter()
            .filter_map(|pack| pack.picked_card.as_ref())
            .collect()
    }

    /// Returns picks for a specific pack number
    pub fn picks_for_pack(&self, pack_number: u8) -> Vec<&EnrichedDraftPack> {
        self.enriched_packs
            .iter()
            .filter(|p| p.pack_number == pack_number)
            .collect()
    }
}
