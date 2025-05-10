use arenabuddy_core::display::match_details::MatchDetails;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use wasm_bindgen_futures::spawn_local;

use crate::{
    app::invoke,
    components::{DeckList, MatchInfo, MulliganDisplay, deck_list::TypedCard},
};

async fn get_match_details(id: &str) -> Option<MatchDetails> {
    let command_object =
        serde_wasm_bindgen::to_value(&serde_json::json!({ "matchId": id })).unwrap();
    serde_wasm_bindgen::from_value(invoke("command_match_details", command_object).await).ok()
}
#[component]
pub(crate) fn MatchDetails() -> impl IntoView {
    let params = use_params_map();
    let (match_details, set_match_details) = signal(MatchDetails::default());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);

    let load = move || {
        set_loading.set(true);
        set_error.set(None);
        let id = params.with(|params| params.get("id").unwrap_or_default().to_string());
        spawn_local(async move {
            if let Some(m) = get_match_details(&id).await {
                set_match_details.set(m);
                set_loading.set(false);
            } else {
                set_error.set(Some(format!("Could not find match details for ID: {id}")));
                set_loading.set(false);
            }
        });
    };

    load();

    let deck_cards = move || {
        match_details
            .get()
            .primary_decklist
            .map(|pd| {
                let mut cards = Vec::new();
                for card_type_cards in &pd.main_deck {
                    for card in card_type_cards.1 {
                        cards.push(TypedCard {
                            name: card.name.clone(),
                            quantity: card.quantity,
                            card_type: *card_type_cards.0,
                            mana_value: card.mana_value,
                        });
                    }
                }
                cards
            })
            .unwrap_or_default()
    };

    view! {
        <div class="container mx-auto px-4 py-8 max-w-8xl">
            <div class="mb-4">
                <a
                    href="/matches"
                    class="inline-flex items-center bg-gray-200 hover:bg-gray-300 text-gray-800 font-semibold py-2 px-4 rounded-full transition-all duration-200 shadow-sm hover:shadow-md"
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        class="h-5 w-5 mr-2"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M10 19l-7-7m0 0l7-7m-7 7h18"
                        />
                    </svg>
                </a>
            </div>

            <div class="bg-gradient-to-r from-purple-700 to-blue-600 rounded-lg shadow-lg mb-8 p-6 text-white">
                <div class="flex justify-between items-center">
                    <h1 class="text-3xl font-bold">Match Details</h1>
                    <button
                        on:click=move |_| load()
                        class="bg-black bg-opacity-20 hover:bg-opacity-30 text-white font-semibold py-2 px-4 rounded-full transition-all duration-200 shadow-md hover:shadow-lg flex items-center"
                        disabled=move || loading.get()
                    >
                        <span class="mr-2">
                            {move || if loading.get() { "Loading..." } else { "Refresh" }}
                        </span>
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            class="h-5 w-5"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                            />
                        </svg>
                    </button>
                </div>
                <p class="text-lg opacity-80 mt-2">
                    <span class="font-semibold">Match ID:</span>
                    {move || match_details.get().id}
                </p>
            </div>

            {move || {
                if loading.get() {
                    view! {
                        <div class="bg-white rounded-lg shadow-md p-8 text-center">
                            <div class="animate-pulse flex flex-col items-center">
                                <div class="w-12 h-12 border-4 border-blue-500 border-t-transparent rounded-full animate-spin mb-4"></div>
                                <p class="text-gray-600">Loading match details...</p>
                            </div>
                        </div>
                    }
                        .into_any()
                } else if let Some(err) = error.get() {
                    view! {
                        <div class="bg-white rounded-lg shadow-md p-8">
                            <div
                                class="bg-red-100 border-l-4 border-red-500 text-red-700 p-4 rounded"
                                role="alert"
                            >
                                <p class="font-bold">Error</p>
                                <p>{err}</p>
                            </div>
                        </div>
                    }
                        .into_any()
                } else {
                    view! {
                        <MatchInfo
                            controller_player_name=Signal::derive(move || match_details.get().controller_player_name)
                            opponent_player_name=Signal::derive(move || match_details.get().opponent_player_name)
                            did_controller_win=Signal::derive(move || match_details.get().did_controller_win)
                        />

                        <DeckList cards=Signal::derive(deck_cards) />

                        // Mulligan Hands Section
                        <div class="mt-8 col-span-full">
                            <MulliganDisplay mulligans=Signal::derive(move || {
                                match_details.get().mulligans.clone()
                            }) />
                        </div>
                    }
                        .into_any()
                }
            }}


        </div>
    }
}
