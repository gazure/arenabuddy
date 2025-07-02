use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::mtga_events::{
    gre::DeckMessage,
    primitives::{SubZoneType, Target, ZoneType},
};

// Client messages to the game server
//

macro_rules! wrapper {
    ($wrapperName:ident, $name:ident, $snake:ident) => {
        #[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $wrapperName {
            #[serde(flatten)]
            pub meta: ClientMeta,
            pub $snake: $name,
        }
    };
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestTypeClientToMatchServiceMessage {
    #[serde(rename = "clientToMatchServiceMessageType")]
    pub client_to_match_service_message_type: String,
    #[serde(rename = "requestId")]
    pub request_id: i32,
    #[serde(rename = "payload")]
    pub payload: ClientMessage,
    pub timestamp: Option<String>,
    #[serde(rename = "transactionId")]
    pub transaction_id: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientMeta {
    pub game_state_id: Option<i32>,
    pub resp_id: Option<i32>,
    pub system_seat_id: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "ClientMessageType_ChooseStartingPlayerResp")]
    ChooseStartingPlayerResp(ChooseStartingPlayerRespWrapper),
    #[serde(rename = "ClientMessageType_SubmitDeckResp")]
    SubmitDeckResp(SubmitDeckRespWrapper),
    #[serde(rename = "ClientMessageType_SetSettingsReq")]
    SetSettingsReq(SetSettingsReqWrapper),
    #[serde(rename = "ClientMessageType_PerformActionResp")]
    PerformActionResp(PerformActionRespWrapper),
    #[serde(rename = "ClientMessageType_MulliganResp")]
    MulliganResp(MulliganRespWrapper),
    #[serde(rename = "ClientMessageType_UIMessage")]
    UIMessage(UIMessageWrapper),
    #[serde(rename = "ClientMessageType_SelectNResp")]
    SelectNResp(SelectNRespWrapper),
    #[serde(rename = "ClientMessageType_SubmitTargetsReq")]
    SubmitTargetsReq(SubmitTargetsReqWrapper),
    #[serde(rename = "ClientMessageType_SubmitTargetsResp")]
    SubmitTargetsResp(SubmitTargetsResp),
    #[serde(rename = "ClientMessageType_SelectTargetsResp")]
    SelectTargetsResp(SelectTargetsRespWrapper),
    #[serde(rename = "ClientMessageType_SubmitAttackersReq")]
    SubmitAttackersReq(SubmitAttackersReq),
    #[serde(rename = "ClientMessageType_DeclareAttackersReq")]
    DeclareAttackersReq(DeclareAttackersReq),
    #[serde(rename = "ClientMessageType_DeclareAttackersResp")]
    DeclareAttackersResp(DeclareAttackersRespWrapper),
    #[serde(rename = "ClientMessageType_SubmitBlockersReq")]
    SubmitBlockersReq(SubmitBlockersReq),
    #[serde(rename = "ClientMessageType_SubmitBlockersResp")]
    SubmitBlockersResp(SubmitBlockersResp),
    #[serde(rename = "ClientMessageType_DeclareBlockersResp")]
    DeclareBlockersResp(DeclareBlockersRespWrapper),
    #[serde(rename = "ClientMessageType_ConcedeReq")]
    ConcedeReq(ConcedeReqWrapper),
    #[serde(rename = "ClientMessageType_EffectCostResp")]
    EffectCostResp(EffectCostRespWrapper),
    #[serde(rename = "ClientMessageType_CastingTimeOptionsResp")]
    CastingTimeOptionsResp(CastingTimeOptionsRespWrapperWrapper),
    #[serde(rename = "ClientMessageType_CancelActionReq")]
    CancelActionReq(CancelActionReqWrapper),
    #[serde(rename = "ClientMessageType_OrderResp")]
    OrderResp(OrderRespWrapper),
    #[serde(rename = "ClientMessageType_SearchResp")]
    SearchResp(SearchRespWrapper),
    #[serde(rename = "ClientMessageType_OptionalActionResp")]
    OptionalActionResp(OptionalActionRespWrapper),
    #[serde(rename = "ClientMessageType_PerformAutoTapActionsResp")]
    PerformAutoTapActionsResp(PerformAutoTapActionsRespWrapper),
    #[serde(rename = "ClientMessageType_EnterSideboardingReq")]
    EnterSideboardingReq(EnterSideboardingReq),
    #[serde(rename = "ClientMessageType_OrderCombatDamageResp")]
    OrderCombatDamageResp(OrderCombatDamageRespWrapper),
    #[serde(rename = "ClientMessageType_AssignDamageResp")]
    AssignDamageResp(AssignDamageRespWrapper),
    #[serde(rename = "ClientMessageType_GroupResp")]
    GroupResp(GroupRespWrapper),
    #[serde(rename = "ClientMessageType_UndoReq")]
    UndoReq(UndoReqWrapper),
}

