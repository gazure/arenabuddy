use arenabuddy_core::player_log::event_log::{CardRef, DamageTarget, GameAction, GameEvent, GameEventLog, PlayerRef};
use dioxus::prelude::*;

// ---------------------------------------------------------------------------
// Top-level component
// ---------------------------------------------------------------------------

#[component]
pub fn EventLogDisplay(event_logs: Vec<GameEventLog>, controller_seat_id: i32) -> Element {
    let mut selected_game = use_signal(|| event_logs.first().map_or(0, |l| l.game_number));

    let game_numbers: Vec<i32> = event_logs.iter().map(|l| l.game_number).collect();

    let filtered: Vec<&GameEventLog> = if selected_game() == 0 {
        event_logs.iter().collect()
    } else {
        event_logs.iter().filter(|l| l.game_number == selected_game()).collect()
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow-md overflow-hidden",
            div { class: "bg-gradient-to-r from-emerald-500 to-emerald-600 py-4 px-6",
                h2 { class: "text-xl font-bold text-white", "Event Log" }
            }
            div { class: "p-6",
                if event_logs.is_empty() {
                    div { class: "text-center text-gray-500 py-8",
                        p { "No event log available for this match." }
                    }
                } else {
                    GameSelector {
                        game_numbers: game_numbers.clone(),
                        selected: selected_game(),
                        on_select: move |num: i32| selected_game.set(num),
                    }
                    for log in filtered {
                        GameTimeline {
                            event_log: log.clone(),
                            controller_seat_id,
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Game selector pills
// ---------------------------------------------------------------------------

#[component]
fn GameSelector(game_numbers: Vec<i32>, selected: i32, on_select: EventHandler<i32>) -> Element {
    rsx! {
        div { class: "flex flex-wrap gap-2 mb-4",
            button {
                class: if selected == 0 {
                    "px-3 py-1 rounded-full text-sm font-medium bg-emerald-500 text-white"
                } else {
                    "px-3 py-1 rounded-full text-sm font-medium bg-gray-200 text-gray-700 hover:bg-gray-300 transition-colors duration-150"
                },
                onclick: move |_| on_select.call(0),
                "All Games"
            }
            for num in game_numbers {
                button {
                    class: if selected == num {
                        "px-3 py-1 rounded-full text-sm font-medium bg-emerald-500 text-white"
                    } else {
                        "px-3 py-1 rounded-full text-sm font-medium bg-gray-200 text-gray-700 hover:bg-gray-300 transition-colors duration-150"
                    },
                    onclick: move |_| on_select.call(num),
                    "Game {num}"
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Per-game timeline
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct TurnEventGroup {
    turn_number: i32,
    active_player_seat: Option<i32>,
    active_player_name: Option<String>,
    events: Vec<GameEvent>,
}

fn group_events_by_turn(events: &[GameEvent]) -> Vec<TurnEventGroup> {
    let mut groups: Vec<TurnEventGroup> = Vec::new();
    let mut current_turn: i32 = 0;

    for event in events {
        let event_turn = event.turn.as_ref().map_or(current_turn, |t| t.turn_number);

        if groups.is_empty() || event_turn != current_turn {
            current_turn = event_turn;
            let (seat, name) = event.turn.as_ref().map_or((None, None), |t| {
                (Some(t.active_player.seat_id), t.active_player.name.clone())
            });
            groups.push(TurnEventGroup {
                turn_number: event_turn,
                active_player_seat: seat,
                active_player_name: name,
                events: Vec::new(),
            });
        }

        // Skip NewTurn actions — the turn header conveys this
        if matches!(event.action, GameAction::NewTurn { .. }) {
            continue;
        }

        if let Some(group) = groups.last_mut() {
            group.events.push(event.clone());
        }
    }

    // Remove empty groups (e.g. a turn that only had a NewTurn event)
    groups.retain(|g| !g.events.is_empty());
    groups
}

#[component]
fn GameTimeline(event_log: GameEventLog, controller_seat_id: i32) -> Element {
    let groups = group_events_by_turn(&event_log.events);

    rsx! {
        div { class: "mb-6",
            h3 { class: "text-lg font-semibold text-gray-700 mb-3 border-b pb-2",
                "Game {event_log.game_number}"
            }
            div { class: "space-y-2",
                for group in groups {
                    TurnGroup { group, controller_seat_id }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Collapsible turn group
// ---------------------------------------------------------------------------

#[component]
fn TurnGroup(group: TurnEventGroup, controller_seat_id: i32) -> Element {
    let mut expanded = use_signal(|| true);

    let is_your_turn = group.active_player_seat.is_some_and(|s| s == controller_seat_id);

    let turn_label = if group.turn_number == 0 {
        "Pre-game".to_string()
    } else {
        format!("Turn {}", group.turn_number)
    };

    let player_label = match &group.active_player_name {
        Some(name) if is_your_turn => format!("{name} (you)"),
        Some(name) => name.clone(),
        None if is_your_turn => "Your turn".to_string(),
        None => "Opponent's turn".to_string(),
    };

    let header_bg = if is_your_turn {
        "bg-blue-50 hover:bg-blue-100"
    } else {
        "bg-red-50 hover:bg-red-100"
    };

    let chevron_class = if expanded() {
        "transform rotate-180 transition-transform duration-200"
    } else {
        "transform rotate-0 transition-transform duration-200"
    };

    rsx! {
        div { class: "border rounded-lg overflow-hidden",
            div {
                class: "px-4 py-2 cursor-pointer flex justify-between items-center {header_bg} transition-colors duration-150",
                onclick: move |_| {
                    let current = expanded();
                    expanded.set(!current);
                },
                div { class: "flex items-center gap-2",
                    span { class: "font-semibold text-gray-700", "{turn_label}" }
                    span { class: "text-sm text-gray-500", "({player_label})" }
                    span { class: "px-2 py-0.5 text-xs rounded-full bg-gray-200 text-gray-600",
                        "{group.events.len()} events"
                    }
                }
                svg {
                    xmlns: "http://www.w3.org/2000/svg",
                    class: "h-4 w-4 text-gray-500 {chevron_class}",
                    fill: "none",
                    view_box: "0 0 24 24",
                    stroke: "currentColor",
                    path {
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        stroke_width: "2",
                        d: "M19 9l-7 7-7-7"
                    }
                }
            }
            if expanded() {
                div { class: "divide-y divide-gray-100",
                    for event in &group.events {
                        EventRow { event: event.clone(), controller_seat_id }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Single event row
// ---------------------------------------------------------------------------

fn card_display(card: &CardRef) -> String {
    card.name.clone().unwrap_or_else(|| {
        card.arena_id
            .map_or(format!("#{}", card.instance_id), |id| format!("Card #{}", id.inner()))
    })
}

fn player_display(player: &PlayerRef) -> String {
    player
        .name
        .clone()
        .unwrap_or_else(|| format!("Player {}", player.seat_id))
}

fn damage_target_display(target: &DamageTarget) -> String {
    match target {
        DamageTarget::Player { player } => player_display(player),
        DamageTarget::Permanent { card } => card_display(card),
    }
}

struct ActionDisplay {
    icon: &'static str,
    description: String,
    row_class: &'static str,
}

#[expect(clippy::too_many_lines)]
fn render_action(action: &GameAction, controller_seat_id: i32) -> Option<ActionDisplay> {
    let display = match action {
        GameAction::NewTurn { .. } => return None,

        GameAction::PhaseChange { phase, step } => {
            let step_str = step.map_or(String::new(), |s| format!(" - {s}"));
            ActionDisplay {
                icon: "\u{23F1}",
                description: format!("{phase}{step_str}"),
                row_class: "text-gray-400 text-xs italic",
            }
        }

        GameAction::CardPlayed {
            player,
            card,
            action_type,
        } => {
            let is_you = player.seat_id == controller_seat_id;
            ActionDisplay {
                icon: "\u{1F0CF}",
                description: format!(
                    "{} plays {} ({})",
                    player_display(player),
                    card_display(card),
                    action_type
                ),
                row_class: if is_you { "text-blue-800" } else { "text-red-800" },
            }
        }

        GameAction::ZoneTransfer {
            card,
            from_zone,
            to_zone,
            category,
        } => {
            let cat = category.as_ref().map_or(String::new(), |c| format!(" ({c})"));
            ActionDisplay {
                icon: "\u{27A1}",
                description: format!("{}: {} \u{2192} {}{}", card_display(card), from_zone, to_zone, cat),
                row_class: "",
            }
        }

        GameAction::AttackersDeclared { attackers } => {
            let names: Vec<String> = attackers.iter().map(|a| card_display(&a.card)).collect();
            ActionDisplay {
                icon: "\u{2694}",
                description: format!("Attackers declared: {}", names.join(", ")),
                row_class: "text-red-700",
            }
        }

        GameAction::BlockersDeclared { blockers } => {
            let names: Vec<String> = blockers.iter().map(|b| card_display(&b.card)).collect();
            ActionDisplay {
                icon: "\u{1F6E1}",
                description: format!("Blockers declared: {}", names.join(", ")),
                row_class: "text-blue-700",
            }
        }

        GameAction::DamageDealt { source, target, amount } => ActionDisplay {
            icon: "\u{26A1}",
            description: format!(
                "{} deals {} damage to {}",
                card_display(source),
                amount,
                damage_target_display(target)
            ),
            row_class: "text-orange-700",
        },

        GameAction::LifeChanged {
            player,
            old_total,
            new_total,
            change,
        } => {
            let sign = if *change > 0 { "+" } else { "" };
            ActionDisplay {
                icon: "\u{2764}",
                description: format!(
                    "{}: {} \u{2192} {} ({sign}{})",
                    player_display(player),
                    old_total,
                    new_total,
                    change
                ),
                row_class: if *change > 0 { "text-green-700" } else { "text-red-700" },
            }
        }

        GameAction::TokenCreated { card, controller } => ActionDisplay {
            icon: "\u{2795}",
            description: format!("{} creates token: {}", player_display(controller), card_display(card)),
            row_class: "",
        },

        GameAction::CounterAdded { card, counter_type } => {
            let ct = counter_type
                .as_ref()
                .map_or("counter".to_string(), |c| format!("{c} counter"));
            ActionDisplay {
                icon: "\u{2B06}",
                description: format!("+1 {} on {}", ct, card_display(card)),
                row_class: "text-green-700",
            }
        }

        GameAction::CounterRemoved { card, counter_type } => {
            let ct = counter_type
                .as_ref()
                .map_or("counter".to_string(), |c| format!("{c} counter"));
            ActionDisplay {
                icon: "\u{2B07}",
                description: format!("-1 {} on {}", ct, card_display(card)),
                row_class: "text-red-700",
            }
        }

        GameAction::GameOver { losing_player, reason } => {
            let reason_str = reason.as_ref().map_or(String::new(), |r| format!(" ({r})"));
            ActionDisplay {
                icon: "\u{1F3C1}",
                description: format!("Game Over: {} loses{}", player_display(losing_player), reason_str),
                row_class: "font-semibold",
            }
        }

        GameAction::PlayerConceded { player } => ActionDisplay {
            icon: "\u{1F3F3}",
            description: format!("{} concedes", player_display(player)),
            row_class: "font-semibold",
        },
    };

    Some(display)
}

#[component]
fn EventRow(event: GameEvent, controller_seat_id: i32) -> Element {
    let Some(display) = render_action(&event.action, controller_seat_id) else {
        return rsx! {};
    };

    let phase_badge = event.turn.as_ref().and_then(|t| {
        t.phase.map(|p| {
            let step_str = t.step.map_or(String::new(), |s| format!(" - {s}"));
            format!("{p}{step_str}")
        })
    });

    // PhaseChange events render as subtle dividers
    if matches!(event.action, GameAction::PhaseChange { .. }) {
        return rsx! {
            div { class: "px-4 py-1 {display.row_class}",
                "{display.icon} {display.description}"
            }
        };
    }

    rsx! {
        div { class: "px-4 py-2 flex items-center gap-3 text-sm {display.row_class}",
            span { class: "flex-shrink-0 w-6 text-center", "{display.icon}" }
            span { class: "flex-grow", "{display.description}" }
            if let Some(badge) = phase_badge {
                span { class: "flex-shrink-0 px-2 py-0.5 text-xs rounded-full bg-gray-100 text-gray-500",
                    "{badge}"
                }
            }
        }
    }
}
