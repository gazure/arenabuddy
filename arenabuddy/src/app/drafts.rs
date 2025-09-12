use arenabuddy_core::models::Draft;
use dioxus::prelude::*;
use dioxus_router::Link;

use crate::{app::Route, backend::Service};

#[component]
fn DraftRow(draft: Draft) -> Element {
    rsx! {
        Link {
            to: Route::DraftDetails { id: draft.id().to_string() },
            class: "table-row hover:bg-gray-100 transition-colors duration-150 cursor-pointer",
            td { class: "py-3 px-4 border-b",
                span { class: "text-blue-600 font-medium",
                    "{draft.set_code()}"
                }
            }
            td { class: "py-3 px-4 border-b", "{draft.format()}" }
            td { class: "py-3 px-4 border-b",
                span { class: if draft.status() == "DraftStatus_Complete" { "text-green-600" } else { "text-yellow-600" },
                    "COMPLETE"
                }
            }
            td { class: "py-3 px-4 border-b text-gray-500", "{draft.created_at()}" }
        }
    }
}

#[component]
pub fn Drafts() -> Element {
    let service = use_context::<Service>();
    let mut drafts = use_signal(|| None::<Vec<Draft>>);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    let service2 = service.clone();
    // Load drafts on component mount
    use_future({
        move || {
            let service2 = service2.clone();
            async move {
                match service2.get_drafts().await {
                    Ok(data) => {
                        drafts.set(Some(data));
                        error.set(None);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load drafts: {e}")));
                        drafts.set(None);
                    }
                }
                loading.set(false);
            }
        }
    });

    let refresh_drafts = {
        let service = service.clone();
        let mut loading = loading;
        let mut drafts = drafts;
        let mut error = error;
        move |_| {
            loading.set(true);
            error.set(None);
            let service = service.clone();
            spawn(async move {
                match service.get_drafts().await {
                    Ok(data) => {
                        drafts.set(Some(data));
                        error.set(None);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to refresh drafts: {e}")));
                        drafts.set(None);
                    }
                }
                loading.set(false);
            });
        }
    };

    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-5xl",
            div { class: "flex justify-between items-center mb-6",
                h1 { class: "text-2xl font-bold text-gray-800", "Draft History" }
                button {
                    onclick: refresh_drafts,
                    class: "bg-blue-600 hover:bg-blue-700 text-white py-2 px-4 rounded shadow transition-colors duration-150 flex items-center",
                    disabled: loading(),
                    if loading() {
                        "Loading..."
                    } else {
                        "Refresh Drafts"
                    }
                }
            }

            if let Some(err) = error() {
                div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4",
                    p { "{err}" }
                }
            }

            div { class: "bg-white rounded-lg shadow-md overflow-hidden",
                if loading() && drafts().is_none() {
                    div { class: "p-12 text-center text-gray-500",
                        div { class: "animate-pulse", "Loading draft data..." }
                    }
                } else if let Some(drafts_data) = drafts.read().as_ref() {
                    div { class: "p-4 border-b bg-gray-50",
                        p { class: "text-gray-600",
                            span { class: "font-medium", "{drafts_data.len()}" }
                            " drafts found"
                        }
                    }
                    if drafts_data.is_empty() {
                        div { class: "p-12 text-center text-gray-500",
                            "No drafts found. Start a draft in MTG Arena!"
                        }
                    } else {
                        div { class: "overflow-x-auto",
                            table { class: "min-w-full table-auto",
                                thead {
                                    tr { class: "bg-gray-100 text-left",
                                        th { class: "py-3 px-4 font-semibold text-gray-700", "Set" }
                                        th { class: "py-3 px-4 font-semibold text-gray-700", "Format" }
                                        th { class: "py-3 px-4 font-semibold text-gray-700", "Status" }
                                        th { class: "py-3 px-4 font-semibold text-gray-700", "Date" }
                                    }
                                }
                                tbody {
                                    for draft in drafts_data {
                                        DraftRow { key: "{draft.id()}", draft: draft.clone() }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "p-12 text-center text-gray-500",
                        "No draft data available"
                    }
                }
            }
        }
    }
}
