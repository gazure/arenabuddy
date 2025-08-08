use std::collections::HashMap;

use arenabuddy_core::{
    display::{card::CardDisplayRecord, deck::DeckDisplayRecord},
    models::CardType,
};
use dioxus::prelude::*;

use crate::components::ManaCost;

#[component]
pub fn DeckList(deck: DeckDisplayRecord, #[props(optional)] title: Option<&'static str>) -> Element {
    let title = title.unwrap_or("Your Deck");
    let main_deck = deck.main_deck;
    let sideboard = deck.sideboard;

    let main_total: u16 = main_deck
        .values()
        .flat_map(|cards| cards.iter())
        .map(|c| c.quantity)
        .sum();
    let sideboard_total: u16 = sideboard.iter().map(|c| c.quantity).sum();
    let total_count = main_total + sideboard_total;

    rsx! {
        div { class: "bg-white rounded-lg shadow-md overflow-hidden",
            div { class: "bg-gradient-to-r from-indigo-500 to-indigo-600 py-4 px-6",
                h2 { class: "text-xl font-bold text-white", "{title}" }
            }
            div { class: "p-6",
                div { class: "deck-content",
                    div { class: "mb-4 text-right text-sm text-gray-500",
                        "Total cards: {total_count} (Main: {main_total}, Sideboard: {sideboard_total})"
                    }

                    div { class: "grid grid-cols-2 gap-6",
                        div { class: "space-y-6",
                            {render_non_land_cards(main_deck.clone())}
                        }
                        div { class: "space-y-6",
                            {render_lands(main_deck.clone())}
                            {render_sideboard(sideboard)}
                        }
                    }
                }
            }

            style {
                r#"
                .deck-scrollbar::-webkit-scrollbar {{
                    width: 8px;
                }}

                .deck-scrollbar::-webkit-scrollbar-track {{
                    background: #f1f1f1;
                    border-radius: 8px;
                }}

                .deck-scrollbar::-webkit-scrollbar-thumb {{
                    background: #c5c5c5;
                    border-radius: 8px;
                }}

                .deck-scrollbar::-webkit-scrollbar-thumb:hover {{
                    background: #a0a0a0;
                }}
                "#
            }
        }
    }
}

fn render_non_land_cards(main_deck: HashMap<CardType, Vec<CardDisplayRecord>>) -> Element {
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

    rsx! {
        for card_type in ordered_types {
            if let Some(cards) = main_deck.get(&card_type) {
                if !cards.is_empty() {
                    div { class: "mb-4",
                        h4 { class: "text-md font-medium text-gray-700 mb-2",
                            "{card_type} ({cards.len()})"
                        }
                        div { class: "space-y-1",
                            for card in cards {
                                {render_card_row(card.clone())}
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_lands(main_deck: HashMap<CardType, Vec<CardDisplayRecord>>) -> Element {
    if let Some(lands) = main_deck.get(&CardType::Land).filter(|l| !l.is_empty()) {
        rsx! {
            div {
                h3 { class: "text-lg font-semibold text-gray-800 border-b pb-2",
                    "Lands ({lands.len()})"
                }
                div { class: "space-y-1 mt-2",
                    for land in lands {
                        {render_card_row(land.clone())}
                    }
                }
            }
        }
    } else {
        rsx! { div {} }
    }
}

fn render_sideboard(sideboard: Vec<CardDisplayRecord>) -> Element {
    if sideboard.is_empty() {
        rsx! { div {} }
    } else {
        rsx! {
            div {
                h3 { class: "text-lg font-semibold text-gray-800 border-b pb-2",
                    "Sideboard ({sideboard.len()})"
                }
                div { class: "space-y-1 mt-2",
                    for card in sideboard {
                        {render_card_row(card)}
                    }
                }
            }
        }
    }
}

fn render_card_row(card: CardDisplayRecord) -> Element {
    rsx! {
        div { class: "flex items-center justify-between py-1 px-2 hover:bg-gray-50 rounded text-sm",
            div { class: "flex items-center space-x-2",
                span { class: "font-medium text-gray-600 w-6 text-center",
                    "{card.quantity}"
                }
                span { class: "truncate", "{card.name}" }
            }
            div { class: "flex-shrink-0 ml-2",
                ManaCost { cost: card.cost() }
            }
        }
    }
}
