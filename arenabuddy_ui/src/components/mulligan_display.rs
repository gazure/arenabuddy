use arenabuddy_core::display::mulligan::Mulligan;
use dioxus::prelude::*;

#[component]
pub fn MulliganDisplay(mulligans: Vec<Mulligan>) -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md overflow-hidden",
            div { class: "bg-gradient-to-r from-amber-500 to-amber-600 py-4 px-6",
                h2 { class: "text-xl font-bold text-white", "Mulligan Decisions" }
            }
            div { class: "p-6",
                if mulligans.is_empty() {
                    {empty_state_view()}
                } else {
                    div { class: "space-y-8",
                        for mulligan in mulligans {
                            {mulligan_card_view(mulligan)}
                        }
                    }
                }
            }
        }
    }
}

fn empty_state_view() -> Element {
    rsx! {
        div { class: "text-center text-gray-500 py-8",
            p { "No mulligan information available" }
        }
    }
}

fn mulligan_card_view(mulligan: Mulligan) -> Element {
    let decision_class = get_decision_class(&mulligan.decision);

    rsx! {
        div { class: "border rounded-lg overflow-hidden shadow-sm",
            // Header section
            div { class: "bg-gray-100 px-4 py-3 border-b",
                div { class: "flex justify-between items-center",
                    h3 { class: "font-semibold text-gray-700",
                        "Game {mulligan.game_number} to Keep {mulligan.number_to_keep}"
                    }

                    // Badges
                    div { class: "flex items-center space-x-2",
                        span { class: "px-2 py-1 text-xs rounded-full bg-purple-100 text-purple-800",
                            "{mulligan.play_draw}"
                        }
                        span { class: "px-2 py-1 text-xs rounded-full {decision_class}",
                            "{mulligan.decision}"
                        }
                    }
                }
                div { class: "mt-1 text-sm text-gray-600",
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
        "keep" => "bg-green-100 text-green-800",
        "mulligan" => "bg-red-100 text-red-800",
        _ => "bg-gray-100 text-gray-800",
    }
}
