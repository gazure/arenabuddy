use arenabuddy_core::display::card::CardDisplayRecord;
use leptos::prelude::*;

use crate::components::cost::ManaCost;

#[component]
pub fn DeckList(
    #[prop(into)] cards: Signal<Vec<CardDisplayRecord>>,
    #[prop(optional)] title: Option<&'static str>,
) -> impl IntoView {
    let title = title.unwrap_or("Your Deck");

    view! {
        <div class="bg-white rounded-lg shadow-md overflow-hidden">
            <div class="bg-gradient-to-r from-indigo-500 to-indigo-600 py-4 px-6">
                <h2 class="text-xl font-bold text-white">{title}</h2>
            </div>
            <div class="p-6">
                <div class="deck-content">
                    {move || {
                        let card_list = cards.get();
                        if card_list.is_empty() {
                            view! {
                                <div class="text-center text-gray-500 py-8">
                                    <p>No deck information available</p>
                                </div>
                            }
                                .into_any()
                        } else {
                            let mut sorted_cards = card_list.clone();
                            sorted_cards.sort_by_key(|card| (card.mana_value, card.name.clone()));
                            let total_count: u16 = sorted_cards.iter().map(|c| c.quantity).sum();

                            view! {
                                <div class="max-h-96 overflow-y-auto pr-2 deck-scrollbar">
                                    <div class="mb-4 text-right text-sm text-gray-500">
                                        {"Total cards: "}{total_count}
                                    </div>
                                    <table class="min-w-full table-fixed">
                                        <thead>
                                            <tr class="border-b">
                                                <th class="text-left py-3 px-4 font-semibold text-gray-600 w-16">
                                                    Count
                                                </th>
                                                <th class="text-left py-3 px-4 font-semibold text-gray-600 w-32">
                                                    Mana
                                                </th>
                                                <th class="text-left py-3 px-4 font-semibold text-gray-600 w-32">
                                                    Type
                                                </th>
                                                <th class="text-left py-3 px-4 font-semibold text-gray-600">
                                                    Card Name
                                                </th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            <For
                                                each=move || sorted_cards.clone()
                                                key=|card| card.name.clone()
                                                children=move |card| {
                                                    view! {
                                                        <tr class="border-b border-gray-100 hover:bg-gray-50">
                                                            <td class="py-3 px-4 text-center font-medium text-gray-600">
                                                                {card.quantity}
                                                            </td>
                                                            <td class="py-3 px-4 text-center text-gray-500">
                                                                <ManaCost cost=card.cost() />
                                                            </td>
                                                            <td class="py-3 px-4 text-gray-500 truncate">
                                                                {card.type_field.to_string()}
                                                            </td>
                                                            <td class="py-3 px-4 truncate">{card.name}</td>
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
