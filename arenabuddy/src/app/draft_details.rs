use arenabuddy_core::models::MTGADraft;
use dioxus::prelude::*;

use crate::{app::pages::Route, backend::Service};

#[component]
pub fn DraftDetails(id: String) -> Element {
    let service = use_context::<Service>();
    let mut draft = use_signal(|| None::<MTGADraft>);
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
        div { class: "container mx-auto px-4 py-8 max-w-6xl",
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
                div { class: "bg-white rounded-lg shadow-md p-6 mb-6",
                    h1 { class: "text-2xl font-bold text-gray-800 mb-4",
                        "{draft_data.draft().set_code()} Draft"
                    }

                    div { class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                        div {
                            p { class: "text-sm text-gray-600", "Format" }
                            p { class: "font-semibold", "{draft_data.draft().format()}" }
                        }
                        div {
                            p { class: "text-sm text-gray-600", "Status" }
                            p {
                                class: "font-semibold",
                                class: if draft_data.draft().status() == "DraftStatus_Complete" { "text-green-600" } else { "text-yellow-600" },
                                "COMPLETE"
                            }
                        }
                        div {
                            p { class: "text-sm text-gray-600", "Date" }
                            p { class: "font-semibold", "{draft_data.draft().created_at()}" }
                        }
                        div {
                            p { class: "text-sm text-gray-600", "Total Picks" }
                            p { class: "font-semibold", "{draft_data.packs().len()}" }
                        }
                    }
                }

                // Packs and Picks
                div { class: "bg-white rounded-lg shadow-md p-6",
                    h2 { class: "text-xl font-bold text-gray-800 mb-4", "Draft Picks" }

                    if draft_data.packs().is_empty() {
                        div { class: "text-center text-gray-500 py-8",
                            "No picks recorded for this draft"
                        }
                    } else {
                        div { class: "space-y-6",
                            for (pack_index, pack_picks) in draft_data.by_packs().into_iter().enumerate() {
                                if !pack_picks.is_empty() {
                                    div {
                                        class: "border-b pb-4 mb-4 last:border-b-0",
                                        h3 { class: "text-lg font-semibold mb-3",
                                            "Pack {pack_index + 1}"
                                        }
                                        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3",
                                            for pick in pack_picks {
                                                {
                                                    let picked_card_name = service.cards.get(&pick.picked_card()).map_or_else(|| format!("Card #{}", pick.picked_card().inner()), |c| c.name.clone());

                                                    let available_count = pick.cards().len();

                                                    rsx! {
                                                        div { class: "bg-gray-50 rounded p-3",
                                                            div { class: "flex justify-between items-start mb-1",
                                                                span { class: "text-sm text-gray-600",
                                                                    "Pick {pick.pick_number()}"
                                                                }
                                                                span { class: "text-xs text-gray-500",
                                                                    "{available_count} cards"
                                                                }
                                                            }
                                                            p { class: "font-medium text-blue-600",
                                                                "{picked_card_name}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "bg-white rounded-lg shadow-md p-12 text-center text-gray-500",
                    "Draft not found"
                }
            }
        }
    }
}
