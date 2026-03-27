#![expect(clippy::too_many_lines)]
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    cards::CardsDatabase,
    events::{
        Event,
        client::{ActionType, ClientMessage},
        gre::GREToClientMessage,
        primitives::{Annotation, AnnotationType, Phase, Step, TurnInfo, ZoneType},
    },
    models::ArenaId,
};

// ---------------------------------------------------------------------------
// Output types (all Serialize for JSON)
// ---------------------------------------------------------------------------

/// One event log per game within a match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameEventLog {
    pub game_number: i32,
    pub events: Vec<GameEvent>,
}

/// A single structured event in the game log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameEvent {
    pub game_state_id: i32,
    pub turn: Option<TurnContext>,
    pub action: GameAction,
}

/// Turn/phase context at the time of the event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnContext {
    pub turn_number: i32,
    pub active_player: PlayerRef,
    pub phase: Option<Phase>,
    pub step: Option<Step>,
}

/// Reference to a player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerRef {
    pub seat_id: i32,
    pub name: Option<String>,
}

/// Reference to a card with resolved name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardRef {
    pub instance_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arena_id: Option<ArenaId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Info about a single attacker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttackerInfo {
    pub card: CardRef,
    pub target: DamageTarget,
}

/// Info about a single blocker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockerInfo {
    pub card: CardRef,
    pub blocking: Vec<CardRef>,
}

/// Target of damage — either a player or a permanent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DamageTarget {
    Player { player: PlayerRef },
    Permanent { card: CardRef },
}

/// The core action enum — what happened.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GameAction {
    // Turn structure
    NewTurn {
        turn_number: i32,
        active_player: PlayerRef,
    },
    PhaseChange {
        phase: Phase,
        step: Option<Step>,
    },

    // Card actions (from client PerformActionResp)
    CardPlayed {
        player: PlayerRef,
        card: CardRef,
        action_type: String,
    },

    // Zone transitions (from ZoneTransfer annotations)
    ZoneTransfer {
        card: CardRef,
        from_zone: String,
        to_zone: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        category: Option<String>,
    },

    // Combat
    AttackersDeclared {
        attackers: Vec<AttackerInfo>,
    },
    BlockersDeclared {
        blockers: Vec<BlockerInfo>,
    },
    DamageDealt {
        source: CardRef,
        target: DamageTarget,
        amount: i32,
    },

    // Life total changes
    LifeChanged {
        player: PlayerRef,
        old_total: i32,
        new_total: i32,
        change: i32,
    },

    // Tokens
    TokenCreated {
        card: CardRef,
        controller: PlayerRef,
    },

    // Counters
    CounterAdded {
        card: CardRef,
        #[serde(skip_serializing_if = "Option::is_none")]
        counter_type: Option<String>,
    },
    CounterRemoved {
        card: CardRef,
        #[serde(skip_serializing_if = "Option::is_none")]
        counter_type: Option<String>,
    },

    // Game end
    GameOver {
        losing_player: PlayerRef,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    PlayerConceded {
        player: PlayerRef,
    },
}

// ---------------------------------------------------------------------------
// Internal tracking state
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct GameStateTracker {
    /// `instance_id` -> (`grp_id`, `owner_seat_id`). Never pruned.
    instance_map: HashMap<i32, (ArenaId, i32)>,
    /// `zone_id` -> (`ZoneType`, `owner_seat_id`)
    zone_map: HashMap<i32, (ZoneType, Option<i32>)>,
    /// player `seat_id` -> `life_total`
    life_totals: HashMap<i32, i32>,
    /// Current turn context
    current_turn: Option<TurnContext>,
    /// Current `game_state_id`
    current_game_state_id: i32,
}

impl GameStateTracker {
    fn reset_for_new_game(&mut self) {
        self.life_totals.clear();
        self.zone_map.clear();
        self.current_turn = None;
        self.current_game_state_id = 0;
        // Keep instance_map — will be repopulated by next game's full GSM
    }
}

// ---------------------------------------------------------------------------
// EventLogBuilder
// ---------------------------------------------------------------------------

pub struct EventLogBuilder<'a> {
    controller_seat_id: i32,
    cards_db: &'a CardsDatabase,
    player_names: HashMap<i32, String>,
    tracker: GameStateTracker,
}

impl<'a> EventLogBuilder<'a> {
    pub fn new(controller_seat_id: i32, cards_db: &'a CardsDatabase, player_names: HashMap<i32, String>) -> Self {
        Self {
            controller_seat_id,
            cards_db,
            player_names,
            tracker: GameStateTracker::default(),
        }
    }

