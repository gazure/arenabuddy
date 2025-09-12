#![expect(clippy::similar_names)]
use uuid::Uuid;

use crate::{
    Error, Result,
    events::{
        business::{BusinessEvent, DraftPackInfoEvent},
        primitives::ArenaId,
    },
    ingest::DraftWriter,
    models::{Draft, DraftPack, MTGADraft},
};

#[derive(Debug, Clone)]
struct RawPack {
    pack_number: u8,
    pick_number: u8,
    card_id: ArenaId,
    cards: Vec<ArenaId>,
}

#[derive(Debug)]
struct PackPick(u8, u8);

impl PackPick {
    pub fn is_last(&self) -> bool {
        self.0 == 3 && self.1 == 13
    }
}

#[derive(Default)]
pub struct DraftBuilder {
    draft_id: Option<Uuid>,
    event_id: Option<String>,
    packs: Vec<RawPack>,

    writers: Vec<Box<dyn DraftWriter>>,
}

impl DraftBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Consumes a business event and extracts relevant draft information
    /// If a draft is finished (i.e. after pack3-pick13) results will be written to any
    /// configured writers
    ///
    /// # Errors
    /// errors if there is an issue writing the draft results to storage
    pub async fn process_business_event(&mut self, event: &BusinessEvent) -> Result<()> {
        if let BusinessEvent::Draft(e) = event {
            let pp = self.process_pack_event(e);
            self.write_draft(pp).await?;
        }
        Ok(())
    }

    fn process_pack_event(&mut self, draft_pack_info_event: &DraftPackInfoEvent) -> PackPick {
        self.draft_id = draft_pack_info_event.draft_id.parse::<Uuid>().ok();
        self.event_id = Some(draft_pack_info_event.event_id.clone());

        let pack = RawPack {
            cards: draft_pack_info_event.cards_in_pack.clone(),
            card_id: draft_pack_info_event.pick_grp_id,
            pack_number: draft_pack_info_event.pack_number,
            pick_number: draft_pack_info_event.pick_number,
        };

        tracingx::info!("Pack #{}, Pick #{}", pack.pack_number, pack.pick_number);
        let last_pack = pack.pack_number;
        let last_pick = pack.pick_number;
        self.packs.push(pack);
        PackPick(last_pack, last_pick)
    }

    async fn write_draft(&mut self, pp: PackPick) -> Result<()> {
        if !pp.is_last() {
            return Ok(());
        }

        if let (Some(draft_id), Some(event_id)) = (self.draft_id, &self.event_id) {
            let (format, set_code) = parse_event_id(event_id);
            let draft = Draft::new(draft_id, set_code, format, String::new());
            let packs: Vec<_> = self
                .packs
                .iter()
                .map(|pack| {
                    DraftPack::new(
                        draft_id,
                        pack.pack_number,
                        pack.pick_number,
                        pack.card_id,
                        pack.cards.clone(),
                    )
                })
                .collect();

            let mtga_draft = MTGADraft::new(draft, packs);

            for writer in &mut self.writers {
                writer.write(&mtga_draft).await?;
            }
            self.reset();
            return Ok(());
        }
        Err(Error::Io("can't locate draft_id or event_id".to_string()))
    }

    pub fn add_writer(&mut self, writer: Box<dyn DraftWriter>) {
        self.writers.push(writer);
    }

    fn reset(&mut self) {
        self.packs.clear();
        self.draft_id = None;
        self.event_id = None;
    }
}

/// returns draft format and set code if found
fn parse_event_id(event_id: &str) -> (String, String) {
    let parts: Vec<_> = event_id.split('_').collect();
    if parts.len() != 3 {
        return (String::new(), String::new());
    }

    (parts[0].to_string(), parts[1].to_string())
}
