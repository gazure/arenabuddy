use crate::mtga_events::primitives::ResultListEntry;
use serde::{Deserialize, Serialize};

///
/// Every match should emit 2 of these logs to indicate the start and end of a match
/// though the start of a match is usually after the `ConnectResp` GRE message with the
/// player's decklist, so something keep in mind
///
///

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestTypeMGRSCEvent {
    #[serde(rename = "matchGameRoomStateChangedEvent")]
    pub mgrsc_event: MatchGameRoomStateChangedEvent,
    #[serde(default)]
    pub request_id: i32,
    pub timestamp: String,
    pub transaction_id: String,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchGameRoomStateChangedEvent {
    pub game_room_info: GameRoomInfo,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRoomInfo {
    pub game_room_config: GameRoomConfig,
    pub players: Option<Vec<MatchPlayer>>,
    pub final_match_result: Option<FinalMatchResult>,
    pub state_type: StateType,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub enum StateType {
    #[serde(rename = "MatchGameRoomStateType_MatchCompleted")]
    MatchCompleted,
    #[default]
    #[serde(rename = "MatchGameRoomStateType_Playing")]
    Playing,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct FinalMatchResult {
    #[serde(rename = "matchId")]
    pub match_id: String,
    #[serde(rename = "resultList")]
    pub result_list: Vec<ResultListEntry>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRoomConfig {
    pub match_id: String,
}
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchPlayer {
    pub player_name: String,
    pub system_seat_id: i32,
    pub team_id: i32,
    pub user_id: String,
}
