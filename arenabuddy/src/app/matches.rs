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
    let mut matches_resource = use_resource(move || {
        let service = service.clone();
        async move { service.get_matches().await }
    });

    let refresh_matches = move |_| {
        matches_resource.restart();
    };

    let resource_value = matches_resource.value();
    let data = resource_value.read();

    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-5xl",
            div { class: "flex justify-between items-center mb-6",
                h1 { class: "text-2xl font-bold text-gray-800", "Match History" }
                button {
                    onclick: refresh_matches,
                    class: "bg-blue-600 hover:bg-blue-700 text-white py-2 px-4 rounded shadow transition-colors duration-150 flex items-center",
                    disabled: data.is_none(),
                    if data.is_none() {
                        "Loading..."
                    } else {
                        "Refresh Matches"
                    }
                }
            }

            match &*data {
                None => rsx! {
                    div { class: "bg-white rounded-lg shadow-md overflow-hidden",
                        div { class: "p-12 text-center text-gray-500",
                            div { class: "animate-pulse", "Loading match data..." }
                        }
                    }
                },

                Some(Err(err)) => rsx! {
                    div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                        p { "Failed to load matches: {err}" }
                    }
                    div { class: "bg-white rounded-lg shadow-md overflow-hidden",
                        div { class: "p-12 text-center text-gray-500",
                            "No match data available"
                        }
                    }
                },

                Some(Ok(matches_data)) => rsx! {
                    div { class: "bg-white rounded-lg shadow-md overflow-hidden",
                        div { class: "p-4 border-b bg-gray-50",
                            p { class: "text-gray-600",
                                span { class: "font-medium", "{matches_data.len()}" }
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
                                        for m in matches_data.iter().rev() {
                                            MatchRow { key: "{m.id()}", m: m.clone() }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}
