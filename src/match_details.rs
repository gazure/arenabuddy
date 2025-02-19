use crate::app::invoke;
use arenabuddy_core::display::match_details::MatchDetails;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use wasm_bindgen_futures::spawn_local;

async fn get_match_details(id: &str) -> Option<MatchDetails> {
    // Build the object we want to send to Tauri
    leptos::logging::log!("hello command");
    let command_object =
        serde_wasm_bindgen::to_value(&serde_json::json!({ "matchId": id })).unwrap();
    serde_wasm_bindgen::from_value(invoke("command_match_details", command_object).await).ok()
}

#[component]
pub(crate) fn MatchDetails() -> impl IntoView {
    leptos::logging::log!("hello component");
    let params = use_params_map();

    let (match_details, set_match_details) = signal(MatchDetails::default());
    let (debug, set_debug) = signal(String::new());
    let load = move || {
        let id = params.with(|params| params.get("id").unwrap_or_default());
        leptos::logging::log!("id: {}", id);
        spawn_local(async move {
            if let Some(m) = get_match_details(&id).await {
                set_match_details.set(m);
            } else {
                set_debug.set("no match details found".to_owned());
            }
            set_debug.set("hello".to_owned());
        });
    };
    load();
    view! {
        <div>
            <h2>{"Match Details"}</h2>
            <button on:click=move |_| load()>refresh</button>
            <p>{"Match ID: "}{move || match_details.get().id}</p>
            <p>{"decklist: "}{move ||
                match_details.get().primary_decklist.map(|pd| {
                            pd.main_deck
                                .values()
                                .flat_map(|cdrs| cdrs.iter().map(|cdr| cdr.name.clone()))
                                .collect::<Vec<_>>()
                                .join("\n")
                        })

                }</p>
            <p>debug: {move || debug.get()}</p>
        </div>
    }
}
