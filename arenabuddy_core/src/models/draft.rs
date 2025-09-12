#![expect(dead_code)]
#![expect(clippy::similar_names)]
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::events::primitives::ArenaId;

#[derive(Debug)]
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
}

#[derive(Debug, Clone)]
pub struct Draft {
    id: Uuid,
    set_code: String,
    format: String,
    status: String,
    created_at: DateTime<Utc>,
}

impl Draft {
    pub fn new(id: Uuid, set_code: String, format: String, status: String) -> Self {
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

    pub fn format(&self) -> &str {
        &self.format
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
    card_id: ArenaId,
    cards: Vec<ArenaId>,
    created_at: DateTime<Utc>,
}

impl DraftPack {
    pub fn new(draft_id: Uuid, pack_number: u8, pick_number: u8, card_id: ArenaId, cards: Vec<ArenaId>) -> Self {
        Self {
            id: 0,
            draft_id,
            pack_number,
            pick_number,
            card_id,
            cards,
            created_at: Utc::now(),
        }
    }

    pub fn pack_number(&self) -> u8 {
        self.pack_number
    }

    pub fn pick_number(&self) -> u8 {
        self.pick_number
    }

    pub fn picked_card(&self) -> ArenaId {
        self.card_id
    }

    pub fn cards(&self) -> &[ArenaId] {
        &self.cards
    }
}
