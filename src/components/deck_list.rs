use arenabuddy_core::{display::deck::DeckDisplayRecord, models::CardType};
use leptos::prelude::*;

use crate::components::cost::ManaCost;

#[component]
pub fn DeckList(
    #[prop(into)] deck: Signal<DeckDisplayRecord>,
    #[prop(optional)] title: Option<&'static str>,
) -> impl IntoView {
    let title = title.unwrap_or("Your Deck");
    let main_deck = deck.get().main_deck.clone();
    let sideboard = deck.get().sideboard.clone();
    let main_total: u16 = main_deck
        .values()
        .flat_map(|cards| cards.iter())
        .map(|c| c.quantity)
        .sum();
    let sideboard_total: u16 = sideboard.iter().map(|c| c.quantity).sum();
    let total_count = main_total + sideboard_total;

    view! {
        <div class="bg-white rounded-lg shadow-md overflow-hidden">
            <div class="bg-gradient-to-r from-indigo-500 to-indigo-600 py-4 px-6">
                <h2 class="text-xl font-bold text-white">{title}</h2>
            </div>
            <div class="p-6">
                <div class="deck-content">
                    {move || {
                        view! {
                            <div class="mb-4 text-right text-sm text-gray-500">
                                {"Total cards: "}{total_count}{" (Main: "}{main_total}
                                {", Sideboard: "}{sideboard_total}{")"}
                            </div>

                            <div class="grid grid-cols-2 gap-6">
                                // Left column: Non-land cards
                                <div class="space-y-6">

                                    {
                                        let main_deck_clone = main_deck.clone();
                                        move || {
                                            let mut non_land_sections = Vec::new();
                                            let ordered_types = vec![
                                                CardType::Creature,
                                                CardType::Planeswalker,
                                                CardType::Artifact,
                                                CardType::Enchantment,
                                                CardType::Instant,
                                                CardType::Sorcery,
                                                CardType::Battle,
                                                CardType::Unknown,
                                            ];
                                            for card_type in ordered_types {
                                                if let Some(cards) = main_deck_clone.get(&card_type) {
                                                    if !cards.is_empty() {
                                                        non_land_sections
                                                            .push(

                                                                // Order card types for display

                                                                view! {
                                                                    <div class="mb-4">
                                                                        <h4 class="text-md font-medium text-gray-700 mb-2">
                                                                            {format!("{} ({})", card_type, cards.len())}
                                                                        </h4>
                                                                        <div class="space-y-1">
                                                                            {cards
                                                                                .iter()
                                                                                .map(|card| {
                                                                                    view! {
                                                                                        <div class="flex items-center justify-between py-1 px-2 hover:bg-gray-50 rounded text-sm">
                                                                                            <div class="flex items-center space-x-2">
                                                                                                <span class="font-medium text-gray-600 w-6 text-center">
                                                                                                    {card.quantity}
                                                                                                </span>
                                                                                                <span class="truncate">{card.name.clone()}</span>
                                                                                            </div>
                                                                                            <div class="flex-shrink-0 ml-2">
                                                                                                <ManaCost cost=card.cost() />
                                                                                            </div>
                                                                                        </div>
                                                                                    }
                                                                                })
                                                                                .collect::<Vec<_>>()}
                                                                        </div>
                                                                    </div>
                                                                },
                                                            );
                                                    }
                                                }
                                            }
                                            non_land_sections
                                        }
                                    }
                                </div>

                                // Right column: Lands and Sideboard
                                <div class="space-y-6">
                                    // Lands section
                                    {
                                        let main_deck_clone = main_deck.clone();
                                        move || {
                                            if let Some(lands) = main_deck_clone.get(&CardType::Land)
                                                && !lands.is_empty()
                                            {
                                                view! {
                                                    <div>
                                                        <h3 class="text-lg font-semibold text-gray-800 border-b pb-2">
                                                            {format!("Lands ({})", lands.len())}
                                                        </h3>
                                                        <div class="space-y-1 mt-2">
                                                            {lands
                                                                .iter()
                                                                .map(|card| {
                                                                    view! {
                                                                        <div class="flex items-center justify-between py-1 px-2 hover:bg-gray-50 rounded text-sm">
                                                                            <div class="flex items-center space-x-2">
                                                                                <span class="font-medium text-gray-600 w-6 text-center">
                                                                                    {card.quantity}
                                                                                </span>
                                                                                <span class="truncate">{card.name.clone()}</span>
                                                                            </div>
                                                                            <div class="flex-shrink-0 ml-2">
                                                                                <ManaCost cost=card.cost() />
                                                                            </div>
                                                                        </div>
                                                                    }
                                                                })
                                                                .collect::<Vec<_>>()}
                                                        </div>
                                                    </div>
                                                }
                                                    .into_any()
                                            } else {
                                                view! { <div></div> }.into_any()
                                            }
                                        }
                                    } // Sideboard section
                                    {
                                        let sideboard_clone = sideboard.clone();
                                        move || {
                                            if sideboard_clone.is_empty() {
                                                view! { <div></div> }.into_any()
                                            } else {
                                                view! {
                                                    <div>
                                                        <h3 class="text-lg font-semibold text-gray-800 border-b pb-2">
                                                            {format!("Sideboard ({})", sideboard_clone.len())}
                                                        </h3>
                                                        <div class="space-y-1 mt-2">
                                                            {sideboard_clone
                                                                .iter()
                                                                .map(|card| {
                                                                    view! {
                                                                        <div class="flex items-center justify-between py-1 px-2 hover:bg-gray-50 rounded text-sm">
                                                                            <div class="flex items-center space-x-2">
                                                                                <span class="font-medium text-gray-600 w-6 text-center">
                                                                                    {card.quantity}
                                                                                </span>
                                                                                <span class="truncate">{card.name.clone()}</span>
                                                                            </div>
                                                                            <div class="flex-shrink-0 ml-2">
                                                                                <ManaCost cost=card.cost() />
                                                                            </div>
                                                                        </div>
                                                                    }
                                                                })
                                                                .collect::<Vec<_>>()}
                                                        </div>
                                                    </div>
                                                }
                                                    .into_any()
                                            }
                                        }
                                    }
                                </div>
                            </div>
                        }
                            .into_any()
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
