use dioxus::prelude::*;

use crate::{
    app::components::{ManaCost, Pagination},
    backend::{CardDatabaseSummary, CardSearchResult, Service},
};

const PAGE_SIZE: usize = 25;

fn format_colors(colors: &[String], empty_label: &str) -> String {
    if colors.is_empty() {
        empty_label.to_string()
    } else {
        colors.join(", ")
    }
}

fn open_url(url: String) {
    if let Err(err) = open::that(url) {
        tracing::error!("Failed to open URL: {err}");
    }
}

#[component]
pub fn Cards() -> Element {
    let service = use_context::<Service>();
    let mut search_query = use_signal(String::new);
    let mut set_filter = use_signal(String::new);
    let mut current_page = use_signal(|| 0usize);
    let mut lookup_id = use_signal(String::new);
    let mut lookup_status = use_signal(|| None::<String>);
    let mut lookup_loading = use_signal(|| false);
    let mut selected_card = use_signal(|| None::<CardSearchResult>);

    let summary_resource = use_resource({
        let service = service.clone();
        move || {
            let service = service.clone();
            async move { service.get_card_database_summary() }
        }
    });

    let mut search_resource = use_resource({
        let service = service.clone();
        move || {
            let service = service.clone();
            let query = search_query();
            let set = set_filter();
            async move {
                let set_filter = if set.trim().is_empty() {
                    None
                } else {
                    Some(set.as_str())
                };
                service.search_cards(&query, set_filter)
            }
        }
    });

    let refresh_cards = move |_| {
        current_page.set(0);
        search_resource.restart();
    };

    let find_by_id = {
        let service = service.clone();
        move |_| {
            let raw_id = lookup_id();
            let trimmed = raw_id.trim().to_string();

            if trimmed.is_empty() {
                lookup_status.set(Some("Enter an Arena ID to find a card.".to_string()));
                selected_card.set(None);
                return;
            }

            let Ok(arena_id) = trimmed.parse::<i64>() else {
                lookup_status.set(Some(format!("`{trimmed}` is not a valid Arena ID.")));
                selected_card.set(None);
                return;
            };

            lookup_loading.set(true);
            lookup_status.set(None);
            let service = service.clone();
            spawn(async move {
                if let Some(card) = service.get_card_by_arena_id(arena_id) {
                    lookup_status.set(Some(format!("Found {} ({})", card.name, card.set)));
                    selected_card.set(Some(card));
                } else {
                    lookup_status.set(Some(format!("No card found with Arena ID {arena_id}.")));
                    selected_card.set(None);
                }
                lookup_loading.set(false);
            });
        }
    };

    let summary_value = summary_resource.value();
    let summary_data = summary_value.read();
    let search_value = search_resource.value();
    let search_data = search_value.read();
    let filters_active = !search_query().trim().is_empty() || !set_filter().trim().is_empty();
    let summary_panel = summary_data.as_ref().cloned();
    let search_results = search_data.as_ref().cloned();

    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-6xl",
            div { class: "flex justify-between items-center mb-6",
                div {
                    h1 { class: "text-2xl font-bold text-gray-100", "Card Database" }
                    p { class: "text-gray-400 mt-1",
                        "Search the embedded Arena card database by name, set, or Arena ID."
                    }
                }
                button {
                    onclick: refresh_cards,
                    class: "bg-amber-600 hover:bg-amber-700 text-white py-2 px-4 rounded transition-colors duration-150 flex items-center",
                    disabled: search_data.is_none(),
                    if search_data.is_none() { "Loading..." } else { "Refresh Cards" }
                }
            }

            div { class: "grid grid-cols-1 md:grid-cols-2 gap-6 mb-6",
                div { class: "bg-gray-800 rounded-lg border border-gray-700 p-6",
                    h2 { class: "text-lg font-semibold text-gray-300 mb-4", "Search cards" }
                    div { class: "space-y-4",
                        div {
                            label { class: "block text-sm text-gray-400 mb-2", "Name prefix" }
                            input {
                                r#type: "text",
                                value: "{search_query}",
                                placeholder: "Try Lightning, Opt, Forest...",
                                class: "bg-gray-700 text-gray-200 border border-gray-600 rounded py-2 px-3 text-sm focus:outline-none focus:border-amber-500 w-full",
                                oninput: move |evt| {
                                    search_query.set(evt.value());
                                    current_page.set(0);
                                }
                            }
                        }
                        div {
                            label { class: "block text-sm text-gray-400 mb-2", "Set" }
                            select {
                                class: "bg-gray-700 text-gray-200 border border-gray-600 rounded py-2 px-3 text-sm focus:outline-none focus:border-amber-500 w-full",
                                onchange: move |evt| {
                                    set_filter.set(evt.value());
                                    current_page.set(0);
                                },
                                option {
                                    value: "",
                                    selected: set_filter().is_empty(),
                                    "All sets"
                                }
                                if let Some(summary) = summary_data.as_ref() {
                                    for set in summary.sets.iter() {
                                        option {
                                            value: "{set.set}",
                                            selected: set_filter() == set.set,
                                            "{set.set} ({set.count})"
                                        }
                                    }
                                }
                            }
                        }
                        p { class: "text-sm text-gray-500",
                            "Name search matches the same prefix behavior as the CLI REPL. Leave the name empty and pick a set to browse that set."
                        }
                    }
                }

                div { class: "bg-gray-800 rounded-lg border border-gray-700 p-6",
                    h2 { class: "text-lg font-semibold text-gray-300 mb-4", "Find by Arena ID" }
                    div { class: "space-y-4",
                        div {
                            label { class: "block text-sm text-gray-400 mb-2", "Arena ID" }
                            input {
                                r#type: "text",
                                value: "{lookup_id}",
                                placeholder: "Example: 91717",
                                class: "bg-gray-700 text-gray-200 border border-gray-600 rounded py-2 px-3 text-sm focus:outline-none focus:border-amber-500 w-full",
                                oninput: move |evt| lookup_id.set(evt.value())
                            }
                        }
                        button {
                            onclick: find_by_id,
                            disabled: lookup_loading(),
                            class: "bg-violet-600 hover:bg-violet-700 disabled:bg-gray-600 text-white py-2 px-4 rounded transition-colors duration-150",
                            if lookup_loading() { "Looking up..." } else { "Find Card" }
                        }
                        if let Some(status) = lookup_status() {
                            p { class: "text-sm text-gray-400", "{status}" }
                        }
                    }
                }
            }

            div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                div { class: "space-y-6",
                    SummaryPanel { summary: summary_panel }
                    SearchResults {
                        results: search_results,
                        current_page,
                        selected_card,
                        filters_active,
                    }
                }

                div {
                    if let Some(card) = selected_card() {
                        CardDetails { key: "{card.id}", card }
                    } else {
                        div { class: "bg-gray-800 rounded-lg border border-gray-700 p-12 text-center text-gray-500",
                            "Select a search result or look up an Arena ID to inspect card details."
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SummaryPanel(summary: Option<CardDatabaseSummary>) -> Element {
    rsx! {
            div { class: "bg-gray-800 rounded-lg border border-gray-700 overflow-hidden",
                div { class: "bg-gray-900 py-3 px-4 border-b border-gray-700",
                    h2 { class: "font-semibold text-gray-300", "Database info" }
                }
                match summary {
                    None => rsx! {
                        div { class: "p-6 text-center text-gray-500",
                            div { class: "animate-pulse", "Loading card database..." }
                        }
                    },
    Some(summary) => rsx! {
                        div { class: "p-6 space-y-4",
                            div { class: "grid grid-cols-2 gap-4",
                                div { class: "bg-gray-900 rounded-lg p-4",
                                    p { class: "text-sm text-gray-500", "Cards" }
                                    p { class: "text-2xl font-bold text-gray-100", "{summary.total_cards}" }
                                }
                                div { class: "bg-gray-900 rounded-lg p-4",
                                    p { class: "text-sm text-gray-500", "Sets" }
                                    p { class: "text-2xl font-bold text-gray-100", "{summary.total_sets}" }
                                }
                            }
                            div {
                                h3 { class: "text-sm font-semibold text-gray-400 mb-2", "Set counts" }
                                div { class: "max-h-48 overflow-y-auto border border-gray-700 rounded",
                                    table { class: "min-w-full table-auto",
                                        tbody {
                                            for set in summary.sets.iter() {
                                                tr { class: "border-b border-gray-700 last:border-0",
                                                    td { class: "py-2 px-3 text-gray-300 font-mono text-sm", "{set.set}" }
                                                    td { class: "py-2 px-3 text-gray-500 text-sm text-right", "{set.count}" }
                                                }
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

#[component]
fn SearchResults(
    results: Option<Vec<CardSearchResult>>,
    current_page: Signal<usize>,
    selected_card: Signal<Option<CardSearchResult>>,
    filters_active: bool,
) -> Element {
    rsx! {
            div { class: "bg-gray-800 rounded-lg border border-gray-700 overflow-hidden",
                div { class: "bg-gray-900 py-3 px-4 border-b border-gray-700",
                    h2 { class: "font-semibold text-gray-300", "Search results" }
                }
                match results {
                    None => rsx! {
                        div { class: "p-12 text-center text-gray-500",
                            div { class: "animate-pulse", "Searching cards..." }
                        }
                    },
    Some(results) => {
                        let total_items = results.len();
                        let total_pages = total_items.div_ceil(PAGE_SIZE).max(1);
                        let page = current_page().min(total_pages.saturating_sub(1));
                        let start = page * PAGE_SIZE;
                        let end = (start + PAGE_SIZE).min(total_items);

                        rsx! {
                            if results.is_empty() {
                                div { class: "p-12 text-center text-gray-500",
                                    if filters_active {
                                        "No cards matched the current filters."
                                    } else {
                                        "Enter a name prefix or choose a set to start browsing."
                                    }
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
                                                th { class: "py-3 px-4 font-semibold text-gray-400", "Card" }
                                                th { class: "py-3 px-4 font-semibold text-gray-400", "Set" }
                                                th { class: "py-3 px-4 font-semibold text-gray-400", "Type" }
                                                th { class: "py-3 px-4 font-semibold text-gray-400", "ID" }
                                            }
                                        }
                                        tbody {
                                            for card in &results[start..end] {
                                                CardResultRow {
                                                    key: "{card.id}",
                                                    card: card.clone(),
                                                    selected_card,
                                                }
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

#[component]
fn CardResultRow(card: CardSearchResult, selected_card: Signal<Option<CardSearchResult>>) -> Element {
    rsx! {
        tr {
            class: "hover:bg-gray-700/50 transition-colors duration-150 cursor-pointer",
            onclick: move |_| selected_card.set(Some(card.clone())),
            td { class: "py-3 px-4 border-b border-gray-700",
                div { class: "font-medium text-gray-100", "{card.name}" }
                if !card.mana_cost.is_empty() {
                    div { class: "mt-1",
                        ManaCost { cost: card.cost() }
                    }
                }
            }
            td { class: "py-3 px-4 border-b border-gray-700 text-amber-400 font-mono text-sm", "{card.set}" }
            td { class: "py-3 px-4 border-b border-gray-700 text-gray-400 text-sm", "{card.type_line}" }
            td { class: "py-3 px-4 border-b border-gray-700 text-gray-500 font-mono text-sm", "{card.id}" }
        }
    }
}

#[component]
fn CardDetails(card: CardSearchResult) -> Element {
    let service = use_context::<Service>();
    let mut raw_json = use_signal(|| None::<String>);
    let mut json_status = use_signal(|| None::<String>);
    let mut json_loading = use_signal(|| false);

    let color_identity = format_colors(&card.color_identity, "Colorless");
    let colors = format_colors(&card.colors, "None");
    let load_json = {
        let service = service.clone();
        let arena_id = card.id;
        move |_| {
            json_loading.set(true);
            json_status.set(None);
            raw_json.set(None);
            let service = service.clone();
            spawn(async move {
                match service.get_card_json(arena_id) {
                    Ok(Some(json)) => raw_json.set(Some(json)),
                    Ok(None) => json_status.set(Some(format!("No card found with Arena ID {arena_id}."))),
                    Err(err) => json_status.set(Some(format!("Failed to load raw JSON: {err}"))),
                }
                json_loading.set(false);
            });
        }
    };

    rsx! {
        div { class: "bg-gray-800 rounded-lg border border-gray-700 overflow-hidden",
            div { class: "bg-gray-900 py-3 px-4 border-b border-gray-700",
                h2 { class: "font-semibold text-gray-300", "Card details" }
            }
            div { class: "p-6 space-y-4",
                if !card.image_uri.is_empty() {
                    div { class: "text-center",
                        img {
                            src: "{card.image_uri}",
                            alt: "{card.name}",
                            class: "rounded-lg shadow-lg",
                            style: "max-width: 250px; width: 100%; height: auto;"
                        }
                        button {
                            onclick: {
                                let image_uri = card.image_uri.clone();
                                move |_| open_url(image_uri.clone())
                            },
                            class: "mt-2 bg-gray-700 hover:bg-gray-600 text-gray-200 py-2 px-4 rounded text-sm transition-colors duration-150",
                            "Open image"
                        }
                    }
                }

                div {
                    h3 { class: "text-2xl font-bold text-gray-100", "{card.name}" }
                    p { class: "text-gray-400 mt-1", "{card.type_line}" }
                }

                div { class: "grid grid-cols-2 gap-4",
                    DetailItem { label: "Arena ID", value: card.id.to_string() }
                    DetailItem { label: "Set", value: card.set.clone() }
                    DetailItem { label: "Mana value", value: card.mana_value.to_string() }
                    DetailItem { label: "Layout", value: if card.layout.is_empty() { "Unknown".to_string() } else { card.layout.clone() } }
                    DetailItem { label: "Colors", value: colors }
                    DetailItem { label: "Color identity", value: color_identity }
                }

                if !card.mana_cost.is_empty() {
                    div {
                        p { class: "text-sm text-gray-500 mb-2", "Mana cost" }
                        ManaCost { cost: card.cost() }
                    }
                }

                if !card.faces.is_empty() {
                    div {
                        h3 { class: "text-sm font-semibold text-gray-400 mb-2", "Card faces" }
                        div { class: "space-y-2",
                            for face in card.faces.iter() {
                                div { class: "bg-gray-900 rounded-lg p-4",
                                    p { class: "font-medium text-gray-200", "{face.name}" }
                                    p { class: "text-sm text-gray-500", "{face.type_line}" }
                                    if !face.mana_cost.is_empty() {
                                        p { class: "text-sm text-gray-400 mt-1", "{face.mana_cost}" }
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    button {
                        onclick: load_json,
                        disabled: json_loading(),
                        class: "bg-amber-600 hover:bg-amber-700 disabled:bg-gray-600 text-white py-2 px-4 rounded transition-colors duration-150",
                        if json_loading() { "Loading JSON..." } else { "Show raw JSON" }
                    }
                    if let Some(status) = json_status() {
                        p { class: "text-sm text-gray-400 mt-2", "{status}" }
                    }
                    if let Some(json) = raw_json() {
                        textarea {
                            readonly: true,
                            class: "border border-gray-600 rounded-md bg-gray-900 font-mono text-sm leading-relaxed text-gray-200 focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-amber-500 w-full h-96 p-4 mt-4",
                            value: "{json}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DetailItem(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "bg-gray-900 rounded-lg p-4",
            p { class: "text-sm text-gray-500", "{label}" }
            p { class: "text-gray-200", "{value}" }
        }
    }
}
