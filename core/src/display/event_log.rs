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
                Self {
                    icon: "\u{1F0CF}",
                    description: format!(
                        "{} plays {} ({})",
                        player_display(player),
                        card_display(card),
                        action_type
                    ),
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
            } => {
                let cat = category.as_ref().map_or(String::new(), |c| format!(" ({c})"));
                Self {
                    icon: "\u{27A1}",
                    description: format!("{}: {} \u{2192} {}{}", card_display(card), from_zone, to_zone, cat),
                    style: ActionStyle::Normal,
                }
            }

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
