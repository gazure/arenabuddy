#![expect(dead_code)]
#![expect(clippy::similar_names)]
use std::fmt::Display;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::ArenaId;

#[derive(Debug, Default, Clone)]
pub struct MTGADraft {
    draft: Draft,
    packs: Vec<DraftPack>,
}

impl MTGADraft {
    pub fn new(draft: Draft, packs: Vec<DraftPack>) -> Self {
        Self { draft, packs }
    }

    pub fn draft(&self) -> &Draft {
        &self.draft
    }

    pub fn packs(&self) -> &[DraftPack] {
        &self.packs
    }

    fn filter(&self, pack_num: u8) -> Vec<&DraftPack> {
        self.packs().iter().filter(|p| p.pack_number() == pack_num).collect()
    }

    pub fn by_packs(&self) -> Vec<Vec<&DraftPack>> {
        vec![self.first(), self.second(), self.third()]
    }

    pub fn first(&self) -> Vec<&DraftPack> {
        self.filter(1)
    }

    pub fn second(&self) -> Vec<&DraftPack> {
        self.filter(2)
    }

    pub fn third(&self) -> Vec<&DraftPack> {
        self.filter(3)
    }

    pub fn cards(&self) -> impl Iterator<Item = ArenaId> {
        self.packs().iter().flat_map(DraftPack::cards).copied()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Draft {
    id: Uuid,
    set_code: String,
    format: Format,
    status: String,
    created_at: DateTime<Utc>,
}

impl Draft {
    pub fn new(id: Uuid, set_code: String, format: Format, status: String) -> Self {
        Self {
            id,
            set_code,
            format,
            status,
            created_at: Utc::now(),
        }
    }

    #[must_use]
    pub fn with_created_at(mut self, and_utc: DateTime<Utc>) -> Draft {
        self.created_at = and_utc;
        self
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn set_code(&self) -> &str {
        &self.set_code
    }

    pub fn format(&self) -> Format {
        self.format
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }
}

impl PartialEq for Draft {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Clone)]
pub struct DraftPack {
    id: u64,
    draft_id: Uuid,
    pack_number: u8,
    pick_number: u8,
    selection_number: u8,
    card_id: ArenaId,
    cards: Vec<ArenaId>,
    created_at: DateTime<Utc>,
}

impl DraftPack {
    pub fn new(
        draft_id: Uuid,
        pack_number: u8,
        pick_number: u8,
        selection_number: u8,
        card_id: ArenaId,
        cards: Vec<ArenaId>,
    ) -> Self {
        Self {
            id: 0,
            draft_id,
            pack_number,
            pick_number,
            selection_number,
            card_id,
            cards,
            created_at: Utc::now(),
        }
    }

    #[must_use]
    pub fn with_id(mut self, id: u64) -> Self {
        self.id = id;
        self
    }

    pub fn pack_number(&self) -> u8 {
        self.pack_number
    }

    pub fn pick_number(&self) -> u8 {
        self.pick_number
    }

    pub fn selection_number(&self) -> u8 {
        self.selection_number
    }

    pub fn picked_card(&self) -> ArenaId {
        self.card_id
    }

    pub fn cards(&self) -> &[ArenaId] {
        &self.cards
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Format {
    PickTwoDraft,
    PremierDraft,
    QuickDraft,
    #[default]
    TraditionalDraft,
    Sealed,
    Other,
}

impl Format {
    /// Parse a format string into the Format enum. Infallible
    #[expect(clippy::should_implement_trait)]
    pub fn from_str(s: impl AsRef<str>) -> Self {
        match s.as_ref() {
            "PickTwoDraft" => Format::PickTwoDraft,
            "PremierDraft" => Format::PremierDraft,
            "QuickDraft" => Format::QuickDraft,
            "TraditionalDraft" => Format::TraditionalDraft,
            "Sealed" => Format::Sealed,
            _ => Format::Other,
        }
    }

    /// Get the string representation of the format
    pub fn as_str(&self) -> &str {
        match self {
            Format::PickTwoDraft => "PickTwoDraft",
            Format::PremierDraft => "PremierDraft",
            Format::QuickDraft => "QuickDraft",
            Format::TraditionalDraft => "TraditionalDraft",
            Format::Sealed => "Sealed",
            Format::Other => "Other",
        }
    }
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
