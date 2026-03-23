use arenabuddy_core::display::match_summary::format_event_id;
use dioxus::prelude::*;

#[component]
pub fn MatchInfo(
    controller_player_name: String,
    opponent_player_name: String,
    did_controller_win: bool,
    format: Option<String>,
    #[props(optional)] controller_archetype: Option<String>,
    #[props(optional)] opponent_archetype: Option<String>,
) -> Element {
    let display_format = format.as_deref().map_or("Unknown", format_event_id);

    rsx! {
        div { class: "bg-gray-800 rounded-lg border border-gray-700 overflow-hidden",
            div { class: "bg-gradient-to-r from-amber-900 to-amber-800 py-4 px-6",
                h2 { class: "text-xl font-bold text-white", "Match Information" }
            }
            div { class: "p-6",
                div { class: "mb-4",
                    h3 { class: "text-lg font-semibold text-gray-300 mb-2", "Players" }
                    div { class: "flex flex-col gap-2",
                        div { class: "bg-blue-900/20 p-3 rounded-md",
                            span { class: "font-semibold", "You" }
                            " {controller_player_name}"
                            if let Some(ref archetype) = controller_archetype {
                                span { class: "ml-2 px-2 py-0.5 text-xs rounded-full bg-violet-900/40 text-violet-300",
                                    "{archetype}"
                                }
                            }
                        }
                        div { class: "bg-red-900/20 p-3 rounded-md",
                            span { class: "font-semibold", "Opponent" }
                            " {opponent_player_name}"
                            if let Some(ref archetype) = opponent_archetype {
                                span { class: "ml-2 px-2 py-0.5 text-xs rounded-full bg-violet-900/40 text-violet-300",
                                    "{archetype}"
                                }
                            }
                        }
                    }
                }

                div { class: "mb-4",
                    h3 { class: "text-lg font-semibold text-gray-300 mb-2", "Game Details" }
                    div { class: "grid grid-cols-2 gap-2",
                        div { class: "bg-gray-900/50 p-3 rounded-md",
                            span { class: "text-sm text-gray-500 block", "Format" }
                            span { class: "font-medium", "{display_format}" }
                        }
                        div { class: "bg-gray-900/50 p-3 rounded-md",
                            span { class: "text-sm text-gray-500 block", "Result" }
                            span { class: "font-medium",
                                if did_controller_win {
                                    span { class: "text-amber-400 font-bold", "Victory" }
                                } else {
                                    span { class: "text-red-400 font-bold", "Defeat" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
