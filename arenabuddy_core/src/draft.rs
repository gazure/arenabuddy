#![expect(dead_code)]
#![expect(clippy::similar_names)]
use derive_builder::Builder;

use crate::{
    models::Draft,
    mtga_events::{
        business::{BusinessEvent, DraftPackInfoEvent},
        draft::RequestTypeDraftNotify,
        primitives::ArenaId,
    },
};

#[derive(Debug, Builder)]
pub struct MTGADraft {
    draft: Draft,
    packs: Vec<(RawPack, RawPick)>,
}

#[derive(Debug, Clone)]
struct RawPack {
    pack_number: u8,
    pick_number: u8,
    cards: Vec<ArenaId>,
}

#[derive(Debug, Clone)]
struct RawPick {
    card_id: ArenaId,
    pick_time_seconds: f64,
}

pub struct DraftBuilder {
    builder: MTGADraftBuilder,
}

impl Default for DraftBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DraftBuilder {
    pub fn new() -> Self {
        Self {
            builder: MTGADraftBuilder::default(),
        }
    }

    // not sure if this is needed yet
    pub fn process_notify_event(&mut self, _event: &RequestTypeDraftNotify) {}

    pub fn process_business_event(&mut self, event: &BusinessEvent) {
        if let BusinessEvent::Draft(e) = event {
            self.process_pack_event(e);
        }
    }

    fn process_pack_event(&mut self, draft_pack_info_event: &DraftPackInfoEvent) {
        let pick = RawPick {
            card_id: draft_pack_info_event.pick_grp_id,
            pick_time_seconds: draft_pack_info_event.time_remaining_on_pick,
        };

        let pack = RawPack {
            cards: draft_pack_info_event.cards_in_pack.clone(),
            pack_number: draft_pack_info_event.pack_number,
            pick_number: draft_pack_info_event.pick_number,
        };

        tracingx::info!("Pack #{}, Pick #{}", pack.pack_number, pack.pick_number);
        if let Some(packs) = self.builder.packs.as_mut() {
            packs.push((pack, pick));
        } else {
            self.builder.packs = Some(vec![(pack, pick)]);
        }
    }
}
