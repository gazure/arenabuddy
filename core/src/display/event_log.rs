use crate::player_log::event_log::{CardRef, DamageTarget, GameAction, PlayerRef};

/// Semantic styling hint for UI rendering (UI-agnostic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStyle {
    /// Normal event
    Normal,
    /// Phase/turn transition (subtle/muted)
    Phase,
    /// Player's own action
    PlayerAction,
    /// Opponent's action
    OpponentAction,
    /// Attacking/offensive action
    Attack,
    /// Blocking/defensive action
    Defense,
    /// Damage dealt
    Damage,
    /// Positive change (life gain, counter added)
    Positive,
    /// Negative change (life loss, counter removed)
    Negative,
    /// Important/emphasized event (game over, concede)
    Emphasized,
}

/// Display-ready representation of a game action with presentation metadata.
#[derive(Debug, Clone)]
pub struct ActionDisplay {
    /// Unicode icon/emoji representing the action type
    pub icon: &'static str,
    /// Human-readable description of the action
    pub description: String,
    /// Semantic styling hint (UI framework maps this to actual styles)
    pub style: ActionStyle,
}

impl ActionDisplay {
    /// Convert a `GameAction` into a display-ready format.
    ///
    /// # Arguments
    /// * `action` - The game action to display
    /// * `controller_seat_id` - The seat ID of the player viewing the log (for context like "you" vs opponent)
    ///
    /// # Returns
    /// `Some(ActionDisplay)` if the action should be displayed, `None` if it should be hidden
    #[expect(clippy::too_many_lines)]
    pub fn from_game_action(action: &GameAction, controller_seat_id: i32) -> Option<Self> {
        let display = match action {
            GameAction::NewTurn { .. } => return None,

            GameAction::PhaseChange { phase, step } => {
                let step_str = step.map_or(String::new(), |s| format!(" - {s}"));
                Self {
                    icon: "\u{23F1}",
                    description: format!("{phase}{step_str}"),
                    style: ActionStyle::Phase,
                }
            }

            GameAction::CardPlayed {
                player,
                card,
                action_type,
            } => {
                let is_you = player.seat_id == controller_seat_id;
                let verb = action_type_verb(action_type);
                Self {
                    icon: "\u{1F0CF}",
                    description: format!("{} {} {}", player_display(player), verb, card_display(card)),
                    style: if is_you {
                        ActionStyle::PlayerAction
                    } else {
                        ActionStyle::OpponentAction
                    },
                }
            }

            GameAction::ZoneTransfer {
                card,
                from_zone,
                to_zone,
                category,
            } => zone_transfer_display(card, from_zone, to_zone, category.as_deref()),

            GameAction::AttackersDeclared { attackers } => {
                let names: Vec<String> = attackers.iter().map(|a| card_display(&a.card)).collect();
                Self {
                    icon: "\u{2694}",
                    description: format!("Attackers declared: {}", names.join(", ")),
                    style: ActionStyle::Attack,
                }
            }

            GameAction::BlockersDeclared { blockers } => {
                let names: Vec<String> = blockers.iter().map(|b| card_display(&b.card)).collect();
                Self {
                    icon: "\u{1F6E1}",
                    description: format!("Blockers declared: {}", names.join(", ")),
                    style: ActionStyle::Defense,
                }
            }

            GameAction::DamageDealt { source, target, amount } => Self {
                icon: "\u{26A1}",
                description: format!(
                    "{} deals {} damage to {}",
                    card_display(source),
                    amount,
                    damage_target_display(target)
                ),
                style: ActionStyle::Damage,
            },

            GameAction::LifeChanged {
                player,
                old_total,
                new_total,
                change,
            } => {
                let sign = if *change > 0 { "+" } else { "" };
                Self {
                    icon: "\u{2764}",
                    description: format!(
                        "{}: {} \u{2192} {} ({sign}{})",
                        player_display(player),
                        old_total,
                        new_total,
                        change
                    ),
                    style: if *change > 0 {
                        ActionStyle::Positive
                    } else {
                        ActionStyle::Negative
                    },
                }
            }

            GameAction::TokenCreated { card, controller } => Self {
                icon: "\u{2795}",
                description: format!("{} creates token: {}", player_display(controller), card_display(card)),
                style: ActionStyle::Normal,
            },

            GameAction::CounterAdded { card, counter_type } => {
                let ct = counter_type
                    .as_ref()
                    .map_or("counter".to_string(), |c| format!("{c} counter"));
                Self {
                    icon: "\u{2B06}",
                    description: format!("+1 {} on {}", ct, card_display(card)),
                    style: ActionStyle::Positive,
                }
            }

            GameAction::CounterRemoved { card, counter_type } => {
                let ct = counter_type
                    .as_ref()
                    .map_or("counter".to_string(), |c| format!("{c} counter"));
                Self {
                    icon: "\u{2B07}",
                    description: format!("-1 {} on {}", ct, card_display(card)),
                    style: ActionStyle::Negative,
                }
            }

            GameAction::GameOver { losing_player, reason } => {
                let reason_str = reason.as_ref().map_or(String::new(), |r| format!(" ({r})"));
                Self {
                    icon: "\u{1F3C1}",
                    description: format!("Game Over: {} loses{}", player_display(losing_player), reason_str),
                    style: ActionStyle::Emphasized,
                }
            }

            GameAction::PlayerConceded { player } => Self {
                icon: "\u{1F3F3}",
                description: format!("{} concedes", player_display(player)),
                style: ActionStyle::Emphasized,
            },
        };

        Some(display)
    }
}

