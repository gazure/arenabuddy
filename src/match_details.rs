use crate::app::invoke;
use arenabuddy_core::display::match_details::MatchDetails;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use wasm_bindgen_futures::spawn_local;

async fn get_match_details(id: &str) -> Option<MatchDetails> {
    // Build the object we want to send to Tauri
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
        leptos::logging::log!("Loading match ID: {}", id);

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
                pd.main_deck
                    .values()
                    .flat_map(|cdrs| cdrs.iter().map(|cdr| (cdr.name.clone(), cdr.quantity)))
                    .collect::<Vec<_>>()
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
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                            <div class="bg-white rounded-lg shadow-md overflow-hidden">
                                <div class="bg-gradient-to-r from-blue-500 to-blue-600 py-4 px-6">
                                    <h2 class="text-xl font-bold text-white">Match Information</h2>
                                </div>
                                <div class="p-6">
                                    <div class="mb-4">
                                        <h3 class="text-lg font-semibold text-gray-700 mb-2">
                                            Players
                                        </h3>
                                        <div class="flex flex-col gap-2">
                                            <div class="bg-blue-50 p-3 rounded-md">
                                                <span class="font-semibold">You</span>
                                                {move || {
                                                    format!(" {}", match_details.get().controller_player_name)
                                                }}
                                            </div>
                                            <div class="bg-red-50 p-3 rounded-md">
                                                <span class="font-semibold">Opponent</span>
                                                {move || {
                                                    format!(" {}", match_details.get().opponent_player_name)
                                                }}
                                            </div>
                                        </div>
                                    </div>

                                    <div class="mb-4">
                                        <h3 class="text-lg font-semibold text-gray-700 mb-2">
                                            Game Details
                                        </h3>
                                        <div class="grid grid-cols-2 gap-2">
                                            <div class="bg-gray-50 p-3 rounded-md">
                                                <span class="text-sm text-gray-500 block">Format</span>
                                                <span class="font-medium">unknown</span>
                                            </div>
                                            <div class="bg-gray-50 p-3 rounded-md">
                                                <span class="text-sm text-gray-500 block">Result</span>
                                                <span class="font-medium">
                                                    {move || {
                                                        if match_details.get().did_controller_win {
                                                            view! {
                                                                <span class="text-green-600 font-bold">Victory</span>
                                                            }
                                                                .into_view()
                                                        } else {
                                                            view! { <span class="text-red-600 font-bold">Defeat</span> }
                                                                .into_view()
                                                        }
                                                    }}
                                                </span>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>

                            <div class="bg-white rounded-lg shadow-md overflow-hidden">
                                <div class="bg-gradient-to-r from-indigo-500 to-indigo-600 py-4 px-6">
                                    <h2 class="text-xl font-bold text-white">Your Deck</h2>
                                </div>
                                <div class="p-6">
                                    {move || {
                                        let cards = deck_cards();
                                        if cards.is_empty() {
                                            view! {
                                                <div class="text-center text-gray-500 py-8">
                                                    <p>No deck information available</p>
                                                </div>
                                            }
                                                .into_any()
                                        } else {
                                            view! {
                                                <div class="max-h-96 overflow-y-auto pr-2 deck-scrollbar">
                                                    <table class="min-w-full">
                                                        <thead>
                                                            <tr class="border-b">
                                                                <th class="text-left py-2 font-semibold text-gray-600">
                                                                    Count
                                                                </th>
                                                                <th class="text-left py-2 font-semibold text-gray-600">
                                                                    Card Name
                                                                </th>
                                                            </tr>
                                                        </thead>
                                                        <tbody>
                                                            <For
                                                                each=move || deck_cards()
                                                                key=|(name, _)| name.clone()
                                                                children=move |(name, count)| {
                                                                    view! {
                                                                        <tr class="border-b border-gray-100 hover:bg-gray-50">
                                                                            <td class="py-2 text-center font-medium text-gray-600">
                                                                                {count}
                                                                            </td>
                                                                            <td class="py-2">{name}</td>
                                                                        </tr>
                                                                    }
                                                                }
                                                            />
                                                        </tbody>
                                                    </table>
                                                </div>
                                            }
                                                .into_any()
                                        }
                                    }}
                                </div>
                            </div>
                        </div>
                    }
                        .into_any()
                }
            }}

            <style>
                {".deck-scrollbar::-webkit-scrollbar {
                width: 8px;
                }
                
                .deck-scrollbar::-webkit-scrollbar-track {
                background: #f1f1f1;
                border-radius: 8px;
                }
                
                .deck-scrollbar::-webkit-scrollbar-thumb {
                background: #c5c5c5;
                border-radius: 8px;
                }
                
                .deck-scrollbar::-webkit-scrollbar-thumb:hover {
                background: #a0a0a0;
                }"}
            </style>
        </div>
    }
}