    pub fn build(mut self, events: &[Event]) -> Vec<GameEventLog> {
        let mut game_logs: Vec<GameEventLog> = Vec::new();
        let mut current_events: Vec<GameEvent> = Vec::new();
        let mut game_number: i32 = 1;

        for event in events {
            match event {
                Event::GRE(gre_event) => {
                    for msg in &gre_event.gre_to_client_event.gre_to_client_messages {
                        match msg {
                            GREToClientMessage::GameStateMessage(wrapper) => {
                                let gsm = &wrapper.game_state_message;
                                self.tracker.current_game_state_id = gsm.game_state_id;
                                self.process_game_state_message(gsm, &mut current_events);
                            }
                            GREToClientMessage::IntermissionReq(_) => {
                                game_logs.push(GameEventLog {
                                    game_number,
                                    events: std::mem::take(&mut current_events),
                                });
                                game_number += 1;
                                self.tracker.reset_for_new_game();
                            }
                            _ => {}
                        }
                    }
                }
                Event::Client(client_msg) => {
                    self.process_client_message(&client_msg.payload, &mut current_events);
                }
                _ => {}
            }
        }

        // Push final game if there are remaining events
        if !current_events.is_empty() {
            game_logs.push(GameEventLog {
                game_number,
                events: current_events,
            });
        }

        game_logs
    }

    // -- GSM processing -----------------------------------------------------

    fn process_game_state_message(&mut self, gsm: &crate::events::gre::GameStateMessage, events: &mut Vec<GameEvent>) {
        // 1. Update instance map from game_objects (BEFORE annotations)
        for go in &gsm.game_objects {
            if let Some(grp_id) = go.grp_id {
                self.tracker
                    .instance_map
                    .insert(go.instance_id, (grp_id, go.owner_seat_id));
            }
        }

        // 2. Update zone map
        for zone in &gsm.zones {
            self.tracker
                .zone_map
                .insert(zone.zone_id, (zone.type_field, zone.owner_seat_id));
        }

        // 3. Life total diffs
        for player in &gsm.players {
            let seat_id = player.controller_seat_id;
            let new_life = player.life_total;
            if let Some(&old_life) = self.tracker.life_totals.get(&seat_id)
                && old_life != new_life
            {
                events.push(self.make_event(GameAction::LifeChanged {
                    player: self.player_ref(seat_id),
                    old_total: old_life,
                    new_total: new_life,
                    change: new_life - old_life,
                }));
            }
            self.tracker.life_totals.insert(seat_id, new_life);
        }

        // 4. Turn info diffs
        if let Some(turn_info) = &gsm.turn_info {
            self.process_turn_info(turn_info, events);
        }

        // 5. Process annotations
        for annotation in &gsm.annotations {
            self.process_annotation(annotation, events);
        }
    }

    fn process_turn_info(&mut self, turn_info: &TurnInfo, events: &mut Vec<GameEvent>) {
        let current = &self.tracker.current_turn;

        let new_turn_number = turn_info.turn_number.unwrap_or(0);
        let active_player = turn_info.active_player.unwrap_or(0);

        // Detect new turn
        let is_new_turn = match current {
            Some(ctx) => ctx.turn_number != new_turn_number,
            None => new_turn_number > 0,
        };

        if is_new_turn && new_turn_number > 0 {
            events.push(self.make_event(GameAction::NewTurn {
                turn_number: new_turn_number,
                active_player: self.player_ref(active_player),
            }));
        }

        // Detect phase/step change
        let phase_changed = match current {
            Some(ctx) => ctx.phase != turn_info.phase || ctx.step != turn_info.step,
            None => turn_info.phase.is_some(),
        };

        if phase_changed && let Some(phase) = turn_info.phase {
            events.push(self.make_event(GameAction::PhaseChange {
                phase,
                step: turn_info.step,
            }));
        }

        // Update tracked turn context
        self.tracker.current_turn = Some(TurnContext {
            turn_number: new_turn_number,
            active_player: self.player_ref(active_player),
            phase: turn_info.phase,
            step: turn_info.step,
        });
    }

    // -- Annotation processing ----------------------------------------------

    fn process_annotation(&self, annotation: &Annotation, events: &mut Vec<GameEvent>) {
        for ann_type in &annotation.type_field {
            match ann_type {
                AnnotationType::ZoneTransfer => {
                    self.process_zone_transfer(annotation, events);
                }
                AnnotationType::DamageDealt => {
                    self.process_damage_dealt(annotation, events);
                }
                AnnotationType::LossOfGame => {
                    self.process_loss_of_game(annotation, events);
                }
                AnnotationType::TokenCreated => {
                    self.process_token_created(annotation, events);
                }
                AnnotationType::CounterAdded => {
                    self.process_counter_change(annotation, true, events);
                }
                AnnotationType::CounterRemoved => {
                    self.process_counter_change(annotation, false, events);
                }
                _ => {}
            }
        }
    }

