use arenabuddy_core::display::mulligan::Mulligan;
use dioxus::prelude::*;

#[component]
pub fn MulliganDisplay(mulligans: Vec<Mulligan>) -> Element {
    rsx! {
        div { class: "bg-gray-800 rounded-lg border border-gray-700 overflow-hidden",
            div { class: "bg-gradient-to-r from-amber-900 to-amber-800 py-4 px-6",
                h2 { class: "text-xl font-bold text-white", "Mulligan Decisions" }
            }
            div { class: "p-6",
                if mulligans.is_empty() {
                    EmptyState {}
                } else {
                    div { class: "space-y-8",
                        for mulligan in mulligans.into_iter() {
                            MulliganCard { mulligan: mulligan }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn EmptyState() -> Element {
    rsx! {
        div { class: "text-center text-gray-500 py-8",
            p { "No mulligan information available" }
        }
    }
}

#[component]
fn MulliganCard(mulligan: Mulligan) -> Element {
    let decision_class = get_decision_class(&mulligan.decision);

    rsx! {
        div { class: "border border-gray-700 rounded-lg overflow-hidden",
            // Header section
            div { class: "bg-gray-900 px-4 py-3 border-b border-gray-700",
                div { class: "flex justify-between items-center",
                    h3 { class: "font-semibold text-gray-300",
                        "Game {mulligan.game_number} to Keep {mulligan.number_to_keep}"
                    }

                    // Badges
                    div { class: "flex items-center space-x-2",
                        span { class: "px-2 py-1 text-xs rounded-full bg-violet-900/40 text-violet-300",
                            "{mulligan.play_draw}"
                        }
                        span { class: "px-2 py-1 text-xs rounded-full {decision_class}",
                            "{mulligan.decision}"
                        }
                    }
                }
                div { class: "mt-1 text-sm text-gray-400",
                    "vs {mulligan.opponent_identity}"
                }
            }

            // Hand section
            div { class: "p-4",
                div { class: "flex flex-wrap gap-2 justify-center",
                    for card in mulligan.hand {
                        div { class: "relative group", key: "{card.name}",
                            div { class: "w-40 h-56 rounded-lg overflow-hidden shadow-md hover:shadow-lg transition-shadow",
                                img {
                                    src: "{card.image_uri}",
                                    alt: "{card.name}",
                                    class: "w-full h-full object-cover"
                                }
                            }
                            div { class: "absolute bottom-0 left-0 right-0 bg-black bg-opacity-70 text-white text-xs p-1 text-center opacity-0 group-hover:opacity-100 transition-opacity",
                                "{card.name}"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn get_decision_class(decision: &str) -> &'static str {
    match decision {
        "Keep" => "bg-emerald-900/40 text-emerald-300",
        "Mulligan" => "bg-red-900/40 text-red-300",
        _ => "bg-gray-700 text-gray-300",
    }
}
