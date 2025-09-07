use arenabuddy_core::models::MTGAMatch;
use dioxus::prelude::*;
use dioxus_router::Link;

use crate::{app::Route, backend::Service};

#[component]
fn MatchRow(m: MTGAMatch) -> Element {
    rsx! {
        Link {
            to: Route::MatchDetails { id: m.id().to_string() },
            class: "table-row hover:bg-gray-100 transition-colors duration-150 cursor-pointer",
            td { class: "py-3 px-4 border-b",
                span { class: "text-blue-600 font-medium",
                    "{m.controller_player_name()}"
                }
            }
            td { class: "py-3 px-4 border-b", "{m.opponent_player_name()}" }
            td { class: "py-3 px-4 border-b text-gray-500", "{m.created_at()}" }
        }
    }
}

#[component]
pub(crate) fn Matches() -> Element {
    let service = use_context::<Service>();
    let mut matches = use_signal(|| None::<Vec<MTGAMatch>>);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    let service2 = service.clone();
    // Load matches on component mount
    use_future({
        move || {
            let service2 = service2.clone();
            async move {
                match service2.get_matches().await {
                    Ok(data) => {
                        matches.set(Some(data));
                        error.set(None);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load matches: {e}")));
                        matches.set(None);
                    }
                }
                loading.set(false);
            }
        }
    });

    let refresh_matches = {
        let service = service.clone();
        let mut loading = loading;
        let mut matches = matches;
        let mut error = error;
        move |_| {
            loading.set(true);
            error.set(None);
            let service = service.clone();
            spawn(async move {
                match service.get_matches().await {
                    Ok(data) => {
                        matches.set(Some(data));
                        error.set(None);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to refresh matches: {e}")));
                        matches.set(None);
                    }
                }
                loading.set(false);
            });
        }
    };

    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-5xl",
            div { class: "flex justify-between items-center mb-6",
                h1 { class: "text-2xl font-bold text-gray-800", "Match History" }
                button {
                    onclick: refresh_matches,
                    class: "bg-blue-600 hover:bg-blue-700 text-white py-2 px-4 rounded shadow transition-colors duration-150 flex items-center",
                    disabled: loading(),
                    if loading() {
                        "Loading..."
                    } else {
                        "Refresh Matches"
                    }
                }
            }

            if let Some(err) = error() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                    p { "{err}" }
                }
            }

            div { class: "bg-white rounded-lg shadow-md overflow-hidden",
                if loading() && matches().is_none() {
                    div { class: "p-12 text-center text-gray-500",
                        div { class: "animate-pulse", "Loading match data..." }
                    }
                } else if let Some(matches_data) = matches.read().as_ref() {
                    div { class: "p-4 border-b bg-gray-50",
                        p { class: "text-gray-600",
                            span { class: "font-medium", "{matches_data.len
    ()}" }
                            " matches found"
                        }
                    }
                    if matches_data.is_empty() {
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
                                    for m in matches_data {
                                        MatchRow { key: "{m.id()}", m: m.clone() }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "p-12 text-center text-gray-500",
                        "No match data available"
                    }
                }
            }
        }
    }
}