    fn process_zone_transfer(&self, annotation: &Annotation, events: &mut Vec<GameEvent>) {
        let mut from_zone_id: Option<i32> = None;
        let mut to_zone_id: Option<i32> = None;
        let mut category: Option<String> = None;

        for detail in &annotation.details {
            match detail.key.as_str() {
                "zone_src" => {
                    from_zone_id = detail.value_int32.first().copied();
                }
                "zone_dest" => {
                    to_zone_id = detail.value_int32.first().copied();
                }
                "category" => {
                    category = detail.value_string.first().cloned();
                }
                _ => {}
            }
        }

        let from_zone = from_zone_id
            .and_then(|id| self.tracker.zone_map.get(&id))
            .map_or_else(|| "Unknown".to_string(), |(zt, _)| format!("{zt}"));

        let to_zone = to_zone_id
            .and_then(|id| self.tracker.zone_map.get(&id))
            .map_or_else(|| "Unknown".to_string(), |(zt, _)| format!("{zt}"));

        for &instance_id in &annotation.affected_ids {
            events.push(self.make_event(GameAction::ZoneTransfer {
                card: self.resolve_card(instance_id),
                from_zone: from_zone.clone(),
                to_zone: to_zone.clone(),
                category: category.clone(),
            }));
        }
    }

    fn process_damage_dealt(&self, annotation: &Annotation, events: &mut Vec<GameEvent>) {
        let mut amount = 0;
        for detail in &annotation.details {
            if detail.key == "damage" {
                amount = detail.value_int32.first().copied().unwrap_or(0);
            }
        }

        let source_instance_id = annotation
            .affector_id
            .and_then(|id| i32::try_from(id).ok())
            .unwrap_or(0);
        let source = self.resolve_card(source_instance_id);

        for &target_id in &annotation.affected_ids {
            // Check if the target is a player (seat ID) or a permanent (instance ID).
            // Player seat IDs are typically 1 or 2; instance IDs are much larger.
            // We also check if the target_id is in our life_totals map as a player indicator.
            let target = if self.tracker.life_totals.contains_key(&target_id) {
                DamageTarget::Player {
                    player: self.player_ref(target_id),
                }
            } else {
                DamageTarget::Permanent {
                    card: self.resolve_card(target_id),
                }
            };

            events.push(self.make_event(GameAction::DamageDealt {
                source: source.clone(),
                target,
                amount,
            }));
        }
    }

    fn process_loss_of_game(&self, annotation: &Annotation, events: &mut Vec<GameEvent>) {
        let mut reason: Option<String> = None;
        for detail in &annotation.details {
            if detail.key == "reason" {
                reason = detail.value_string.first().cloned();
            }
        }

        for &seat_id in &annotation.affected_ids {
            events.push(self.make_event(GameAction::GameOver {
                losing_player: self.player_ref(seat_id),
                reason: reason.clone(),
            }));
        }
    }

    fn process_token_created(&self, annotation: &Annotation, events: &mut Vec<GameEvent>) {
        for &instance_id in &annotation.affected_ids {
            let controller_seat = self
                .tracker
                .instance_map
                .get(&instance_id)
                .map_or(0, |(_, owner)| *owner);

            events.push(self.make_event(GameAction::TokenCreated {
                card: self.resolve_card(instance_id),
                controller: self.player_ref(controller_seat),
            }));
        }
    }

    fn process_counter_change(&self, annotation: &Annotation, added: bool, events: &mut Vec<GameEvent>) {
        let mut counter_type: Option<String> = None;
        for detail in &annotation.details {
            if detail.key == "counterType" || detail.key == "type" {
                counter_type = detail
                    .value_string
                    .first()
                    .cloned()
                    .or_else(|| detail.value_int32.first().map(|v| format!("counter_{v}")));
            }
        }

        for &instance_id in &annotation.affected_ids {
            let action = if added {
                GameAction::CounterAdded {
                    card: self.resolve_card(instance_id),
                    counter_type: counter_type.clone(),
                }
            } else {
                GameAction::CounterRemoved {
                    card: self.resolve_card(instance_id),
                    counter_type: counter_type.clone(),
                }
            };
            events.push(self.make_event(action));
        }
    }

    // -- Client message processing ------------------------------------------

