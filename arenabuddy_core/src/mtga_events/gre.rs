use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::mtga_events::primitives::{
    Action, Annotation, MulliganType, OptionPrompt, Player, PlayerDieRoll, Power, Prompt,
    ResultListEntry, Skin, Stop, Target, Timer, Toughness, TurnInfo, Zone,
};

// GRE refers to the server-side MTGA engine
//
// no clue what it actually stands for, but these are a bunch of events that come from
// the server to the game client

macro_rules! wrapper {
    ($wrapperName:ident, $name:ident, $snake:ident) => {
        #[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $wrapperName {
            #[serde(flatten)]
            pub meta: GreMeta,
            pub $snake: $name,
        }
    };
    ($wrapperName:ident) => {
        #[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $wrapperName {
            #[serde(flatten)]
            pub meta: GreMeta,
            #[serde(flatten)]
            pub extra: HashMap<String, Value>,
        }
    };
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestTypeGREToClientEvent {
    pub gre_to_client_event: GREToClientEvent,
    pub request_id: Option<i32>,
    pub timestamp: String,
    pub transaction_id: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GREToClientEvent {
    pub gre_to_client_messages: Vec<GREToClientMessage>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum GREToClientMessage {
    #[serde(rename = "GREMessageType_ConnectResp")]
    ConnectResp(ConnectRespWrapper),
    #[serde(rename = "GREMessageType_DieRollResultsResp")]
    DieRollResults(DieRollResultsRespWrapper),
    #[serde(rename = "GREMessageType_GameStateMessage")]
    GameStateMessage(GameStateMessageWrapper),
    #[serde(rename = "GREMessageType_ChooseStartingPlayerReq")]
    ChooseStartingPlayerReq(ChooseStartingPlayerReqWrapper),
    #[serde(rename = "GREMessageType_MulliganReq")]
    MulliganReq(MulliganReqWrapper),
    #[serde(rename = "GREMessageType_SelectNReq")]
    SelectNReq(SelectNReqWrapper),
    #[serde(rename = "GREMessageType_ActionsAvailableReq")]
    ActionsAvailableReq(ActionsAvailableReq),
    #[serde(rename = "GREMessageType_SetSettingsResp")]
    SetSettingsResp(SetSettingsRespWrapper),
    #[serde(rename = "GREMessageType_SelectTargetsReq")]
    SelectTargetsReq(SelectTargetsReqWrapper),
    #[serde(rename = "GREMessageType_SubmitTargetsResp")]
    SubmitTargetsResp(SubmitTargetsRespWrapper),
    #[serde(rename = "GREMessageType_CastingTimeOptionsReq")]
    CastingTimeOptionsReq(CastingTimeOptionsReqWrapper),
    #[serde(rename = "GREMessageType_PayCostsReq")]
    PayCostsReq(PayCostsReqWrapper),
    #[serde(rename = "GREMessageType_SelectNResp")]
    SelectNResp(SelectNRespWrapper),
    #[serde(rename = "GREMessageType_DeclareAttackersReq")]
    DeclareAttackersReq(DeclareAttackersReqWrapper),
    #[serde(rename = "GREMessageType_SubmitAttackersResp")]
    SubmitAttackersResp(SubmitAttackersRespWrapper),
    #[serde(rename = "GREMessageType_DeclareBlockersReq")]
    DeclareBlockersReq(DeclareBlockersReqWrapper),
    #[serde(rename = "GREMessageType_SubmitBlockersResp")]
    SubmitBlockersResp(SubmitBlockersRespWrapper),
    #[serde(rename = "GREMessageType_IntermissionReq")]
    IntermissionReq(IntermissionReqWrapper),
    #[serde(rename = "GREMessageType_PromptReq")]
    PromptReq(PromptReqWrapper),
    #[serde(rename = "GREMessageType_QueuedGameStateMessage")]
    QueuedGameStateMessage(QueuedStateMessageWrapper),
    #[serde(rename = "GREMessageType_TimerStateMessage")]
    TimerStateMessage(TimerStateMessageWrapper),
    #[serde(rename = "GREMessageType_UIMessage")]
    UIMessage(UIMessageWrapper),
    #[serde(rename = "GREMessageType_SubmitDeckConfirmation")]
    SubmitDeckConfirmation(SubmitDeckConfirmationWrapper),
    #[serde(rename = "GREMessageType_OrderReq")]
    OrderReq(OrderReqWrapper),
    #[serde(rename = "GREMessageType_SubmitDeckReq")]
    SubmitDeckReq(SubmitDeckReqWrapper),
    #[serde(rename = "GREMessageType_SearchReq")]
    SearchReq(SearchReqWrapper),
    #[serde(rename = "GREMessageType_OptionalActionMessage")]
    OptionalActionMessage(OptionalActionMessageWrapper),
    #[serde(rename = "GREMessageType_GroupReq")]
    GroupReq(GroupReqWrapper),
    #[serde(rename = "GREMessageType_GroupResp")]
    GroupRespWrapper(GroupRespWrapper),
    #[serde(rename = "GREMessageType_TimeoutMessage")]
    TimeoutMessage(TimeoutMessageWrapper),
    #[serde(rename = "GREMessageType_EdictalMessage")]
    EdictalMessage(EdictalMessageWrapper),
    #[serde(rename = "GREMessageType_OrderCombatDamageReq")]
    OrderCombatMessageReq(OrderCombatDamageReqWrapper),
    #[serde(rename = "GREMessageType_OrderDamageConfirmation")]
    OrderDamageConfirmation(OrderDamageConfirmationWrapper),
    #[serde(rename = "GREMessageType_AssignDamageReq")]
    AssignDamageReq(AssignDamageReqWrapper),
    #[serde(rename = "GREMessageType_AssignDamageConfirmation")]
    AssignDamageConfirmation(AssignDamageConfirmationWrapper),
    #[serde(rename = "GREMessageType_IllegalRequest")]
    IllegalRequest(IllegalRequestWrapper),
    #[default]
    Default,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GreMeta {
    #[serde(default)]
    pub msg_id: i32,
    #[serde(default)]
    pub system_seat_ids: Vec<i32>,
    pub game_state_id: Option<i32>,
}

wrapper!(IllegalRequestWrapper);
wrapper!(AssignDamageReqWrapper);
wrapper!(AssignDamageConfirmationWrapper);
wrapper!(OrderDamageConfirmationWrapper);
wrapper!(OrderCombatDamageReqWrapper);
wrapper!(EdictalMessageWrapper);
wrapper!(TimeoutMessageWrapper);
wrapper!(GroupRespWrapper);
wrapper!(GroupReqWrapper);
wrapper!(OptionalActionMessageWrapper);
wrapper!(SearchReqWrapper);
wrapper!(SubmitDeckWrapper);
wrapper!(OrderReqWrapper);
wrapper!(SubmitBlockersRespWrapper);
wrapper!(DeclareBlockersReqWrapper);
wrapper!(UIMessageWrapper);
wrapper!(SubmitDeckConfirmationWrapper);
wrapper!(SubmitDeckReqWrapper);
wrapper!(SubmitAttackersRespWrapper);
wrapper!(DeclareAttackersReqWrapper);
wrapper!(SelectNRespWrapper);
wrapper!(PayCostsReqWrapper);
wrapper!(IntermissionReqWrapper, IntermissionReq, intermission_req);
wrapper!(CastingTimeOptionsReqWrapper);
wrapper!(ChooseStartingPlayerReqWrapper);
wrapper!(
    SubmitTargetsRespWrapper,
    SubmitTargetsResp,
    submit_targets_resp
);
wrapper!(ConnectRespWrapper, ConnectResp, connect_resp);
wrapper!(
    DieRollResultsRespWrapper,
    DieRollResultsResp,
    die_roll_results_resp
);
wrapper!(
    ActionsAvailableReqWrapper,
    ActionsAvailableReq,
    actions_available_req
);
wrapper!(PromptReqWrapper, Prompt, prompt);
wrapper!(SetSettingsRespWrapper, SetSettingsResp, set_settings_resp);
wrapper!(QueuedStateMessageWrapper);
wrapper!(TimerStateMessageWrapper);
wrapper!(
    GameStateMessageWrapper,
    GameStateMessage,
    game_state_message
);

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectNReqWrapper {
    pub select_n_req: SelectNReq,
    pub prompt: Option<Prompt>,
    pub allow_cancel: Option<String>,
    #[serde(default)]
    pub allow_undo: bool,
    #[serde(flatten)]
    pub meta: GreMeta,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectTargetsReqWrapper {
    pub select_targets_req: SelectTargetsReq,
    pub prompt: Option<Prompt>,
    pub allow_cancel: Option<String>,
    #[serde(default)]
    pub allow_undo: bool,
    #[serde(flatten)]
    pub meta: GreMeta,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulliganReqWrapper {
    pub mulligan_req: MulliganReq,
    pub prompt: Option<Prompt>,
    #[serde(flatten)]
    pub meta: GreMeta,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntermissionReq {
    pub intermission_prompt: Option<Prompt>,
    #[serde(default)]
    pub options: Vec<OptionPrompt>,
    pub result: ResultListEntry,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectNReq {
    pub context: String,
    pub id_type: Option<String>,
    #[serde(default)]
    pub ids: Vec<i32>,
    pub list_type: String,
    pub max_sel: Option<i32>,
    #[serde(default)]
    pub max_weight: i32,
    #[serde(default)]
    pub min_sel: i32,
    #[serde(default)]
    pub min_weight: i32,
    pub option_context: String,
    pub prompt: Option<Prompt>,
    pub source_id: Option<i32>,
    #[serde(default)]
    pub unfiltered_ids: Vec<i32>,
    pub validation_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionsAvailableReq {
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(default)]
    pub inactive_actions: Vec<Action>,
    pub prompt: Option<Prompt>,
    pub game_state_id: i32,
    pub msg_id: i32,
    pub system_seat_ids: Vec<i32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectTargetsReq {
    pub ability_grp_id: i32,
    pub source_id: Option<i32>,
    #[serde(default)]
    pub targets: Vec<SelectTargetsSpec>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectTargetsSpec {
    pub max_targets: i32,
    #[serde(default)]
    pub min_targets: i32,
    pub prompt: Option<Prompt>,
    pub selected_targets: Option<i32>,
    pub target_idx: i32,
    pub targeting_ability_grp_id: i32,
    pub targets: Vec<Target>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitTargetsResp {
    pub result: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSettingsResp {
    pub settings: Settings,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulliganReq {
    #[serde(default)]
    pub mulligan_count: i32,
    #[serde(default)]
    #[serde(rename = "mulliganType")]
    pub type_field: MulliganType,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub id: i32,
    #[serde(rename = "type")]
    #[serde(default)]
    pub type_field: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectResp {
    pub deck_message: DeckMessage,
    pub gre_changelist: i32,
    pub gre_version: GreVersion,
    pub grp_version: GrpVersion,
    pub proto_ver: String,
    pub settings: Settings,
    #[serde(default)]
    pub skins: Vec<Skin>,
    pub status: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckMessage {
    #[serde(default)]
    pub deck_cards: Vec<i32>,
    #[serde(default)]
    pub sideboard_cards: Vec<i32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GreVersion {
    pub build_version: i32,
    pub major_version: i32,
    pub minor_version: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrpVersion {
    pub major_version: i32,
    pub minor_version: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub auto_optional_payment_cancellation_setting: String,
    pub auto_pass_option: String,
    pub auto_select_replacement_setting: Option<String>,
    pub auto_tap_stops_setting: String,
    pub default_auto_pass_option: String,
    pub graveyard_order: String,
    pub mana_selection_type: String,
    pub smart_stops_setting: String,
    pub stack_auto_pass_option: String,
    pub stops: Vec<Stop>,
    pub transient_stops: Vec<Stop>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameStateMessage {
    #[serde(default)]
    pub actions: Vec<ActionWrapper>,
    #[serde(default)]
    pub annotations: Vec<Annotation>,
    #[serde(default)]
    pub diff_deleted_instance_ids: Vec<i32>,
    #[serde(default)]
    pub diff_deleted_persistent_annotation_ids: Vec<i32>,
    #[serde(default)]
    pub game_objects: Vec<GameObject>,
    pub game_state_id: i32,
    #[serde(default)]
    pub persistent_annotations: Vec<Annotation>,
    #[serde(default)]
    pub players: Vec<Player>,
    pub prev_game_state_id: Option<i32>,
    #[serde(default)]
    pub timers: Vec<Timer>,
    #[serde(default)]
    #[serde(rename = "type")]
    pub type_field: String,
    pub update: String,
    #[serde(default)]
    pub zones: Vec<Zone>,
    pub turn_info: Option<TurnInfo>,
    pub pending_message_count: Option<i32>,
    pub game_info: Option<GameInfo>,
    #[serde(default)]
    pub teams: Vec<Team>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionWrapper {
    pub action: Action,
    pub seat_id: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameObject {
    #[serde(default)]
    pub abilities: Vec<i32>,
    #[serde(default)]
    pub card_types: Vec<String>,
    #[serde(default)]
    pub color: Vec<String>,
    pub controller_seat_id: Option<i32>,
    pub grp_id: i32,
    pub instance_id: i32,
    pub name: Option<i32>,
    pub overlay_grp_id: Option<i32>,
    pub owner_seat_id: i32,
    #[serde(default)]
    #[serde(rename = "type")]
    pub type_field: GameObjectType,
    #[serde(default)]
    pub viewers: Vec<i32>,
    pub visibility: String,
    pub zone_id: Option<i32>,
    #[serde(default)]
    pub subtypes: Vec<String>,
    #[serde(default)]
    pub super_types: Vec<String>,
    pub base_skin_code: Option<String>,
    pub skin_code: Option<String>,
    pub is_tapped: Option<bool>,
    pub power: Option<Power>,
    pub toughness: Option<Toughness>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameObjectType {
    #[default]
    #[serde(rename = "GameObjectType_Card")]
    Card,
    #[serde(rename = "GameObjectType_RevealedCard")]
    RevealedCard,
    #[serde(rename = "GameObjectType_TriggerHolder")]
    TriggerHolder,
    #[serde(rename = "GameObjectType_MDFCBack")]
    MDFCBack,
    #[serde(rename = "GameObjectType_Ability")]
    Ability,
    #[serde(rename = "GameObjectType_Token")]
    Token,
    #[serde(rename = "GameObjectType_Adventure")]
    Adventure,
    #[serde(rename = "GameObjectType_DisturbBack")]
    DisturbBack,
    #[serde(rename = "GameObjectType_RoomLeft")]
    RoomLeft,
    #[serde(rename = "GameObjectType_RoomRight")]
    RoomRight,
    #[serde(rename = "GameObjectType_SplitLeft")]
    SplitLeft,
    #[serde(rename = "GameObjectType_Omen")]
    Omen,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub deck_constraint_info: DeckConstraintInfo,
    pub game_number: i32,
    #[serde(rename = "matchID")]
    pub match_id: String,
    pub match_state: String,
    pub match_win_condition: String,
    pub mulligan_type: String,
    pub stage: String,
    pub super_format: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub variant: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckConstraintInfo {
    pub max_deck_size: i32,
    pub max_sideboard_size: i32,
    pub min_deck_size: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: i32,
    pub player_ids: Vec<i32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DieRollResultsResp {
    pub player_die_rolls: Vec<PlayerDieRoll>,
}

#[cfg(test)]
mod test {

    use std::mem::size_of;

    use super::*;

    #[test]
    fn test_gre_sizes() {
        assert_eq!(size_of::<GREToClientMessage>(), 648);
        assert_eq!(size_of::<GreMeta>(), 40);
        assert_eq!(size_of::<ConnectRespWrapper>(), 448);
    }
}
