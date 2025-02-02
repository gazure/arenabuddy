use crate::app::invoke;
use arenabuddy_core::models::mtga_match::MTGAMatch;
use leptos::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

async fn retrieve_matches() -> Vec<MTGAMatch> {
    serde_wasm_bindgen::from_value(invoke("command_matches", JsValue::null()).await)
        .unwrap_or_default()
}

// Component for Matches page
#[component]
pub(crate) fn Matches() -> impl IntoView {
    let (length, set_length) = signal(0usize);
    let (matches, set_matches) = signal(Vec::<MTGAMatch>::new());
    let load = move || {
        spawn_local(async move {
            let m = retrieve_matches().await;
            set_length.set(m.len());
            set_matches.set(m);
        });
    };
    load();
    view! {
        <div>
            <h1>"Matches Page"</h1>
            <button on:click=move |_| load() >refresh</button>
            <p>{move || length.get()}</p>
            {move || {
                matches.get().into_iter().map(|m| view! { <div>{m.id}</div> }).collect::<Vec<_>>()
                }}
        // Add your matches page content here
        </div>
    }
}
