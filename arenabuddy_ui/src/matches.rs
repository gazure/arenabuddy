use arenabuddy_core::models::MTGAMatch;
use dioxus::prelude::*;
use wasm_bindgen::JsValue;

use crate::app::invoke;

async fn retrieve_matches() -> Vec<MTGAMatch> {
    serde_wasm_bindgen::from_value(invoke("command_matches", JsValue::null()).await)
        .unwrap_or_default()
}

#[component]
fn MatchRow(m: MTGAMatch) -> Element {
    let link = format!("/match/{}", m.id());
    rsx! {
        tr { class: "hover:bg-gray-100 transition-colors duration-150",
            td { class: "py-3 px-4 border-b",
                a {
                    href: "{link}",
                    class: "text-blue-600 hover:text-blue-800 hover:underline font-medium",
                    "{m.controller_player_name()}"
                }
            }
            td { class: "py-3 px-4 border-b", "{m.opponent_player_name()}" }
            td { class: "py-3 px-4 border-b text-gray-500", "{m.created_at()}" }
        }
    }
}

// Component for Matches page
#[component]
pub(crate) fn Matches() -> Element {
    let mut length = use_signal(|| 0usize);
    let mut matches = use_signal(|| Vec::<MTGAMatch>::new());
    let mut loading = use_signal(|| true);

    let mut load = move || {
        loading.set(true);
        spawn(async move {
            let m = retrieve_matches().await;
            length.set(m.len());
            matches.set(m);
            loading.set(false);
        });
    };

    use_effect(move || {
        load();
    });

    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-5xl",
            div { class: "flex justify-between items-center mb-6",
                h1 { class: "text-2xl font-bold text-gray-800", "Match History" }
                button {
                    onclick: move |_| load(),
                    class: "bg-blue-600 hover:bg-blue-700 text-white py-2 px-4 rounded shadow transition-colors duration-150 flex items-center",
                    disabled: loading(),
                    if loading() { "Loading..." } else { "Refresh Matches" }
                }
            }

            div { class: "bg-white rounded-lg shadow-md overflow-hidden",
                div { class: "p-4 border-b bg-gray-50",
                    p { class: "text-gray-600",
                        span { class: "font-medium", "{length()}" }
                        " matches found"
                    }
                }

                if loading() && matches().is_empty() {
                    div { class: "p-12 text-center text-gray-500",
                        div { class: "animate-pulse", "Loading match data..." }
                    }
                } else if matches().is_empty() {
                    div { class: "p-12 text-center text-gray-500",
                        "No matches found. Play some games in MTG Arena!"
                    }
                } else {
                    div { class: "overflow-x-auto",
                        table { class: "min-w-full table-auto",
                            thead {
                                tr { class: "bg-gray-100 text-left",
                                    th { class: "py-3 px-4 font-semibold text-gray-700", "Controller" }
                                    th { class: "py-3 px-4 font-semibold text-gray-700", "Opponent" }
                                    th { class: "py-3 px-4 font-semibold text-gray-700", "Date" }
                                }
                            }
                            tbody {
                                for m in matches() {
                                    MatchRow { key: "{m.id()}", m: m }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}