/// Format a card reference for display.
pub fn card_display(card: &CardRef) -> String {
    card.name.clone().unwrap_or_else(|| {
        card.arena_id
            .map_or(format!("#{}", card.instance_id), |id| format!("Card #{}", id.inner()))
    })
}

/// Format a player reference for display.
pub fn player_display(player: &PlayerRef) -> String {
    player
        .name
        .clone()
        .unwrap_or_else(|| format!("Player {}", player.seat_id))
}

/// Format a damage target for display.
pub fn damage_target_display(target: &DamageTarget) -> String {
    match target {
        DamageTarget::Player { player } => player_display(player),
        DamageTarget::Permanent { card } => card_display(card),
    }
}

/// Map a raw `ActionType` debug string to a player-friendly MTG verb.
fn action_type_verb(action_type: &str) -> &'static str {
    match action_type {
        "Cast" | "CastAdventure" | "CastLeftRoom" | "CastRightRoom" | "CastLeft" | "CastRight" | "CastOmen" => "casts",
        "Activate" => "activates",
        "Special" | "SpecialTurnFaceUp" => "uses special ability on",
        _ => "plays",
    }
}

/// Check whether a category string contains a substring (case-insensitive).
fn category_contains(category: Option<&str>, needle: &str) -> bool {
    category.is_some_and(|c| c.to_ascii_lowercase().contains(&needle.to_ascii_lowercase()))
}

