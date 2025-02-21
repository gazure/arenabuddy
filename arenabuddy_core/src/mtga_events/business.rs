use chrono::Utc;
use serde::{Deserialize, Deserializer, Serialize, de::Error};
use serde_json::Value;

///
/// Structs for MTGA "Business" events
///

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestTypeBusinessEvent {
    id: String,
    pub request: BusinessEventRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BusinessEventRequest {
    pub event_id: Option<String>,
    pub event_type: Option<i32>,
    pub event_time: Option<chrono::DateTime<Utc>>,
    pub match_id: Option<String>,
    pub seat_id: Option<i32>,
    pub team_id: Option<i32>,
    pub game_number: Option<i32>,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

impl<'de> Deserialize<'de> for RequestTypeBusinessEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: Value = Deserialize::deserialize(deserializer)?;
        let id = v["id"].as_str().ok_or(Error::missing_field("id"))?;
        let request = v["request"]
            .as_str()
            .ok_or(Error::missing_field("request"))?;
        let business_event =
            serde_json::from_str(request).map_err(|e| Error::custom(e.to_string()))?;
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

impl BusinessEventRequest {
    pub fn is_relevant(&self) -> bool {
        self.event_id.is_some()
            && self.event_type.is_some()
            && self.event_time.is_some()
            && self.match_id.is_some()
            && self.seat_id.is_some()
            && self.team_id.is_some()
            && self.game_number.is_some()
    }
}
