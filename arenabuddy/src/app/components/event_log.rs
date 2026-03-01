use arenabuddy_core::{
    display::event_log::{ActionDisplay, ActionStyle},
    player_log::event_log::{GameAction, GameEvent, GameEventLog},
};
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
        div { class: "bg-gray-800 rounded-lg border border-gray-700 overflow-hidden",
            div { class: "bg-gradient-to-r from-emerald-900 to-emerald-800 py-4 px-6",
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
                    "px-3 py-1 rounded-full text-sm font-medium bg-emerald-600 text-white"
                } else {
                    "px-3 py-1 rounded-full text-sm font-medium bg-gray-700 text-gray-300 hover:bg-gray-600 transition-colors duration-150"
                },
                onclick: move |_| on_select.call(0),
                "All Games"
            }
            for num in game_numbers {
                button {
                    class: if selected == num {
                        "px-3 py-1 rounded-full text-sm font-medium bg-emerald-600 text-white"
                    } else {
                        "px-3 py-1 rounded-full text-sm font-medium bg-gray-700 text-gray-300 hover:bg-gray-600 transition-colors duration-150"
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
            h3 { class: "text-lg font-semibold text-gray-300 mb-3 border-b border-gray-700 pb-2",
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
        "bg-blue-900/20 hover:bg-blue-900/30"
    } else {
        "bg-red-900/20 hover:bg-red-900/30"
    };

    let chevron_class = if expanded() {
        "transform rotate-180 transition-transform duration-200"
    } else {
        "transform rotate-0 transition-transform duration-200"
    };

    rsx! {
        div { class: "border border-gray-700 rounded-lg overflow-hidden",
            div {
                class: "px-4 py-2 cursor-pointer flex justify-between items-center {header_bg} transition-colors duration-150",
                onclick: move |_| {
                    let current = expanded();
                    expanded.set(!current);
                },
                div { class: "flex items-center gap-2",
                    span { class: "font-semibold text-gray-300", "{turn_label}" }
                    span { class: "text-sm text-gray-500", "({player_label})" }
                    span { class: "px-2 py-0.5 text-xs rounded-full bg-gray-700 text-gray-400",
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
                div { class: "divide-y divide-gray-700",
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

/// Map semantic `ActionStyle` to Tailwind CSS classes
fn style_to_css(style: ActionStyle) -> &'static str {
    match style {
        ActionStyle::Normal => "",
        ActionStyle::Phase => "text-gray-500 text-xs italic",
        ActionStyle::PlayerAction => "text-blue-300",
        ActionStyle::OpponentAction => "text-red-300",
        ActionStyle::Attack | ActionStyle::Negative => "text-red-400",
        ActionStyle::Defense => "text-blue-400",
        ActionStyle::Damage => "text-orange-400",
        ActionStyle::Positive => "text-emerald-400",
        ActionStyle::Emphasized => "font-semibold",
    }
}

#[component]
fn EventRow(event: GameEvent, controller_seat_id: i32) -> Element {
    let Some(display) = ActionDisplay::from_game_action(&event.action, controller_seat_id) else {
        return rsx! {};
    };

    let phase_badge = event.turn.as_ref().and_then(|t| {
        t.phase.map(|p| {
            let step_str = t.step.map_or(String::new(), |s| format!(" - {s}"));
            format!("{p}{step_str}")
        })
    });

    let css_class = style_to_css(display.style);

    // PhaseChange events render as subtle dividers
    if matches!(event.action, GameAction::PhaseChange { .. }) {
        return rsx! {
            div { class: "px-4 py-1 {css_class}",
                "{display.icon} {display.description}"
            }
        };
    }

    rsx! {
        div { class: "px-4 py-2 flex items-center gap-3 text-sm {css_class}",
            span { class: "flex-shrink-0 w-6 text-center", "{display.icon}" }
            span { class: "flex-grow", "{display.description}" }
            if let Some(badge) = phase_badge {
                span { class: "flex-shrink-0 px-2 py-0.5 text-xs rounded-full bg-gray-700 text-gray-400",
                    "{badge}"
                }
            }
        }
    }
}
