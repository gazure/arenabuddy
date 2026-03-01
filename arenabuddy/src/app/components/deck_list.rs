use arenabuddy_core::{
    display::{card::CardDisplayRecord, deck::DeckDisplayRecord},
    models::CardType,
};
use dioxus::prelude::*;

use crate::app::components::ManaCost;

#[component]
pub fn DeckList(
    deck: DeckDisplayRecord,
    #[props(optional)] title: Option<&'static str>,
    #[props(default = true)] show_quantities: bool,
) -> Element {
    let title = title.unwrap_or("Your Deck");
    let (main_total, sideboard_total) = deck.totals();
    let hovered_card = use_signal(|| None::<(CardDisplayRecord, (f64, f64))>);

    rsx! {
        div { class: "bg-gray-800 rounded-lg border border-gray-700 overflow-hidden",
            div { class: "bg-gradient-to-r from-violet-800 to-indigo-900 py-4 px-6",
                h2 { class: "text-xl font-bold text-white", "{title}" }
            }
            div { class: "p-6",
                div { class: "deck-content",
                    if show_quantities {
                        div { class: "mb-4 text-right text-sm text-gray-500",
                            "Main: {main_total}, Sideboard: {sideboard_total}"
                        }
                    }

                    div { class: "grid grid-cols-2 gap-6",
                        div { class: "space-y-6",
                            NonLandCards { deck: deck.clone(), hovered_card, show_quantities }
                        }
                        div { class: "space-y-6",
                            Lands { deck: deck.clone(), hovered_card, show_quantities }
                            Sideboard { deck: deck.clone(), hovered_card, show_quantities }
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
                    background: #1f2937;
                    border-radius: 8px;
                }}

                .deck-scrollbar::-webkit-scrollbar-thumb {{
                    background: #4b5563;
                    border-radius: 8px;
                }}

                .deck-scrollbar::-webkit-scrollbar-thumb:hover {{
                    background: #6b7280;
                }}
                "#
            }
        }

        // Floating card preview
        if let Some((card, (x, y))) = hovered_card() {
            div {
                class: "fixed z-50 pointer-events-none",
                style: "left: {x + 10.0}px; top: {y - 200.0}px;",
                div { class: "bg-black bg-opacity-90 rounded-lg p-2 shadow-2xl",
                    img {
                        src: "{card.image_uri}",
                        alt: "{card.name}",
                        class: "rounded",
                        style: "width: 250px; height: auto;",
                    }
                }
            }
        }
    }
}

#[component]
fn NonLandCards(
    deck: DeckDisplayRecord,
    hovered_card: Signal<Option<(CardDisplayRecord, (f64, f64))>>,
    #[props(default = true)] show_quantities: bool,
) -> Element {
    let main_deck = &deck.main_deck;

    rsx! {
        for card_type in CardType::iter().filter(|ct| *ct != CardType::Land) {
            if let Some(cards) = main_deck.get(&card_type) {
                if !cards.is_empty() {
                    div { class: "mb-4",
                        h4 { class: "text-md font-medium text-gray-300 mb-2",
                            if show_quantities {
                                "{card_type} ({deck.total_by_type(card_type)})"
                            } else {
                                "{card_type}"
                            }
                        }
                        div { class: "space-y-1",
                            for card in cards {
                                CardRow { card: card.clone(), hovered_card, show_quantities }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Lands(
    deck: DeckDisplayRecord,
    hovered_card: Signal<Option<(CardDisplayRecord, (f64, f64))>>,
    #[props(default = true)] show_quantities: bool,
) -> Element {
    let main_deck = &deck.main_deck;
    if let Some(lands) = main_deck.get(&CardType::Land).filter(|l| !l.is_empty()) {
        rsx! {
            div {
                h3 { class: "text-lg font-semibold text-gray-200 border-b border-gray-700 pb-2",
                    if show_quantities {
                        "Lands ({deck.total_by_type(CardType::Land)})"
                    } else {
                        "Lands"
                    }
                }
                div { class: "space-y-1 mt-2",
                    for land in lands {
                        CardRow { card: land.clone(), hovered_card, show_quantities }
                    }
                }
            }
        }
    } else {
        rsx! { div {} }
    }
}

#[component]
fn Sideboard(
    deck: DeckDisplayRecord,
    hovered_card: Signal<Option<(CardDisplayRecord, (f64, f64))>>,
    #[props(default = true)] show_quantities: bool,
) -> Element {
    let sideboard = &deck.sideboard;
    if sideboard.is_empty() {
        rsx! { div {} }
    } else {
        rsx! {
            div {
                h3 { class: "text-lg font-semibold text-gray-200 border-b border-gray-700 pb-2",
                    if show_quantities {
                        "Sideboard ({sideboard.len()})"
                    } else {
                        "Sideboard"
                    }
                }
                div { class: "space-y-1 mt-2",
                    for card in sideboard {
                        CardRow { card: card.clone(), hovered_card, show_quantities }
                    }
                }
            }
        }
    }
}

#[component]
fn CardRow(
    card: CardDisplayRecord,
    hovered_card: Signal<Option<(CardDisplayRecord, (f64, f64))>>,
    #[props(default = true)] show_quantities: bool,
) -> Element {
    rsx! {
        div {
            class: "flex items-center justify-between py-1 px-2 hover:bg-gray-700/50 rounded text-sm cursor-pointer",
            onmouseenter: move |event| {
                let coords = event.client_coordinates();
                hovered_card.set(Some((card.clone(), (coords.x, coords.y))));
            },
            onmouseleave: move |_| {
                hovered_card.set(None);
            },
            div { class: "flex items-center space-x-2",
                if show_quantities {
                    span { class: "font-medium text-gray-400 w-6 text-center",
                        "{card.quantity}"
                    }
                }
                span { class: "truncate", "{card.name.clone()}" }
            }
            div { class: "flex-shrink-0 ml-2",
                ManaCost { cost: card.cost() }
            }
        }
    }
}
