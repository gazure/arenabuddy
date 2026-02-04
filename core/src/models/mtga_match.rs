use chrono::{DateTime, Utc};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

use crate::proto;

/// Represents a match in Magic: The Gathering Arena
#[derive(Debug, Default, Clone, Serialize, Deserialize, Builder, PartialEq)]
#[builder(setter(into))]
pub struct MTGAMatch {
    id: String,
    controller_seat_id: i32,
    controller_player_name: String,
    opponent_player_name: String,
    created_at: DateTime<Utc>,
}

impl MTGAMatch {
    /// Creates a new `MTGAMatch` with the current timestamp
    ///
    /// # Arguments
    ///
    /// * `id` - A unique identifier for the match
    /// * `controller_seat_id` - The seat ID of the controller player
    /// * `controller_player_name` - The name of the controller player
    /// * `opponent_player_name` - The name of the opponent player
    ///
    /// # Returns
    ///
    /// A new `MTGAMatch` instance with the current UTC timestamp
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
        }
    }

    /// Creates a new `MTGAMatch` with a specified timestamp
    ///
    /// # Arguments
    ///
    /// * `id` - A unique identifier for the match
    /// * `controller_seat_id` - The seat ID of the controller player
    /// * `controller_player_name` - The name of the controller player
    /// * `opponent_player_name` - The name of the opponent player
    /// * `created_at` - The timestamp when the match was created
    ///
    /// # Returns
    ///
    /// A new `MTGAMatch` instance with the specified timestamp
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
        }
    }

    /// Returns the match ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the controller's seat ID
    pub fn controller_seat_id(&self) -> i32 {
        self.controller_seat_id
    }

    /// Returns the controller's player name
    pub fn controller_player_name(&self) -> &str {
        &self.controller_player_name
    }

    /// Returns the opponent's player name
    pub fn opponent_player_name(&self) -> &str {
        &self.opponent_player_name
    }

    /// Returns the match creation timestamp
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns whether the specified seat ID is the controller
    pub fn is_controller(&self, seat_id: i32) -> bool {
        self.controller_seat_id == seat_id
    }

    /// Returns the player name for the given seat ID
    pub fn player_name_for_seat(&self, seat_id: i32) -> Option<&str> {
        if self.is_controller(seat_id) {
            Some(&self.controller_player_name)
        } else {
            // We don't know the exact opponent seat ID, but we know it's not the controller
            Some(&self.opponent_player_name)
        }
    }
}

impl From<&proto::MtgaMatch> for MTGAMatch {
    fn from(proto: &proto::MtgaMatch) -> Self {
        let created_at = DateTime::parse_from_rfc3339(&proto.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_default();
        Self::new_with_timestamp(
            &proto.id,
            proto.controller_seat_id,
            &proto.controller_player_name,
            &proto.opponent_player_name,
            created_at,
        )
    }
}

impl From<&MTGAMatch> for proto::MtgaMatch {
    fn from(m: &MTGAMatch) -> Self {
        Self {
            id: m.id.clone(),
            controller_seat_id: m.controller_seat_id,
            controller_player_name: m.controller_player_name.clone(),
            opponent_player_name: m.opponent_player_name.clone(),
            created_at: m.created_at.to_rfc3339(),
        }
    }
}
