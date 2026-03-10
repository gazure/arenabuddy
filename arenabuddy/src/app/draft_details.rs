use arenabuddy_core::display::{
    card::CardDisplayRecord,
    draft::{DraftDetailsDisplay, EnrichedDraftPack},
};
use dioxus::prelude::*;

use crate::{app::pages::Route, backend::Service};

#[component]
pub fn DraftDetails(id: String) -> Element {
    let service = use_context::<Service>();

    // Load draft details using use_resource
    let draft_resource = use_resource({
        let service = service.clone();
        let id = id.clone();
        move || {
            let service = service.clone();
            let id = id.clone();
            async move { service.get_draft_details(id).await }
        }
    });

    let resource_value = draft_resource.value();
    let data = resource_value.read();

    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-7xl",
            // Back button
            div { class: "mb-6",
                Link {
                    to: Route::Drafts {  },
                    class: "inline-flex items-center bg-gray-700 hover:bg-gray-600 text-gray-200 font-semibold py-2 px-4 rounded-full transition-all duration-200",
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

            match data.as_ref() {
                None => rsx! {
                    div { class: "bg-gray-800 rounded-lg border border-gray-700 p-12 text-center text-gray-500",
                        div { class: "animate-pulse", "Loading draft details..." }
                    }
                },
                Some(Err(err)) => rsx! {
                    div { class: "bg-red-900/30 border border-red-700 text-red-300 px-4 py-3 rounded mb-4",
                        p { "Failed to load draft details: {err}" }
                    }
                },
                Some(Ok(draft_data)) => rsx! {
                    // Draft Header
                    DraftHeader { draft: draft_data.clone() }

                    // Draft Packs
                    DraftPacksSection { draft: draft_data.clone() }
                },
            }
        }
    }
}

#[component]
fn DraftHeader(draft: DraftDetailsDisplay) -> Element {
    rsx! {
        div { class: "bg-gray-800 rounded-lg border border-gray-700 p-6 mb-6",
            h1 { class: "text-2xl font-bold text-gray-100 mb-4",
                "{draft.metadata().set_code()} Draft"
            }

            div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                div {
                    p { class: "text-sm text-gray-400", "Format" }
                    p { class: "font-semibold", "{draft.metadata().format()}" }
                }
                div {
                    p { class: "text-sm text-gray-400", "Status" }
                    p {
                        class: "font-semibold",
                        class: if draft.metadata().status() == "DraftStatus_Complete" { "text-amber-400" } else { "text-yellow-400" },
                        if draft.metadata().status() == "DraftStatus_Complete" { "COMPLETE" } else { "IN PROGRESS" }
                    }
                }
                div {
                    p { class: "text-sm text-gray-400", "Date" }
                    p { class: "font-semibold",
                            "{super::format_local_datetime(*draft.metadata().created_at())}"
                        }
                }
                div {
                    p { class: "text-sm text-gray-400", "Total Picks" }
                    p { class: "font-semibold", "{draft.total_picks()}" }
                }
            }
        }
    }
}

#[component]
fn DraftPacksSection(draft: DraftDetailsDisplay) -> Element {
    rsx! {
        div { class: "bg-gray-800 rounded-lg border border-gray-700 p-6",
            h2 { class: "text-xl font-bold text-gray-100 mb-4", "Draft Picks" }

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
        div { class: "border-b border-gray-700 pb-6 last:border-b-0",
            h3 { class: "text-lg font-semibold text-gray-200 mb-4 flex items-center",
                span { class: "bg-amber-900/40 text-amber-300 px-3 py-1 rounded-full text-sm mr-2",
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
        div { class: "border border-gray-700 rounded-lg p-3 bg-gray-900 hover:bg-gray-800 transition-colors",
            // Pack header with pick info and picked card inline
            div { class: "flex justify-between items-start gap-3",
                div { class: "flex items-start gap-3 flex-1",
                    // Pick info
                    div { class: "flex-shrink-0",
                        div { class: "flex items-center gap-2 mb-1",
                            span { class: "text-sm font-medium text-gray-300",
                                "Pick {pack.pick_number()}"
                            }
                            span { class: "text-xs text-gray-500",
                                "({pack.available_count()} cards)"
                            }
                        }
                        if let Some(picked_name) = pack.picked_card_name() {
                            p { class: "text-xs text-amber-400 font-medium truncate max-w-[150px]",
                                "{picked_name}"
                            }
                        }
                    }

                    // Picked card image (small)
                    if let Some(picked) = pack.picked() {
                        CardDisplay { card: picked.clone(), is_picked: true, size: "medium" }
                    }
                }

                button {
                    class: "text-xs text-amber-400 hover:text-amber-300 font-medium flex-shrink-0",
                    onclick: move |_| show_available.set(!show_available()),
                    if show_available() { "Hide" } else { "Show all" }
                }
            }

            // Available cards (collapsible)
            if show_available() {
                div { class: "mt-4 pt-4 border-t border-gray-700",
                    p { class: "text-xs text-gray-500 mb-3", "Available cards:" }
                    div { class: "grid grid-cols-4 sm:grid-cols-5 md:grid-cols-6 lg:grid-cols-8 xl:grid-cols-10 gap-2",
                        for card in pack.available() {
                            CardDisplay { card: card.clone(), is_picked: false, size: "small" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CardDisplay(card: CardDisplayRecord, is_picked: bool, size: String) -> Element {
    let bg_class = if is_picked {
        "bg-amber-900/30 border-amber-500"
    } else {
        "bg-gray-800 border-gray-600"
    };

    let text_class = if is_picked {
        "text-amber-300 font-semibold"
    } else {
        "text-gray-300"
    };

    let size_class = match size.as_str() {
        "small" => "w-[120px]",
        "medium" => "w-[160px]",
        _ => "w-[180px]",
    };

    rsx! {
        div {
            class: "rounded-lg border-2 {bg_class} hover:shadow-lg transition-all cursor-pointer overflow-hidden {size_class}",
            title: "{card.name}",

            div { class: "flex flex-col",
                // Card image
                if !card.image_uri.is_empty() {
                    div { class: "relative",
                        img {
                            src: "{card.image_uri}",
                            alt: "{card.name}",
                            class: "w-full h-auto object-cover",
                            loading: "lazy"
                        }
                        if is_picked && size != "small" {
                            div { class: "absolute top-1 right-1 bg-amber-500 text-white text-xs px-1.5 py-0.5 rounded-full shadow font-semibold",
                                "✓"
                            }
                        }
                    }
                } else {
                    // Fallback when no image
                    div { class: "p-3 bg-gray-700",
                        // Card name
                        p { class: "{text_class} text-sm font-medium",
                            "{card.name}"
                        }

                        // Card details
                        div { class: "flex justify-between items-center mt-1",
                            span { class: "text-xs text-gray-400",
                                "{card.type_field}"
                            }
                            if !card.mana.is_empty() {
                                span { class: "text-xs text-gray-300 font-mono",
                                    "{card.mana}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
