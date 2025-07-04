use arenabuddy_core::models::Cost;
use leptos::prelude::*;

#[component]
pub fn ManaCost(cost: Cost) -> impl IntoView {
    view! {
        <div class="flex items-center">
            {cost
                .into_iter()
                .map(|symbol| {
                    let svg = format!("/public/mana/{}", symbol.svg_file());
                    view! { <img src=svg alt=symbol.to_string() class="w-4 h-4" /> }
                })
                .collect_view()}

        </div>
    }
}
