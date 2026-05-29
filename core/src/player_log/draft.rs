#![expect(clippy::similar_names)]
use uuid::Uuid;

use crate::{
    Error, Result,
    events::business::{BusinessEvent, DraftPackInfoEvent},
    models::{ArenaId, Draft, DraftPack, Format, MTGADraft},
    multimap::MultiMap,
    player_log::ingest::DraftWriter,
};

#[derive(Debug, Clone)]
struct RawPack {
    pack_number: u8,
    pick_number: u8,
    card_id: ArenaId,
    cards: Vec<ArenaId>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PackPick(u8, u8);

#[derive(Default)]
pub struct DraftBuilder {
    draft_id: Option<Uuid>,
    event_id: Option<String>,
    packs: MultiMap<PackPick, RawPack>,

    writers: Vec<Box<dyn DraftWriter>>,
}

impl DraftBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a writer to the draft builder
    pub fn add_writer(&mut self, writer: Box<dyn DraftWriter>) {
        self.writers.push(writer);
    }

    /// Consumes a business event and extracts relevant draft information
    /// If a draft is finished (i.e. after pack3-pick13) results will be written to any
    /// configured writers
    ///
    /// # Errors
    /// errors if there is an issue writing the draft results to storage
    pub async fn process_event(&mut self, event: &BusinessEvent) -> Result<()> {
        if let BusinessEvent::Draft(e) = event {
            tracing::debug!("Processing draft event: {e:?}");
            let format = parse_event_id(&e.event_id).0;
            self.process_pack_event(e);

            if self.finish_draft(format) {
                self.write_draft().await?;
            }
        }
        Ok(())
    }

    fn process_pack_event(&mut self, draft_pack_info_event: &DraftPackInfoEvent) {
        self.draft_id = draft_pack_info_event.draft_id.parse::<Uuid>().ok();
        self.event_id = Some(draft_pack_info_event.event_id.clone());

        let pack = RawPack {
            cards: draft_pack_info_event.cards_in_pack.clone(),
            card_id: draft_pack_info_event.pick_grp_id,
            pack_number: draft_pack_info_event.pack_number,
            pick_number: draft_pack_info_event.pick_number,
        };

        tracing::info!("Pack #{}, Pick #{}", pack.pack_number, pack.pick_number);
        let last_pack = pack.pack_number;
        let last_pick = pack.pick_number;
        let pp = PackPick(last_pack, last_pick);
        self.packs.entry(pp).or_default().push(pack);
    }

    async fn write_draft(&mut self) -> Result<()> {
        if let (Some(draft_id), Some(event_id)) = (self.draft_id, &self.event_id) {
            let (format, set_code) = parse_event_id(event_id);
            let draft = Draft::new(draft_id, set_code, format, String::new());
            let packs: Vec<_> = self
                .packs
                .vec_values()
                .flat_map(|pp| {
                    pp.iter().enumerate().map(|(selection_num, pack)| {
                        DraftPack::new(
                            draft_id,
                            pack.pack_number,
                            pack.pick_number,
                            selection_num.try_into().unwrap_or_else(|e| {
                                tracing::warn!(
                                    "Could not identify selection number for PackPick: {pack:?}. error: {e}"
                                );
                                0u8
                            }),
                            pack.card_id,
                            pack.cards.clone(),
                        )
                    })
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

    fn reset(&mut self) {
        self.packs.clear();
        self.draft_id = None;
        self.event_id = None;
    }

    fn finish_draft(&self, format: Format) -> bool {
        match format {
            Format::PickTwoDraft => self.packs.get_all(&PackPick(3, 7)).is_some_and(|ps| ps.len() == 2),
            _ => self.packs.get_all(&PackPick(3, 13)).is_some_and(|ps| !ps.is_empty()),
        }
    }
}

/// returns draft format and set code if found
fn parse_event_id(event_id: &str) -> (Format, String) {
    let parts: Vec<_> = event_id.split('_').collect();
    if parts.len() != 3 {
        return (Format::default(), String::default());
    }

    (Format::parse_format(parts[0]), parts[1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Format;

    #[test]
    fn parse_event_id_premier_draft() {
        let (format, set_code) = parse_event_id("PremierDraft_FDN_20250101");
        assert_eq!(format, Format::PremierDraft);
        assert_eq!(set_code, "FDN");
    }

    #[test]
    fn parse_event_id_quick_draft() {
        let (format, set_code) = parse_event_id("QuickDraft_DSK_20241015");
        assert_eq!(format, Format::QuickDraft);
        assert_eq!(set_code, "DSK");
    }

    #[test]
    fn parse_event_id_traditional_draft() {
        let (format, set_code) = parse_event_id("TraditionalDraft_MKM_20240301");
        assert_eq!(format, Format::TraditionalDraft);
        assert_eq!(set_code, "MKM");
    }

    #[test]
    fn parse_event_id_pick_two_draft() {
        let (format, set_code) = parse_event_id("PickTwoDraft_BLB_20240801");
        assert_eq!(format, Format::PickTwoDraft);
        assert_eq!(set_code, "BLB");
    }

    #[test]
    fn parse_event_id_too_few_parts() {
        let (format, set_code) = parse_event_id("PremierDraft_FDN");
        assert_eq!(format, Format::TraditionalDraft); // default
        assert_eq!(set_code, "");
    }

    #[test]
    fn parse_event_id_too_many_parts() {
        let (format, set_code) = parse_event_id("a_b_c_d");
        assert_eq!(format, Format::TraditionalDraft); // default
        assert_eq!(set_code, "");
    }

    #[test]
    fn parse_event_id_empty() {
        let (format, set_code) = parse_event_id("");
        assert_eq!(format, Format::TraditionalDraft); // default
        assert_eq!(set_code, "");
    }

    #[test]
    fn parse_event_id_unknown_format() {
        let (format, set_code) = parse_event_id("CubeDraft_VOW_20220101");
        assert_eq!(format, Format::Other);
        assert_eq!(set_code, "VOW");
    }

    #[test]
    fn finish_draft_regular_format_needs_pack3_pick13() {
        let mut builder = DraftBuilder::new();
        assert!(!builder.finish_draft(Format::PremierDraft));

        let pp = PackPick(3, 13);
        builder.packs.entry(pp).or_default().push(RawPack {
            pack_number: 3,
            pick_number: 13,
            card_id: ArenaId::new(1),
            cards: vec![],
        });
        assert!(builder.finish_draft(Format::PremierDraft));
    }

    #[test]
    fn finish_draft_pick_two_needs_pack3_pick7_twice() {
        let mut builder = DraftBuilder::new();
        let pp = PackPick(3, 7);

        builder.packs.entry(pp).or_default().push(RawPack {
            pack_number: 3,
            pick_number: 7,
            card_id: ArenaId::new(1),
            cards: vec![],
        });
        assert!(!builder.finish_draft(Format::PickTwoDraft), "need 2 picks");

        builder.packs.entry(pp).or_default().push(RawPack {
            pack_number: 3,
            pick_number: 7,
            card_id: ArenaId::new(2),
            cards: vec![],
        });
        assert!(builder.finish_draft(Format::PickTwoDraft));
    }

    #[test]
    fn finish_draft_wrong_pack_pick_returns_false() {
        let mut builder = DraftBuilder::new();
        let pp = PackPick(2, 13);
        builder.packs.entry(pp).or_default().push(RawPack {
            pack_number: 2,
            pick_number: 13,
            card_id: ArenaId::new(1),
            cards: vec![],
        });
        assert!(!builder.finish_draft(Format::PremierDraft));
    }
}
