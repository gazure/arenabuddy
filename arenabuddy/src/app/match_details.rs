use dioxus::prelude::*;

use crate::{
    app::{
        Route,
        components::{DeckList, EventLogDisplay, MatchInfo, MulliganDisplay},
    },
    backend::Service,
};

#[component]
pub(crate) fn MatchDetails(id: String) -> Element {
    let service = use_context::<Service>();

    let mut match_details = use_resource({
        let service = service.clone();
        let id = id.clone();
        move || {
            let service = service.clone();
            let id = id.clone();
            async move { service.get_match_details(id).await }
        }
    });

    let refresh = move |_| {
        match_details.restart();
    };

    let resource_value = match_details.value();
    let data = resource_value.read();

    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-8xl",
            div { class: "mb-4",
                Link {
                    to: Route::Matches{},
                    class: "inline-flex items-center bg-gray-700 hover:bg-gray-600 text-gray-200 font-semibold py-2 px-4 rounded-full transition-all duration-200",
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        class: "h-5 w-5 mr-2",
                        fill: "none",
                        view_box: "0 0 24 24",
                        stroke: "currentColor",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M10 19l-7-7m0 0l7-7m-7 7h18"
                        }
                    }
                }
            }

            div { class: "bg-gradient-to-r from-violet-900 to-purple-800 rounded-lg shadow-lg shadow-black/20 mb-8 p-6 text-white",
                div { class: "flex justify-between items-center",
                    h1 { class: "text-3xl font-bold", "Match Details" }
                    div { class: "flex gap-2",
                        button {
                            onclick: refresh,
                            class: "bg-black bg-opacity-20 hover:bg-opacity-30 text-white font-semibold py-2 px-4 rounded-full transition-all duration-200 shadow-md hover:shadow-lg flex items-center",
                            disabled: data.is_none(),
                            span { class: "mr-2",
                                if data.is_none() { "Loading..." } else { "Refresh" }
                            }
                            svg {
                                xmlns: "http://www.w3.org/2000/svg",
                                class: "h-5 w-5",
                                fill: "none",
                                view_box: "0 0 24 24",
                                stroke: "currentColor",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                                }
                            }
                        }

                    }
                }
                p { class: "text-lg opacity-80 mt-2",
                    span { class: "font-semibold", "Match ID:" }
                    " {id}"
                }
            }

            match data.as_ref() {
                None => rsx! {
                    div { class: "bg-gray-800 rounded-lg border border-gray-700 p-8 text-center",
                        div { class: "animate-pulse flex flex-col items-center",
                            div { class: "w-12 h-12 border-4 border-amber-500 border-t-transparent rounded-full animate-spin mb-4" }
                            p { class: "text-gray-400", "Loading match details..." }
                        }
                    }
                },

                Some(Err(err)) => rsx! {
                    div { class: "bg-gray-800 rounded-lg border border-gray-700 p-8",
                        div {
                            class: "bg-red-900/30 border-l-4 border-red-500 text-red-300 p-4 rounded",
                            role: "alert",
                            p { class: "font-bold", "Error" }
                            p { "Could not find match details for ID: {id}: {err}" }
                        }
                    }
                },

                Some(Ok(details)) => {
                    let mut active_tab = use_signal(|| 0u8);
                    let event_count: usize = details.event_logs.iter().map(|l| l.events.len()).sum();

                    rsx! {
                        MatchInfo {
                            controller_player_name: details.controller_player_name.clone(),
                            opponent_player_name: details.opponent_player_name.clone(),
                            did_controller_win: details.did_controller_win
                        }

                        div { class: "flex gap-1 mb-6 border-b border-gray-700",
                            button {
                                class: if active_tab() == 0 {
                                    "px-4 py-2 font-medium text-amber-400 border-b-2 border-amber-400"
                                } else {
                                    "px-4 py-2 font-medium text-gray-500 hover:text-gray-300"
                                },
                                onclick: move |_| active_tab.set(0),
                                "Overview"
                            }
                            button {
                                class: if active_tab() == 1 {
                                    "px-4 py-2 font-medium text-amber-400 border-b-2 border-amber-400"
                                } else {
                                    "px-4 py-2 font-medium text-gray-500 hover:text-gray-300"
                                },
                                onclick: move |_| active_tab.set(1),
                                "Event Log"
                                if event_count > 0 {
                                    span { class: "ml-2 px-2 py-0.5 text-xs rounded-full bg-emerald-900/40 text-emerald-300",
                                        "{event_count}"
                                    }
                                }
                            }
                        }

                        match active_tab() {
                            0 => rsx! {
                                if let Some(ref deck) = details.primary_decklist {
                                    DeckList {
                                        title: "Your deck",
                                        deck: deck.clone()
                                    }
                                }

                                if let Some(ref opponent_deck) = details.opponent_deck {
                                    DeckList {
                                        title: "Opponent's cards",
                                        deck: opponent_deck.clone(),
                                        show_quantities: false
                                    }
                                }

                                if let Some(ref diffs) = details.differences {
                                    if !diffs.is_empty() {
                                        div { class: "mt-8",
                                            h2 { class: "text-xl font-bold text-gray-100 mb-4", "Sideboard Changes" }
                                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                                for (i, diff) in diffs.iter().enumerate() {
                                                    div { class: "bg-gray-800 rounded-lg border border-gray-700 p-4",
                                                        h3 { class: "text-lg font-semibold text-gray-300 mb-3",
                                                            "Game {i + 1} → Game {i + 2}"
                                                        }
                                                        if diff.added.is_empty() && diff.removed.is_empty() {
                                                            p { class: "text-gray-500 text-sm", "No changes" }
                                                        } else {
                                                            if !diff.removed.is_empty() {
                                                                div { class: "mb-3",
                                                                    p { class: "text-sm font-medium text-red-400 mb-1", "Out" }
                                                                    for card in diff.removed.iter() {
                                                                        div { class: "flex justify-between text-sm py-0.5",
                                                                            span { class: "text-gray-300", "{card.name}" }
                                                                            span { class: "text-red-400", "-{card.quantity}" }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            if !diff.added.is_empty() {
                                                                div {
                                                                    p { class: "text-sm font-medium text-green-400 mb-1", "In" }
                                                                    for card in diff.added.iter() {
                                                                        div { class: "flex justify-between text-sm py-0.5",
                                                                            span { class: "text-gray-300", "{card.name}" }
                                                                            span { class: "text-green-400", "+{card.quantity}" }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div { class: "mt-8 col-span-full",
                                    MulliganDisplay { mulligans: details.mulligans.clone() }
                                }
                            },
                            _ => rsx! {
                                EventLogDisplay {
                                    event_logs: details.event_logs.clone(),
                                    controller_seat_id: details.controller_seat_id,
                                }
                            },
                        }
                    }
                },
            }
        }
    }
}
