use chrono::Utc;
use serde::{Deserialize, Deserializer, Serialize, de::Error};
use serde_json::Value;

use crate::events::primitives::ArenaId;

/// Structs for MTGA "Business" events

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestTypeBusinessEvent {
    pub id: String,
    pub request: BusinessEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BusinessEvent {
    Game(GameBusinessEvent),
    Draft(DraftPackInfoEvent),
    Pick(DraftPickEvent),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GameBusinessEvent {
    pub event_id: String,
    pub event_type: i32,
    pub event_time: chrono::DateTime<Utc>,
    pub match_id: String,
    pub seat_id: i32,
    pub team_id: i32,
    pub game_number: i32,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DraftPackInfoEvent {
    pub player_id: Option<String>,
    pub client_platform: Option<String>,
    pub draft_id: String,
    pub event_id: String,
    pub seat_number: i32,
    pub pack_number: u8,
    pub pick_number: u8,
    pub pick_grp_id: ArenaId,
    pub cards_in_pack: Vec<ArenaId>,
    pub auto_pick: bool,
    pub time_remaining_on_pick: f64,
    pub event_type: i32,
    pub event_time: chrono::DateTime<Utc>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DraftPickEvent {
    pub draft_id: String,
    pub grp_ids: Vec<ArenaId>,
    pub pack: u8,
    pub pick: u8,
}

impl<'de> Deserialize<'de> for RequestTypeBusinessEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: Value = Deserialize::deserialize(deserializer)?;
        let id = v["id"].as_str().ok_or(Error::missing_field("id"))?;
        let request = v["request"].as_str().ok_or(Error::missing_field("request"))?;
        let business_event = serde_json::from_str(request).map_err(|e| Error::custom(e.to_string()))?;
        Ok(RequestTypeBusinessEvent {
            id: id.to_string(),
            request: business_event,
        })
    }
}

impl RequestTypeBusinessEvent {
    pub fn is_relevant(&self) -> bool {
        self.request.is_relevant()
    }
}

impl BusinessEvent {
    pub fn is_relevant(&self) -> bool {
        match self {
            BusinessEvent::Game(event) => event.is_relevant(),
            _ => false,
        }
    }

    pub fn as_game(&self) -> Option<&GameBusinessEvent> {
        match self {
            BusinessEvent::Game(event) => Some(event),
            _ => None,
        }
    }

    pub fn as_draft(&self) -> Option<&DraftPackInfoEvent> {
        match self {
            BusinessEvent::Draft(event) => Some(event),
            _ => None,
        }
    }
}

impl GameBusinessEvent {
    pub fn is_relevant(&self) -> bool {
        // All required fields are non-optional now, so if it deserializes, it's relevant
        true
    }
}

impl DraftPackInfoEvent {
    pub fn is_relevant(&self) -> bool {
        // Check that we have the essential draft information
        !self.cards_in_pack.is_empty()
    }
}
