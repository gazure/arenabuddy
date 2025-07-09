use std::collections::HashMap;

use arenabuddy_core::{
    display::{card::CardDisplayRecord, deck::DeckDisplayRecord},
    models::CardType,
};
use leptos::prelude::*;

use crate::components::cost::ManaCost;

#[component]
pub fn DeckList(
    #[prop(into)] deck: Signal<DeckDisplayRecord>,
    #[prop(optional)] title: Option<&'static str>,
) -> impl IntoView {
    let title = title.unwrap_or("Your Deck");
    let deck_data = deck.get();
    let main_deck = deck_data.main_deck.clone();
    let sideboard = deck_data.sideboard.clone();

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
                    <div class="mb-4 text-right text-sm text-gray-500">
                        {"Total cards: "}{total_count}{" (Main: "}{main_total}
                        {", Sideboard: "}{sideboard_total}{")"}
                    </div>

                    <div class="grid grid-cols-2 gap-6">
                        <div class="space-y-6">
                            {render_non_land_cards(main_deck.clone())}
                        </div>
                        <div class="space-y-6">
                            {render_lands(main_deck.clone())}
                            {render_sideboard(sideboard.clone())}
                        </div>
                    </div>
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

fn render_non_land_cards(main_deck: HashMap<CardType, Vec<CardDisplayRecord>>) -> impl IntoView {
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

    let sections: Vec<_> = ordered_types
        .into_iter()
        .filter_map(|card_type| {
            main_deck.get(&card_type).filter(|cards| !cards.is_empty()).cloned().map(|cards| {
                view! {
                    <div class="mb-4">
                        <h4 class="text-md font-medium text-gray-700 mb-2">
                            {format!("{} ({})", card_type, cards.len())}
                        </h4>
                        <div class="space-y-1">
                            {cards.into_iter().map(|card| render_card_row(card)).collect::<Vec<_>>()}
                        </div>
                    </div>
                }
            })
        })
        .collect();

    sections
}

fn render_lands(main_deck: HashMap<CardType, Vec<CardDisplayRecord>>) -> impl IntoView {
    if let Some(lands) = main_deck
        .get(&CardType::Land)
        .filter(|l| !l.is_empty())
        .cloned()
    {
        view! {
            <div>
                <h3 class="text-lg font-semibold text-gray-800 border-b pb-2">
                    {format!("Lands ({})", lands.len())}
                </h3>
                <div class="space-y-1 mt-2">
                    {lands.into_iter().map(|card| render_card_row(card)).collect::<Vec<_>>()}
                </div>
            </div>
        }
        .into_any()
    } else {
        view! { <div></div> }.into_any()
    }
}

fn render_sideboard(sideboard: Vec<CardDisplayRecord>) -> impl IntoView {
    if sideboard.is_empty() {
        view! { <div></div> }.into_any()
    } else {
        view! {
            <div>
                <h3 class="text-lg font-semibold text-gray-800 border-b pb-2">
                    {format!("Sideboard ({})", sideboard.len())}
                </h3>
                <div class="space-y-1 mt-2">
                    {sideboard.into_iter().map(|card| render_card_row(card)).collect::<Vec<_>>()}
                </div>
            </div>
        }
        .into_any()
    }
}

fn render_card_row(card: CardDisplayRecord) -> impl IntoView {
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
}
