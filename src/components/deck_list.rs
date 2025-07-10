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
    let main_deck = Memo::new(move |_| deck.get().main_deck);
    let sideboard = Memo::new(move |_| deck.get().sideboard);

    let main_total: Memo<u16> = Memo::new(move |_| {
        main_deck
            .get()
            .values()
            .flat_map(|cards| cards.iter())
            .map(|c| c.quantity)
            .sum()
    });
    let sideboard_total: Memo<u16> =
        Memo::new(move |_| sideboard.get().iter().map(|c| c.quantity).sum());
    let total_count = Memo::new(move |_| main_total.get() + sideboard_total.get());

    view! {
        <div class="bg-white rounded-lg shadow-md overflow-hidden">
            <div class="bg-gradient-to-r from-indigo-500 to-indigo-600 py-4 px-6">
                <h2 class="text-xl font-bold text-white">{title}</h2>
            </div>
            <div class="p-6">
                <div class="deck-content">
                    <div class="mb-4 text-right text-sm text-gray-500">
                        {"Total cards: "}{move || total_count.get()}{" (Main: "}{move|| main_total.get()}
                        {", Sideboard: "}{move || sideboard_total.get()}{")"}
                    </div>

                    <div class="grid grid-cols-2 gap-6">
                        <div class="space-y-6">
                            {move || render_non_land_cards(main_deck.get())}
                        </div>
                        <div class="space-y-6">
                            {move || render_lands(main_deck.get())}
                            {move || render_sideboard(sideboard.get())}
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
            main_deck
                .get(&card_type)
                .filter(|cards| !cards.is_empty())
                .cloned()
                .map(|cards| {
                    view! {
                        <div class="mb-4">
                            <h4 class="text-md font-medium text-gray-700 mb-2">
                                {format!("{} ({})", card_type, cards.len())}
                            </h4>
                            <div class="space-y-1">
                                {cards.into_iter().map(render_card_row).collect::<Vec<_>>()}
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
                    {lands.into_iter().map(render_card_row).collect::<Vec<_>>()}
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
                    {sideboard.into_iter().map(render_card_row).collect::<Vec<_>>()}
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
