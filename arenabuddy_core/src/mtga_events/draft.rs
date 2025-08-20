use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RequestTypeDraftNotify {
    #[serde(rename = "draftId")]
    pub draft_id: String,
    pub self_pick: u8,
    pub self_pack: u8,
    pub pack_cards: String,
}
