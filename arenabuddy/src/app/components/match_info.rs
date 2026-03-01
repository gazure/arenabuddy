use dioxus::prelude::*;

#[component]
pub fn MatchInfo(controller_player_name: String, opponent_player_name: String, did_controller_win: bool) -> Element {
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
                        }
                        div { class: "bg-red-900/20 p-3 rounded-md",
                            span { class: "font-semibold", "Opponent" }
                            " {opponent_player_name}"
                        }
                    }
                }

                div { class: "mb-4",
                    h3 { class: "text-lg font-semibold text-gray-300 mb-2", "Game Details" }
                    div { class: "grid grid-cols-2 gap-2",
                        div { class: "bg-gray-900/50 p-3 rounded-md",
                            span { class: "text-sm text-gray-500 block", "Format" }
                            span { class: "font-medium", "unknown" }
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
