use arenabuddy_core::{
    display::draft::{DraftDetailsDisplay, EnrichedDraftPack},
    models::Card,
};
use dioxus::prelude::*;

use crate::{app::pages::Route, backend::Service};

#[component]
pub fn DraftDetails(id: String) -> Element {
    let service = use_context::<Service>();
    let mut draft = use_signal(|| None::<DraftDetailsDisplay>);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    let service2 = service.clone();
    let id2 = id.clone();

    // Load draft details on component mount
    use_future({
        move || {
            let service2 = service2.clone();
            let id2 = id2.clone();
            async move {
                match service2.get_draft_details(id2).await {
                    Ok(data) => {
                        draft.set(Some(data));
                        error.set(None);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load draft details: {e}")));
                        draft.set(None);
                    }
                }
                loading.set(false);
            }
        }
    });

    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-7xl",
            // Back button
            div { class: "mb-6",
                Link {
                    to: Route::Drafts {  },
                    class: "inline-flex items-center bg-gray-200 hover:bg-gray-300 text-gray-800 font-semibold py-2 px-4 rounded-full transition-all duration-200 shadow-sm hover:shadow-md",
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        class: "h-5 w-5 mr-2",
                        fill: "none",
                        view_box: "0 0 24 24",
                        stroke: "currentColor",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M10 19l-7-7m0 0l7-7m-7 7h18"
                        }
                    }
                    "Back to Drafts"
                }
            }

            if let Some(err) = error() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                    p { "{err}" }
                }
            }

            if loading() {
                div { class: "bg-white rounded-lg shadow-md p-12 text-center text-gray-500",
                    div { class: "animate-pulse", "Loading draft details..." }
                }
            } else if let Some(draft_data) = draft.read().as_ref() {
                // Draft Header
                DraftHeader { draft: draft_data.clone() }

                // Draft Packs
                DraftPacksSection { draft: draft_data.clone() }
            } else {
                div { class: "bg-white rounded-lg shadow-md p-12 text-center text-gray-500",
                    "Draft not found"
                }
            }
        }
    }
}

#[component]
fn DraftHeader(draft: DraftDetailsDisplay) -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6 mb-6",
            h1 { class: "text-2xl font-bold text-gray-800 mb-4",
                "{draft.metadata().set_code()} Draft"
            }

            div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                div {
                    p { class: "text-sm text-gray-600", "Format" }
                    p { class: "font-semibold", "{draft.metadata().format()}" }
                }
                div {
                    p { class: "text-sm text-gray-600", "Status" }
                    p {
                        class: "font-semibold",
                        class: if draft.metadata().status() == "DraftStatus_Complete" { "text-green-600" } else { "text-yellow-600" },
                        if draft.metadata().status() == "DraftStatus_Complete" { "COMPLETE" } else { "IN PROGRESS" }
                    }
                }
                div {
                    p { class: "text-sm text-gray-600", "Date" }
                    p { class: "font-semibold", "{draft.metadata().created_at()}" }
                }
                div {
                    p { class: "text-sm text-gray-600", "Total Picks" }
                    p { class: "font-semibold", "{draft.total_picks()}" }
                }
            }
        }
    }
}

#[component]
fn DraftPacksSection(draft: DraftDetailsDisplay) -> Element {
    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-6",
            h2 { class: "text-xl font-bold text-gray-800 mb-4", "Draft Picks" }

            if draft.packs().is_empty() {
                div { class: "text-center text-gray-500 py-8",
                    "No picks recorded for this draft"
                }
            } else {
                div { class: "space-y-8",
                    // Group packs by pack number
                    for (pack_num, packs) in draft.by_packs().into_iter().enumerate() {
                        if !packs.is_empty() {
                            PackSection {
                                pack_number: pack_num + 1,
                                packs: packs.into_iter().cloned().collect()
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PackSection(pack_number: usize, packs: Vec<EnrichedDraftPack>) -> Element {
    rsx! {
        div { class: "border-b pb-6 last:border-b-0",
            h3 { class: "text-lg font-semibold text-gray-800 mb-4 flex items-center",
                span { class: "bg-blue-100 text-blue-800 px-3 py-1 rounded-full text-sm mr-2",
                    "Pack {pack_number}"
                }
                span { class: "text-sm text-gray-500 font-normal",
                    "{packs.len()} picks"
                }
            }

            div { class: "space-y-4",
                for pack in packs {
                    DraftPackRow { pack: pack }
                }
            }
        }
    }
}

#[component]
fn DraftPackRow(pack: EnrichedDraftPack) -> Element {
    let mut show_available = use_signal(|| false);

    rsx! {
        div { class: "border rounded-lg p-4 bg-gray-50 hover:bg-gray-100 transition-colors",
            // Pack header with pick info
            div { class: "flex justify-between items-start mb-3",
                div { class: "flex items-center gap-2",
                    span { class: "text-sm font-medium text-gray-700",
                        "Pick {pack.pick_number}"
                    }
                    span { class: "text-xs text-gray-500",
                        "({pack.available_count()} cards available)"
                    }
                }

                button {
                    class: "text-xs text-blue-600 hover:text-blue-800 font-medium",
                    onclick: move |_| show_available.set(!show_available()),
                    if show_available() { "Hide Cards" } else { "Show Cards" }
                }
            }

            // Picked card
            div { class: "mb-2",
                p { class: "text-xs text-gray-500 mb-1", "Picked:" }
                if let Some(picked) = pack.picked() {
                    CardDisplay { card: picked.clone(), is_picked: true }
                } else {
                    div { class: "text-gray-400 italic", "No card picked" }
                }
            }

            // Available cards (collapsible)
            if show_available() {
                div { class: "mt-4 pt-4 border-t border-gray-200",
                    p { class: "text-xs text-gray-500 mb-2", "Available cards:" }
                    div { class: "grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-2",
                        for card in pack.available() {
                            CardDisplay { card: card.clone(), is_picked: false }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CardDisplay(card: Card, is_picked: bool) -> Element {
    let bg_class = if is_picked {
        "bg-blue-100 border-blue-300"
    } else {
        "bg-white border-gray-200"
    };

    let text_class = if is_picked {
        "text-blue-700 font-semibold"
    } else {
        "text-gray-700"
    };

    rsx! {
        div {
            class: "p-2 rounded border {bg_class} hover:shadow-sm transition-shadow",
            title: "{card.name()}",

            div { class: "flex flex-col",
                // Card name
                p { class: "{text_class} text-sm truncate",
                    "{card.name()}"
                }

                // Card details
                div { class: "flex justify-between items-center mt-1",
                    span { class: "text-xs text-gray-500 truncate",
                        "{card.type_line()}"
                    }
                    if !card.mana_cost_str().is_empty() {
                        span { class: "text-xs text-gray-600 font-mono",
                            "{card.mana_cost_str()}"
                        }
                    }
                }
            }
        }
    }
}
