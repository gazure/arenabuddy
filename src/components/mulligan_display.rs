use arenabuddy_core::display::{card::CardDisplayRecord, mulligan::Mulligan};
use leptos::prelude::*;

#[component]
pub fn MulliganDisplay(#[prop(into)] mulligans: Signal<Vec<Mulligan>>) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow-md overflow-hidden">
            <div class="bg-gradient-to-r from-amber-500 to-amber-600 py-4 px-6">
                <h2 class="text-xl font-bold text-white">Mulligan Decisions</h2>
            </div>
            <div class="p-6">
                {move || {
                    if mulligans.get().is_empty() {
                        view! {
                            <div class="text-center text-gray-500 py-8">
                                <p>No mulligan information available</p>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
                                <For
                                    each=move || mulligans.get().clone()
                                    key=|m| format!("game-{}-mulligan", m.game_number)
                                    children=move |mulligan| {
                                        view! {
                                            <div class="border rounded-lg overflow-hidden shadow-sm">
                                                <div class="bg-gray-100 px-4 py-3 border-b">
                                                    <div class="flex justify-between items-center">
                                                        <h3 class="font-semibold text-gray-700">
                                                            {"Game "}{mulligan.game_number} {" Mulligan"}
                                                        </h3>
                                                        <div class="flex items-center space-x-2">
                                                            <span class="px-2 py-1 text-xs rounded-full bg-purple-100 text-purple-800">
                                                                {mulligan.play_draw.clone()}
                                                            </span>
                                                            <span class="px-2 py-1 text-xs rounded-full bg-blue-100 text-blue-800">
                                                                {"Kept "}{mulligan.number_to_keep}
                                                            </span>
                                                            <span class=move || {
                                                                let decision_class = match mulligan.decision.as_str() {
                                                                    "keep" => "bg-green-100 text-green-800",
                                                                    "mulligan" => "bg-red-100 text-red-800",
                                                                    _ => "bg-gray-100 text-gray-800",
                                                                };
                                                                format!("px-2 py-1 text-xs rounded-full {decision_class}")
                                                            }>{mulligan.decision.clone()}</span>
                                                        </div>
                                                    </div>
                                                    <div class="mt-1 text-sm text-gray-600">
                                                        {"vs "}{mulligan.opponent_identity.clone()}
                                                    </div>
                                                </div>
                                                <div class="p-4">
                                                    <div class="flex flex-wrap gap-2 justify-center">
                                                        <For
                                                            each=move || mulligan.hand.clone()
                                                            key=|card| card.name.clone()
                                                            children=move |card: CardDisplayRecord| {
                                                                view! {
                                                                    <div class="relative group">
                                                                        <div class="w-40 h-56 rounded-lg overflow-hidden shadow-md hover:shadow-lg transition-shadow">
                                                                            <img
                                                                                src=card.image_uri
                                                                                alt=card.name.clone()
                                                                                class="w-full h-full object-cover"
                                                                                onerror="this.onerror=null; this.src='https://cards.scryfall.io/large/front/0/c/0c082aa8-bf7f-47f2-baf8-43ad253fd7d7.jpg?1562826021'"
                                                                            />
                                                                        </div>
                                                                        <div class="absolute bottom-0 left-0 right-0 bg-black bg-opacity-70 text-white text-xs p-1 text-center opacity-0 group-hover:opacity-100 transition-opacity">
                                                                            {card.name.clone()}
                                                                        </div>
                                                                    </div>
                                                                }
                                                            }
                                                        />
                                                    </div>
                                                </div>
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}
