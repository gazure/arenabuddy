#![expect(dead_code)]
#![expect(clippy::similar_names)]
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::mtga_events::primitives::ArenaId;

#[derive(Debug, Clone)]
pub struct Draft {
    id: Uuid,
    set_code: String,
    format: String,
    status: String,
    created_at: DateTime<Utc>,
}

impl Draft {
    pub fn new(id: Uuid, set_code: String, format: String, status: String, created_at: DateTime<Utc>) -> Self {
        Self {
            id,
            set_code,
            format,
            status,
            created_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DraftPack {
    id: u64,
    draft_id: Uuid,
    pack_number: u8,
    pick_number: u8,
    cards: Vec<ArenaId>,
    created_at: DateTime<Utc>,
}

impl DraftPack {
    pub fn new(
        id: u64,
        draft_id: Uuid,
        pack_number: u8,
        pick_number: u8,
        cards: Vec<ArenaId>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            draft_id,
            pack_number,
            pick_number,
            cards,
            created_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DraftPick {
    id: u64,
    draft_pack_id: u64,
    card_id: ArenaId,
    pick_time_seconds: f64,
    created_at: DateTime<Utc>,
}

impl DraftPick {
    pub fn new(
        id: u64,
        draft_pack_id: u64,
        card_id: ArenaId,
        pick_time_seconds: f64,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            draft_pack_id,
            card_id,
            pick_time_seconds,
            created_at,
        }
    }
}
