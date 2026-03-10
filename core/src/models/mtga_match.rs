use chrono::{DateTime, Utc};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

/// Represents a match in Magic: The Gathering Arena
#[derive(Debug, Default, Clone, Serialize, Deserialize, Builder, PartialEq)]
#[builder(setter(into))]
pub struct MTGAMatch {
    id: String,
    controller_seat_id: i32,
    controller_player_name: String,
    opponent_player_name: String,
    created_at: DateTime<Utc>,
    #[builder(default)]
    format: Option<String>,
}

impl MTGAMatch {
    pub fn new(
        id: impl Into<String>,
        controller_seat_id: i32,
        controller_player_name: impl Into<String>,
        opponent_player_name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            controller_seat_id,
            controller_player_name: controller_player_name.into(),
            opponent_player_name: opponent_player_name.into(),
            created_at: Utc::now(),
            format: None,
        }
    }

    pub fn new_with_timestamp(
        id: impl Into<String>,
        controller_seat_id: i32,
        controller_player_name: impl Into<String>,
        opponent_player_name: impl Into<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.into(),
            controller_seat_id,
            controller_player_name: controller_player_name.into(),
            opponent_player_name: opponent_player_name.into(),
            created_at,
            format: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn controller_seat_id(&self) -> i32 {
        self.controller_seat_id
    }

    pub fn controller_player_name(&self) -> &str {
        &self.controller_player_name
    }

    pub fn opponent_player_name(&self) -> &str {
        &self.opponent_player_name
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn format(&self) -> Option<&str> {
        self.format.as_deref()
    }

    pub fn is_controller(&self, seat_id: i32) -> bool {
        self.controller_seat_id == seat_id
    }

    pub fn player_name_for_seat(&self, seat_id: i32) -> &str {
        if self.is_controller(seat_id) {
            &self.controller_player_name
        } else {
            &self.opponent_player_name
        }
    }
}