wrapper!(
    AssignDamageRespWrapper,
    AssignDamageResp,
    assign_damage_resp
);
wrapper!(SetSettingsReqWrapper, SetSettingsReq, set_settings_req);
wrapper!(SubmitDeckRespWrapper, SubmitDeckResp, submit_deck_resp);
wrapper!(MulliganRespWrapper, MulliganResp, mulligan_resp);
wrapper!(
    PerformActionRespWrapper,
    PerformActionResp,
    perform_action_resp
);
wrapper!(UIMessageWrapper, UIMessage, ui_message);
wrapper!(SelectNRespWrapper, SelectNResp, select_n_resp);
wrapper!(
    SelectTargetsRespWrapper,
    SelectTargetsResp,
    select_targets_resp
);
wrapper!(
    DeclareAttackersRespWrapper,
    DeclareAttackersResp,
    declare_attackers_resp
);
wrapper!(ConcedeReqWrapper, ConcedeReq, concede_req);
wrapper!(EffectCostRespWrapper, EffectCostResp, effect_cost_resp);
wrapper!(
    ChooseStartingPlayerRespWrapper,
    ChooseStartingPlayerResp,
    choose_starting_player_resp
);
wrapper!(CancelActionReqWrapper, CancelActionReq, cancel_action_req);
wrapper!(
    CastingTimeOptionRespWrapper,
    CastingTimeOptionResp,
    casting_time_option_resp
);
wrapper!(
    CastingTimeOptionsRespWrapperWrapper,
    CastingTimeOptionRespWrapper,
    casting_time_options_resp
);
wrapper!(
    PerformAutoTapActionsRespWrapper,
    PerformAutoTapActionsResp,
    perform_auto_tap_actions_resp
);
wrapper!(OrderRespWrapper, OrderResp, order_resp);
wrapper!(SearchRespWrapper, SearchResp, search_resp);
wrapper!(OptionalActionRespWrapper, OptionalActionResp, optional_resp);
wrapper!(GroupRespWrapper, GroupResp, group_resp);
wrapper!(
    OrderCombatDamageRespWrapper,
    OrderCombatDamageResp,
    order_combat_damage_resp
);

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct UndoReqWrapper {
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupResp {
    group_type: GroupType,
    groups: Vec<Group>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    #[serde(default)]
    pub ids: Vec<i32>,
    pub sub_zone_type: Option<SubZoneType>,
    pub zone_type: ZoneType,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub enum GroupType {
    #[default]
    #[serde(rename = "GroupType_Ordered")]
    Ordered,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct AssignDamageResp {
    #[serde(flatten)]
    pub meta: ClientMeta,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct OrderCombatDamageResp {
    #[serde(flatten)]
    pub meta: ClientMeta,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct EnterSideboardingReq {
    #[serde(flatten)]
    pub meta: ClientMeta,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct PerformAutoTapActionsResp {}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct OptionalActionResp {
    response: OptionResponse,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub enum OptionResponse {
    #[default]
    #[serde(rename = "OptionResponse_Cancel_No")]
    CancelNo,
    #[serde(rename = "OptionResponse_Allow_Yes")]
    AllowYes,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResp {
    pub items_found: Vec<i32>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResp {
    ids: Vec<i32>,
    ordering: OrderingType,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub enum OrderingType {
    #[default]
    #[serde(rename = "OrderingType_OrderAsIndicated")]
    OrderAsIndicated,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct CancelActionReq {}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CastingTimeOptionResp {
    pub casting_time_option_type: CastingTimeOptionType,
    pub cto_id: Option<i32>,
    pub select_n_resp: Option<SelectNResp>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub enum CastingTimeOptionType {
    #[default]
    #[serde(rename = "CastingTimeOptionType_ChooseOrCost")]
    ChooseOrCost,
    #[serde(rename = "CastingTimeOptionType_Modal")]
    Modal,
    #[serde(rename = "CastingTimeOptionType_ChooseX")]
    ChooseX,
    #[serde(rename = "CastingTimeOptionType_Selection")]
    Selection,
    #[serde(rename = "CastingTimeOptionType_AdditionalCost")]
    AdditionalCost,
    #[serde(rename = "CastingTimeOptionType_Done")]
    Done,
    #[serde(rename = "CastingTimeOptionType_Kicker")]
    Kicker,
    #[serde(rename = "CastingTimeOptionType_Bargain")]
    Bargain,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct SetSettingsReq {
    pub settings: ClientSettings,
}

// TODO: settings to enum types
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientSettings {
    pub stack_auto_pass_option: Option<String>,
    pub auto_pass_option: Option<String>,
    pub auto_select_replacement_setting: Option<String>,
    pub default_auto_pass_option: Option<String>,
    pub mana_selection_type: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct SubmitDeckResp {
    pub deck: DeckMessage,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChooseStartingPlayerResp {
    #[serde(default)]
    pub system_seat_id: i32,
    pub team_id: i32,
    // TODO: enum for team_type
    pub team_type: String,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct SubmitBlockersReq {
    #[serde(flatten)]
    pub meta: ClientMeta,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct SubmitBlockersResp {
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclareBlockersRespWrapper {
    #[serde(flatten)]
    pub meta: ClientMeta,
    pub declare_blockers_resp: DeclareBlockersResp,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclareBlockersResp {
    #[serde(default)]
    pub selected_blockers: Vec<Blocker>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Blocker {
    #[serde(default)]
    pub attacker_instance_ids: Vec<i32>,
    pub blocker_instance_id: i32,
    pub max_attackers: i32,
    #[serde(default)]
    pub selected_attacker_instance_ids: Vec<i32>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectCostResp {
    #[serde(default)]
    pub cost_selection: CostSelection,
    pub effect_cost_type: EffectCostType,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostSelection {
    #[serde(default)]
    ids: Vec<i32>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub enum EffectCostType {
    #[default]
    #[serde(rename = "EffectCostType_Select")]
    Select,
    #[serde(rename = "EffectCostType_GatherCreatures")]
    GatherCreatures,
    #[serde(rename = "EffectCostType_GatherCounters")]
    GatherCounters,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConcedeReq {
    scope: MatchScope,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub enum MatchScope {
    #[default]
    #[serde(rename = "MatchScope_Game")]
    Game,
    #[serde(rename = "MatchScope_Match")]
    Match,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct SubmitAttackersReq {
    #[serde(flatten)]
    pub extra: ClientMeta,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct DeclareAttackersReq {
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclareAttackersResp {
    #[serde(default)]
    pub auto_declare: bool,
    pub auto_declare_damage_recipient: Option<DamageRecipient>,
    #[serde(default)]
    pub selected_attackers: Vec<Attacker>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attacker {
    pub attacker_instance_id: i32,
    pub legal_damage_recipients: Vec<DamageRecipient>,
    pub selected_damage_recipient: Option<DamageRecipient>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageRecipient {
    pub player_system_seat_id: Option<i32>,
    pub planswalker_instance_id: Option<i32>,
    #[serde(rename = "type")]
    pub type_field: DamageRecType,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub enum DamageRecType {
    #[default]
    #[serde(rename = "DamageRecType_Player")]
    Player,
    #[serde(rename = "DamageRecType_PlanesWalker")]
    Planeswalker,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectTargetsResp {
    target: SelectTarget,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectTarget {
    pub target_idx: i32,
    #[serde(default)]
    pub targets: Vec<Target>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitTargetsReqWrapper {
    #[serde(flatten)]
    pub meta: ClientMeta,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct SubmitTargetsResp {
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectNResp {
    #[serde(default)]
    pub ids: Vec<i32>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UIMessage {
    seat_ids: Vec<i32>,
    on_hover: Option<Value>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct MulliganResp {
    pub decision: MulliganOption,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum MulliganOption {
    #[default]
    #[serde(rename = "MulliganOption_AcceptHand")]
    AcceptHand,
    #[serde(rename = "MulliganOption_Mulligan")]
    Mulligan,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct PerformActionResp {
    pub actions: Vec<Action>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub action_type: ActionType,
    pub facet_id: Option<i32>,
    pub grp_id: Option<i32>,
    pub instance_id: Option<i32>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub enum ActionType {
    #[default]
    #[serde(rename = "ActionType_Play")]
    Play,
    #[serde(rename = "ActionType_PlayMDFC")]
    PlayMDFC,
    #[serde(rename = "ActionType_Activate")]
    Activate,
    #[serde(rename = "ActionType_Activate_Mana")]
    ActivateMana,
    #[serde(rename = "ActionType_Cast")]
    Cast,
    #[serde(rename = "ActionType_Pass")]
    Pass,
    #[serde(rename = "ActionType_Special")]
    Special,
    #[serde(rename = "ActionType_CastAdventure")]
    CastAdventure,
    #[serde(rename = "ActionType_Make_Payment")]
    MakePayment,
    #[serde(rename = "ActionType_OpeningHandAction")]
    OpeningHandAction,
    #[serde(rename = "ActionType_Special_TurnFaceUp")]
    SpecialTurnFaceUp,
    #[serde(rename = "ActionType_CastLeftRoom")]
    CastLeftRoom,
    #[serde(rename = "ActionType_CastRightRoom")]
    CastRightRoom,
    #[serde(rename = "ActionType_CastRight")]
    CastRight,
    #[serde(rename = "ActionType_CastLeft")]
    CastLeft,
}
