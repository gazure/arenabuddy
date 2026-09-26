use arenabuddy_core::semantic_search::{SearchOptions, SearchResults};
use dioxus::{core::Task, prelude::*};

use super::{cards::CardDetails, components::ManaCost, pages::Route};
use crate::backend::{BackgroundRuntime, CardSearchResult, Service, semantic_search};

/// Renders an explicitly submitted Jev search using the saved desktop credential.
#[component]
pub(super) fn SemanticSearch() -> Element {
    let runtime = use_context::<BackgroundRuntime>();
    let service = use_context::<Service>();
    let mut query = use_signal(String::new);
    let mut format = use_signal(String::new);
    let mut candidates = use_signal(|| 60u16);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut results = use_signal(|| None::<SearchResults>);
    let mut submitted_query = use_signal(String::new);
    let mut selected = use_signal(|| None::<CardSearchResult>);
    let mut running = use_signal(|| None::<Task>);

    let submit = move |event: FormEvent| {
        event.prevent_default();
        if busy() || query.read().trim().is_empty() {
            return;
        }
        let mut options = SearchOptions::for_query(query.read().trim().to_string());
        options.candidates = candidates();
        options.limit = 25;
        let selected_format = format();
        options.format = (!selected_format.is_empty()).then_some(selected_format);
        submitted_query.set(options.query.clone());
        busy.set(true);
        error.set(None);
        results.set(None);
        selected.set(None);
        let runtime = runtime.clone();
        let cards = service.cards.clone();
        let task = spawn(async move {
            match semantic_search::search(runtime, cards, options).await {
                Ok(found) => results.set(Some(found)),
                Err(message) => error.set(Some(message)),
            }
            busy.set(false);
            running.set(None);
        });
        running.set(Some(task));
    };

    rsx! {
        div { class: "space-y-6",
            form {
                onsubmit: submit,
                class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6 space-y-4",
                h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100", "Search by meaning" }
                p { class: "text-gray-600 dark:text-gray-400",
                    "Describe card abilities, types, stats, or strategy. Jev searches card text and stats; it cannot inspect artwork."
                }
                label { r#for: "jev-query", class: "block text-gray-900 dark:text-gray-100", "What are you looking for?" }
                input {
                    id: "jev-query", r#type: "text", value: "{query}", disabled: busy(),
                    placeholder: "Cheap creatures that reward casting spells",
                    class: "w-full p-2 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100",
                    oninput: move |event| query.set(event.value()),
                }
                div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                    div {
                        label { r#for: "jev-format", class: "block text-gray-900 dark:text-gray-100", "Format override" }
                        select {
                            id: "jev-format", value: "{format}", disabled: busy(),
                            class: "w-full p-2 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100",
                            onchange: move |event| format.set(event.value()),
                            option { value: "", "Use the query" }
                            for (value, label) in [("standard", "Standard"), ("alchemy", "Alchemy"), ("historic", "Historic"), ("timeless", "Timeless"), ("brawl", "Brawl"), ("standardbrawl", "Standard Brawl"), ("gladiator", "Gladiator"), ("pioneer", "Pioneer")] {
                                option { value, "{label}" }
                            }
                        }
                    }
                    div {
                        label { r#for: "jev-candidates", class: "block text-gray-900 dark:text-gray-100", "Maximum cards to rank" }
                        select {
                            id: "jev-candidates", value: "{candidates}", disabled: busy(),
                            class: "w-full p-2 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100",
                            onchange: move |event| { if let Ok(value) = event.value().parse() { candidates.set(value); } },
                            for value in [30, 60, 100, 200, 500] { option { value: "{value}", "{value}" } }
                        }
                    }
                }
                p { class: "text-sm text-gray-500",
                    "Searching sends your query and candidate card rules to TypeSafe using your saved key. More candidates can take longer and use more tokens."
                }
                div { class: "flex items-center space-x-3",
                    button {
                        r#type: "submit", disabled: busy() || query.read().trim().is_empty(),
                        class: "bg-violet-600 hover:bg-violet-700 disabled:bg-gray-600 text-white py-2 px-4 rounded",
                        if busy() { "Searching…" } else { "Search with Jev" }
                    }
                    if busy() {
                        button {
                            r#type: "button", class: "text-gray-900 dark:text-gray-100 px-3 py-1 rounded",
                            onclick: move |_| {
                                if let Some(task) = running.take() { task.cancel(); }
                                busy.set(false);
                                error.set(Some("Search canceled. Requests already sent may still use tokens.".into()));
                            },
                            "Cancel"
                        }
                    }
                    Link { to: Route::Settings {}, class: "text-amber-600 dark:text-amber-400", "API key settings" }
                }
                if busy() { p { role: "status", class: "text-gray-600 dark:text-gray-400", "Interpreting your query and finding matching cards…" } }
                if let Some(message) = error() { p { role: "alert", class: "text-red-600 dark:text-red-400", "{message}" } }
            }
            if let Some(found) = results.read().as_ref() {
                div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                    div { class: "space-y-4",
                        div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6 space-y-4",
                            h2 { class: "font-semibold text-gray-900 dark:text-gray-100", "Results for “{submitted_query}”" }
                            p { class: "text-sm text-gray-600 dark:text-gray-400",
                                if found.filter_summary.is_empty() { "No structured filters applied." }
                                else { {found.filter_summary.join(" · ")} }
                            }
                            p { class: "text-sm text-gray-600 dark:text-gray-400",
                                if found.ranked {
                                    "Ranked {found.ranked_count} of {found.matching_count} cards matching the filters. Showing up to 25 matches with relevance ≥ 2/3. Relevance is not a probability."
                                } else {
                                    "{found.matching_count} cards match the filters. Showing up to 25 in name order."
                                }
                            }
                            for notice in &found.notices { p { class: "text-sm text-amber-600 dark:text-amber-400", "{notice}" } }
                            p { class: "text-sm text-gray-500", "Usage: {found.usage.input_tokens} input tokens · {found.usage.output_tokens} output tokens" }
                        }
                        if found.matches.is_empty() {
                            p { role: "status", class: "text-gray-600 dark:text-gray-400", "No strong matches found. Try broader wording, fewer constraints, or more candidates." }
                        }
                        for item in &found.matches {
                            {
                                let card = CardSearchResult::from(&item.card);
                                let cost = card.cost();
                                rsx! {
                                    button {
                                        key: "{card.id}",
                                        class: "w-full text-left bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4",
                                        onclick: move |_| selected.set(Some(card.clone())),
                                        div { class: "font-semibold text-gray-900 dark:text-gray-100", "{item.card.name}" }
                                        ManaCost { cost }
                                        p { class: "text-sm text-gray-600 dark:text-gray-400", "{item.card.type_line}" }
                                        if let Some(score) = item.score { p { class: "text-sm text-amber-600 dark:text-amber-400", "Relevance: {score:.2}/3" } }
                                    }
                                }
                            }
                        }
                    }
                    div {
                        if let Some(card) = selected() { CardDetails { key: "{card.id}", card } }
                        else { p { class: "text-gray-500", "Select a card to inspect its rules, faces, and legality." } }
                    }
                }
            }
        }
    }
}
