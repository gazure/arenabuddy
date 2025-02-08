use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub(crate) fn MatchDetails() -> impl IntoView {
    let params = use_params_map();
    let id = params.with(|params| params.get("id").unwrap_or_default());
    view! {
        <div>
            <h2>{"Match Details"}</h2>
            <p>{"Match ID: "}{id}</p>
        </div>
    }
}
