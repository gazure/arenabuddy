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
