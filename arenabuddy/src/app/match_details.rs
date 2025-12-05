use dioxus::prelude::*;

use crate::{
    app::{
        Route,
        components::{DeckList, MatchInfo, MulliganDisplay},
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

    let service3 = service.clone();
    let id3 = id.clone();
    let sync_to_sheets = move |_| {
        let service4 = service3.clone();
        let id4 = id3.clone();
        spawn(async move {
            if let Err(e) = service4.sync_match_to_sheets(id4).await {
                tracingx::error!("Failed to sync to sheets: {e}");
            }
        });
    };

    let resource_value = match_details.value();
    let data = resource_value.read();

    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-8xl",
            div { class: "mb-4",
                Link {
                    to: Route::Matches{},
                    class: "inline-flex items-center bg-gray-200 hover:bg-gray-300 text-gray-800 font-semibold py-2 px-4 rounded-full transition-all duration-200 shadow-sm hover:shadow-md",
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

            div { class: "bg-gradient-to-r from-purple-700 to-blue-600 rounded-lg shadow-lg mb-8 p-6 text-white",
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
                        button {
                            onclick: sync_to_sheets,
                            class: "bg-black bg-opacity-20 hover:bg-opacity-30 text-white font-semibold py-2 px-4 rounded-full transition-all duration-200 shadow-md hover:shadow-lg flex items-center",
                            disabled: data.is_none(),
                            span { class: "mr-2", "Sync to Sheets" }
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
                                    d: "M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
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
                    div { class: "bg-white rounded-lg shadow-md p-8 text-center",
                        div { class: "animate-pulse flex flex-col items-center",
                            div { class: "w-12 h-12 border-4 border-blue-500 border-t-transparent rounded-full animate-spin mb-4" }
                            p { class: "text-gray-600", "Loading match details..." }
                        }
                    }
                },

                Some(Err(err)) => rsx! {
                    div { class: "bg-white rounded-lg shadow-md p-8",
                        div {
                            class: "bg-red-100 border-l-4 border-red-500 text-red-700 p-4 rounded",
                            role: "alert",
                            p { class: "font-bold", "Error" }
                            p { "Could not find match details for ID: {id}: {err}" }
                        }
                    }
                },

                Some(Ok(details)) => rsx! {
                    MatchInfo {
                        controller_player_name: details.controller_player_name.clone(),
                        opponent_player_name: details.opponent_player_name.clone(),
                        did_controller_win: details.did_controller_win
                    }

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

                    div { class: "mt-8 col-span-full",
                        MulliganDisplay { mulligans: details.mulligans.clone() }
                    }
                },
            }
        }
    }
}
