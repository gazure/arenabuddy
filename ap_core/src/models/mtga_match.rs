use chrono::{DateTime, Utc};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Builder)]
pub struct MTGAMatch {
    pub id: String,
    pub controller_seat_id: i32,
    pub controller_player_name: String,
    pub opponent_player_name: String,
    pub created_at: DateTime<Utc>,
}