/// Map a zone transfer to player-friendly MTG terminology.
fn zone_transfer_display(card: &CardRef, from_zone: &str, to_zone: &str, category: Option<&str>) -> ActionDisplay {
    let name = card_display(card);

    match (from_zone, to_zone) {
        // Casting: Hand → Stack
        ("Hand", "Stack") => ActionDisplay {
            icon: "\u{1F0CF}",
            description: format!("{name} is cast"),
            style: ActionStyle::Normal,
        },
        // Drawing: Library → Hand
        ("Library", "Hand") => ActionDisplay {
            icon: "\u{1F4E5}",
            description: format!("{name} drawn"),
            style: ActionStyle::Normal,
        },
        // Resolving onto battlefield: Stack → Battlefield
        ("Stack", "Battlefield") => ActionDisplay {
            icon: "\u{2B07}",
            description: format!("{name} enters the battlefield"),
            style: ActionStyle::Normal,
        },
        // Countered: Stack → Graveyard with counter category
        ("Stack", "Graveyard") if category_contains(category, "counter") => ActionDisplay {
            icon: "\u{1F6AB}",
            description: format!("{name} is countered"),
            style: ActionStyle::Negative,
        },
        // Instant/sorcery resolves: Stack → Graveyard
        ("Stack", "Graveyard") => ActionDisplay {
            icon: "\u{2705}",
            description: format!("{name} resolves"),
            style: ActionStyle::Normal,
        },
        // Destroyed: Battlefield → Graveyard
        ("Battlefield", "Graveyard") if category_contains(category, "destroy") => ActionDisplay {
            icon: "\u{1F480}",
            description: format!("{name} is destroyed"),
            style: ActionStyle::Negative,
        },
        // Sacrificed: Battlefield → Graveyard
        ("Battlefield", "Graveyard") if category_contains(category, "sacrifice") => ActionDisplay {
            icon: "\u{1F480}",
            description: format!("{name} is sacrificed"),
            style: ActionStyle::Negative,
        },
        // Dies (generic): Battlefield → Graveyard
        ("Battlefield", "Graveyard") => ActionDisplay {
            icon: "\u{1F480}",
            description: format!("{name} dies"),
            style: ActionStyle::Negative,
        },
        // Discarded: Hand → Graveyard
        ("Hand", "Graveyard") => ActionDisplay {
            icon: "\u{274C}",
            description: format!("{name} discarded"),
            style: ActionStyle::Negative,
        },
        // Bounced: Battlefield → Hand
        ("Battlefield", "Hand") => ActionDisplay {
            icon: "\u{21A9}",
            description: format!("{name} returned to hand"),
            style: ActionStyle::Normal,
        },
        // Tucked: Battlefield → Library
        ("Battlefield", "Library") => ActionDisplay {
            icon: "\u{21A9}",
            description: format!("{name} put on top of library"),
            style: ActionStyle::Normal,
        },
        // Enters from library (e.g. ramp, Collected Company)
        ("Library", "Battlefield") => ActionDisplay {
            icon: "\u{2B07}",
            description: format!("{name} enters the battlefield from library"),
            style: ActionStyle::Normal,
        },
        // Milled: Library → Graveyard
        ("Library", "Graveyard") => ActionDisplay {
            icon: "\u{2B07}",
            description: format!("{name} milled"),
            style: ActionStyle::Normal,
        },
        // Recursion: Graveyard → Battlefield
        ("Graveyard", "Battlefield") => ActionDisplay {
            icon: "\u{21A9}",
            description: format!("{name} returns from graveyard"),
            style: ActionStyle::Positive,
        },
        // Graveyard → Hand
        ("Graveyard", "Hand") => ActionDisplay {
            icon: "\u{21A9}",
            description: format!("{name} returned to hand from graveyard"),
            style: ActionStyle::Positive,
        },
        // Enters from exile
        ("Exile", "Battlefield") => ActionDisplay {
            icon: "\u{2B07}",
            description: format!("{name} enters the battlefield from exile"),
            style: ActionStyle::Positive,
        },
        // Exile → Hand
        ("Exile", "Hand") => ActionDisplay {
            icon: "\u{21A9}",
            description: format!("{name} returned to hand from exile"),
            style: ActionStyle::Positive,
        },
        // Catch-all: anything → Exile
        (_, "Exile") => ActionDisplay {
            icon: "\u{1F6AB}",
            description: format!("{name} is exiled"),
            style: ActionStyle::Negative,
        },
        // Fallback: show raw zones for unmapped transitions
        _ => ActionDisplay {
            icon: "\u{27A1}",
            description: format!("{name}: {from_zone} \u{2192} {to_zone}"),
            style: ActionStyle::Normal,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ArenaId;

    fn named_card(name: &str) -> CardRef {
        CardRef {
            instance_id: 1,
            arena_id: Some(ArenaId::new(100)),
            name: Some(name.to_string()),
        }
    }

    fn unnamed_card_with_arena_id(arena_id: i32) -> CardRef {
        CardRef {
            instance_id: 42,
            arena_id: Some(ArenaId::new(arena_id)),
            name: None,
        }
    }

    fn unnamed_card_instance_only(instance_id: i32) -> CardRef {
        CardRef {
            instance_id,
            arena_id: None,
            name: None,
        }
    }

    fn named_player(name: &str, seat: i32) -> PlayerRef {
        PlayerRef {
            seat_id: seat,
            name: Some(name.to_string()),
        }
    }

    fn anonymous_player(seat: i32) -> PlayerRef {
        PlayerRef {
            seat_id: seat,
            name: None,
        }
    }

    // -- card_display ---------------------------------------------------------

    #[test]
    fn card_display_with_name() {
        assert_eq!(card_display(&named_card("Lightning Bolt")), "Lightning Bolt");
    }

    #[test]
    fn card_display_fallback_to_arena_id() {
        assert_eq!(card_display(&unnamed_card_with_arena_id(12345)), "Card #12345");
    }

    #[test]
    fn card_display_fallback_to_instance_id() {
        assert_eq!(card_display(&unnamed_card_instance_only(99)), "#99");
    }

    // -- player_display -------------------------------------------------------

    #[test]
    fn player_display_with_name() {
        assert_eq!(player_display(&named_player("Alice", 1)), "Alice");
    }

    #[test]
    fn player_display_without_name() {
        assert_eq!(player_display(&anonymous_player(2)), "Player 2");
    }

    // -- damage_target_display ------------------------------------------------

    #[test]
    fn damage_target_player() {
        let target = DamageTarget::Player {
            player: named_player("Bob", 2),
        };
        assert_eq!(damage_target_display(&target), "Bob");
    }

    #[test]
    fn damage_target_permanent() {
        let target = DamageTarget::Permanent {
            card: named_card("Grizzly Bears"),
        };
        assert_eq!(damage_target_display(&target), "Grizzly Bears");
    }

    // -- action_type_verb -----------------------------------------------------

    #[test]
    fn action_type_verb_cast_variants() {
        for variant in [
            "Cast",
            "CastAdventure",
            "CastLeftRoom",
            "CastRightRoom",
            "CastLeft",
            "CastRight",
            "CastOmen",
        ] {
            assert_eq!(action_type_verb(variant), "casts", "failed for {variant}");
        }
    }

    #[test]
    fn action_type_verb_activate() {
        assert_eq!(action_type_verb("Activate"), "activates");
    }

    #[test]
    fn action_type_verb_special() {
        assert_eq!(action_type_verb("Special"), "uses special ability on");
        assert_eq!(action_type_verb("SpecialTurnFaceUp"), "uses special ability on");
    }

    #[test]
    fn action_type_verb_unknown_defaults_to_plays() {
        assert_eq!(action_type_verb("SomethingNew"), "plays");
        assert_eq!(action_type_verb("Play"), "plays");
    }

    // -- category_contains ----------------------------------------------------

    #[test]
    fn category_contains_case_insensitive() {
        assert!(category_contains(Some("CounterSpell"), "counter"));
        assert!(category_contains(Some("DESTROY"), "destroy"));
    }

    #[test]
    fn category_contains_none() {
        assert!(!category_contains(None, "counter"));
    }

    // -- zone_transfer_display ------------------------------------------------

    #[test]
    fn zone_transfer_hand_to_stack_is_cast() {
        let d = zone_transfer_display(&named_card("Bolt"), "Hand", "Stack", None);
        assert!(d.description.contains("is cast"));
        assert_eq!(d.style, ActionStyle::Normal);
    }

    #[test]
    fn zone_transfer_library_to_hand_is_drawn() {
        let d = zone_transfer_display(&named_card("Island"), "Library", "Hand", None);
        assert!(d.description.contains("drawn"));
    }

    #[test]
    fn zone_transfer_stack_to_battlefield_enters() {
        let d = zone_transfer_display(&named_card("Bear"), "Stack", "Battlefield", None);
        assert!(d.description.contains("enters the battlefield"));
    }

    #[test]
    fn zone_transfer_stack_to_graveyard_countered() {
        let d = zone_transfer_display(&named_card("Spell"), "Stack", "Graveyard", Some("CounterSpell"));
        assert!(d.description.contains("is countered"));
        assert_eq!(d.style, ActionStyle::Negative);
    }

    #[test]
    fn zone_transfer_stack_to_graveyard_resolves() {
        let d = zone_transfer_display(&named_card("Bolt"), "Stack", "Graveyard", None);
        assert!(d.description.contains("resolves"));
    }

    #[test]
    fn zone_transfer_battlefield_to_graveyard_destroyed() {
        let d = zone_transfer_display(&named_card("Creature"), "Battlefield", "Graveyard", Some("Destroy"));
        assert!(d.description.contains("is destroyed"));
    }

    #[test]
    fn zone_transfer_battlefield_to_graveyard_sacrificed() {
        let d = zone_transfer_display(&named_card("Token"), "Battlefield", "Graveyard", Some("Sacrifice"));
        assert!(d.description.contains("is sacrificed"));
    }

    #[test]
    fn zone_transfer_battlefield_to_graveyard_dies() {
        let d = zone_transfer_display(&named_card("Creature"), "Battlefield", "Graveyard", None);
        assert!(d.description.contains("dies"));
    }

    #[test]
    fn zone_transfer_hand_to_graveyard_discarded() {
        let d = zone_transfer_display(&named_card("Card"), "Hand", "Graveyard", None);
        assert!(d.description.contains("discarded"));
    }

    #[test]
    fn zone_transfer_battlefield_to_hand_bounced() {
        let d = zone_transfer_display(&named_card("Perm"), "Battlefield", "Hand", None);
        assert!(d.description.contains("returned to hand"));
    }

    #[test]
    fn zone_transfer_battlefield_to_library_tucked() {
        let d = zone_transfer_display(&named_card("Card"), "Battlefield", "Library", None);
        assert!(d.description.contains("put on top of library"));
    }

    #[test]
    fn zone_transfer_library_to_battlefield_ramp() {
        let d = zone_transfer_display(&named_card("Land"), "Library", "Battlefield", None);
        assert!(d.description.contains("enters the battlefield from library"));
    }

    #[test]
    fn zone_transfer_library_to_graveyard_milled() {
        let d = zone_transfer_display(&named_card("Card"), "Library", "Graveyard", None);
        assert!(d.description.contains("milled"));
    }

    #[test]
    fn zone_transfer_graveyard_to_battlefield_recursion() {
        let d = zone_transfer_display(&named_card("Phoenix"), "Graveyard", "Battlefield", None);
        assert!(d.description.contains("returns from graveyard"));
        assert_eq!(d.style, ActionStyle::Positive);
    }

    #[test]
    fn zone_transfer_graveyard_to_hand() {
        let d = zone_transfer_display(&named_card("Card"), "Graveyard", "Hand", None);
        assert!(d.description.contains("returned to hand from graveyard"));
        assert_eq!(d.style, ActionStyle::Positive);
    }

    #[test]
    fn zone_transfer_exile_to_battlefield() {
        let d = zone_transfer_display(&named_card("Card"), "Exile", "Battlefield", None);
        assert!(d.description.contains("enters the battlefield from exile"));
        assert_eq!(d.style, ActionStyle::Positive);
    }

    #[test]
    fn zone_transfer_exile_to_hand() {
        let d = zone_transfer_display(&named_card("Card"), "Exile", "Hand", None);
        assert!(d.description.contains("returned to hand from exile"));
    }

    #[test]
    fn zone_transfer_anything_to_exile() {
        let d = zone_transfer_display(&named_card("Creature"), "Battlefield", "Exile", None);
        assert!(d.description.contains("is exiled"));
        assert_eq!(d.style, ActionStyle::Negative);
    }

    #[test]
    fn zone_transfer_fallback_shows_raw_zones() {
        let d = zone_transfer_display(&named_card("Card"), "Limbo", "Sideboard", None);
        assert!(d.description.contains("Limbo"));
        assert!(d.description.contains("Sideboard"));
    }

    // -- ActionDisplay::from_game_action --------------------------------------

    #[test]
    fn from_game_action_new_turn_returns_none() {
        let action = GameAction::NewTurn {
            turn_number: 1,
            active_player: named_player("Alice", 1),
        };
        assert!(ActionDisplay::from_game_action(&action, 1).is_none());
    }

    #[test]
    fn from_game_action_phase_change() {
        let action = GameAction::PhaseChange {
            phase: crate::events::primitives::Phase::Combat,
            step: Some(crate::events::primitives::Step::DeclareAttack),
        };
        let display = ActionDisplay::from_game_action(&action, 1).expect("should produce display");
        assert_eq!(display.style, ActionStyle::Phase);
        assert!(display.description.contains("Combat"));
    }

    #[test]
    fn from_game_action_card_played_by_controller() {
        let action = GameAction::CardPlayed {
            player: named_player("Me", 1),
            card: named_card("Lightning Bolt"),
            action_type: "Cast".to_string(),
        };
        let display = ActionDisplay::from_game_action(&action, 1).expect("should produce display");
        assert_eq!(display.style, ActionStyle::PlayerAction);
        assert!(display.description.contains("casts"));
        assert!(display.description.contains("Lightning Bolt"));
    }

    #[test]
    fn from_game_action_card_played_by_opponent() {
        let action = GameAction::CardPlayed {
            player: named_player("Opp", 2),
            card: named_card("Counterspell"),
            action_type: "Cast".to_string(),
        };
        let display = ActionDisplay::from_game_action(&action, 1).expect("should produce display");
        assert_eq!(display.style, ActionStyle::OpponentAction);
    }

    #[test]
    fn from_game_action_life_gained() {
        let action = GameAction::LifeChanged {
            player: named_player("Alice", 1),
            old_total: 20,
            new_total: 23,
            change: 3,
        };
        let display = ActionDisplay::from_game_action(&action, 1).expect("should produce display");
        assert_eq!(display.style, ActionStyle::Positive);
        assert!(display.description.contains("+3"));
    }

    #[test]
    fn from_game_action_life_lost() {
        let action = GameAction::LifeChanged {
            player: named_player("Alice", 1),
            old_total: 20,
            new_total: 17,
            change: -3,
        };
        let display = ActionDisplay::from_game_action(&action, 1).expect("should produce display");
        assert_eq!(display.style, ActionStyle::Negative);
        assert!(display.description.contains("-3"));
    }

    #[test]
    fn from_game_action_damage_dealt() {
        let action = GameAction::DamageDealt {
            source: named_card("Bolt"),
            target: DamageTarget::Player {
                player: named_player("Opp", 2),
            },
            amount: 3,
        };
        let display = ActionDisplay::from_game_action(&action, 1).expect("should produce display");
        assert_eq!(display.style, ActionStyle::Damage);
        assert!(display.description.contains("3 damage"));
    }

    #[test]
    fn from_game_action_game_over() {
        let action = GameAction::GameOver {
            losing_player: named_player("Opp", 2),
            reason: Some("life total".to_string()),
        };
        let display = ActionDisplay::from_game_action(&action, 1).expect("should produce display");
        assert_eq!(display.style, ActionStyle::Emphasized);
        assert!(display.description.contains("loses"));
        assert!(display.description.contains("life total"));
    }

    #[test]
    fn from_game_action_concede() {
        let action = GameAction::PlayerConceded {
            player: named_player("Opp", 2),
        };
        let display = ActionDisplay::from_game_action(&action, 1).expect("should produce display");
        assert_eq!(display.style, ActionStyle::Emphasized);
        assert!(display.description.contains("concedes"));
    }

    #[test]
    fn from_game_action_counter_added() {
        let action = GameAction::CounterAdded {
            card: named_card("Hydra"),
            counter_type: Some("+1/+1".to_string()),
        };
        let display = ActionDisplay::from_game_action(&action, 1).expect("should produce display");
        assert_eq!(display.style, ActionStyle::Positive);
        assert!(display.description.contains("+1/+1 counter"));
    }

    #[test]
    fn from_game_action_counter_removed_generic() {
        let action = GameAction::CounterRemoved {
            card: named_card("Saga"),
            counter_type: None,
        };
        let display = ActionDisplay::from_game_action(&action, 1).expect("should produce display");
        assert_eq!(display.style, ActionStyle::Negative);
        assert!(display.description.contains("counter"));
    }

    #[test]
    fn from_game_action_token_created() {
        let action = GameAction::TokenCreated {
            card: named_card("Soldier Token"),
            controller: named_player("Me", 1),
        };
        let display = ActionDisplay::from_game_action(&action, 1).expect("should produce display");
        assert!(display.description.contains("creates token"));
    }
}
