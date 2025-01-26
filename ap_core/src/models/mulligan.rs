use derive_builder::Builder;

#[derive(Debug, Clone, Builder)]
pub struct MulliganInfo {
    pub match_id: String,
    pub game_number: i32,
    pub number_to_keep: i32,
    pub hand: String,
    pub play_draw: String,
    pub opponent_identity: String,
    pub decision: String,
}