    fn process_client_message(&self, payload: &ClientMessage, events: &mut Vec<GameEvent>) {
        match payload {
            ClientMessage::PerformActionResp(wrapper) => {
                for action in &wrapper.perform_action_resp.actions {
                    // Skip non-interesting action types
                    match action.action_type {
                        ActionType::Pass
                        | ActionType::MakePayment
                        | ActionType::ActivateMana
                        | ActionType::SpellPayment
                        | ActionType::OpeningHandAction => continue,
                        _ => {}
                    }

                    let card = if let Some(instance_id) = action.instance_id {
                        let mut card_ref = self.resolve_card(instance_id);
                        // If grp_id is directly on the action and we didn't resolve a name,
                        // try resolving from grp_id
                        if card_ref.name.is_none()
                            && let Some(grp_id) = action.grp_id
                        {
                            let arena_id = ArenaId::from(grp_id);
                            card_ref.arena_id = Some(arena_id);
                            card_ref.name = self.cards_db.get_pretty_name(&grp_id.to_string());
                        }
                        card_ref
                    } else if let Some(grp_id) = action.grp_id {
                        let arena_id = ArenaId::from(grp_id);
                        CardRef {
                            instance_id: 0,
                            arena_id: Some(arena_id),
                            name: self.cards_db.get_pretty_name(&grp_id.to_string()),
                        }
                    } else {
                        continue;
                    };

                    let action_type_str = format!("{:?}", action.action_type);

                    events.push(self.make_event(GameAction::CardPlayed {
                        player: self.player_ref(self.controller_seat_id),
                        card,
                        action_type: action_type_str,
                    }));
                }
            }
            ClientMessage::DeclareAttackersResp(wrapper) => {
                let resp = &wrapper.declare_attackers_resp;
                if resp.selected_attackers.is_empty() {
                    return;
                }

                let attackers: Vec<AttackerInfo> = resp
                    .selected_attackers
                    .iter()
                    .map(|attacker| {
                        let target = if let Some(recipient) = &attacker.selected_damage_recipient {
                            if let Some(pw_id) = recipient.planswalker_instance_id {
                                DamageTarget::Permanent {
                                    card: self.resolve_card(pw_id),
                                }
                            } else if let Some(player_seat) = recipient.player_system_seat_id {
                                DamageTarget::Player {
                                    player: self.player_ref(player_seat),
                                }
                            } else {
                                DamageTarget::Player {
                                    player: self.player_ref(0),
                                }
                            }
                        } else {
                            DamageTarget::Player {
                                player: self.player_ref(0),
                            }
                        };

                        AttackerInfo {
                            card: self.resolve_card(attacker.attacker_instance_id),
                            target,
                        }
                    })
                    .collect();

                events.push(self.make_event(GameAction::AttackersDeclared { attackers }));
            }
            ClientMessage::DeclareBlockersResp(wrapper) => {
                let resp = &wrapper.declare_blockers_resp;
                if resp.selected_blockers.is_empty() {
                    return;
                }

                let blockers: Vec<BlockerInfo> = resp
                    .selected_blockers
                    .iter()
                    .map(|blocker| BlockerInfo {
                        card: self.resolve_card(blocker.blocker_instance_id),
                        blocking: blocker
                            .selected_attacker_instance_ids
                            .iter()
                            .map(|&id| self.resolve_card(id))
                            .collect(),
                    })
                    .collect();

                events.push(self.make_event(GameAction::BlockersDeclared { blockers }));
            }
            ClientMessage::ConcedeReq(_) => {
                events.push(self.make_event(GameAction::PlayerConceded {
                    player: self.player_ref(self.controller_seat_id),
                }));
            }
            _ => {}
        }
    }

    // -- Helpers ------------------------------------------------------------

    fn resolve_card(&self, instance_id: i32) -> CardRef {
        if let Some((grp_id, _owner)) = self.tracker.instance_map.get(&instance_id) {
            let name = self.cards_db.get_pretty_name(&grp_id.to_string());
            CardRef {
                instance_id,
                arena_id: Some(*grp_id),
                name,
            }
        } else {
            debug!("Could not resolve instance_id {instance_id} to a card");
            CardRef {
                instance_id,
                arena_id: None,
                name: None,
            }
        }
    }

    fn player_ref(&self, seat_id: i32) -> PlayerRef {
        PlayerRef {
            seat_id,
            name: self.player_names.get(&seat_id).cloned(),
        }
    }

    fn make_event(&self, action: GameAction) -> GameEvent {
        GameEvent {
            game_state_id: self.tracker.current_game_state_id,
            turn: self.tracker.current_turn.clone(),
            action,
        }
    }
}
