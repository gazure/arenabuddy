use arenabuddy_core::display::match_details::MatchDetails as MatchDetailsData;
use dioxus::prelude::*;

use crate::{
    components::{DeckList, MatchInfo, MulliganDisplay},
    service::command_match_details,
    state::AsyncState,
};

async fn get_match_details(id: &str) -> Option<MatchDetailsData> {
    command_match_details(id.to_string()).await.ok()
}

#[component]
pub(crate) fn MatchDetails(id: String) -> Element {
    let state = use_signal(|| AsyncState::Loading);

    let mut load_data = {
        let mut state = state.clone();
        let id = id.clone();
        move || {
            state.set(AsyncState::Loading);
            let id_clone = id.clone();
            spawn(async move {
                match get_match_details(&id_clone).await {
                    Some(details) => state.set(AsyncState::Success(details)),
                    None => state.set(AsyncState::Error(format!(
                        "Could not find match details for ID: {id_clone}"
                    ))),
                }
            });
        }
    };

    let refresh_load = {
        let mut state = state.clone();
        let id = id.clone();
        move |_| {
            state.set(AsyncState::Loading);
            let id_clone = id.clone();
            spawn(async move {
                match get_match_details(&id_clone).await {
                    Some(details) => state.set(AsyncState::Success(details)),
                    None => state.set(AsyncState::Error(format!(
                        "Could not find match details for ID: {id_clone}"
                    ))),
                }
            });
        }
    };

    use_effect(move || {
        load_data();
    });

    let deck_display = move || {
        state
            .read()
            .details()
            .and_then(|details| details.primary_decklist.as_ref())
            .cloned()
            .unwrap_or_default()
    };

    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-8xl",
            div { class: "mb-4",
                a {
                    href: "/matches",
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
                    button {
                        onclick: refresh_load,
                        class: "bg-black bg-opacity-20 hover:bg-opacity-30 text-white font-semibold py-2 px-4 rounded-full transition-all duration-200 shadow-md hover:shadow-lg flex items-center",
                        disabled: state.read().is_loading(),
                        span { class: "mr-2",
                            if state.read().is_loading() { "Loading..." } else { "Refresh" }
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
                p { class: "text-lg opacity-80 mt-2",
                    span { class: "font-semibold", "Match ID:" }
                    " {id}"
                }
            }

            match state.read().clone() {
                AsyncState::Loading => rsx! {
                    div { class: "bg-white rounded-lg shadow-md p-8 text-center",
                        div { class: "animate-pulse flex flex-col items-center",
                            div { class: "w-12 h-12 border-4 border-blue-500 border-t-transparent rounded-full animate-spin mb-4" }
                            p { class: "text-gray-600", "Loading match details..." }
                        }
                    }
                },

                AsyncState::Error(err) => rsx! {
                    div { class: "bg-white rounded-lg shadow-md p-8",
                        div {
                            class: "bg-red-100 border-l-4 border-red-500 text-red-700 p-4 rounded",
                            role: "alert",
                            p { class: "font-bold", "Error" }
                            p { "{err}" }
                        }
                    }
                },

                AsyncState::Success(details) => rsx! {
                    MatchInfo {
                        controller_player_name: details.controller_player_name.clone(),
                        opponent_player_name: details.opponent_player_name.clone(),
                        did_controller_win: details.did_controller_win
                    }

                    DeckList { deck: deck_display() }

                    div { class: "mt-8 col-span-full",
                        MulliganDisplay { mulligans: details.mulligans.clone() }
                    }
                },
            }
        }
    }
}