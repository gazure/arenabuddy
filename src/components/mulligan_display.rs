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
                    let mulligan_list = mulligans.get();
                    if mulligan_list.is_empty() {
                        return empty_state_view().into_any();
                    }

                    view! {
                        <div class="space-y-8">
                            <For
                                each=move || mulligan_list.clone()
                                key=|m| format!("game-{}-mulligan", m.game_number)
                                children=move |mulligan| mulligan_card_view(mulligan)
                            />
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}

fn empty_state_view() -> impl IntoView {
    view! {
        <div class="text-center text-gray-500 py-8">
            <p>No mulligan information available</p>
        </div>
    }
}

fn mulligan_card_view(mulligan: Mulligan) -> impl IntoView {
    let decision_class = get_decision_class(&mulligan.decision);

    view! {
        <div class="border rounded-lg overflow-hidden shadow-sm">
            // Header section
            <div class="bg-gray-100 px-4 py-3 border-b">
                <div class="flex justify-between items-center">
                    <h3 class="font-semibold text-gray-700">
                        {"Game "}{mulligan.game_number} {" to Keep "}{mulligan.number_to_keep}
                    </h3>

                    // Badges
                    <div class="flex items-center space-x-2">
                        <span class="px-2 py-1 text-xs rounded-full bg-purple-100 text-purple-800">
                            {mulligan.play_draw}
                        </span>
                        <span class=format!("px-2 py-1 text-xs rounded-full {decision_class}")>
                            {mulligan.decision}
                        </span>
                    </div>
                </div>
                <div class="mt-1 text-sm text-gray-600">
                    {"vs "}{mulligan.opponent_identity}
                </div>
            </div>

            // Hand section
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
                                        {card.name}
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

fn get_decision_class(decision: &str) -> &'static str {
    match decision {
        "keep" => "bg-green-100 text-green-800",
        "mulligan" => "bg-red-100 text-red-800",
        _ => "bg-gray-100 text-gray-800",
    }
}
