use crate::mtga_events::gre::Reference;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionPrompt {
    pub option_prompt: Prompt,
    pub response_type: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub enum MulliganType {
    #[default]
    #[serde(rename = "MulliganType_London")]
    London,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub prompt_id: Option<i32>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    pub parameter_name: String,
    pub reference: Option<Reference>,
    pub prompt_id: Option<i32>,
    #[serde(default)]
    #[serde(rename = "type")]
    pub type_field: String,
    pub number_value: Option<i32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stop {
    pub applies_to: String,
    pub status: String,
    pub stop_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skin {
    pub catalog_id: i32,
    pub skin_code: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub action_type: String,
    pub instance_id: Option<i32>,
    #[serde(default)]
    pub mana_cost: Vec<ManaCost>,
    pub ability_grp_id: Option<i32>,
    #[serde(default)]
    pub mana_payment_options: Vec<ManaPaymentOption>,
    pub facet_id: Option<i32>,
    pub grp_id: Option<i32>,
    pub should_stop: Option<bool>,
    pub auto_tap_solution: Option<AutoTapSolution>,
    #[serde(default)]
    pub targets: Vec<TargetCollection>,
    pub is_batchable: Option<bool>,
    pub unique_ability_id: Option<i32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManaCost {
    pub color: Vec<String>,
    pub count: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManaPaymentOption {
    pub mana: Vec<Mana>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mana {
    pub ability_grp_id: i32,
    pub color: String,
    pub mana_id: i32,
    pub specs: Vec<Spec>,
    pub src_instance_id: i32,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Spec {
    #[default]
    #[serde(rename = "ManaSpecType_Predictive")]
    Predictive,
    #[serde(rename = "ManaSpecType_FromCave")]
    FromCave,
    #[serde(rename = "ManaSpecType_FromCreature")]
    FromCreature,
    #[serde(rename = "ManaSpecType_Restricted")]
    Restricted,
    #[serde(rename = "ManaSpecType_FromTreasure")]
    FromTreasure,
    #[serde(rename = "ManaSpecType_AdditionalEffect")]
    AdditionalEffect,
    #[serde(rename = "ManaSpecType_CantBeCountered")]
    CantBeCountered,
    #[serde(rename = "ManaSpecType_FromSnow")]
    FromSnow,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoTapSolution {
    pub auto_tap_actions: Vec<Action>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCollection {
    pub target_idx: i32,
    #[serde(default)]
    pub targets: Vec<Target>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub highlight: Option<String>,
    pub target_instance_id: i32,
    pub legal_action: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    #[serde(default)]
    pub affected_ids: Vec<i32>,
    pub affector_id: Option<i32>,
    pub id: i32,
    #[serde(default)]
    #[serde(rename = "type")]
    pub type_field: Vec<AnnotationType>,
    #[serde(default)]
    pub details: Vec<AnnotationDetail>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnnotationType {
    #[serde(rename = "AnnotationType_ResolutionStart")]
    ResolutionStart,
    #[serde(rename = "AnnotationType_ResolutionComplete")]
    ResolutionComplete,
    #[serde(rename = "AnnotationType_CardRevealed")]
    CardRevealed,
    #[serde(rename = "AnnotationType_RevealedCardCreated")]
    RevealedCardCreated,
    #[serde(rename = "AnnotationType_RevealedCardDeleted")]
    RevealedCardDeleted,
    #[serde(rename = "AnnotationType_ObjectIdChanged")]
    ObjectIdChanged,
    #[serde(rename = "AnnotationType_ZoneTransfer")]
    ZoneTransfer,
    #[serde(rename = "AnnotationType_SyntheticEvent")]
    SyntheticEvent,
    #[serde(rename = "AnnotationType_ModifiedLife")]
    ModifiedLife,
    #[serde(rename = "AnnotationType_ModifiedType")]
    ModifiedType,
    #[serde(rename = "AnnotationType_EnteredZoneThisTurn")]
    EnteredZoneThisTurn,
    #[serde(rename = "AnnotationType_PhaseOrStepModified")]
    PhaseOrStepModified,
    #[serde(rename = "AnnotationType_NewTurnStarted")]
    NewTurnStarted,
    #[serde(rename = "AnnotationType_UserActionTaken")]
    UserActionTaken,
    #[serde(rename = "AnnotationType_AbilityInstanceCreated")]
    AbilityInstanceCreated,
    #[serde(rename = "AnnotationType_AbilityInstanceDeleted")]
    AbilityInstanceDeleted,
    #[serde(rename = "AnnotationType_PlayerSelectingTargets")]
    PlayerSelectingTargets,
    #[serde(rename = "AnnotationType_PlayerSubmittedTargets")]
    PlayerSubmittedTargets,
    #[serde(rename = "AnnotationType_TappedUntappedPermanent")]
    TappedUntappedPermanent,
    #[serde(rename = "AnnotationType_Designation")]
    Designation,
    #[serde(rename = "AnnotationType_GainDesignation")]
    GainDesignation,
    #[serde(rename = "AnnotationType_ChoiceResult")]
    ChoiceResult,
    #[serde(rename = "AnnotationType_ReplacementEffect")]
    ReplacementEffect,
    #[serde(rename = "AnnotationType_ObjectsSelected")]
    ObjectsSelected,
    #[serde(rename = "AnnotationType_Counter")]
    Counter,
    #[serde(rename = "AnnotationType_CounterAdded")]
    CounterAdded,
    #[serde(rename = "AnnotationType_CounterRemoved")]
    CounterRemoved,
    #[serde(rename = "AnnotationType_MultistepEffectStarted")]
    MultistepEffectStarted,
    #[serde(rename = "AnnotationType_MultistepEffectComplete")]
    MultistepEffectComplete,
    #[serde(rename = "AnnotationType_LayeredEffect")]
    LayeredEffect,
    #[serde(rename = "AnnotationType_LayeredEffectCreated")]
    LayeredEffectCreated,
    #[serde(rename = "AnnotationType_LayeredEffectDeleted")]
    LayeredEffectDeleted,
    #[serde(rename = "AnnotationType_LayeredEffectDestroyed")]
    LayeredEffectDestroyed,
    #[serde(rename = "AnnotationType_DamageDealt")]
    DamageDealt,
    #[serde(rename = "AnnotationType_TargetSpec")]
    TargetSpec,
    #[serde(rename = "AnnotationType_ManaPaid")]
    ManaPaid,
    #[serde(rename = "AnnotationType_TriggeringObject")]
    TriggeringObject,
    #[serde(rename = "AnnotationType_LinkInfo")]
    LinkInfo,
    #[serde(rename = "AnnotationType_ShouldntPlay")]
    ShouldntPlay,
    #[serde(rename = "AnnotationType_ModifiedToughness")]
    ModifiedToughness,
    #[serde(rename = "AnnotationType_ModifiedPower")]
    ModifiedPower,
    #[serde(rename = "AnnotationType_PowerToughnessModCreated")]
    PowerToughnessModCreated,
    #[serde(rename = "AnnotationType_Qualification")]
    Qualification,
    #[serde(rename = "AnnotationType_CrewedThisTurn")]
    CrewedThisTurn,
    #[serde(rename = "AnnotationType_DamagedThisTurn")]
    DamagedThisTurn,
    #[serde(rename = "AnnotationType_LoyaltyActivationsRemaining")]
    LoyaltyActivationsRemaining,
    #[serde(rename = "AnnotationType_TokenCreated")]
    TokenCreated,
    #[serde(rename = "AnnotationType_TokenDeleted")]
    TokenDeleted,
    #[serde(rename = "AnnotationType_ManaDetails")]
    ManaDetails,
    #[serde(rename = "AnnotationType_Shuffle")]
    Shuffle,
    #[serde(rename = "AnnotationType_InstanceRevealedToOpponent")]
    InstanceRevealedToOpponent,
    #[serde(rename = "AnnotationType_DisqualifiedEffect")]
    DisqualifiedEffect,
    #[serde(rename = "AnnotationType_CastingTimeOption")]
    CastingTimeOption,
    #[serde(rename = "AnnotationType_AddAbility")]
    AddAbility,
    #[serde(rename = "AnnotationType_RemoveAbility")]
    RemoveAbility,
    #[serde(rename = "AnnotationType_AbilityWordActive")]
    AbilityWordActive,
    #[serde(rename = "AnnotationType_ModifiedColor")]
    ModifiedColor,
    #[serde(rename = "AnnotationType_LossOfGame")]
    LossOfGame,
    #[serde(rename = "AnnotationType_MiscContinuousEffect")]
    MiscContinuousEffect,
    #[serde(rename = "AnnotationType_DisplayCardUnderCard")]
    DisplayCardUnderCard,
    #[serde(rename = "AnnotationType_Attachment")]
    Attachment,
    #[serde(rename = "AnnotationType_AttachmentCreated")]
    AttachmentCreated,
    #[serde(rename = "AnnotationType_GroupedIds")]
    GroupedIds,
    #[serde(rename = "AnnotationType_ReplacementEffectApplied")]
    ReplacementEffectApplied,
    #[serde(rename = "AnnotationType_Scry")]
    Scry,
    #[serde(rename = "AnnotationType_DelayedTriggerAffectees")]
    DelayedTriggerAffectees,
    #[serde(rename = "AnnotationType_DungeonStatus")]
    DungeonStatus,
    #[serde(rename = "AnnotationType_TokenImmediatelyDied")]
    TokenImmediatelyDied,
    #[serde(rename = "AnnotationType_ChoosingAttachments")]
    ChoosingAttachments,
    #[serde(rename = "AnnotationType_AbilityExhausted")]
    AbilityExhausted,
    #[serde(rename = "AnnotationType_ModifiedName")]
    ModifiedName,
    #[serde(rename = "AnnotationType_ControllerChanged")]
    ControllerChanged,
    #[serde(rename = "AnnotationType_TemporaryPermanent")]
    TemporaryPermanent,
    #[serde(rename = "AnnotationType_UseOrCostsManaCost")]
    UseOrCostsManaCost,
    #[serde(rename = "AnnotationType_ClassLevel")]
    ClassLevel,
    #[serde(rename = "AnnotationType_PredictedDirectDamage")]
    PredictedDirectDamage,
    #[serde(rename = "AnnotationType_FaceDown")]
    FaceDown,
    #[serde(rename = "AnnotationType_SaddledThisTurn")]
    SaddledThisTurn,
    #[serde(rename = "AnnotationType_TurnPermanent")]
    TurnPermanent,
    #[serde(rename = "AnnotationType_LoseDesignation")]
    LoseDesignation,
    #[serde(rename = "AnnotationType_TextChange")]
    TextChange,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationDetail {
    pub key: String,
    #[serde(default)]
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(default)]
    pub value_int32: Vec<i32>,
    #[serde(default)]
    pub value_string: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Power {
    #[serde(default)]
    pub value: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Toughness {
    #[serde(default)]
    pub value: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GsmPlayer {
    pub controller_seat_id: i32,
    pub controller_type: String,
    pub life_total: i32,
    pub max_hand_size: i32,
    pub starting_life_total: i32,
    pub system_seat_number: i32,
    pub team_id: i32,
    pub timer_ids: Vec<i32>,
    pub pending_message_type: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timer {
    pub behavior: String,
    pub duration_sec: i32,
    pub timer_id: i32,
    #[serde(default)]
    #[serde(rename = "type")]
    pub type_field: String,
    pub warning_threshold_sec: Option<i32>,
    pub running: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnInfo {
    pub active_player: Option<i32>,
    pub decision_player: Option<i32>,
    pub next_phase: Option<String>,
    pub next_step: Option<String>,
    pub phase: Option<Phase>,
    pub priority_player: Option<i32>,
    pub turn_number: Option<i32>,
    pub step: Option<Step>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Zone {
    pub owner_seat_id: Option<i32>,
    #[serde(rename = "type")]
    #[serde(default)]
    pub type_field: ZoneType,
    pub visibility: Visibility,
    pub zone_id: i32,
    #[serde(default)]
    pub viewers: Vec<i32>,
    #[serde(default)]
    pub object_instance_ids: Vec<i32>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Visibility {
    #[default]
    #[serde(rename = "Visibility_Public")]
    Public,
    #[serde(rename = "Visibility_Private")]
    Private,
    #[serde(rename = "Visibility_Hidden")]
    Hidden,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDieRoll {
    pub roll_value: i32,
    pub system_seat_id: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub controller_seat_id: i32,
    pub controller_type: String,
    #[serde(default)]
    pub life_total: i32,
    pub max_hand_size: i32,
    pub starting_life_total: i32,
    pub system_seat_number: i32,
    pub team_id: i32,
    pub timer_ids: Vec<i32>,
    pub pending_message_type: Option<String>,
    pub turn_number: Option<i32>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultListEntry {
    pub scope: String,
    #[serde(rename = "winningTeamId")]
    pub winning_team_id: i32,
    pub reason: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Phase {
    #[serde(rename = "Phase_Beginning")]
    Beginning,
    #[serde(rename = "Phase_Main1")]
    PrecombatMain,
    #[serde(rename = "Phase_Combat")]
    Combat,
    #[serde(rename = "Phase_Main2")]
    PostcombatMain,
    #[serde(rename = "Phase_Ending")]
    End,
}

impl Display for Phase {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Step {
    #[serde(rename = "Step_Untap")]
    Untap,
    #[serde(rename = "Step_Upkeep")]
    Upkeep,
    #[serde(rename = "Step_Draw")]
    Draw,
    #[serde(rename = "Step_BeginCombat")]
    BeginCombat,
    #[serde(rename = "Step_DeclareAttack")]
    DeclareAttack,
    #[serde(rename = "Step_DeclareBlock")]
    DeclareBlock,
    #[serde(rename = "Step_FirstStrikeDamage")]
    FirstStrikeDamage,
    #[serde(rename = "Step_CombatDamage")]
    CombatDamage,
    #[serde(rename = "Step_EndCombat")]
    EndCombat,
    #[serde(rename = "Step_End")]
    End,
    #[serde(rename = "Step_Cleanup")]
    Cleanup,
}

impl Display for Step {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ZoneType {
    #[default]
    #[serde(rename = "ZoneType_Battlefield")]
    Battlefield,
    #[serde(rename = "ZoneType_Stack")]
    Stack,
    #[serde(rename = "ZoneType_Exile")]
    Exile,
    #[serde(rename = "ZoneType_Graveyard")]
    Graveyard,
    #[serde(rename = "ZoneType_Hand")]
    Hand,
    #[serde(rename = "ZoneType_Library")]
    Library,
    #[serde(rename = "ZoneType_Limbo")]
    Limbo,
    #[serde(rename = "ZoneType_Sideboard")]
    Sideboard,
    #[serde(rename = "ZoneType_Pending")]
    Pending,
    #[serde(rename = "ZoneType_Suppressed")]
    Suppressed,
    #[serde(rename = "ZoneType_Revealed")]
    Revealed,
    #[serde(rename = "ZoneType_RevealedSideboard")]
    RevealedSideboard,
    #[serde(rename = "ZoneType_RevealedExile")]
    RevealedExile,
    #[serde(rename = "ZoneType_RevealedGraveyard")]
    RevealedGraveyard,
    #[serde(rename = "ZoneType_RevealedHand")]
    RevealedHand,
    #[serde(rename = "ZoneType_RevealedLibrary")]
    RevealedLibrary,
    #[serde(rename = "ZoneType_RevealedLimbo")]
    RevealedLimbo,
    #[serde(rename = "ZoneType_RevealedStack")]
    RevealedStack,
    #[serde(rename = "ZoneType_RevealedBattlefield")]
    RevealedBattlefield,
    #[serde(rename = "ZoneType_RevealedCommand")]
    RevealedCommand,
    #[serde(rename = "ZoneType_Command")]
    Command,
    #[serde(rename = "ZoneType_RevealedCommandZone")]
    RevealedCommandZone,
    #[serde(rename = "ZoneType_RevealedTemporary")]
    RevealedTemporary,
    #[serde(rename = "ZoneType_Temporary")]
    Temporary,
    #[serde(rename = "ZoneType_RevealedTemporaryZone")]
    RevealedTemporaryZone,
    #[serde(rename = "ZoneType_RevealedToken")]
    RevealedToken,
    #[serde(rename = "ZoneType_Token")]
    Token,
    #[serde(rename = "ZoneType_RevealedTokenZone")]
    RevealedTokenZone,
    #[serde(rename = "ZoneType_RevealedPlayer")]
    RevealedPlayer,
    #[serde(rename = "ZoneType_Player")]
    Player,
    #[serde(rename = "ZoneType_RevealedPlayerZone")]
    RevealedPlayerZone,
}

impl Display for ZoneType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub enum SubZoneType {
    #[default]
    #[serde(rename = "SubZoneType_Top")]
    Top,
    #[serde(rename = "SubZoneType_Bottom")]
    Bottom,
}
