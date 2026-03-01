use arenabuddy_core::models::Draft;
use dioxus::prelude::*;
use dioxus_router::Link;

use crate::{
    app::{Route, components::Pagination},
    backend::Service,
};

const PAGE_SIZE: usize = 25;

#[component]
fn DraftRow(draft: Draft) -> Element {
    rsx! {
        Link {
            to: Route::DraftDetails { id: draft.id().to_string() },
            class: "table-row hover:bg-gray-700/50 transition-colors duration-150 cursor-pointer",
            td { class: "py-3 px-4 border-b border-gray-700",
                span { class: "text-amber-400 font-medium",
                    "{draft.set_code()}"
                }
            }
            td { class: "py-3 px-4 border-b border-gray-700", "{draft.format()}" }
            td { class: "py-3 px-4 border-b border-gray-700",
                span { class: if draft.status() == "DraftStatus_Complete" { "text-amber-400" } else { "text-yellow-400" },
                    if draft.status() == "DraftStatus_Complete" { "COMPLETE" } else { "IN PROGRESS" }
                }
            }
            td { class: "py-3 px-4 border-b border-gray-700 text-gray-500", "{draft.created_at()}" }
        }
    }
}

#[component]
pub fn Drafts() -> Element {
    let service = use_context::<Service>();
    let mut current_page = use_signal(|| 0usize);

    let mut drafts_resource = use_resource({
        let service = service.clone();
        move || {
            let service = service.clone();
            async move { service.get_drafts().await }
        }
    });

    let refresh_drafts = move |_| {
        current_page.set(0);
        drafts_resource.restart();
    };

    let resource_value = drafts_resource.value();
    let data = resource_value.read();

    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-5xl",
            div { class: "flex justify-between items-center mb-6",
                h1 { class: "text-2xl font-bold text-gray-100", "Draft History" }
                button {
                    onclick: refresh_drafts,
                    class: "bg-amber-600 hover:bg-amber-700 text-white py-2 px-4 rounded transition-colors duration-150 flex items-center",
                    disabled: data.is_none(),
                    if data.is_none() {
                        "Loading..."
                    } else {
                        "Refresh Drafts"
                    }
                }
            }

            div { class: "bg-gray-800 rounded-lg border border-gray-700 overflow-hidden",
                match data.as_ref() {
                    None => rsx! {
                        div { class: "p-12 text-center text-gray-500",
                            div { class: "animate-pulse", "Loading draft data..." }
                        }
                    },
                    Some(Err(err)) => rsx! {
                        div { class: "bg-red-900/30 border border-red-700 text-red-300 px-4 py-3 rounded m-4",
                            p { "Failed to load drafts: {err}" }
                        }
                    },
                    Some(Ok(drafts_data)) => {
                        let total_items = drafts_data.len();
                        let total_pages = total_items.div_ceil(PAGE_SIZE).max(1);
                        let page = current_page().min(total_pages.saturating_sub(1));
                        let start = page * PAGE_SIZE;
                        let end = (start + PAGE_SIZE).min(total_items);

                        rsx! {
                            if drafts_data.is_empty() {
                                div { class: "p-12 text-center text-gray-500",
                                    "No drafts found. Start a draft in MTG Arena!"
                                }
                            } else {
                                Pagination {
                                    current_page,
                                    total_pages,
                                    total_items,
                                    page_size: PAGE_SIZE,
                                }
                                div { class: "overflow-x-auto",
                                    table { class: "min-w-full table-auto",
                                        thead {
                                            tr { class: "bg-gray-900 text-left",
                                                th { class: "py-3 px-4 font-semibold text-gray-400", "Set" }
                                                th { class: "py-3 px-4 font-semibold text-gray-400", "Format" }
                                                th { class: "py-3 px-4 font-semibold text-gray-400", "Status" }
                                                th { class: "py-3 px-4 font-semibold text-gray-400", "Date" }
                                            }
                                        }
                                        tbody {
                                            for draft in &drafts_data[start..end] {
                                                DraftRow { key: "{draft.id()}", draft: draft.clone() }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}
