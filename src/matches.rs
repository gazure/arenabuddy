use arenabuddy_core::models::MTGAMatch;
use leptos::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::app::invoke;

async fn retrieve_matches() -> Vec<MTGAMatch> {
    serde_wasm_bindgen::from_value(invoke("command_matches", JsValue::null()).await)
        .unwrap_or_default()
}

#[component]
fn MatchRow(m: MTGAMatch) -> impl IntoView {
    let link = format!("/match/{}", m.id);
    view! {
        <tr class="hover:bg-gray-100 transition-colors duration-150">
            <td class="py-3 px-4 border-b">
                <a href=link class="text-blue-600 hover:text-blue-800 hover:underline font-medium">
                    {m.controller_player_name}
                </a>
            </td>
            <td class="py-3 px-4 border-b">{m.opponent_player_name}</td>
            <td class="py-3 px-4 border-b text-gray-500">{m.created_at.to_string()}</td>
        </tr>
    }
}

// Component for Matches page
#[component]
pub(crate) fn Matches() -> impl IntoView {
    let (length, set_length) = signal(0usize);
    let (matches, set_matches) = signal(Vec::<MTGAMatch>::new());
    let (loading, set_loading) = signal(true);

    let load = move || {
        set_loading.set(true);
        spawn_local(async move {
            let m = retrieve_matches().await;
            set_length.set(m.len());
            set_matches.set(m);
            set_loading.set(false);
        });
    };

    load();

    view! {
        <div class="container mx-auto px-4 py-8 max-w-5xl">
            <div class="flex justify-between items-center mb-6">
                <h1 class="text-2xl font-bold text-gray-800">Match History</h1>
                <button
                    on:click=move |_| load()
                    class="bg-blue-600 hover:bg-blue-700 text-white py-2 px-4 rounded shadow transition-colors duration-150 flex items-center"
                    disabled=move || loading.get()
                >
                    {move || if loading.get() { "Loading..." } else { "Refresh Matches" }}
                </button>
            </div>

            <div class="bg-white rounded-lg shadow-md overflow-hidden">
                <div class="p-4 border-b bg-gray-50">
                    <p class="text-gray-600">
                        <span class="font-medium">{move || length.get()}</span>
                        {" matches found"}
                    </p>
                </div>

                {move || {
                    if loading.get() && matches.get().is_empty() {
                        view! {
                            <div class="p-12 text-center text-gray-500">
                                <div class="animate-pulse">Loading match data...</div>
                            </div>
                        }
                            .into_any()
                    } else if matches.get().is_empty() {
                        view! {
                            <div class="p-12 text-center text-gray-500">
                                No matches found. Play some games in MTG Arena!
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="overflow-x-auto">
                                <table class="min-w-full table-auto">
                                    <thead>
                                        <tr class="bg-gray-100 text-left">
                                            <th class="py-3 px-4 font-semibold text-gray-700">
                                                Controller
                                            </th>
                                            <th class="py-3 px-4 font-semibold text-gray-700">
                                                Opponent
                                            </th>
                                            <th class="py-3 px-4 font-semibold text-gray-700">Date</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <For
                                            each=move || matches.get().clone()
                                            key=|m| m.id.clone()
                                            children=move |m| {
                                                view! { <MatchRow m=m /> }
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
    }
}
